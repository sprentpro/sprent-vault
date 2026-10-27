use anchor_lang::prelude::*;

use crate::accounting::{self, NavInputs, PPS_SCALE};
use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::manifest::*;
use crate::oracle;
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CancelInput {
    pub order_sequence_number: u64,
    pub order_index_hint: Option<u32>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct OrderInput {
    pub base_atoms: u64,
    pub price_mantissa: u32,
    pub price_exponent: i8,
    pub is_bid: bool,
    pub last_valid_slot: u32,

    pub order_type: u8,
}

#[derive(Accounts)]
pub struct QuoteBatch<'info> {
    pub operator: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: pinned vault authority PDA.
    #[account(mut, address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: pinned market.
    #[account(mut, address = config.manifest_market @ SprentError::AccountSubstitution)]
    pub market: UncheckedAccount<'info>,
    /// CHECK: pinned oracle; layout verified when orders are placed.
    #[account(address = config.oracle @ SprentError::AccountSubstitution)]
    pub oracle: UncheckedAccount<'info>,
    /// CHECK: fixed Manifest program id.
    #[account(address = MANIFEST_PROGRAM_ID @ SprentError::ManifestProgramMismatch)]
    pub manifest_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelAll<'info> {
    pub authority: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: pinned vault authority PDA.
    #[account(mut, address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: pinned market.
    #[account(mut, address = config.manifest_market @ SprentError::AccountSubstitution)]
    pub market: UncheckedAccount<'info>,
    /// CHECK: fixed Manifest program id.
    #[account(address = MANIFEST_PROGRAM_ID @ SprentError::ManifestProgramMismatch)]
    pub manifest_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn quote_batch(
    ctx: Context<QuoteBatch>,
    cancels: Vec<CancelInput>,
    orders: Vec<OrderInput>,
) -> Result<()> {
    let c = &ctx.accounts.config;
    let halted = c.state == vault_state::HALTED;
    let signer = ctx.accounts.operator.key();
    require!(
        signer == c.operator || (halted && signer == c.guardian),
        SprentError::Unauthorized
    );
    require!(
        !cancels.is_empty() || !orders.is_empty(),
        SprentError::EmptyBatch
    );
    require!(
        cancels.len() <= MAX_CANCELS_PER_BATCH_HARD,
        SprentError::TooManyCancels
    );
    let clock = Clock::get()?;
    let v = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        clock.slot,
    )?;
    let is_cancelled = |seq: u64| cancels.iter().any(|x| x.order_sequence_number == seq);
    let live_cancels: Vec<WireCancel> = v
        .listed()
        .iter()
        .filter(|o| is_cancelled(o.seq))
        .map(|o| WireCancel {
            order_sequence_number: o.seq,
            order_index_hint: Some(o.index),
        })
        .collect();

    if !orders.is_empty() {
        require!(
            c.state == vault_state::RUNNING || halted,
            SprentError::VaultNotRunning
        );

        if halted {
            require!(
                orders.iter().all(|o| !o.is_bid),
                SprentError::ReduceOnlyWhileHalted
            );
        }
        require!(
            orders.len() <= c.params.max_orders_per_batch as usize,
            SprentError::TooManyOrders
        );
        let rd = oracle::read_oracle(
            c.oracle_mode,
            &ctx.accounts.oracle.to_account_info(),
            &c.oracle_feed_id,
            clock.unix_timestamp,
            c.params.oracle_max_age_s,
            c.params.oracle_max_conf_bps,
            c.base_mint_decimals,
            c.quote_mint_decimals,
        )?;
        let oracle_fp = oracle::require_usable(&rd)?;
        let mut ask_base: u128 = 0;
        let mut bid_quote: u128 = 0;
        let mut bid_base: u128 = 0;
        let mut max_bid: Option<u128> = None;
        let mut min_ask: Option<u128> = None;
        for o in orders.iter() {
            require!(
                o.order_type == ORDER_TYPE_POST_ONLY,
                SprentError::NotPostOnly
            );
            require!(o.base_atoms > 0, SprentError::ZeroSize);
            let lvs = o.last_valid_slot as u64;
            require!(
                lvs > clock.slot && lvs <= clock.slot + c.params.max_order_ttl_slots as u64,
                SprentError::InvalidExpiry
            );
            let p = oracle::order_price_fp(o.price_mantissa, o.price_exponent)?;
            require!(
                oracle::within_band(p, oracle_fp, c.params.oracle_band_bps)?,
                SprentError::PriceOutsideBand
            );
            let q = oracle::bid_notional_quote(o.base_atoms, p)?;
            if o.is_bid {
                bid_quote = bid_quote.checked_add(q).ok_or(SprentError::MathOverflow)?;
                bid_base = bid_base
                    .checked_add(o.base_atoms as u128)
                    .ok_or(SprentError::MathOverflow)?;
                max_bid = Some(max_bid.map_or(p, |m| m.max(p)));
            } else {
                ask_base = ask_base
                    .checked_add(o.base_atoms as u128)
                    .ok_or(SprentError::MathOverflow)?;
                min_ask = Some(min_ask.map_or(p, |m| m.min(p)));
            }
        }
        require!(
            ask_base <= c.params.max_inventory_base_atoms as u128,
            SprentError::InventoryCapExceeded
        );
        require!(
            bid_quote <= c.params.max_bid_quote_atoms as u128,
            SprentError::BidCapExceeded
        );
        if let (Some(b), Some(a)) = (max_bid, min_ask) {
            require!(b < a, SprentError::SelfCross);
        }

        let remaining = v.listed().iter().filter(|o| !is_cancelled(o.seq));
        let mut rem_count = 0usize;
        let mut rem_notional: u128 = 0;
        let mut rem_bid_base: u128 = 0;
        for o in remaining {
            rem_count += 1;
            if o.is_bid {
                rem_notional = rem_notional
                    .checked_add(o.quote_atoms().ok_or(SprentError::MathOverflow)?)
                    .ok_or(SprentError::MathOverflow)?;
                rem_bid_base += o.base_atoms as u128;
            }
        }
        require!(
            rem_count + orders.len() <= MAX_VAULT_ORDERS,
            SprentError::TooManyOpenOrders
        );
        if c.total_shares > 0 && !halted {
            let nav = live_nav(c, &v, oracle_fp)?;
            let nav_u = nav.max(0) as u128;
            let pps_live = nav_u
                .checked_mul(PPS_SCALE)
                .ok_or(SprentError::MathOverflow)?
                / c.total_shares;
            let hard_floor = c.peak_pps * (BPS_DENOMINATOR - c.params.loss_hard_bps as u64) as u128;
            if nav <= 0 || pps_live * (BPS_DENOMINATOR as u128) < hard_floor {
                let n = cancel_listed(
                    v.listed(),
                    &ctx.accounts.config,
                    &ctx.accounts.vault_authority.to_account_info(),
                    &ctx.accounts.market.to_account_info(),
                    &ctx.accounts.system_program.to_account_info(),
                    &ctx.accounts.manifest_program.to_account_info(),
                )?;
                let c = &mut ctx.accounts.config;
                c.state = vault_state::HALTED;
                emit!(LossLimitHalted {
                    pps_live,
                    peak_pps: c.peak_pps,
                    cancelled: n
                });
                return Ok(());
            }
            let soft_floor =
                c.day_ref_pps * (BPS_DENOMINATOR - c.params.loss_soft_bps as u64) as u128;
            if bid_base > 0 {
                require!(
                    pps_live * BPS_DENOMINATOR as u128 >= soft_floor,
                    SprentError::LossLimitSoft
                );
            }

            if bid_base > 0 {
                require!(
                    (rem_notional + bid_quote) * BPS_DENOMINATOR as u128
                        <= nav_u * c.params.max_open_notional_bps as u128,
                    SprentError::OpenNotionalExceeded
                );
            }
            if bid_base > 0 {
                let mut held: u128 = c.tracked_idle_base as u128 + v.seat_base as u128;
                for o in v.listed().iter().filter(|o| !o.is_bid) {
                    held += o.base_atoms as u128;
                }
                let projected = held + rem_bid_base + bid_base;
                let value = projected
                    .checked_mul(oracle_fp)
                    .ok_or(SprentError::MathOverflow)?
                    / accounting::PRICE_FP_ONE;
                require!(
                    value * BPS_DENOMINATOR as u128 <= nav_u * c.params.max_inventory_bps as u128,
                    SprentError::InventoryCapExceeded
                );
            }
        }
    }

    if live_cancels.is_empty() && orders.is_empty() {
        emit!(QuoteBatchPlaced {
            seq: c.batch_seq,
            cancels: 0,
            orders: 0
        });
        return Ok(());
    }
    let batch = WireBatch {
        trader_index_hint: None,
        cancels: live_cancels.clone(),
        orders: orders
            .iter()
            .map(|o| WirePlace {
                base_atoms: o.base_atoms,
                price_mantissa: o.price_mantissa,
                price_exponent: o.price_exponent,
                is_bid: o.is_bid,
                last_valid_slot: o.last_valid_slot,
                order_type: ORDER_TYPE_POST_ONLY,
            })
            .collect(),
    };
    manifest_batch(
        &batch,
        &ctx.accounts.config,
        &ctx.accounts.vault_authority.to_account_info(),
        &ctx.accounts.market.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.manifest_program.to_account_info(),
    )?;
    let c = &mut ctx.accounts.config;
    c.batch_seq = c
        .batch_seq
        .checked_add(1)
        .ok_or(SprentError::MathOverflow)?;
    note_inventory_payout(c, v.seat_quote);
    emit!(QuoteBatchPlaced {
        seq: c.batch_seq,
        cancels: live_cancels.len() as u8,
        orders: orders.len() as u8
    });
    Ok(())
}

