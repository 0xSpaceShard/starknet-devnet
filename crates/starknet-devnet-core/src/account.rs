use std::sync::Arc;

use blockifier::state::state_api::StateReader;
use starknet_api::core::{calculate_contract_address, PatriciaKey};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_api::transaction::{Calldata, ContractAddressSalt};
use starknet_api::{patricia_key, stark_felt};
use starknet_rs_core::types::FieldElement;
use starknet_rs_core::utils::get_storage_var_address;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0Json, ContractClass};
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::error::Error;
use starknet_types::felt::{Balance, ClassHash, Felt, Key};
use starknet_types::num_bigint::BigUint;
use starknet_types::traits::HashProducer;

use crate::constants::{
    CAIRO_0_ACCOUNT_CONTRACT_PATH, CHARGEABLE_ACCOUNT_ADDRESS, CHARGEABLE_ACCOUNT_PRIVATE_KEY,
    CHARGEABLE_ACCOUNT_PUBLIC_KEY,
};
use crate::error::DevnetResult;
use crate::state::state_readers::DictState;
use crate::state::{CustomState, StarknetState};
use crate::traits::{Accounted, Deployed};

/// data taken from https://github.com/0xSpaceShard/starknet-devnet/blob/fb96e0cc3c1c31fb29892ecefd2a670cf8a32b51/starknet_devnet/account.py
const ACCOUNT_CLASS_HASH_HEX_FOR_ADDRESS_COMPUTATION: &str =
    "0x3FCBF77B28C96F4F2FB5BD2D176AB083A12A5E123ADEB0DE955D7EE228C9854";

pub enum FeeToken {
    ETH,
    STRK,
}

#[derive(Clone)]
pub struct Account {
    pub public_key: Key,
    pub private_key: Key,
    pub account_address: ContractAddress,
    pub initial_balance: Balance,
    pub class_hash: ClassHash,
    pub(crate) contract_class: ContractClass,
    pub(crate) eth_fee_token_address: ContractAddress,
    pub(crate) strk_fee_token_address: ContractAddress,
}

impl Account {
    pub(crate) fn new_chargeable(
        eth_fee_token_address: ContractAddress,
        strk_fee_token_address: ContractAddress,
    ) -> DevnetResult<Self> {
        let account_contract_class = Cairo0Json::raw_json_from_path(CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
        let class_hash = account_contract_class.generate_hash()?;

        // insanely big - should practically never run out of funds
        let initial_balance = BigUint::from(u128::MAX);
        let initial_balance_hex = format!("0x{}", initial_balance.to_str_radix(16));
        Ok(Self {
            public_key: Key::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_PUBLIC_KEY).unwrap(),
            private_key: Key::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_PRIVATE_KEY).unwrap(),
            account_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(CHARGEABLE_ACCOUNT_ADDRESS).unwrap(),
            )
            .unwrap(),
            initial_balance: Felt::from_prefixed_hex_str(&initial_balance_hex).unwrap(),
            class_hash,
            contract_class: account_contract_class.into(),
            eth_fee_token_address,
            strk_fee_token_address,
        })
    }

    pub(crate) fn new(
        initial_balance: Balance,
        public_key: Key,
        private_key: Key,
        class_hash: ClassHash,
        contract_class: ContractClass,
        eth_fee_token_address: ContractAddress,
        strk_fee_token_address: ContractAddress,
    ) -> DevnetResult<Self> {
        Ok(Self {
            initial_balance,
            public_key,
            private_key,
            class_hash,
            contract_class,
            account_address: Account::compute_account_address(&public_key)?,
            eth_fee_token_address,
            strk_fee_token_address,
        })
    }

    fn compute_account_address(public_key: &Key) -> DevnetResult<ContractAddress> {
        let account_address = calculate_contract_address(
            ContractAddressSalt(stark_felt!(20u32)),
            Felt::from_prefixed_hex_str(ACCOUNT_CLASS_HASH_HEX_FOR_ADDRESS_COMPUTATION)?.into(),
            &Calldata(Arc::new(vec![(*public_key).into()])),
            starknet_api::core::ContractAddress(patricia_key!(0u32)),
        )
        .map_err(Error::StarknetApiError)?;

        Ok(ContractAddress::from(account_address))
    }

    fn eth_balance_storage_key(&self) -> DevnetResult<ContractStorageKey> {
        let storage_var_address = starknet_types::patricia_key::PatriciaKey::new(Felt::new(
            get_storage_var_address("ERC20_balances", &[FieldElement::from(self.account_address)])
                .map_err(|_| Error::ProgramError)?
                .to_bytes_be(),
        )?)?;
        Ok(ContractStorageKey::new(self.eth_fee_token_address, storage_var_address))
    }

    fn strk_balance_storage_key(&self) -> DevnetResult<ContractStorageKey> {
        let storage_var_address = starknet_types::patricia_key::PatriciaKey::new(Felt::new(
            get_storage_var_address("ERC20_balances", &[FieldElement::from(self.account_address)])
                .map_err(|_| Error::ProgramError)?
                .to_bytes_be(),
        )?)?;
        Ok(ContractStorageKey::new(self.strk_fee_token_address, storage_var_address))
    }
}

