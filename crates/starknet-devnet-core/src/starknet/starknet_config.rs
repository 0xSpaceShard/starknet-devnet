use std::net::IpAddr;

use starknet_types::chain_id::ChainId;
use starknet_types::contract_class::{Cairo0ContractClass, Cairo0Json, ContractClass};
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

use crate::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_PATH, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_GAS_PRICE,
    DEVNET_DEFAULT_HOST, DEVNET_DEFAULT_INITIAL_BALANCE, DEVNET_DEFAULT_PORT,
    DEVNET_DEFAULT_TEST_SEED, DEVNET_DEFAULT_TIMEOUT, DEVNET_DEFAULT_TOTAL_ACCOUNTS,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum DumpOn {
    Exit,
    Transaction,
}

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum StateArchiveCapacity {
    #[default]
    #[clap(name = "none")]
    None,
    #[clap(name = "full")]
    Full,
}

#[derive(Clone, Debug)]
pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    pub account_contract_class: ContractClass,
    pub account_contract_class_hash: Felt,
    pub predeployed_accounts_initial_balance: Felt,
    pub host: IpAddr,
    pub port: u16,
    pub start_time: Option<u64>,
    pub timeout: u16,
    pub gas_price: u64,
    pub chain_id: ChainId,
    pub dump_on: Option<DumpOn>,
    pub dump_path: Option<String>,
    /// on initialization, re-execute loaded txs (if any)
    pub re_execute_on_init: bool,
    pub state_archive: StateArchiveCapacity,
}

impl Default for StarknetConfig {
    fn default() -> Self {
        let account_contract_class =
            Cairo0Json::raw_json_from_path(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        StarknetConfig {
            seed: DEVNET_DEFAULT_TEST_SEED,
            total_accounts: DEVNET_DEFAULT_TOTAL_ACCOUNTS,
            account_contract_class_hash: account_contract_class.generate_hash().unwrap(),
            account_contract_class: ContractClass::Cairo0(Cairo0ContractClass::RawJson(
                account_contract_class,
            )),
            predeployed_accounts_initial_balance: DEVNET_DEFAULT_INITIAL_BALANCE.into(),
            host: DEVNET_DEFAULT_HOST,
            port: DEVNET_DEFAULT_PORT,
            start_time: None,
            timeout: DEVNET_DEFAULT_TIMEOUT,
            gas_price: DEVNET_DEFAULT_GAS_PRICE,
            chain_id: DEVNET_DEFAULT_CHAIN_ID,
            dump_on: None,
            dump_path: None,
            re_execute_on_init: true,
            state_archive: StateArchiveCapacity::default(),
        }
    }
}
