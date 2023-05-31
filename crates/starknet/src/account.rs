use std::sync::Arc;

use starknet_api::core::PatriciaKey;
use starknet_api::hash::StarkHash;

use starknet_api::{
    core::calculate_contract_address,
    hash::StarkFelt,
    patricia_key, stark_felt,
    transaction::{Calldata, ContractAddressSalt},
};
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass;

use crate::error::Error;
use crate::traits::{Accounted, StateChanger};
use crate::types::{Balance, ClassHash, ContractAddress, Key};

#[derive(Debug, Clone)]
pub struct Account {
    pub balance: Balance,
    pub class_hash: ClassHash,
    pub public_key: Key,
    pub private_key: Key,
    pub contract_class: ContractClass,
    pub account_address: ContractAddress,
}

impl Account {
    fn new(
        balance: Balance,
        public_key: Key,
        private_key: Key,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> Result<Self, Error> {
        let account_address = calculate_contract_address(
            ContractAddressSalt(stark_felt!(20u32)),
            class_hash.into(),
            &Calldata(Arc::new(vec![public_key.into()])),
            starknet_api::core::ContractAddress(patricia_key!(0u32)),
        )
        .map_err(Error::StarknetApiError)?;

        Ok(Self {
            balance,
            public_key,
            private_key,
            class_hash,
            contract_class,
            account_address: ContractAddress::from(account_address),
        })
    }
}

impl Accounted for Account {
    fn deploy(&self, _state: impl StateChanger) -> Result<(), Error> {
        Ok(())
    }

    fn declare(&self, state: &mut impl StateChanger) -> Result<(), Error> {
        state.declare_contract_class(self.class_hash, &self.contract_class)
    }
}