impl Deployed for Account {
    fn deploy(&self, state: &mut StarknetState) -> DevnetResult<()> {
        self.declare_if_undeclared(state, self.class_hash, &self.contract_class)?;

        state.predeploy_contract(self.account_address, self.class_hash)?;

        // set public key directly in the most underlying state
        let public_key_storage_var = starknet_types::patricia_key::PatriciaKey::new(Felt::new(
            get_storage_var_address("Account_public_key", &[])
                .map_err(|_| Error::ProgramError)?
                .to_bytes_be(),
        )?)?;

        state.state.state.set_storage_at(
            self.account_address.try_into()?,
            public_key_storage_var.try_into()?,
            self.public_key.into(),
        )?;

        // set balance directly in the most underlying state
        self.set_initial_balance(&mut state.state.state)?;

        Ok(())
    }

    fn get_address(&self) -> ContractAddress {
        self.account_address
    }
}

impl Accounted for Account {
    fn set_initial_balance(&self, state: &mut DictState) -> DevnetResult<()> {
        let storage_var_address = starknet_types::patricia_key::PatriciaKey::new(Felt::new(
            get_storage_var_address("ERC20_balances", &[FieldElement::from(self.account_address)])
                .map_err(|_| Error::ProgramError)?
                .to_bytes_be(),
        )?)?;

        for fee_token_address in [self.eth_fee_token_address, self.strk_fee_token_address] {
            state.set_storage_at(
                fee_token_address.try_into()?,
                storage_var_address.try_into()?,
                self.initial_balance.into(),
            )?;
        }

        Ok(())
    }

    fn get_balance(&self, state: &mut impl StateReader, token: FeeToken) -> DevnetResult<Balance> {
        let balance_storage_key = match token {
            FeeToken::ETH => self.eth_balance_storage_key()?,
            FeeToken::STRK => self.strk_balance_storage_key()?,
        };
        let balance = state.get_storage_at(
            (*balance_storage_key.get_contract_address()).try_into()?,
            (*balance_storage_key.get_storage_key()).try_into()?,
        )?;
        Ok(balance.into())
    }
}

#[cfg(test)]
mod tests {
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_core::utils::get_storage_var_address;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;

    use super::Account;
    use crate::account::FeeToken;
    use crate::constants::CAIRO_1_ERC20_CONTRACT_CLASS_HASH;
    use crate::state::{CustomState, StarknetState};
    use crate::traits::{Accounted, Deployed};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{dummy_contract_address, dummy_felt};

