use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::{constants::CONFIG_SEED, state::StakeConfig};

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + StakeConfig::INIT_SPACE,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, StakeConfig>,

    /// CHECK: Metaplex Core collection account
    pub collection: UncheckedAccount<'info>,

    pub reward_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_config(
    ctx: Context<InitConfig>,
    reward_rate: u64,
    burn_bonus: u64,
) -> Result<()> {
    ctx.accounts.config.set_inner(StakeConfig {
        authority: ctx.accounts.authority.key(),
        collection: ctx.accounts.collection.key(),
        reward_mint: ctx.accounts.reward_mint.key(),
        reward_rate,
        burn_bonus,
        bump: ctx.bumps.config,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_init_config_parameters() {
        let authority = Pubkey::new_unique();
        let collection = Pubkey::new_unique();
        let reward_mint = Pubkey::new_unique();
        let reward_rate = 5;
        let burn_bonus = crate::constants::BURN_BONUS_TOKENS;

        let config = StakeConfig {
            authority,
            collection,
            reward_mint,
            reward_rate,
            burn_bonus,
            bump: 254,
        };

        assert_eq!(config.authority, authority);
        assert_eq!(config.collection, collection);
        assert_eq!(config.reward_mint, reward_mint);
        assert_eq!(config.reward_rate, 5);
        assert_eq!(config.burn_bonus, 10_000_000);
        assert_eq!(config.bump, 254);
    }
}

