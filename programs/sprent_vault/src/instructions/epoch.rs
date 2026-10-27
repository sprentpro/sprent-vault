use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use anchor_spl::token_interface::TokenAccount as ITokenAccount;

use crate::accounting::{self, AcctError, NavInputs};
use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::manifest::*;
use crate::oracle;
use crate::state::*;

#[derive(Accounts)]
pub struct CloseEpoch<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + EpochNav::INIT_SPACE,
        seeds = [EPOCH_SEED, config.key().as_ref(), config.current_epoch.to_le_bytes().as_ref()],
        bump
    )]
    pub epoch_nav: Box<Account<'info, EpochNav>>,
    /// CHECK: pinned market; read-only view in the handler.
    #[account(address = config.manifest_market @ SprentError::AccountSubstitution)]
    pub market: UncheckedAccount<'info>,
    /// CHECK: pinned oracle; layout verified in the handler.
    #[account(address = config.oracle @ SprentError::AccountSubstitution)]
    pub oracle: UncheckedAccount<'info>,
    #[account(address = config.vault_base_ata @ SprentError::AccountSubstitution)]
    pub vault_base_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(address = config.vault_quote_ata @ SprentError::AccountSubstitution)]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SettlePosition<'info> {
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(seeds = [EPOCH_SEED, config.key().as_ref(), depositor.next_settle_epoch.to_le_bytes().as_ref()], bump = epoch_nav.bump)]
    pub epoch_nav: Box<Account<'info, EpochNav>>,
    #[account(mut, seeds = [DEPOSITOR_SEED, config.key().as_ref(), depositor.owner.as_ref()], bump = depositor.bump)]
    pub depositor: Box<Account<'info, Depositor>>,
}

