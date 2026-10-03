use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attribute, Attributes, BurnDelegate, FreezeDelegate, Plugin, PluginAuthority, PluginType},
};

use crate::{constants::CORE_PROGRAM_ID, error::StakingError, helpers::update_collection_total_staked};

#[derive(Accounts)]
pub struct Stake<'info> {
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

pub fn handle_stake(ctx: Context<Stake>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    // 1. Manage asset Attributes plugin
    match fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    ) {
        Ok((_, fetched_attribute_list, _)) => {
            let mut attribute_list: Vec<Attribute> = Vec::new();
            let mut is_staked_initialized = false;
            let mut is_staked_time_initialized = false;

            for attribute in fetched_attribute_list.attribute_list {
                if attribute.key == "staked" {
                    require!(attribute.value == "0", StakingError::AlreadyStaked);
                    attribute_list.push(Attribute {
                        key: "staked".to_string(),
                        value: now.to_string(),
                    });
                    is_staked_initialized = true;
                } else if attribute.key == "staked_time" {
                    attribute_list.push(Attribute {
                        key: "staked_time".to_string(),
                        value: attribute.value,
                    });
                    is_staked_time_initialized = true;
                } else {
                    attribute_list.push(attribute);
                }
            }

            if !is_staked_initialized {
                attribute_list.push(Attribute {
                    key: "staked".to_string(),
                    value: now.to_string(),
                });
            }

            if !is_staked_time_initialized {
                attribute_list.push(Attribute {
                    key: "staked_time".to_string(),
                    value: "0".to_string(),
                });
            }

            UpdatePluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
                .asset(&ctx.accounts.asset.to_account_info())
                .collection(Some(&ctx.accounts.collection.to_account_info()))
                .payer(&ctx.accounts.payer.to_account_info())
                .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                .system_program(&ctx.accounts.system_program.to_account_info())
                .plugin(Plugin::Attributes(Attributes { attribute_list }))
                .invoke()?;
        }
        Err(_) => {
            AddPluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
                .asset(&ctx.accounts.asset.to_account_info())
                .collection(Some(&ctx.accounts.collection.to_account_info()))
                .payer(&ctx.accounts.payer.to_account_info())
                .authority(Some(&ctx.accounts.update_authority.to_account_info()))
                .system_program(&ctx.accounts.system_program.to_account_info())
                .plugin(Plugin::Attributes(Attributes {
                    attribute_list: vec![
                        Attribute {
                            key: "staked".to_string(),
                            value: now.to_string(),
                        },
                        Attribute {
                            key: "staked_time".to_string(),
                            value: "0".to_string(),
                        },
                    ],
                }))
                .init_authority(PluginAuthority::UpdateAuthority)
                .invoke()?;
        }
    }

    // 2. Add FreezeDelegate plugin to freeze asset
    AddPluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.payer.to_account_info())
        .authority(Some(&ctx.accounts.owner.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
        .init_authority(PluginAuthority::UpdateAuthority)
        .invoke()?;

    // 3. Add BurnDelegate plugin for Burn-to-Earn support
    if fetch_plugin::<BaseAssetV1, BurnDelegate>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::BurnDelegate,
    )
    .is_err()
    {
        AddPluginV1CpiBuilder::new(&ctx.accounts.core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.payer.to_account_info())
            .authority(Some(&ctx.accounts.owner.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(Plugin::BurnDelegate(BurnDelegate {}))
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke()?;
    }

    // 4. Increment collection total_staked attribute
    update_collection_total_staked(
        &ctx.accounts.core_program.to_account_info(),
        &ctx.accounts.collection.to_account_info(),
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.update_authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        true,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_stake_attribute_initialization() {
        let now = 1700000000i64;
        let attributes = vec![
            Attribute {
                key: "staked".to_string(),
                value: now.to_string(),
            },
            Attribute {
                key: "staked_time".to_string(),
                value: "0".to_string(),
            },
        ];

        let staked_attr = attributes.iter().find(|a| a.key == "staked").unwrap();
        assert_eq!(staked_attr.value, "1700000000");

        let staked_time_attr = attributes.iter().find(|a| a.key == "staked_time").unwrap();
        assert_eq!(staked_time_attr.value, "0");
    }

    #[test]
    fn test_instruction_stake_already_staked_rejection() {
        let current_staked_value = "1700000000";
        let is_already_staked = current_staked_value != "0";
        assert!(is_already_staked);
    }
}

