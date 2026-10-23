use anchor_lang::prelude::*;

pub mod vault_state {
    pub const RUNNING: u8 = 0;
    pub const HALTED: u8 = 1;
    pub const RECONCILE_FROZEN: u8 = 2;
}

pub mod network {
    pub const DEVNET: u8 = 0;
    pub const MAINNET: u8 = 1;
    pub const LOCALNET: u8 = 2;
}

pub mod epoch_status {

    pub const FROZEN_ATTEMPT: u8 = 1;

    pub const FINAL: u8 = 2;
}

pub mod freeze_reason {
    pub const NONE: u8 = 0;
    pub const ORACLE_STALE: u8 = 1;
    pub const ORACLE_CONFIDENCE: u8 = 2;
    pub const ORACLE_PRICE: u8 = 3;
    pub const RECONCILE: u8 = 4;
    pub const BOOK_DEVIATION: u8 = 5;
    pub const NEGATIVE_NAV: u8 = 6;
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, InitSpace, PartialEq, Eq,
)]
pub struct RiskParams {
    pub max_inventory_base_atoms: u64,

    pub max_bid_quote_atoms: u64,

    pub max_orders_per_batch: u8,

    pub max_order_ttl_slots: u32,

    pub working_capital_bps: u16,

    pub loss_soft_bps: u16,

    pub loss_hard_bps: u16,

    pub max_inventory_bps: u16,

    pub max_open_notional_bps: u16,

    pub oracle_band_bps: u16,

    pub oracle_max_age_s: u32,

    pub oracle_max_conf_bps: u16,

    pub nav_mid_band_bps: u16,

    pub perf_fee_bps: u16,
    pub epoch_duration_s: i64,
    pub entry_delay_s: i64,
    pub withdraw_cooldown_s: i64,
    pub deposit_cap_total_quote: u64,
    pub deposit_cap_per_user_quote: u64,
    pub min_deposit_quote: u64,
}

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub desk_id: u16,
    pub admin: Pubkey,

    pub guardian: Pubkey,

    pub operator: Pubkey,
    pub manifest_program: Pubkey,
    pub manifest_market: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_token_program: Pubkey,
    pub quote_token_program: Pubkey,

    pub vault_authority: Pubkey,
    pub vault_base_ata: Pubkey,
    pub vault_quote_ata: Pubkey,
    pub manifest_base_vault: Pubkey,
    pub manifest_quote_vault: Pubkey,

    pub oracle: Pubkey,

    pub oracle_mode: u8,

    pub treasury_s_quote_ata: Pubkey,
    pub oracle_feed_id: [u8; 32],
    pub base_mint_decimals: u8,
    pub quote_mint_decimals: u8,
    pub config_bump: u8,
    pub vault_authority_bump: u8,
    pub network: u8,
    pub state: u8,
    pub params: RiskParams,

    pub current_epoch: u64,
    pub last_close_ts: i64,

    pub last_epoch_pps: u128,

    pub last_epoch_nav: u128,
    pub last_epoch_shares: u128,

    pub day_ref_pps: u128,

    pub peak_pps: u128,

    pub deposit_cap_absolute_quote: u64,

    pub pending_inventory_payout_quote: u64,

    pub total_shares: u128,
    pub dead_shares: u128,

    pub tracked_idle_quote: u64,
    pub tracked_idle_base: u64,

    pub pending_deposits_quote: u64,

    pub payable_withdrawals_quote: u64,

    pub fee_payable_quote: u64,
    pub fees_crystallized_total: u128,
    pub fees_swept_total: u128,

    pub total_net_deposited_quote: u64,
    pub open_positions: u32,
    pub positions_to_settle: u32,
    pub positions_settled: u32,

    pub venue_base_net_funded: i64,
    pub venue_quote_net_funded: i64,
    pub batch_seq: u64,
    pub reserved: [u8; 192],
}

#[account]
#[derive(InitSpace)]
pub struct Depositor {
    pub desk: Pubkey,

    pub owner: Pubkey,
    pub bump: u8,

    pub active: bool,

    pub next_settle_epoch: u64,
    pub shares: u128,

    pub hwm_pps: u128,
    pub entry_pps: u128,

    pub net_deposited_quote: u64,
    pub cumulative_deposited_quote: u128,
    pub pending_deposit_quote: u64,
    pub pending_deposit_ts: i64,
    pub withdraw_request_shares: u128,
    pub withdraw_request_ts: i64,

    pub claimable_quote: u64,
    pub fees_paid_quote: u128,
    pub redeemed_quote_total: u128,
    pub claimed_quote_total: u128,
    pub reserved: [u8; 96],
}

#[account]
#[derive(InitSpace)]
pub struct EpochNav {
    pub epoch: u64,
    pub status: u8,
    pub freeze_reason: u8,
    pub bump: u8,
    pub close_ts: i64,
    pub slot: u64,
    pub oracle_price: i64,
    pub oracle_conf: u64,
    pub oracle_expo: i32,
    pub oracle_publish_time: i64,

    pub price_fp: u128,

    pub book_mid_fp: u128,
    pub seat_base: u64,
    pub seat_quote: u64,
    pub tracked_idle_base: u64,
    pub tracked_idle_quote: u64,
    pub ata_base: u64,
    pub ata_quote: u64,
    pub pending_deposits_quote: u64,
    pub payable_withdrawals_quote: u64,
    pub fee_payable_quote: u64,

    pub nav_quote: u128,
    pub total_shares: u128,

    pub pps: u128,
    pub positions_to_settle: u32,
}
