use anchor_lang::prelude::*;

#[error_code]
pub enum SprentError {
    #[msg("Signer is not allowed to call this instruction")]
    Unauthorized,
    #[msg("Account does not match the address pinned in the vault config")]
    AccountSubstitution,
    #[msg("Vault is not running")]
    VaultNotRunning,
    #[msg("Invalid parameter")]
    InvalidParams,
    #[msg("Account is not owned by the Manifest program")]
    MarketOwnerMismatch,
    #[msg("Manifest market header does not match the expected layout")]
    MarketLayoutMismatch,
    #[msg("Market mints or decimals do not match")]
    MarketMintMismatch,
    #[msg("Market vault address does not match")]
    MarketVaultMismatch,
    #[msg("Base mint must be the native SOL mint")]
    BaseMintNotNative,
    #[msg("Only the classic SPL token program is supported")]
    UnsupportedTokenProgram,
    #[msg("Manifest program account is not the expected executable")]
    ManifestProgramMismatch,
    #[msg("Instruction serialization failed")]
    SerializationFailed,
    #[msg("Oracle account owner mismatch")]
    OracleOwnerMismatch,
    #[msg("Oracle account layout mismatch")]
    OracleLayoutMismatch,
    #[msg("Oracle feed id mismatch")]
    OracleFeedMismatch,
    #[msg("Oracle update is not fully verified")]
    OracleNotFullyVerified,
    #[msg("Oracle price is stale")]
    OracleStale,
    #[msg("Oracle confidence interval too wide")]
    OracleConfidenceTooWide,
    #[msg("Oracle price is not positive")]
    OraclePriceInvalid,
    #[msg("Only PostOnly orders may be placed")]
    NotPostOnly,
    #[msg("Order price is outside the oracle band")]
    PriceOutsideBand,
    #[msg("Order price cannot be represented")]
    PriceOutOfRange,
    #[msg("Order size must be positive")]
    ZeroSize,
    #[msg("Too many orders in one batch")]
    TooManyOrders,
    #[msg("Too many cancels in one batch")]
    TooManyCancels,
    #[msg("Empty batch")]
    EmptyBatch,
    #[msg("Ask size exceeds the inventory cap")]
    InventoryCapExceeded,
    #[msg("Bid notional exceeds the quote cap")]
    BidCapExceeded,
    #[msg("Bids and asks in the batch cross each other")]
    SelfCross,
    #[msg("Order expiry slot is missing or too far")]
    InvalidExpiry,
    #[msg("Funding would exceed the working capital share")]
    WorkingCapitalExceeded,
    #[msg("Funding would consume reserved idle quote")]
    ReserveBreached,
    #[msg("Venue withdrawal did not arrive in the vault account")]
    WithdrawDeltaMismatch,
    #[msg("Amount must be positive")]
    ZeroAmount,
    #[msg("Deposit below minimum")]
    DepositTooSmall,
    #[msg("Deposit cap exceeded")]
    DepositCapExceeded,
    #[msg("Depositor account belongs to another owner")]
    DepositorOwnerMismatch,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Vault SOL buffer below rent-exempt minimum")]
    VaultSolBufferTooSmall,
    #[msg("Deposit would mint zero shares")]
    ZeroShares,
    #[msg("Deposit cap above the absolute market capacity cap")]
    DepositCapAboveAbsolute,
    #[msg("Vault is frozen pending reconciliation")]
    VaultFrozen,
    #[msg("Epoch duration has not elapsed")]
    EpochNotElapsed,
    #[msg("Positions of the previous epoch are not all settled")]
    CrankIncomplete,
    #[msg("Epoch already closed")]
    EpochAlreadyClosed,
    #[msg("Vault still has resting orders on the venue")]
    OpenOrdersPresent,
    #[msg("Epoch is not final")]
    EpochNotFinal,
    #[msg("Nothing to settle for this position")]
    NothingToSettle,
    #[msg("Invalid withdrawal amount")]
    InvalidWithdrawAmount,
    #[msg("Nothing to claim")]
    NothingToClaim,
    #[msg("Not enough idle quote; defund the venue first")]
    InsufficientIdle,
    #[msg("No fees to sweep")]
    NothingToSweep,
    #[msg("Base mint has a Token-2022 extension the vault does not accept")]
    UnsupportedMintExtension,
    #[msg("Soft loss limit reached: new bids are refused")]
    LossLimitSoft,
    #[msg("Resting bid notional would exceed the open notional cap")]
    OpenNotionalExceeded,
    #[msg("Too many resting vault orders")]
    TooManyOpenOrders,
    #[msg("No vault order to cancel")]
    NothingToCancel,
    #[msg("Idle quote already covers payouts")]
    NoPayoutShortfall,
    #[msg("Halted desk: only reduce-only asks are allowed")]
    ReduceOnlyWhileHalted,
    #[msg("Reward window record of the last sync is required")]
    RewardWindowMissing,
    #[msg("Wallet is excluded from staking (team / foundation / LP)")]
    StakerExcluded,
    #[msg("Stake is still warming up")]
    StakeNotWarm,
    #[msg("Unstake is still cooling down")]
    StakeCooling,
    #[msg("Claim nonce mismatch")]
    ClaimNonceMismatch,
    #[msg("Buyback intent is not in the expected state")]
    BuybackStateMismatch,
    #[msg("Buyback spent more than its budget")]
    BuybackOverBudget,
    #[msg("Mint supply did not drop by the burned amount")]
    BurnMismatch,
    #[msg("Excluded wallet list is full")]
    ExcludedListFull,
    #[msg("An exclusion in effect still has stake to remove (apply_exclusion first)")]
    ExclusionNotApplied,
}
