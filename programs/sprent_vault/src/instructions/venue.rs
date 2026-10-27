use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use anchor_spl::token_interface::{TokenAccount as ITokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::manifest::*;
use crate::state::*;

#[derive(Accounts)]
pub struct VenueTransfer<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: pinned vault authority PDA.
    #[account(mut, address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: pinned market.
    #[account(mut, address = config.manifest_market @ SprentError::AccountSubstitution)]
    pub market: UncheckedAccount<'info>,
    #[account(mut, address = config.vault_base_ata @ SprentError::AccountSubstitution)]
    pub vault_base_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(mut, address = config.vault_quote_ata @ SprentError::AccountSubstitution)]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: pinned Manifest base vault.
    #[account(mut, address = config.manifest_base_vault @ SprentError::AccountSubstitution)]
    pub manifest_base_vault: UncheckedAccount<'info>,
    /// CHECK: pinned Manifest quote vault.
    #[account(mut, address = config.manifest_quote_vault @ SprentError::AccountSubstitution)]
    pub manifest_quote_vault: UncheckedAccount<'info>,
    /// CHECK: pinned base mint.
    #[account(address = config.base_mint @ SprentError::AccountSubstitution)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: pinned quote mint.
    #[account(address = config.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: UncheckedAccount<'info>,
    #[account(address = config.base_token_program @ SprentError::AccountSubstitution)]
    pub base_token_program: Interface<'info, TokenInterface>,
    #[account(address = config.quote_token_program @ SprentError::AccountSubstitution)]
    pub token_program: Program<'info, Token>,
    /// CHECK: fixed Manifest program id.
    #[account(address = MANIFEST_PROGRAM_ID @ SprentError::ManifestProgramMismatch)]
    pub manifest_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn fund_venue(
    ctx: Context<VenueTransfer>,
    base_amount: u64,
    quote_amount: u64,
) -> Result<()> {
    let c = &ctx.accounts.config;
    let signer = ctx.accounts.authority.key();

    let halted = c.state == vault_state::HALTED;
    require!(
        signer == c.operator || (halted && signer == c.guardian),
        SprentError::Unauthorized
    );
    require!(
        c.state == vault_state::RUNNING || (halted && quote_amount == 0),
        SprentError::VaultNotRunning
    );
    require!(base_amount > 0 || quote_amount > 0, SprentError::ZeroAmount);

    if quote_amount > 0 {
        let idle = c.tracked_idle_quote;
        let reserved = reserved_idle_quote(c)?;
        require!(
            idle >= quote_amount && idle - quote_amount >= reserved,
            SprentError::ReserveBreached
        );
        let venue_now = c.venue_quote_net_funded.max(0) as u128;
        let venue_after = venue_now + quote_amount as u128;
        let total = (idle - reserved) as u128 + venue_now;
        require!(
            venue_after * BPS_DENOMINATOR as u128 <= total * c.params.working_capital_bps as u128,
            SprentError::WorkingCapitalExceeded
        );
    }
    if base_amount > 0 {
        require!(
            c.tracked_idle_base >= base_amount,
            SprentError::ReserveBreached
        );
    }

    if base_amount > 0 {
        venue_move(&ctx, TAG_DEPOSIT, true, base_amount)?;
    }
    if quote_amount > 0 {
        venue_move(&ctx, TAG_DEPOSIT, false, quote_amount)?;
    }
    let c = &mut ctx.accounts.config;
    c.tracked_idle_base -= base_amount;
    c.tracked_idle_quote -= quote_amount;
    c.venue_base_net_funded = c
        .venue_base_net_funded
        .checked_add(base_amount as i64)
        .ok_or(SprentError::MathOverflow)?;
    c.venue_quote_net_funded = c
        .venue_quote_net_funded
        .checked_add(quote_amount as i64)
        .ok_or(SprentError::MathOverflow)?;
    emit!(VenueFunded {
        base_amount,
        quote_amount
    });
    Ok(())
}

pub(crate) fn defund_venue(
    mut ctx: Context<VenueTransfer>,
    base_amount: u64,
    quote_amount: u64,
) -> Result<()> {
    let c = &ctx.accounts.config;
    let signer = ctx.accounts.authority.key();
    require!(
        signer == c.operator || signer == c.guardian,
        SprentError::Unauthorized
    );
    require!(base_amount > 0 || quote_amount > 0, SprentError::ZeroAmount);
    withdraw_verified(&mut ctx, base_amount, quote_amount)?;
    emit!(VenueDefunded {
        authority: signer,
        base_amount,
        quote_amount
    });
    Ok(())
}

