use std::num::NonZeroU128;

use clap::Error;
use serde::{Serialize, Serializer};
use starknet_rs_core::types::Felt;
use starknet_types::chain_id::ChainId;
use starknet_types::contract_class::ContractClass;
use starknet_types::rpc::state::Balance;
use starknet_types::traits::HashProducer;
use url::Url;

use crate::constants::{
    CAIRO_1_ACCOUNT_CONTRACT_SIERRA, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_DATA_GAS_PRICE,
    DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_INITIAL_BALANCE, DEVNET_DEFAULT_TEST_SEED,
    DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DumpOn {
    Exit,
    Block,
    Request,
}

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
#[clap(rename_all = "snake_case")]
pub enum StateArchiveCapacity {
    #[default]
    None,
    Full,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockGenerationOn {
    Transaction,
    Demand,
    Interval(u64),
}

impl std::str::FromStr for BlockGenerationOn {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transaction" => Ok(BlockGenerationOn::Transaction),
            "demand" => Ok(BlockGenerationOn::Demand),
            value => {
                let interval_value = value
                    .parse::<u64>()
                    .map_err(|_| Error::new(clap::error::ErrorKind::InvalidValue))?;

                if interval_value > 0 {
                    Ok(BlockGenerationOn::Interval(interval_value))
                } else {
                    Err(Error::new(clap::error::ErrorKind::InvalidValue))
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ForkConfig {
    #[serde(serialize_with = "serialize_config_url")]
    pub url: Option<Url>,
    pub block_number: Option<u64>,
}

pub fn serialize_config_url<S>(url: &Option<Url>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match url {
        Some(url) => serializer.serialize_str(url.as_ref()),
        None => serializer.serialize_none(),
    }
}

pub fn serialize_initial_balance<S>(balance: &Balance, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&balance.to_str_radix(10))
}

pub fn serialize_chain_id<S>(chain_id: &ChainId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{chain_id}"))
}

#[derive(Clone, Debug, Serialize)]
pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    #[serde(skip_serializing)]
    pub account_contract_class: ContractClass,
    pub account_contract_class_hash: Felt,
    #[serde(serialize_with = "serialize_initial_balance")]
    pub predeployed_accounts_initial_balance: Balance,
    pub start_time: Option<u64>,
    pub gas_price_wei: NonZeroU128,
    pub gas_price_fri: NonZeroU128,
    pub data_gas_price_wei: NonZeroU128,
    pub data_gas_price_fri: NonZeroU128,
    #[serde(serialize_with = "serialize_chain_id")]
    pub chain_id: ChainId,
    pub dump_on: Option<DumpOn>,
    pub dump_path: Option<String>,
    pub block_generation_on: BlockGenerationOn,
    pub lite_mode: bool,
    pub state_archive: StateArchiveCapacity,
    pub fork_config: ForkConfig,
    pub disable_account_impersonation: bool,
}

impl Default for StarknetConfig {
    fn default() -> Self {
        let account_contract_class: ContractClass =
            ContractClass::cairo_1_from_sierra_json_str(CAIRO_1_ACCOUNT_CONTRACT_SIERRA)
                .unwrap()
                .into();
        StarknetConfig {
            seed: DEVNET_DEFAULT_TEST_SEED,
            total_accounts: DEVNET_DEFAULT_TOTAL_ACCOUNTS,
            account_contract_class_hash: account_contract_class.generate_hash().unwrap(),
            account_contract_class,
            predeployed_accounts_initial_balance: DEVNET_DEFAULT_INITIAL_BALANCE.into(),
            start_time: None,
            gas_price_wei: DEVNET_DEFAULT_GAS_PRICE,
            gas_price_fri: DEVNET_DEFAULT_GAS_PRICE,
            data_gas_price_wei: DEVNET_DEFAULT_DATA_GAS_PRICE,
            data_gas_price_fri: DEVNET_DEFAULT_DATA_GAS_PRICE,
            chain_id: DEVNET_DEFAULT_CHAIN_ID,
            dump_on: None,
            dump_path: None,
            block_generation_on: BlockGenerationOn::Transaction,
            lite_mode: false,
            state_archive: StateArchiveCapacity::default(),
            fork_config: ForkConfig::default(),
            disable_account_impersonation: false,
        }
    }
}
