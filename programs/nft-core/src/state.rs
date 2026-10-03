use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Counter {
    pub count: u64,
    pub authority: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    pub authority: Pubkey,
    pub collection: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_rate: u64,
    pub burn_bonus: u64,
    pub bump: u8,
}