pub(crate) fn close_epoch(ctx: Context<CloseEpoch>) -> Result<()> {
    let signer = ctx.accounts.authority.key();
    let c = &ctx.accounts.config;
    require!(
        signer == c.operator || signer == c.admin || signer == c.guardian,
        SprentError::Unauthorized
    );
    require!(
        c.state != vault_state::RECONCILE_FROZEN,
        SprentError::VaultFrozen
    );
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;
    require!(
        now >= c.last_close_ts.saturating_add(c.params.epoch_duration_s),
        SprentError::EpochNotElapsed
    );
    require!(
        c.positions_settled >= c.positions_to_settle,
        SprentError::CrankIncomplete
    );
    require!(
        ctx.accounts.epoch_nav.status != epoch_status::FINAL,
        SprentError::EpochAlreadyClosed
    );

    let venue = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        clock.slot,
    )?;
    require!(venue.vault_orders == 0, SprentError::OpenOrdersPresent);
    let px = oracle::read_oracle(
        c.oracle_mode,
        &ctx.accounts.oracle.to_account_info(),
        &c.oracle_feed_id,
        now,
        c.params.oracle_max_age_s,
        c.params.oracle_max_conf_bps,
        c.base_mint_decimals,
        c.quote_mint_decimals,
    )?;
    let mut reason = px.freeze;
    let price_fp: u128 = px.price_fp;
    let mut mid: u128 = 0;
    if let (Some(b), Some(a)) = (venue.best_bid_fp, venue.best_ask_fp) {
        mid = (b / 2)
            .checked_add(a / 2)
            .ok_or(SprentError::MathOverflow)?;
        if reason == freeze_reason::NONE
            && !oracle::within_band(mid, price_fp, c.params.nav_mid_band_bps)?
        {
            reason = freeze_reason::BOOK_DEVIATION;
        }
    }
    let ata_base = ctx.accounts.vault_base_ata.amount;
    let ata_quote = ctx.accounts.vault_quote_ata.amount;
    if reason == freeze_reason::NONE
        && (ata_base < c.tracked_idle_base || ata_quote < c.tracked_idle_quote)
    {
        reason = freeze_reason::RECONCILE;
    }
    let nav = accounting::nav_quote(&NavInputs {
        tracked_idle_quote: c.tracked_idle_quote,
        tracked_idle_base: c.tracked_idle_base,
        seat_quote: venue.seat_quote,
        seat_base: venue.seat_base,
        pending_deposits_quote: c.pending_deposits_quote,
        payable_withdrawals_quote: c.payable_withdrawals_quote,
        fee_payable_quote: c.fee_payable_quote,
        price_fp,
    })
    .map_err(acct_err)?;
    if reason == freeze_reason::NONE && nav < 0 {
        reason = freeze_reason::NEGATIVE_NAV;
    }
    let nav_u = nav.max(0) as u128;

    let epoch = c.current_epoch;
    let e = &mut ctx.accounts.epoch_nav;
    e.epoch = epoch;
    e.bump = ctx.bumps.epoch_nav;
    e.freeze_reason = reason;
    e.close_ts = now;
    e.slot = clock.slot;
    e.oracle_price = px.price;
    e.oracle_conf = px.conf;
    e.oracle_expo = px.expo;
    e.oracle_publish_time = px.publish_time;
    e.price_fp = price_fp;
    e.book_mid_fp = mid;
    e.seat_base = venue.seat_base;
    e.seat_quote = venue.seat_quote;
    e.tracked_idle_base = c.tracked_idle_base;
    e.tracked_idle_quote = c.tracked_idle_quote;
    e.ata_base = ata_base;
    e.ata_quote = ata_quote;
    e.pending_deposits_quote = c.pending_deposits_quote;
    e.payable_withdrawals_quote = c.payable_withdrawals_quote;
    e.fee_payable_quote = c.fee_payable_quote;
    e.nav_quote = nav_u;
    e.total_shares = c.total_shares;

    let c = &mut ctx.accounts.config;
    if reason != freeze_reason::NONE {
        e.status = epoch_status::FROZEN_ATTEMPT;
        c.state = vault_state::RECONCILE_FROZEN;
        emit!(EpochFrozen {
            epoch,
            reason,
            nav_quote: nav,
            price_fp,
            book_mid_fp: mid
        });
        return Ok(());
    }
    let pps = accounting::pps_of(nav_u, c.total_shares, c.last_epoch_pps).map_err(acct_err)?;
    e.pps = pps;
    e.positions_to_settle = c.open_positions;
    e.status = epoch_status::FINAL;
    c.last_epoch_pps = pps;
    c.day_ref_pps = pps;
    if pps > c.peak_pps {
        c.peak_pps = pps;
    }
    if c.total_shares > 0
        && pps * (BPS_DENOMINATOR as u128)
            < c.peak_pps * ((BPS_DENOMINATOR - c.params.loss_hard_bps as u64) as u128)
    {
        c.state = vault_state::HALTED;
        emit!(LossLimitHalted {
            pps_live: pps,
            peak_pps: c.peak_pps,
            cancelled: 0
        });
    }
    c.last_epoch_nav = nav_u;
    c.last_epoch_shares = c.total_shares;
    c.last_close_ts = now;
    c.current_epoch = epoch.checked_add(1).ok_or(SprentError::MathOverflow)?;
    c.positions_to_settle = c.open_positions;
    c.positions_settled = 0;
    note_inventory_payout(c, venue.seat_quote);
    emit!(EpochClosed {
        epoch,
        nav_quote: nav_u,
        total_shares: c.total_shares,
        pps,
        price_fp,
        seat_base: venue.seat_base,
        seat_quote: venue.seat_quote,
        surplus_quote: ata_quote - c.tracked_idle_quote,
        surplus_base: ata_base - c.tracked_idle_base,
        positions_to_settle: c.positions_to_settle,
    });
    Ok(())
}