    /// Testing if generated account address has the same value as the first account in
    /// https://github.com/0xSpaceShard/starknet-devnet/blob/9d867e38e6d465e568e82a47e82e40608f6d220f/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn account_address_should_be_equal() {
        let expected_result = ContractAddress::new(
            Felt::from_prefixed_hex_str(
                "0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502",
            )
            .unwrap(),
        )
        .unwrap();
        let generated_result = Account::compute_account_address(
            &Felt::from_prefixed_hex_str(
                "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b5",
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn account_address_should_not_be_equal() {
        let expected_result = ContractAddress::new(
            Felt::from_prefixed_hex_str(
                "0x6e3205f9b7c4328f00f718fdecf56ab31acfb3cd6ffeb999dcbac41236ea502",
            )
            .unwrap(),
        )
        .unwrap();
        let generated_result = Account::compute_account_address(
            &Felt::from_prefixed_hex_str(
                "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b6",
            )
            .unwrap(),
        )
        .unwrap();

        assert_ne!(expected_result, generated_result);
    }

    #[test]
    fn correct_balance_storage_key() {
        let default_felt = Felt::default();
        let fee_token_address =
            ContractAddress::new(Felt::from_prefixed_hex_str("0xFEEE").unwrap()).unwrap();
        // here we can use the same address for eth fee and strk fee, because we are not extracting
        // any data from the state of those addresses. Only checking if the storage key is correct
        let mut account = Account::new(
            default_felt,
            default_felt,
            default_felt,
            default_felt,
            dummy_cairo_0_contract_class().into(),
            fee_token_address,
            fee_token_address,
        )
        .unwrap();
        let account_address = ContractAddress::new(Felt::from(111)).unwrap();
        account.account_address = account_address;

        let expected_balance_storage_key = ContractStorageKey::new(
            fee_token_address,
            starknet_types::patricia_key::PatriciaKey::new(
                Felt::new(
                    get_storage_var_address(
                        "ERC20_balances",
                        &[FieldElement::from(account_address)],
                    )
                    .unwrap()
                    .to_bytes_be(),
                )
                .unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(expected_balance_storage_key, account.eth_balance_storage_key().unwrap());
    }

    #[test]
    fn account_deployed_successfully() {
        let (account, mut state) = setup();
        assert!(account.deploy(&mut state).is_ok());
    }

    #[test]
    fn account_get_balance_should_return_correct_value() {
        let (mut account, mut state) = setup();
        let expected_balance = Felt::from(100);
        account.initial_balance = expected_balance;
        account.deploy(&mut state).unwrap();
        let generated_balance = account.get_balance(&mut state, FeeToken::ETH).unwrap();

        assert_eq!(expected_balance, generated_balance);

        let generated_balance = account.get_balance(&mut state, FeeToken::STRK).unwrap();

        assert_eq!(expected_balance, generated_balance);
    }

    #[test]
    fn account_changed_balance_successfully_without_deployment() {
        let (account, mut state) = setup();
        assert!(account.set_initial_balance(&mut state.state.state).is_ok());
    }

    #[test]
    fn account_get_address_correct() {
        let (mut account, _) = setup();
        let expected_address = ContractAddress::new(Felt::from(11111)).unwrap();
        account.account_address = expected_address;
        assert_eq!(expected_address, account.get_address());
    }

    fn setup() -> (Account, StarknetState) {
        let mut state = StarknetState::default();
        let fee_token_address = dummy_contract_address();

        // deploy the erc20 contract
        state
            .predeploy_contract(
                fee_token_address,
                Felt::from_prefixed_hex_str(CAIRO_1_ERC20_CONTRACT_CLASS_HASH).unwrap(),
            )
            .unwrap();

        (
            Account::new(
                Felt::from(10),
                Felt::from(13431515),
                Felt::from(11),
                dummy_felt(),
                dummy_cairo_0_contract_class().into(),
                fee_token_address,
                fee_token_address,
            )
            .unwrap(),
            state,
        )
    }
}
