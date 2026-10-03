use anchor_lang::prelude::*;

#[constant]
pub const COUNTER_SEED: &[u8] = b"counter";

#[constant]
pub const HELLO_WORLD_LAMPORTS: u64 = 1;

#[constant]
pub const MAX_COUNT: u64 = 10;

#[constant]
pub const CONFIG_SEED: &[u8] = b"stake_config";

#[constant]
pub const DEFAULT_REWARD_RATE: u64 = 1;

#[constant]
pub const BURN_BONUS_TOKENS: u64 = 10_000_000;

pub const CORE_PROGRAM_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

