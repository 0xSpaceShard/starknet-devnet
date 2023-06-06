use std::sync::Arc;

use starknet_api::core::PatriciaKey;
use starknet_api::hash::StarkHash;

use starknet_api::{
    core::{calculate_contract_address},
    hash::StarkFelt,
    patricia_key, stark_felt,
    transaction::{Calldata, ContractAddressSalt},
};

use crate::traits::{Accounted, StateChanger};
use starknet_types::{
    contract_address::ContractAddress,
    contract_class::ContractClass,
    error::Error,
    felt::{Balance, ClassHash, Felt, Key},
    DevnetResult,
};

/// in hex it equals 0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854
/// data taken from https://github.com/0xSpaceShard/starknet-devnet/blob/fb96e0cc3c1c31fb29892ecefd2a670cf8a32b51/starknet_devnet/account.py
const ACCOUNT_CLASS_HASH_FOR_ADDRESS_COMPUTATION: [u8;32] = [
    3, 252, 191, 119, 178, 140, 150, 244, 242, 251, 91, 210, 209, 118, 171, 8, 58, 18, 165, 225, 35, 173, 235, 13, 233,
    85, 215, 238, 34, 140, 152, 84,
];

#[derive(Debug, Clone)]
pub struct Account {
    pub(crate) balance: Balance,
    pub(crate) class_hash: ClassHash,
    pub(crate) public_key: Key,
    pub(crate) private_key: Key,
    pub(crate) contract_class: ContractClass,
    pub(crate) account_address: ContractAddress,
}

impl Account {
    pub(crate) fn new(
        balance: Balance,
        public_key: Key,
        private_key: Key,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<Self> {
        Ok(Self {
            balance,
            public_key,
            private_key,
            class_hash,
            contract_class,
            account_address: Account::compute_account_address(&public_key)?,
        })
    }

    fn compute_account_address(public_key: &Key) -> DevnetResult<ContractAddress> {
        let account_address = calculate_contract_address(
            ContractAddressSalt(stark_felt!(20u32)),
            Felt::new(ACCOUNT_CLASS_HASH_FOR_ADDRESS_COMPUTATION).into(),
            &Calldata(Arc::new(vec![(*public_key).into()])),
            starknet_api::core::ContractAddress(patricia_key!(0u32)),
        )
        .map_err(Error::StarknetApiError)?;

        Ok(ContractAddress::from(account_address))
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

#[cfg(test)]
mod tests {
    use starknet_types::{contract_address::ContractAddress, felt::Felt};

    use super::Account;

    /// Testing if generated account address has the same value as the first account in
    /// https://github.com/0xSpaceShard/starknet-devnet/blob/9d867e38e6d465e568e82a47e82e40608f6d220f/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn account_address_should_be_equal() {
        let expected_result = ContractAddress::new(
            Felt::from_prefixed_hex_str("0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502").unwrap(),
        );
        let generated_result = Account::compute_account_address(
            &Felt::from_prefixed_hex_str("0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b5").unwrap(),
        )
        .unwrap();

        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn account_address_should_not_be_equal() {
        let expected_result = ContractAddress::new(
            Felt::from_prefixed_hex_str("0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502").unwrap(),
        );
        let generated_result = Account::compute_account_address(
            &Felt::from_prefixed_hex_str("0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b6").unwrap(),
        )
        .unwrap();

        assert_ne!(expected_result, generated_result);
    }
}
