use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType},
};

use crate::{
    constants::{CONFIG_SEED, CORE_PROGRAM_ID},
    error::StakingError,
    helpers::calculate_staking_rewards,
    state::StakeConfig,
};

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    pub owner: Signer<'info>,
    pub update_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Core asset account
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core collection account
    pub collection: UncheckedAccount<'info>,

    #[account(
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump = config.bump,
        has_one = collection,
        has_one = reward_mint,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(mut)]
    pub reward_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub user_reward_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Core program id checked via constraint
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // 1. Fetch asset attributes
    let (_, fetched_attributes, _) = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .map_err(|_| StakingError::MissingAttributesPlugin)?;

    let mut staked_timestamp: i64 = 0;
    let mut staked_time: u64 = 0;
    let mut is_staked = false;

    for attr in &fetched_attributes.attribute_list {
        if attr.key == "staked" {
            staked_timestamp = attr.value.parse::<i64>().unwrap_or(0);
            if staked_timestamp > 0 && attr.value != "0" {
                is_staked = true;
            }
        } else if attr.key == "staked_time" {
            staked_time = attr.value.parse::<u64>().unwrap_or(0);
        }
    }

    require!(is_staked, StakingError::NotStaked);

    // 2. Calculate rewards
    let rewards = calculate_staking_rewards(
        staked_timestamp,
        staked_time,
        now,
        ctx.accounts.config.reward_rate,
    )?;

    require!(rewards > 0, StakingError::NoRewardsToClaim);

    // 3. Mint reward tokens via CPI to token program
    let collection_key = ctx.accounts.collection.key();
    let signer_seeds: &[&[&[u8]]] = &[&[
        CONFIG_SEED,
        collection_key.as_ref(),
        &[ctx.accounts.config.bump],
    ]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.reward_mint.to_account_info(),
        to: ctx.accounts.user_reward_ata.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    mint_to(cpi_ctx, rewards)?;

    // 4. Update asset attributes (reset staked time to now, staked_time to 0)
    let mut updated_attribute_list: Vec<Attribute> = Vec::new();
    for attr in fetched_attributes.attribute_list {
        if attr.key == "staked" {
            updated_attribute_list.push(Attribute {
                key: "staked".to_string(),
                value: now.to_string(),
            });
        } else if attr.key == "staked_time" {
            updated_attribute_list.push(Attribute {
                key: "staked_time".to_string(),
                value: "0".to_string(),
            });
        } else {
            updated_attribute_list.push(attr);
        }
    }

    UpdatePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::Attributes(Attributes {
            attribute_list: updated_attribute_list,
        }))
        .invoke()?;

    // NFT remains staked and frozen
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_claim_rewards_without_unstaking_calculation() {
        let staked_timestamp = 1000i64;
        let staked_time = 0u64;
        let claim_time = 1500i64;
        let reward_rate = 2u64;

        let rewards = calculate_staking_rewards(staked_timestamp, staked_time, claim_time, reward_rate).unwrap();
        assert_eq!(rewards, 1000); // (1500 - 1000) * 2 = 1000 tokens

        // After claiming, staked timestamp is reset to claim_time and staked_time to 0
        let new_staked = claim_time;
        let new_staked_time = 0u64;
        let next_check = 1600i64;
        let new_rewards = calculate_staking_rewards(new_staked, new_staked_time, next_check, reward_rate).unwrap();
        assert_eq!(new_rewards, 200); // (1600 - 1500) * 2 = 200 tokens
    }

    #[test]
    fn test_instruction_claim_rewards_unstaked_fails() {
        let staked_timestamp = 0i64; // Marked as 0 (unstaked)
        let res = calculate_staking_rewards(staked_timestamp, 0, 1500, 1);
        assert!(res.is_err());
    }
}

