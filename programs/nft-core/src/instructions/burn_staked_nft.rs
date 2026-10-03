use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::{BurnV1CpiBuilder, RemovePluginV1CpiBuilder},
    types::{Attributes, PluginType},
};

use crate::{
    constants::{CONFIG_SEED, CORE_PROGRAM_ID},
    error::StakingError,
    helpers::{calculate_staking_rewards, update_collection_total_staked},
    state::StakeConfig,
};

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    pub owner: Signer<'info>,
    pub update_authority: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Core asset account
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core collection account
    #[account(mut)]
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

pub fn handle_burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // 1. Fetch asset attributes to verify staked status and calculate accumulated rewards
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

    // 2. Calculate rewards: accumulated rewards + fixed burn bonus (e.g., 10,000,000 tokens)
    let accumulated_rewards = calculate_staking_rewards(
        staked_timestamp,
        staked_time,
        now,
        ctx.accounts.config.reward_rate,
    )?;

    let total_burn_reward = accumulated_rewards
        .checked_add(ctx.accounts.config.burn_bonus)
        .ok_or(StakingError::ArithmeticOverflow)?;

    // 3. Mint rewards to user's ATA
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
    mint_to(cpi_ctx, total_burn_reward)?;

    // 4. Remove FreezeDelegate plugin so asset can be burned smoothly
    let _ = RemovePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin_type(PluginType::FreezeDelegate)
        .invoke();

    // 5. Burn the NFT via Core CPI using BurnDelegate / owner authority
    BurnV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.owner.to_account_info()))
        .system_program(Some(&ctx.accounts.system_program.to_account_info()))
        .invoke()?;

    // 6. Decrement collection total_staked counter
    update_collection_total_staked(
        &ctx.accounts.core_program.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.update_authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        false,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_burn_staked_nft_10m_fixed_bonus() {
        let staked_at = 2000i64;
        let now = 2050i64; // 50 seconds staked
        let reward_rate = 1u64;
        let burn_bonus = crate::constants::BURN_BONUS_TOKENS;

        let accumulated = calculate_staking_rewards(staked_at, 0, now, reward_rate).unwrap();
        assert_eq!(accumulated, 50);

        let total_rewards = accumulated + burn_bonus;
        assert_eq!(total_rewards, 10_000_050); // 50 + 10,000,000
    }

    #[test]
    fn test_instruction_burn_staked_nft_after_claim_still_gets_10m() {
        // Even if user recently claimed and accumulated is 0, they still get full 10M
        let staked_at = 3000i64; // Just claimed at 3000
        let now = 3000i64;
        let reward_rate = 1u64;
        let burn_bonus = crate::constants::BURN_BONUS_TOKENS;

        let accumulated = calculate_staking_rewards(staked_at, 0, now, reward_rate).unwrap();
        assert_eq!(accumulated, 0);

        let total_rewards = accumulated + burn_bonus;
        assert_eq!(total_rewards, 10_000_000);
    }
}

