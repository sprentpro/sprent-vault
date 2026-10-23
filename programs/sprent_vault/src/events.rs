use anchor_lang::prelude::*;

use crate::state::*;

#[event]
pub struct VaultInitialized {
    pub desk_id: u16,
    pub admin: Pubkey,
    pub operator: Pubkey,
    pub guardian: Pubkey,
    pub market: Pubkey,
    pub vault_authority: Pubkey,
}

#[event]
pub struct Deposited {
    pub owner: Pubkey,
    pub amount: u64,
    pub epoch: u64,
    pub pending_total: u64,
}

#[event]
pub struct VenueFunded {
    pub base_amount: u64,
    pub quote_amount: u64,
}

#[event]
pub struct VenueDefunded {
    pub authority: Pubkey,
    pub base_amount: u64,
    pub quote_amount: u64,
}

#[event]
pub struct QuoteBatchPlaced {
    pub seq: u64,
    pub cancels: u8,
    pub orders: u8,
}

#[event]
pub struct OrdersCancelled {
    pub authority: Pubkey,
    pub count: u8,
}

#[event]
pub struct ParamsUpdated {
    pub params: RiskParams,
}

#[event]
pub struct RolesUpdated {
    pub operator: Pubkey,
    pub guardian: Pubkey,
}

#[event]
pub struct StateChanged {
    pub authority: Pubkey,
    pub state: u8,
}

#[event]
pub struct TreasuryInitialized {
    pub treasury_s: Pubkey,
    pub quote_ata: Pubkey,
}

#[event]
pub struct WithdrawRequested {
    pub owner: Pubkey,
    pub shares: u128,
    pub epoch: u64,
}

#[event]
pub struct EpochClosed {
    pub epoch: u64,
    pub nav_quote: u128,
    pub total_shares: u128,
    pub pps: u128,
    pub price_fp: u128,
    pub seat_base: u64,
    pub seat_quote: u64,
    pub surplus_quote: u64,
    pub surplus_base: u64,
    pub positions_to_settle: u32,
}

#[event]
pub struct EpochFrozen {
    pub epoch: u64,
    pub reason: u8,
    pub nav_quote: i128,
    pub price_fp: u128,
    pub book_mid_fp: u128,
}

#[event]
pub struct PositionSettled {
    pub owner: Pubkey,
    pub epoch: u64,
    pub pps: u128,
    pub fee_quote: u64,
    pub fee_shares: u128,
    pub redeemed_shares: u128,
    pub redeemed_quote: u64,
    pub minted_shares: u128,
    pub refunded_quote: u64,
    pub shares: u128,
    pub hwm_pps: u128,
}

#[event]
pub struct WithdrawClaimed {
    pub owner: Pubkey,
    pub amount: u64,
    pub remaining: u64,
}

#[event]
pub struct FeesSwept {
    pub amount: u64,
    pub remaining: u64,
}

#[event]
pub struct LossLimitHalted {
    pub pps_live: u128,
    pub peak_pps: u128,
    pub cancelled: u8,
}

#[event]
pub struct EmergencyCancelled {
    pub guardian: Pubkey,
    pub cancelled: u8,
    pub base_returned: u64,
    pub quote_returned: u64,
}

#[event]
pub struct InventoryPayoutPending {
    pub desk_id: u16,
    pub owed_quote: u64,
    pub liquid_quote: u64,
    pub pending_inventory_payout_quote: u64,
}

#[event]
pub struct PlatformInitialized {
    pub admin: Pubkey,
    pub buyback_executor: Pubkey,
    pub s_quote_ata: Pubkey,
}

#[event]
pub struct TokenSideInitialized {
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub stake_pool: Pubkey,
}

#[event]
pub struct ServiceDistributed {
    pub seq: u64,
    pub gross: u64,
    pub costs: u64,
    pub net: u64,
    pub holder: u64,
    pub buyback: u64,
    pub team: u64,
    pub window: u64,
    pub window_reward: u64,
    pub carry: u64,
}

#[event]
pub struct TeamWithdrawn {
    pub amount: u64,
    pub destination: Pubkey,
}

#[event]
pub struct Staked {
    pub owner: Pubkey,
    pub amount: u64,
    pub warm_until: i64,
}

#[event]
pub struct StakeActivated {
    pub owner: Pubkey,
    pub amount: u64,
    pub effective: u64,
}

#[event]
pub struct UnstakeRequested {
    pub owner: Pubkey,
    pub amount: u64,
    pub cool_until: i64,
}

#[event]
pub struct UnstakeWithdrawn {
    pub owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct RewardClaimed {
    pub owner: Pubkey,
    pub amount: u64,
    pub nonce: u64,
}

#[event]
pub struct ExclusionChanged {
    pub wallet: Pubkey,
    pub excluded: bool,
    pub effective_window: u64,
    pub stake_to_remove: bool,
}

#[event]
pub struct ExclusionApplied {
    pub owner: Pubkey,
    pub window: u64,
    pub removed_effective: u64,
    pub removed_warming: u64,
}

#[event]
pub struct BuybackOpened {
    pub intent_id: u64,
    pub source: u8,
    pub budget: u64,
}

#[event]
pub struct BuybackRecorded {
    pub intent_id: u64,
    pub source: u8,
    pub spent: u64,
    pub refunded: u64,
    pub bought: u64,
    pub buy_sig: [u8; 64],
}

#[event]
pub struct BuybackBurned {
    pub intent_id: u64,
    pub source: u8,
    pub amount: u64,
    pub burned: u64,
    pub pending_burn: u64,
}