pub(crate) fn emergency_cancel(mut ctx: Context<VenueTransfer>) -> Result<()> {
    let c = &ctx.accounts.config;
    require_keys_eq!(
        ctx.accounts.authority.key(),
        c.guardian,
        SprentError::Unauthorized
    );
    let slot = Clock::get()?.slot;
    let v = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        slot,
    )?;
    let mut cancelled = 0u8;
    if !v.listed().is_empty() {
        cancelled = cancel_listed(
            v.listed(),
            c,
            &ctx.accounts.vault_authority.to_account_info(),
            &ctx.accounts.market.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            &ctx.accounts.manifest_program.to_account_info(),
        )?;
    }
    let after = read_venue(
        &ctx.accounts.market.to_account_info(),
        &ctx.accounts.config.vault_authority,
        slot,
    )?;
    require!(after.vault_orders == 0, SprentError::OpenOrdersPresent);
    let (b, q) = (after.seat_base, after.seat_quote);
    if b > 0 || q > 0 {
        withdraw_verified(&mut ctx, b, q)?;
    }
    let c = &mut ctx.accounts.config;
    c.state = vault_state::HALTED;
    note_inventory_payout(c, 0);
    emit!(EmergencyCancelled {
        guardian: c.guardian,
        cancelled,
        base_returned: b,
        quote_returned: q
    });
    Ok(())
}

pub(crate) fn defund_for_payouts(mut ctx: Context<VenueTransfer>) -> Result<()> {
    let c = &ctx.accounts.config;
    let v = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        Clock::get()?.slot,
    )?;
    let owed = c
        .payable_withdrawals_quote
        .checked_add(c.fee_payable_quote)
        .ok_or(SprentError::MathOverflow)?;
    let idle = c
        .tracked_idle_quote
        .saturating_sub(c.pending_deposits_quote);
    let short = owed.saturating_sub(idle);
    let amount = short.min(v.seat_quote);
    require!(amount > 0, SprentError::NoPayoutShortfall);
    withdraw_verified(&mut ctx, 0, amount)?;
    let seat_left = v.seat_quote - amount;
    note_inventory_payout(&mut ctx.accounts.config, seat_left);
    emit!(VenueDefunded {
        authority: ctx.accounts.authority.key(),
        base_amount: 0,
        quote_amount: amount
    });
    Ok(())
}

fn reserved_idle_quote(c: &VaultConfig) -> Result<u64> {
    c.pending_deposits_quote
        .checked_add(c.payable_withdrawals_quote)
        .and_then(|x| x.checked_add(c.fee_payable_quote))
        .ok_or_else(|| error!(SprentError::MathOverflow))
}

fn withdraw_verified(
    ctx: &mut Context<VenueTransfer>,
    base_amount: u64,
    quote_amount: u64,
) -> Result<()> {
    let base_before = ctx.accounts.vault_base_ata.amount;
    let quote_before = ctx.accounts.vault_quote_ata.amount;
    if base_amount > 0 {
        venue_move(ctx, TAG_WITHDRAW, true, base_amount)?;
    }
    if quote_amount > 0 {
        venue_move(ctx, TAG_WITHDRAW, false, quote_amount)?;
    }
    ctx.accounts.vault_base_ata.reload()?;
    ctx.accounts.vault_quote_ata.reload()?;
    require!(
        ctx.accounts.vault_base_ata.amount
            == base_before
                .checked_add(base_amount)
                .ok_or(SprentError::MathOverflow)?
            && ctx.accounts.vault_quote_ata.amount
                == quote_before
                    .checked_add(quote_amount)
                    .ok_or(SprentError::MathOverflow)?,
        SprentError::WithdrawDeltaMismatch
    );
    let c = &mut ctx.accounts.config;
    c.tracked_idle_base = c
        .tracked_idle_base
        .checked_add(base_amount)
        .ok_or(SprentError::MathOverflow)?;
    c.tracked_idle_quote = c
        .tracked_idle_quote
        .checked_add(quote_amount)
        .ok_or(SprentError::MathOverflow)?;
    c.venue_base_net_funded = c
        .venue_base_net_funded
        .checked_sub(base_amount as i64)
        .ok_or(SprentError::MathOverflow)?;
    c.venue_quote_net_funded = c
        .venue_quote_net_funded
        .checked_sub(quote_amount as i64)
        .ok_or(SprentError::MathOverflow)?;
    Ok(())
}

fn venue_move(ctx: &Context<VenueTransfer>, tag: u8, is_base: bool, amount: u64) -> Result<()> {
    let c = &ctx.accounts.config;
    let a = &ctx.accounts;
    let (ata, mvault, mint, tp) = if is_base {
        (
            a.vault_base_ata.to_account_info(),
            a.manifest_base_vault.to_account_info(),
            a.base_mint.to_account_info(),
            a.base_token_program.to_account_info(),
        )
    } else {
        (
            a.vault_quote_ata.to_account_info(),
            a.manifest_quote_vault.to_account_info(),
            a.quote_mint.to_account_info(),
            a.token_program.to_account_info(),
        )
    };
    let ix = token_move_ix(
        tag,
        c.vault_authority,
        c.manifest_market,
        ata.key(),
        mvault.key(),
        tp.key(),
        mint.key(),
        amount,
    )?;
    invoke_as_vault(
        &ix,
        &[
            a.vault_authority.to_account_info(),
            a.market.to_account_info(),
            ata,
            mvault,
            tp,
            mint,
            a.manifest_program.to_account_info(),
        ],
        &c.key(),
        c.vault_authority_bump,
    )
}
