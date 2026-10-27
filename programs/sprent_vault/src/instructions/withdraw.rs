use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::state::*;

#[derive(Accounts)]
pub struct OwnerPosition<'info> {
    pub owner: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(
        mut,
        seeds = [DEPOSITOR_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump = depositor.bump,
        has_one = owner @ SprentError::DepositorOwnerMismatch
    )]
    pub depositor: Box<Account<'info, Depositor>>,
}

#[derive(Accounts)]
pub struct ClaimWithdraw<'info> {
    pub owner: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(
        mut,
        seeds = [DEPOSITOR_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump = depositor.bump,
        has_one = owner @ SprentError::DepositorOwnerMismatch
    )]
    pub depositor: Box<Account<'info, Depositor>>,
    #[account(address = config.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,

    #[account(mut, token::mint = quote_mint, token::authority = owner)]
    pub owner_quote: Box<Account<'info, TokenAccount>>,
    /// CHECK: pinned vault authority PDA.
    #[account(address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, address = config.vault_quote_ata @ SprentError::AccountSubstitution)]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

pub(crate) fn request_withdraw(ctx: Context<OwnerPosition>, shares: u128) -> Result<()> {
    let d = &mut ctx.accounts.depositor;
    require!(
        shares > 0 && shares <= d.shares,
        SprentError::InvalidWithdrawAmount
    );
    d.withdraw_request_shares = shares;
    d.withdraw_request_ts = Clock::get()?.unix_timestamp;
    emit!(WithdrawRequested {
        owner: d.owner,
        shares,
        epoch: ctx.accounts.config.current_epoch
    });
    Ok(())
}

pub(crate) fn claim_withdraw(ctx: Context<ClaimWithdraw>) -> Result<()> {
    let c = &ctx.accounts.config;
    let d = &ctx.accounts.depositor;
    require!(d.claimable_quote > 0, SprentError::NothingToClaim);
    let available = c
        .tracked_idle_quote
        .saturating_sub(c.pending_deposits_quote);
    let pay = d.claimable_quote.min(available);
    require!(pay > 0, SprentError::InsufficientIdle);
    vault_transfer_quote(
        &ctx.accounts.token_program,
        &ctx.accounts.vault_quote_ata,
        &ctx.accounts.quote_mint,
        &ctx.accounts.owner_quote.to_account_info(),
        &ctx.accounts.vault_authority,
        c,
        pay,
    )?;
    let c = &mut ctx.accounts.config;
    let d = &mut ctx.accounts.depositor;
    d.claimable_quote -= pay;
    d.claimed_quote_total = d
        .claimed_quote_total
        .checked_add(pay as u128)
        .ok_or(SprentError::MathOverflow)?;
    c.payable_withdrawals_quote = c
        .payable_withdrawals_quote
        .checked_sub(pay)
        .ok_or(SprentError::MathOverflow)?;
    c.tracked_idle_quote = c
        .tracked_idle_quote
        .checked_sub(pay)
        .ok_or(SprentError::MathOverflow)?;
    emit!(WithdrawClaimed {
        owner: d.owner,
        amount: pay,
        remaining: d.claimable_quote
    });
    Ok(())
}