pub(crate) fn cancel_all(ctx: Context<CancelAll>, cancels: Vec<CancelInput>) -> Result<()> {
    let c = &ctx.accounts.config;
    let signer = ctx.accounts.authority.key();
    require!(
        signer == c.operator || signer == c.guardian,
        SprentError::Unauthorized
    );
    require!(!cancels.is_empty(), SprentError::EmptyBatch);
    require!(
        cancels.len() <= MAX_CANCELS_PER_BATCH_HARD,
        SprentError::TooManyCancels
    );
    let v = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        Clock::get()?.slot,
    )?;
    let pick: Vec<VaultOrder> = v
        .listed()
        .iter()
        .copied()
        .filter(|o| cancels.iter().any(|x| x.order_sequence_number == o.seq))
        .collect();
    require!(!pick.is_empty(), SprentError::NothingToCancel);
    let n = cancel_listed(
        &pick,
        c,
        &ctx.accounts.vault_authority.to_account_info(),
        &ctx.accounts.market.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.manifest_program.to_account_info(),
    )?;
    emit!(OrdersCancelled {
        authority: signer,
        count: n
    });
    Ok(())
}

pub(crate) fn cancel_orders_scan(ctx: Context<CancelAll>, expired_only: bool) -> Result<()> {
    let c = &ctx.accounts.config;
    let signer = ctx.accounts.authority.key();
    if !expired_only {
        require!(
            signer == c.operator || signer == c.guardian,
            SprentError::Unauthorized
        );
    }
    let v = read_venue(
        &ctx.accounts.market.to_account_info(),
        &c.vault_authority,
        Clock::get()?.slot,
    )?;
    let pick: Vec<VaultOrder> = v
        .listed()
        .iter()
        .copied()
        .filter(|o| o.expired || !expired_only)
        .collect();
    require!(!pick.is_empty(), SprentError::NothingToCancel);
    let n = cancel_listed(
        &pick,
        c,
        &ctx.accounts.vault_authority.to_account_info(),
        &ctx.accounts.market.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.manifest_program.to_account_info(),
    )?;
    emit!(OrdersCancelled {
        authority: signer,
        count: n
    });
    Ok(())
}