pub(crate) fn settle_position(ctx: Context<SettlePosition>) -> Result<()> {
    let e = &ctx.accounts.epoch_nav;
    let c = &mut ctx.accounts.config;
    let d = &mut ctx.accounts.depositor;
    require!(e.status == epoch_status::FINAL, SprentError::EpochNotFinal);
    require!(
        d.active
            && d.next_settle_epoch == e.epoch
            && e.epoch.checked_add(1) == Some(c.current_epoch),
        SprentError::NothingToSettle
    );
    let p = e.pps;
    let ep = accounting::EpochPrice {
        nav: e.nav_quote,
        shares: e.total_shares,
        pps: e.pps,
    };

    let cr =
        accounting::crystallize(d.shares, d.hwm_pps, p, c.params.perf_fee_bps).map_err(acct_err)?;
    d.shares = cr.shares_after;
    d.hwm_pps = cr.hwm_after;
    c.total_shares = c
        .total_shares
        .checked_sub(cr.fee_shares)
        .ok_or(SprentError::MathOverflow)?;
    c.fee_payable_quote = c
        .fee_payable_quote
        .checked_add(cr.fee_quote)
        .ok_or(SprentError::MathOverflow)?;
    c.fees_crystallized_total = c
        .fees_crystallized_total
        .checked_add(cr.fee_quote as u128)
        .ok_or(SprentError::MathOverflow)?;
    d.fees_paid_quote = d
        .fees_paid_quote
        .checked_add(cr.fee_quote as u128)
        .ok_or(SprentError::MathOverflow)?;

    let mut redeemed: u64 = 0;
    let mut redeemed_shares: u128 = 0;
    if d.withdraw_request_shares > 0
        && d.withdraw_request_ts
            .saturating_add(c.params.withdraw_cooldown_s)
            <= e.close_ts
    {
        redeemed_shares = d.withdraw_request_shares.min(d.shares);
        redeemed = accounting::redeem_quote(redeemed_shares, &ep).map_err(acct_err)?;
        d.shares -= redeemed_shares;
        c.total_shares = c
            .total_shares
            .checked_sub(redeemed_shares)
            .ok_or(SprentError::MathOverflow)?;
        d.withdraw_request_shares = 0;
        d.claimable_quote = d
            .claimable_quote
            .checked_add(redeemed)
            .ok_or(SprentError::MathOverflow)?;
        c.payable_withdrawals_quote = c
            .payable_withdrawals_quote
            .checked_add(redeemed)
            .ok_or(SprentError::MathOverflow)?;
        d.redeemed_quote_total = d
            .redeemed_quote_total
            .checked_add(redeemed as u128)
            .ok_or(SprentError::MathOverflow)?;
        let principal_out = redeemed.min(d.net_deposited_quote);
        d.net_deposited_quote -= principal_out;
        c.total_net_deposited_quote = c.total_net_deposited_quote.saturating_sub(principal_out);
    }

    let mut minted: u128 = 0;
    let mut refunded: u64 = 0;
    if d.pending_deposit_quote > 0
        && d.pending_deposit_ts.saturating_add(c.params.entry_delay_s) <= e.close_ts
    {
        let amount = d.pending_deposit_quote;
        d.pending_deposit_quote = 0;
        c.pending_deposits_quote = c
            .pending_deposits_quote
            .checked_sub(amount)
            .ok_or(SprentError::MathOverflow)?;
        match accounting::mint_shares(amount, &ep, c.total_shares) {
            Ok(m) => {
                minted = m.user_shares;
                d.shares = d
                    .shares
                    .checked_add(m.user_shares)
                    .ok_or(SprentError::MathOverflow)?;
                d.hwm_pps = p;
                d.entry_pps = p;
                c.total_shares = c
                    .total_shares
                    .checked_add(m.user_shares + m.dead_shares)
                    .ok_or(SprentError::MathOverflow)?;
                c.dead_shares = c
                    .dead_shares
                    .checked_add(m.dead_shares)
                    .ok_or(SprentError::MathOverflow)?;
            }
            Err(AcctError::ZeroShares) => {
                refunded = amount;
                d.claimable_quote = d
                    .claimable_quote
                    .checked_add(amount)
                    .ok_or(SprentError::MathOverflow)?;
                c.payable_withdrawals_quote = c
                    .payable_withdrawals_quote
                    .checked_add(amount)
                    .ok_or(SprentError::MathOverflow)?;
                let principal_out = amount.min(d.net_deposited_quote);
                d.net_deposited_quote -= principal_out;
                c.total_net_deposited_quote =
                    c.total_net_deposited_quote.saturating_sub(principal_out);
            }
            Err(e) => return Err(acct_err(e)),
        }
    }

    d.next_settle_epoch = e.epoch + 1;
    c.positions_settled = c
        .positions_settled
        .checked_add(1)
        .ok_or(SprentError::MathOverflow)?;
    if d.shares == 0 && d.pending_deposit_quote == 0 && d.withdraw_request_shares == 0 {
        d.active = false;
        c.open_positions = c.open_positions.saturating_sub(1);
    }
    emit!(PositionSettled {
        owner: d.owner,
        epoch: e.epoch,
        pps: p,
        fee_quote: cr.fee_quote,
        fee_shares: cr.fee_shares,
        redeemed_shares,
        redeemed_quote: redeemed,
        minted_shares: minted,
        refunded_quote: refunded,
        shares: d.shares,
        hwm_pps: d.hwm_pps,
    });
    Ok(())
}
