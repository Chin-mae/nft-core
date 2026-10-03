use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseCollectionV1,
    fetch_plugin,
    instructions::{AddCollectionPluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder},
    types::{Attribute, Attributes, Plugin, PluginAuthority, PluginType},
};

use crate::error::StakingError;

pub fn update_collection_total_staked<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    increment: bool,
) -> Result<()> {
    match fetch_plugin::<BaseCollectionV1, Attributes>(collection, PluginType::Attributes) {
        Ok((_, fetched_attributes, _)) => {
            let mut attribute_list: Vec<Attribute> = Vec::new();
            let mut found = false;

            for attr in fetched_attributes.attribute_list {
                if attr.key == "total_staked" {
                    found = true;
                    let current_count: u64 = attr.value.parse().unwrap_or(0);
                    let new_count = if increment {
                        current_count.saturating_add(1)
                    } else {
                        current_count.saturating_sub(1)
                    };
                    attribute_list.push(Attribute {
                        key: "total_staked".to_string(),
                        value: new_count.to_string(),
                    });
                } else {
                    attribute_list.push(attr);
                }
            }

            if !found {
                let new_count: u64 = if increment { 1 } else { 0 };
                attribute_list.push(Attribute {
                    key: "total_staked".to_string(),
                    value: new_count.to_string(),
                });
            }

            UpdateCollectionPluginV1CpiBuilder::new(core_program)
                .collection(collection)
                .payer(payer)
                .authority(Some(authority))
                .system_program(system_program)
                .plugin(Plugin::Attributes(Attributes { attribute_list }))
                .invoke()?;
        }
        Err(_) => {
            let initial_count: u64 = if increment { 1 } else { 0 };
            AddCollectionPluginV1CpiBuilder::new(core_program)
                .collection(collection)
                .payer(payer)
                .authority(Some(authority))
                .system_program(system_program)
                .plugin(Plugin::Attributes(Attributes {
                    attribute_list: vec![Attribute {
                        key: "total_staked".to_string(),
                        value: initial_count.to_string(),
                    }],
                }))
                .init_authority(PluginAuthority::UpdateAuthority)
                .invoke()?;
        }
    }

    Ok(())
}

pub fn calculate_staking_rewards(
    staked_timestamp: i64,
    staked_time_accumulated: u64,
    current_timestamp: i64,
    reward_rate: u64,
) -> Result<u64> {
    if staked_timestamp <= 0 {
        return err!(StakingError::NotStaked);
    }

    let elapsed = current_timestamp
        .checked_sub(staked_timestamp)
        .ok_or(StakingError::ArithmeticOverflow)?;

    let elapsed_u64 = if elapsed > 0 { elapsed as u64 } else { 0 };

    let total_time = elapsed_u64
        .checked_add(staked_time_accumulated)
        .ok_or(StakingError::ArithmeticOverflow)?;

    let rewards = total_time
        .checked_mul(reward_rate)
        .ok_or(StakingError::ArithmeticOverflow)?;

    Ok(rewards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_staking_rewards() {
        let staked_at = 1000;
        let now = 1060; // 60 seconds later
        let rate = 2;   // 2 tokens per second
        let rewards = calculate_staking_rewards(staked_at, 0, now, rate).unwrap();
        assert_eq!(rewards, 120);
    }

    #[test]
    fn test_calculate_staking_rewards_with_accumulated() {
        let staked_at = 1000;
        let accumulated = 40; // 40 seconds previously accumulated
        let now = 1060;       // 60 seconds current session
        let rate = 1;
        let rewards = calculate_staking_rewards(staked_at, accumulated, now, rate).unwrap();
        assert_eq!(rewards, 100);
    }

    #[test]
    fn test_not_staked_error() {
        let res = calculate_staking_rewards(0, 0, 1000, 1);
        assert!(res.is_err());
    }

    #[test]
    fn test_fixed_burn_bonus() {
        let staked_at = 1000;
        let now = 1100; // 100 seconds
        let rate = 1;
        let accumulated = calculate_staking_rewards(staked_at, 0, now, rate).unwrap();
        assert_eq!(accumulated, 100);
        let total_burn_reward = accumulated + crate::constants::BURN_BONUS_TOKENS;
        assert_eq!(total_burn_reward, 10_000_100);
    }
}

