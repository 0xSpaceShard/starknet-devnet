use starknet_rs_ff::FieldElement;
use starknet_rs_signers::SigningKey;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, Key};

use crate::account::Account;
use crate::error::DevnetResult;
use crate::traits::AccountGenerator;
use crate::utils::generate_u128_random_numbers;

#[derive(Default)]
pub(crate) struct PredeployedAccounts {
    seed: u32,
    initial_balance: Felt,
    fee_token_address: ContractAddress,
    accounts: Vec<Account>,
}

impl PredeployedAccounts {
    pub(crate) fn new(
        seed: u32,
        initial_balance: Felt,
        fee_token_address: ContractAddress,
    ) -> Self {
        Self { seed, initial_balance, fee_token_address, accounts: Vec::new() }
    }
}

impl PredeployedAccounts {
    fn generate_private_keys(&self, number_of_accounts: u8) -> DevnetResult<Vec<Key>> {
        let random_numbers = generate_u128_random_numbers(self.seed, number_of_accounts)?;
        let private_keys = random_numbers.into_iter().map(Key::from).collect::<Vec<Key>>();

        Ok(private_keys)
    }

    fn generate_public_key(&self, private_key: &Key) -> DevnetResult<Key> {
        let private_key_field_element = FieldElement::from(*private_key);

        let public_key = Key::from(
            SigningKey::from_secret_scalar(private_key_field_element).verifying_key().scalar(),
        );

        Ok(public_key)
    }

    pub fn get_accounts(&self) -> &Vec<Account> {
        &self.accounts
    }
}

impl AccountGenerator for PredeployedAccounts {
    type Acc = Account;

    fn generate_accounts(
        &mut self,
        number_of_accounts: u8,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<&Vec<Self::Acc>> {
        let private_keys = self.generate_private_keys(number_of_accounts)?;

        for private_key in private_keys {
            let account = Account::new(
                self.initial_balance,
                self.generate_public_key(&private_key)?,
                private_key,
                class_hash,
                contract_class.clone(),
                self.fee_token_address,
            )?;
            self.accounts.push(account);
        }

        Ok(&self.accounts)
    }
}

#[cfg(test)]
mod tests {
    use jsonschema::JSONSchema;
    use serde_json::json;
    use starknet_types::contract_class::Cairo0Json;
    use starknet_types::felt::{Felt, Key};
    use starknet_types::traits::{ToDecimalString, ToHexString};

    use crate::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH};
    use crate::predeployed_accounts::PredeployedAccounts;
    use crate::traits::AccountGenerator;
    use crate::utils::test_utils::dummy_contract_address;

    const SEED: u32 = 123;

    const PRIVATE_KEYS_IN_HEX: [&str; 3] = [
        "0xc4da537c1651ddae44867db30d67b366",
        "0xd6a82a951b923e0a443cdef36840ff07",
        "0x610e4ad509c47055dff4948fe6b4f832",
    ];

    const PUBLIC_KEYS_IN_HEX: [&str; 3] = [
        "0x60dea6c1228f1db4ca1f9db11c01b6e9cce5e627f7181dcaa27d69cbdbe57b5",
        "0x5a91f0ea25312accb20d8041b12260bff31a3490e5730a690b0ec8fe10ffbb",
        "0x2b45af00df833ea1a4895c49a18ebd84309b79d658cae05e274a7b1cac47016",
    ];

    const PREDEPLOYED_ACCOUNTS_JSON_SCHEMA_WITH_DATA_FILE_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/predeployed_accounts_fixed_seed.json");

    /// Test correct generation of private keys
    /// Test expected results are taken from https://github.com/0xSpaceShard/starknet-devnet/blob/master/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn correct_private_key_from_fixed_seed() {
        let predeployed_acc = predeployed_account_instance();
        let generated_private_keys = predeployed_acc.generate_private_keys(3).unwrap();

        let expected_result = PRIVATE_KEYS_IN_HEX
            .into_iter()
            .map(|hex_str| Felt::from_prefixed_hex_str(hex_str).unwrap())
            .collect::<Vec<Felt>>();

        assert_eq!(expected_result, generated_private_keys);

        let string_result = generated_private_keys
            .into_iter()
            .map(|pk| pk.to_prefixed_hex_str())
            .collect::<Vec<String>>();

        assert_eq!(string_result, PRIVATE_KEYS_IN_HEX.to_vec());
    }

    #[test]
    fn private_key_from_different_seeds_should_be_different() {
        let predeployed_acc =
            PredeployedAccounts::new(999, Felt::from(1), dummy_contract_address());
        let generated_private_key = predeployed_acc.generate_private_keys(1).unwrap()[0];

        let non_expected_result = Felt::from_prefixed_hex_str(PRIVATE_KEYS_IN_HEX[0]).unwrap();

        assert_ne!(generated_private_key, non_expected_result);
    }

    /// Test correct generation of public keys
    /// Test expected results are taken from https://github.com/0xSpaceShard/starknet-devnet/blob/master/test/support/schemas/predeployed_accounts_fixed_seed.json
    #[test]
    fn correct_public_key_from_private_key() {
        let predeployed_acc = predeployed_account_instance();
        let generated_private_keys = predeployed_acc.generate_private_keys(3).unwrap();

        let generated_public_keys = generated_private_keys
            .into_iter()
            .map(|pk| predeployed_acc.generate_public_key(&pk).unwrap())
            .collect::<Vec<Key>>();

        let expected_public_keys = PUBLIC_KEYS_IN_HEX
            .into_iter()
            .map(|public_key_hex| Felt::from_prefixed_hex_str(public_key_hex).unwrap())
            .collect::<Vec<Key>>();

        assert_eq!(generated_public_keys, expected_public_keys);
    }

    #[test]
    fn check_generated_predeployed_accounts_against_json_schema() {
        let mut predeployed_acc =
            PredeployedAccounts::new(123, Felt::from(1000), dummy_contract_address());
        let class_hash = Felt::from_prefixed_hex_str(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap();
        let json_str = std::fs::read_to_string(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();

        let contract_class = Cairo0Json::raw_json_from_json_str(&json_str).unwrap();

        let generated_accounts =
            predeployed_acc.generate_accounts(3, class_hash, contract_class.into()).unwrap();
        let schema_json: serde_json::Value = serde_json::from_str(
            std::fs::read_to_string(PREDEPLOYED_ACCOUNTS_JSON_SCHEMA_WITH_DATA_FILE_PATH)
                .unwrap()
                .as_str(),
        )
        .unwrap();
        let schema = JSONSchema::compile(&schema_json).unwrap();

        let generated_accounts_json = generated_accounts
            .iter()
            .map(|acc| {
                json!({
                    "address": acc.account_address.to_prefixed_hex_str(),
                    "initial_balance": acc.initial_balance.to_decimal_string(),
                    "private_key": acc.private_key.to_prefixed_hex_str(),
                    "public_key": acc.public_key.to_prefixed_hex_str()
                })
            })
            .collect::<Vec<serde_json::Value>>();

        assert!(schema.is_valid(&serde_json::to_value(&generated_accounts_json).unwrap()));
    }

    fn predeployed_account_instance() -> PredeployedAccounts {
        PredeployedAccounts::new(SEED, Felt::from(100), dummy_contract_address())
    }
}