fn live_nav(c: &VaultConfig, v: &VenueView, price_fp: u128) -> Result<i128> {
    let mut bid_q: u64 = 0;
    let mut ask_b: u64 = 0;
    for o in v.listed() {
        if o.is_bid {
            let q = u64::try_from(o.quote_atoms().ok_or(SprentError::MathOverflow)?)
                .map_err(|_| SprentError::MathOverflow)?;
            bid_q = bid_q.checked_add(q).ok_or(SprentError::MathOverflow)?;
        } else {
            ask_b = ask_b
                .checked_add(o.base_atoms)
                .ok_or(SprentError::MathOverflow)?;
        }
    }
    accounting::nav_quote(&NavInputs {
        tracked_idle_quote: c.tracked_idle_quote,
        tracked_idle_base: c.tracked_idle_base,
        seat_quote: v
            .seat_quote
            .checked_add(bid_q)
            .ok_or(SprentError::MathOverflow)?,
        seat_base: v
            .seat_base
            .checked_add(ask_b)
            .ok_or(SprentError::MathOverflow)?,
        pending_deposits_quote: c.pending_deposits_quote,
        payable_withdrawals_quote: c.payable_withdrawals_quote,
        fee_payable_quote: c.fee_payable_quote,
        price_fp,
    })
    .map_err(acct_err)
}
