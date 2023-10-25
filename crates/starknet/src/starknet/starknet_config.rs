use std::net::IpAddr;

use starknet_types::chain_id::ChainId;
use starknet_types::contract_class::{Cairo0ContractClass, Cairo0Json, ContractClass};
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

use crate::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_PATH, DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_HOST,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum DumpOn {
    Exit,
    Transaction,
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
    pub timeout: u16,
    pub gas_price: u64,
    pub chain_id: ChainId,
    pub dump_on: Option<DumpOn>,
    pub dump_path: Option<String>,
    pub pending_block_timestamp_shift: u64,
}

impl Default for StarknetConfig {
    fn default() -> Self {
        let account_contract_class =
            Cairo0Json::raw_json_from_path(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        Self {
            seed: u32::default(),
            total_accounts: u8::default(),
            account_contract_class_hash: account_contract_class.generate_hash().unwrap(),
            account_contract_class: ContractClass::Cairo0(Cairo0ContractClass::RawJson(
                account_contract_class,
            )),
            predeployed_accounts_initial_balance: Felt::default(),
            host: DEVNET_DEFAULT_HOST,
            port: u16::default(),
            timeout: u16::default(),
            gas_price: Default::default(),
            chain_id: DEVNET_DEFAULT_CHAIN_ID,
            dump_on: None,
            dump_path: None,
            pending_block_timestamp_shift: 0,
        }
    }
}
