use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{get_return_data, set_return_data};
use anchor_lang::system_program;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

use crate::accounting::AcctError;
use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::manifest::*;
use crate::state::*;

pub(crate) fn acct_err(e: AcctError) -> Error {
    match e {
        AcctError::ZeroShares => error!(SprentError::ZeroShares),
        AcctError::Overflow => error!(SprentError::MathOverflow),
    }
}

pub(crate) fn validate_params(p: &RiskParams, abs_cap: u64) -> Result<()> {
    let bps = BPS_DENOMINATOR as u16;
    require!(
        p.deposit_cap_total_quote <= abs_cap,
        SprentError::DepositCapAboveAbsolute
    );
    require!(
        p.nav_mid_band_bps > 0 && p.nav_mid_band_bps <= 2_000,
        SprentError::InvalidParams
    );
    require!(
        p.working_capital_bps <= bps
            && p.loss_soft_bps > 0
            && p.loss_soft_bps < p.loss_hard_bps
            && p.loss_hard_bps < bps
            && p.max_inventory_bps > 0
            && p.max_inventory_bps <= bps
            && p.max_open_notional_bps > 0
            && p.max_open_notional_bps <= bps
            && p.max_order_ttl_slots > 0
            && p.oracle_band_bps > 0
            && p.oracle_band_bps <= 2_000
            && p.oracle_max_conf_bps > 0
            && p.oracle_max_conf_bps <= bps
            && p.oracle_max_age_s > 0
            && p.perf_fee_bps <= 5_000
            && p.max_orders_per_batch >= 1
            && p.max_orders_per_batch <= MAX_ORDERS_PER_BATCH_HARD
            && p.epoch_duration_s > 0
            && p.entry_delay_s >= 0
            && p.withdraw_cooldown_s >= 0
            && p.deposit_cap_per_user_quote <= p.deposit_cap_total_quote,
        SprentError::InvalidParams
    );
    Ok(())
}

pub(crate) fn note_inventory_payout(c: &mut VaultConfig, seat_quote: u64) {
    let owed = c
        .payable_withdrawals_quote
        .saturating_add(c.fee_payable_quote);
    let liquid = c
        .tracked_idle_quote
        .saturating_sub(c.pending_deposits_quote)
        .saturating_add(seat_quote);
    let pending = owed.saturating_sub(liquid);
    if pending != c.pending_inventory_payout_quote {
        c.pending_inventory_payout_quote = pending;
        emit!(InventoryPayoutPending {
            desk_id: c.desk_id,
            owed_quote: owed,
            liquid_quote: liquid,
            pending_inventory_payout_quote: pending
        });
    }
}

pub(crate) fn cancel_listed<'info>(
    list: &[VaultOrder],
    config: &Account<'info, VaultConfig>,
    vault_authority: &AccountInfo<'info>,
    market: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    manifest_program: &AccountInfo<'info>,
) -> Result<u8> {
    if list.is_empty() {
        return Ok(0);
    }
    let batch = WireBatch {
        trader_index_hint: None,
        cancels: list
            .iter()
            .map(|o| WireCancel {
                order_sequence_number: o.seq,
                order_index_hint: Some(o.index),
            })
            .collect(),
        orders: vec![],
    };
    manifest_batch(
        &batch,
        config,
        vault_authority,
        market,
        system,
        manifest_program,
    )?;
    Ok(list.len() as u8)
}

pub(crate) fn manifest_batch<'info>(
    batch: &WireBatch,
    config: &Account<'info, VaultConfig>,
    vault_authority: &AccountInfo<'info>,
    market: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    manifest_program: &AccountInfo<'info>,
) -> Result<()> {
    let ix = batch_update_ix(
        config.vault_authority,
        config.manifest_market,
        system_program::ID,
        batch,
    )?;
    invoke_as_vault(
        &ix,
        &[
            vault_authority.clone(),
            market.clone(),
            system.clone(),
            manifest_program.clone(),
        ],
        &config.key(),
        config.vault_authority_bump,
    )?;

    if let Some((pid, data)) = get_return_data() {
        if pid == MANIFEST_PROGRAM_ID {
            set_return_data(&data);
        }
    }
    Ok(())
}

pub(crate) fn vault_transfer_quote<'info>(
    token_program: &Program<'info, Token>,
    from: &Account<'info, TokenAccount>,
    mint: &Account<'info, Mint>,
    to: &AccountInfo<'info>,
    vault_authority: &UncheckedAccount<'info>,
    config: &Account<'info, VaultConfig>,
    amount: u64,
) -> Result<()> {
    let config_key = config.key();
    let bump = [config.vault_authority_bump];
    let seeds: &[&[u8]] = &[VAULT_AUTH_SEED, config_key.as_ref(), &bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            TransferChecked {
                from: from.to_account_info(),
                mint: mint.to_account_info(),
                to: to.clone(),
                authority: vault_authority.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        mint.decimals,
    )
}
