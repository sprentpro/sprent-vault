use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

use crate::accounting::{self};
use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::state::*;

#[derive(Accounts)]
pub struct DepositQuote<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Depositor::INIT_SPACE,
        seeds = [DEPOSITOR_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump
    )]
    pub depositor: Box<Account<'info, Depositor>>,
    #[account(address = config.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(mut, token::mint = quote_mint, token::authority = owner)]
    pub owner_quote: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = config.vault_quote_ata @ SprentError::AccountSubstitution)]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn deposit(ctx: Context<DepositQuote>, amount: u64) -> Result<()> {
    require!(amount > 0, SprentError::ZeroAmount);
    let owner = ctx.accounts.owner.key();
    let config = &mut ctx.accounts.config;
    require!(
        config.state == vault_state::RUNNING,
        SprentError::VaultNotRunning
    );
    require!(
        amount >= config.params.min_deposit_quote,
        SprentError::DepositTooSmall
    );

    let d = &mut ctx.accounts.depositor;
    if d.owner == Pubkey::default() {
        d.owner = owner;
        d.bump = ctx.bumps.depositor;
    }
    require_keys_eq!(d.owner, owner, SprentError::DepositorOwnerMismatch);

    let user_after = d
        .net_deposited_quote
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;
    let total_after = config
        .total_net_deposited_quote
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;
    require!(
        user_after <= config.params.deposit_cap_per_user_quote
            && total_after <= config.params.deposit_cap_total_quote,
        SprentError::DepositCapExceeded
    );

    let pending_after = d
        .pending_deposit_quote
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;
    let first_mint = config.total_shares == 0;
    let last = accounting::EpochPrice {
        nav: config.last_epoch_nav,
        shares: config.last_epoch_shares,
        pps: config.last_epoch_pps,
    };
    accounting::mint_shares(
        pending_after,
        &last,
        if first_mint { 0 } else { config.total_shares },
    )
    .map_err(acct_err)?;

    token::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.owner_quote.to_account_info(),
                mint: ctx.accounts.quote_mint.to_account_info(),
                to: ctx.accounts.vault_quote_ata.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.quote_mint.decimals,
    )?;

    let now = Clock::get()?.unix_timestamp;
    d.desk = config.key();
    if !d.active {
        d.active = true;
        d.next_settle_epoch = config.current_epoch;
        config.open_positions = config
            .open_positions
            .checked_add(1)
            .ok_or(SprentError::MathOverflow)?;
    }
    d.net_deposited_quote = user_after;
    d.cumulative_deposited_quote = d
        .cumulative_deposited_quote
        .checked_add(amount as u128)
        .ok_or(SprentError::MathOverflow)?;
    d.pending_deposit_quote = pending_after;
    d.pending_deposit_ts = now;
    config.total_net_deposited_quote = total_after;
    config.pending_deposits_quote = config
        .pending_deposits_quote
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;
    config.tracked_idle_quote = config
        .tracked_idle_quote
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;

    emit!(Deposited {
        owner,
        amount,
        epoch: config.current_epoch,
        pending_total: d.pending_deposit_quote
    });
    Ok(())
}
