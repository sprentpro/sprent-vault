use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::state::*;

#[derive(Accounts)]
pub struct SweepFees<'info> {
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(address = config.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: pinned vault authority PDA.
    #[account(address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, address = config.vault_quote_ata @ SprentError::AccountSubstitution)]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = config.treasury_s_quote_ata @ SprentError::AccountSubstitution)]
    pub treasury_s_quote_ata: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

pub(crate) fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
    let c = &ctx.accounts.config;
    require!(c.fee_payable_quote > 0, SprentError::NothingToSweep);
    let available = c
        .tracked_idle_quote
        .saturating_sub(c.pending_deposits_quote)
        .saturating_sub(c.payable_withdrawals_quote);
    let pay = c.fee_payable_quote.min(available);
    require!(pay > 0, SprentError::InsufficientIdle);
    vault_transfer_quote(
        &ctx.accounts.token_program,
        &ctx.accounts.vault_quote_ata,
        &ctx.accounts.quote_mint,
        &ctx.accounts.treasury_s_quote_ata.to_account_info(),
        &ctx.accounts.vault_authority,
        c,
        pay,
    )?;
    let c = &mut ctx.accounts.config;
    c.fee_payable_quote -= pay;
    c.tracked_idle_quote = c
        .tracked_idle_quote
        .checked_sub(pay)
        .ok_or(SprentError::MathOverflow)?;
    c.fees_swept_total = c
        .fees_swept_total
        .checked_add(pay as u128)
        .ok_or(SprentError::MathOverflow)?;
    emit!(FeesSwept {
        amount: pay,
        remaining: c.fee_payable_quote
    });
    Ok(())
}
