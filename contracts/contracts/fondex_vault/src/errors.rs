use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FondexError {
    AmountMustBePositive = 1,
    ZeroShares = 2,
    ZeroAssets = 3,
    InsufficientLiquidity = 4,
    DepositCapExceeded = 5,
    StrategyUnavailable = 6,
    TooManyStrategies = 10,
    StrategyAlreadyExists = 11,
    StrategyNotFound = 12,
    AssetMismatch = 13,
    AllocationTooHigh = 14,
    PendingStrategyExists = 15,
    NoPendingStrategy = 16,
    TimelockNotExpired = 17,
    StrategyNotEmpty = 18,
    InvalidTimelock = 19,
    InvalidDepositCap = 20,
}
