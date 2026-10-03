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
}
