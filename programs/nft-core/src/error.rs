use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Only the counter authority can update this counter")]
    Unauthorized,
    #[msg("Counter has reached the maximum value")]
    CounterOverflow,
}

#[error_code]
pub enum StakingError {
    #[msg("NFT is already staked")]
    AlreadyStaked,
    #[msg("NFT is not currently staked")]
    NotStaked,
    #[msg("No rewards to claim")]
    NoRewardsToClaim,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Invalid authority")]
    InvalidAuthority,
    #[msg("Collection mismatch")]
    CollectionMismatch,
    #[msg("Missing attributes plugin")]
    MissingAttributesPlugin,
}

