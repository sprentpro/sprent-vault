use anchor_lang::prelude::*;

pub mod accounting;
pub mod constants;
pub mod errors;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod manifest;
pub mod oracle;
pub mod state;

pub use constants::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g");

#[program]
pub mod sprent_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: InitializeArgs) -> Result<()> {
        instructions::initialize(ctx, args)
    }

    pub fn claim_seat(ctx: Context<ClaimSeat>) -> Result<()> {
        instructions::claim_seat(ctx)
    }

    pub fn deposit(ctx: Context<DepositQuote>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    pub fn fund_venue(
        ctx: Context<VenueTransfer>,
        base_amount: u64,
        quote_amount: u64,
    ) -> Result<()> {
        instructions::fund_venue(ctx, base_amount, quote_amount)
    }

    pub fn defund_venue(
        ctx: Context<VenueTransfer>,
        base_amount: u64,
        quote_amount: u64,
    ) -> Result<()> {
        instructions::defund_venue(ctx, base_amount, quote_amount)
    }

    pub fn quote_batch(
        ctx: Context<QuoteBatch>,
        cancels: Vec<CancelInput>,
        orders: Vec<OrderInput>,
    ) -> Result<()> {
        instructions::quote_batch(ctx, cancels, orders)
    }

    pub fn cancel_all(ctx: Context<CancelAll>, cancels: Vec<CancelInput>) -> Result<()> {
        instructions::cancel_all(ctx, cancels)
    }

    pub fn cancel_orders_scan(ctx: Context<CancelAll>, expired_only: bool) -> Result<()> {
        instructions::cancel_orders_scan(ctx, expired_only)
    }

    pub fn emergency_cancel(ctx: Context<VenueTransfer>) -> Result<()> {
        instructions::emergency_cancel(ctx)
    }

    pub fn defund_for_payouts(ctx: Context<VenueTransfer>) -> Result<()> {
        instructions::defund_for_payouts(ctx)
    }

    pub fn init_treasury(ctx: Context<InitTreasury>) -> Result<()> {
        instructions::init_treasury(ctx)
    }

    pub fn init_platform(ctx: Context<InitPlatform>, args: PlatformArgs) -> Result<()> {
        instructions::init_platform(ctx, args)
    }

    pub fn init_token_side(ctx: Context<InitTokenSide>) -> Result<()> {
        instructions::init_token_side(ctx)
    }

    pub fn distribute_service(ctx: Context<DistributeService>, declared_costs: u64) -> Result<()> {
        instructions::distribute_service(ctx, declared_costs)
    }

    pub fn withdraw_team(ctx: Context<WithdrawTeam>, amount: u64) -> Result<()> {
        instructions::withdraw_team(ctx, amount)
    }

    pub fn stake(ctx: Context<StakeOp>, amount: u64) -> Result<()> {
        instructions::stake(ctx, amount)
    }

    pub fn activate_stake(ctx: Context<StakeCrank>) -> Result<()> {
        instructions::activate_stake(ctx)
    }

    pub fn request_unstake(ctx: Context<StakeOwnerOp>, amount: u64) -> Result<()> {
        instructions::request_unstake(ctx, amount)
    }

    pub fn withdraw_unstaked(ctx: Context<StakeTokenOut>) -> Result<()> {
        instructions::withdraw_unstaked(ctx)
    }

    pub fn claim_reward(ctx: Context<ClaimReward>, nonce: u64) -> Result<()> {
        instructions::claim_reward(ctx, nonce)
    }

    pub fn set_excluded<'info>(
        ctx: Context<'_, '_, '_, 'info, SetExcluded<'info>>,
        add: Vec<Pubkey>,
        remove: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::set_excluded(ctx, add, remove)
    }

    pub fn apply_exclusion(ctx: Context<StakeCrank>) -> Result<()> {
        instructions::apply_exclusion(ctx)
    }

    pub fn open_creator_buyback(
        ctx: Context<OpenCreatorBuyback>,
        intent_id: u64,
        budget_lamports: u64,
    ) -> Result<()> {
        instructions::open_creator_buyback(ctx, intent_id, budget_lamports)
    }

    pub fn take_service_buyback(
        ctx: Context<TakeServiceBuyback>,
        intent_id: u64,
        amount: u64,
    ) -> Result<()> {
        instructions::take_service_buyback(ctx, intent_id, amount)
    }

    pub fn record_buyback(
        ctx: Context<RecordBuyback>,
        buy_sig: [u8; 64],
        spent: u64,
        bought: u64,
    ) -> Result<()> {
        instructions::record_buyback(ctx, buy_sig, spent, bought)
    }

    pub fn burn_bought(ctx: Context<BurnBought>, amount: u64) -> Result<()> {
        instructions::burn_bought(ctx, amount)
    }

    pub fn request_withdraw(ctx: Context<OwnerPosition>, shares: u128) -> Result<()> {
        instructions::request_withdraw(ctx, shares)
    }

    pub fn close_epoch(ctx: Context<CloseEpoch>) -> Result<()> {
        instructions::close_epoch(ctx)
    }

    pub fn settle_position(ctx: Context<SettlePosition>) -> Result<()> {
        instructions::settle_position(ctx)
    }

    pub fn claim_withdraw(ctx: Context<ClaimWithdraw>) -> Result<()> {
        instructions::claim_withdraw(ctx)
    }

    pub fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
        instructions::sweep_fees(ctx)
    }

    pub fn set_params(ctx: Context<AdminOnly>, params: RiskParams) -> Result<()> {
        instructions::set_params(ctx, params)
    }

    pub fn set_roles(ctx: Context<AdminOnly>, operator: Pubkey, guardian: Pubkey) -> Result<()> {
        instructions::set_roles(ctx, operator, guardian)
    }

    pub fn set_state(ctx: Context<SetState>, new_state: u8) -> Result<()> {
        instructions::set_state(ctx, new_state)
    }
}
