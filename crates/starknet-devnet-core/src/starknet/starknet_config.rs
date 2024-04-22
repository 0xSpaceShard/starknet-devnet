use std::num::NonZeroU128;

use starknet_types::chain_id::ChainId;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::Felt;
use starknet_types::rpc::state::Balance;
use starknet_types::traits::HashProducer;
use url::Url;

use crate::constants::{
    CAIRO_1_ACCOUNT_CONTRACT_SIERRA, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_DATA_GAS_PRICE,
    DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_INITIAL_BALANCE, DEVNET_DEFAULT_TEST_SEED,
    DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum DumpOn {
    Exit,
    Block,
}

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum StateArchiveCapacity {
    #[default]
    #[clap(name = "none")]
    None,
    #[clap(name = "full")]
    Full,
}

#[derive(Debug, Clone, Default)]
pub struct ForkConfig {
    pub url: Option<Url>,
    pub block_number: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    pub account_contract_class: ContractClass,
    pub account_contract_class_hash: Felt,
    pub predeployed_accounts_initial_balance: Balance,
    pub start_time: Option<u64>,
    pub gas_price: NonZeroU128,
    pub data_gas_price: NonZeroU128,
    pub chain_id: ChainId,
    pub dump_on: Option<DumpOn>,
    pub dump_path: Option<String>,
    pub blocks_on_demand: bool,
    /// on initialization, re-execute loaded txs (if any)
    pub re_execute_on_init: bool,
    pub state_archive: StateArchiveCapacity,
    pub fork_config: ForkConfig,
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
            gas_price: DEVNET_DEFAULT_GAS_PRICE,
            data_gas_price: DEVNET_DEFAULT_DATA_GAS_PRICE,
            chain_id: DEVNET_DEFAULT_CHAIN_ID,
            dump_on: None,
            dump_path: None,
            blocks_on_demand: false,
            re_execute_on_init: true,
            state_archive: StateArchiveCapacity::default(),
            fork_config: ForkConfig::default(),
        }
    }
}
