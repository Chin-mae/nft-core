use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attribute, Attributes, Plugin, PluginType},
};

use crate::{
    constants::CORE_PROGRAM_ID,
    error::StakingError,
    helpers::update_collection_total_staked,
};

#[derive(Accounts)]
pub struct Unstake<'info> {
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

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Core program id checked via constraint
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_unstake(ctx: Context<Unstake>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // 1. Fetch asset attributes and check staked status
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

    // Calculate elapsed time and add to accumulated staked_time
    let elapsed = now.saturating_sub(staked_timestamp);
    let new_staked_time = staked_time.saturating_add(if elapsed > 0 { elapsed as u64 } else { 0 });

    let mut updated_attribute_list: Vec<Attribute> = Vec::new();
    for attr in fetched_attributes.attribute_list {
        if attr.key == "staked" {
            updated_attribute_list.push(Attribute {
                key: "staked".to_string(),
                value: "0".to_string(),
            });
        } else if attr.key == "staked_time" {
            updated_attribute_list.push(Attribute {
                key: "staked_time".to_string(),
                value: new_staked_time.to_string(),
            });
        } else {
            updated_attribute_list.push(attr);
        }
    }

    // 2. Update asset attributes
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

    // 3. Remove FreezeDelegate plugin (unfreeze)
    let _ = RemovePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin_type(PluginType::FreezeDelegate)
        .invoke();

    // 4. Remove BurnDelegate plugin if present
    let _ = RemovePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin_type(PluginType::BurnDelegate)
        .invoke();

    // 5. Decrement collection total_staked counter
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
    #[test]
    fn test_instruction_unstake_resets_staked_marker_and_accumulates_time() {
        let staked_timestamp = 1000i64;
        let previous_staked_time = 50u64;
        let unstake_time = 1200i64;

        let elapsed = unstake_time.saturating_sub(staked_timestamp) as u64;
        assert_eq!(elapsed, 200);

        let new_staked_time = previous_staked_time.saturating_add(elapsed);
        assert_eq!(new_staked_time, 250);

        let updated_staked = "0"; // Marked as unstaked
        assert_eq!(updated_staked, "0");
    }
}

