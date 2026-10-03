pub mod constants;
pub mod error;
pub mod helpers;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use error::*;
pub use helpers::*;
pub use instructions::*;
pub use state::*;

declare_id!("CiMmW74LozC86HTRK9U29qCP92wFv1HMedrwmgohSUk3");

#[program]
pub mod nft_core {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handle_initialize(ctx)
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        instructions::increment::handle_increment(ctx)
    }

    pub fn init_config(
        ctx: Context<InitConfig>,
        reward_rate: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        instructions::init_config::handle_init_config(ctx, reward_rate, burn_bonus)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        instructions::stake::handle_stake(ctx)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        instructions::unstake::handle_unstake(ctx)
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        instructions::claim_rewards::handle_claim_rewards(ctx)
    }

    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        instructions::burn_staked_nft::handle_burn_staked_nft(ctx)
    }
}
