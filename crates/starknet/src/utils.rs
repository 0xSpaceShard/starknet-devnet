use cairo_felt::Felt252;
use starknet_api::hash::{pedersen_hash, StarkFelt};
use starknet_types::felt::Felt;
use starknet_types::num_integer::Integer;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};

use crate::error::DevnetResult;

pub(crate) fn generate_u128_random_numbers(
    seed: u32,
    random_numbers_count: u8,
) -> DevnetResult<Vec<u128>> {
    Ok(random_number_generator::generate_u128_random_numbers(seed, random_numbers_count))
}

/// Returns the storage address of a Starknet storage variable given its name and arguments.
pub(crate) fn get_storage_var_address(
    storage_var_name: &str,
    args: &[Felt],
) -> DevnetResult<StorageKey> {
    let storage_var_name_hash =
        starknet_rs_core::utils::starknet_keccak(storage_var_name.as_bytes());
    let storage_var_name_hash = StarkFelt::new(storage_var_name_hash.to_bytes_be())?;

    let storage_key_hash = args
        .iter()
        .fold(storage_var_name_hash, |res, arg| pedersen_hash(&res, &StarkFelt::from(arg)));

    let storage_key = Felt252::from_bytes_be(storage_key_hash.bytes()).mod_floor(
        &Felt252::from_bytes_be(&starknet_api::core::L2_ADDRESS_UPPER_BOUND.to_bytes_be()),
    );

    Ok(PatriciaKey::new(Felt::new(storage_key.to_be_bytes())?)?)
}

#[cfg(test)]
pub(crate) mod test_utils {
    use cairo_lang_starknet::casm_contract_class::CasmContractClass;
    use cairo_lang_starknet::contract_class::ContractClass as SierraContractClass;
    use starknet_api::transaction::Fee;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{
        compute_casm_class_hash, Cairo0ContractClass, Cairo0Json, ContractClass,
    };
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::Felt;
    use starknet_types::patricia_key::StorageKey;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
    use starknet_types::traits::HashProducer;

    use crate::constants::DEVNET_DEFAULT_CHAIN_ID;
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;

    pub(crate) fn dummy_felt() -> Felt {
        Felt::from_prefixed_hex_str("0xDD10").unwrap()
    }

    pub(crate) fn dummy_contract_storage_key() -> ContractStorageKey {
        ContractStorageKey::new(
            ContractAddress::new(Felt::from_prefixed_hex_str("0xFE").unwrap()).unwrap(),
            StorageKey::try_from(dummy_felt()).unwrap(),
        )
    }

    pub(crate) fn dummy_cairo_1_contract_class() -> SierraContractClass {
        let json_str = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/cairo_1_test.json"
        ))
        .unwrap();

        ContractClass::cairo_1_from_sierra_json_str(&json_str).unwrap()
    }

    pub(crate) fn dummy_contract_address() -> ContractAddress {
        ContractAddress::new(Felt::from_prefixed_hex_str("0xADD4E55").unwrap()).unwrap()
    }

    pub(crate) fn dummy_declare_transaction_v1() -> DeclareTransactionV0V1 {
        let chain_id = DEVNET_DEFAULT_CHAIN_ID.to_felt();
        let contract_class = dummy_cairo_0_contract_class();
        let broadcasted_tx = BroadcastedDeclareTransactionV1::new(
            dummy_contract_address(),
            Fee(100),
            &vec![],
            dummy_felt(),
            &contract_class.clone().into(),
            Felt::from(1),
        );
        let class_hash = contract_class.generate_hash().unwrap();
        let transaction_hash =
            broadcasted_tx.calculate_transaction_hash(&chain_id, &class_hash).unwrap();

        broadcasted_tx.create_declare(class_hash, transaction_hash)
    }

    pub(crate) fn dummy_broadcasted_declare_transaction_v2(
        sender_address: &ContractAddress,
    ) -> BroadcastedDeclareTransactionV2 {
        let contract_class = dummy_cairo_1_contract_class();

        let compiled_class_hash = compute_casm_class_hash(
            &CasmContractClass::from_contract_class(contract_class.clone(), true).unwrap(),
        )
        .unwrap();

        BroadcastedDeclareTransactionV2::new(
            &contract_class,
            compiled_class_hash,
            *sender_address,
            Fee(4000),
            &Vec::new(),
            Felt::from(0),
            Felt::from(2),
        )
    }

    pub(crate) fn cairo_0_account_without_validations() -> Cairo0ContractClass {
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );

        Cairo0Json::raw_json_from_path(account_json_path).unwrap().into()
    }

    pub(crate) fn get_bytes_from_u32(num: u32) -> [u8; 32] {
        let num_bytes = num.to_be_bytes();
        let mut result = [0u8; 32];
        let starting_idx = result.len() - num_bytes.len();
        let ending_idx = result.len();

        result[starting_idx..ending_idx].copy_from_slice(&num_bytes[..(ending_idx - starting_idx)]);

        result
    }
}

#[cfg(any(test, feature = "test_utils"))]
pub mod exported_test_utils {
    use starknet_types::contract_class::Cairo0Json;

    pub fn dummy_cairo_l1l2_contract() -> Cairo0Json {
        let json_str = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/cairo_0_l1l2.json"
        ))
        .unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap()
    }

    pub fn dummy_cairo_0_contract_class() -> Cairo0Json {
        let json_str = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/cairo_0_test.json"
        ))
        .unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use starknet_types::traits::ToHexString;

    use super::get_storage_var_address;
    use super::test_utils::{self, get_bytes_from_u32};

    #[test]
    fn correct_bytes_from_number() {
        let result = get_bytes_from_u32(123);
        assert!(result[31] == 123)
    }

    #[test]
    fn correct_number_generated_based_on_fixed_seed() {
        let generated_numbers = random_number_generator::generate_u128_random_numbers(123, 2);
        let expected_output: Vec<u128> =
            vec![261662301160200998434711212977610535782, 285327960644938307249498422906269531911];
        assert_eq!(generated_numbers, expected_output);
    }

    #[test]
    fn correct_simple_storage_var_address_generated() {
        let expected_storage_var_address =
            blockifier::abi::abi_utils::get_storage_var_address("simple", &[]);
        let generated_storage_var_address = get_storage_var_address("simple", &[]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().bytes(),
            generated_storage_var_address.to_felt().bytes()
        );
    }

    #[test]
    fn correct_complex_storage_var_address_generated() {
        let prefixed_hex_felt_string = test_utils::dummy_felt().to_prefixed_hex_str();

        let expected_storage_var_address = blockifier::abi::abi_utils::get_storage_var_address(
            "complex",
            &[prefixed_hex_felt_string.as_str().try_into().unwrap()],
        );

        let generated_storage_var_address =
            get_storage_var_address("complex", &[test_utils::dummy_felt()]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().bytes(),
            generated_storage_var_address.to_felt().bytes()
        );
    }
}
