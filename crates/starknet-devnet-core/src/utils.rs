use blockifier::bouncer::{BouncerConfig, BouncerWeights, BuiltinCount};
use blockifier::versioned_constants::VersionedConstants;
use serde_json::Value;
use starknet_api::block::StarknetVersion;
use starknet_rs_core::types::contract::CompiledClass;
use starknet_rs_core::types::Felt;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};

use crate::error::{DevnetResult, Error};

pub mod random_number_generator {
    use rand::{thread_rng, Rng, SeedableRng};
    use rand_mt::Mt64;

    pub fn generate_u32_random_number() -> u32 {
        thread_rng().gen()
    }

    pub(crate) fn generate_u128_random_numbers(seed: u32, random_numbers_count: u8) -> Vec<u128> {
        let mut result: Vec<u128> = Vec::new();
        let mut rng: Mt64 = SeedableRng::seed_from_u64(seed as u64);

        for _ in 0..random_numbers_count {
            result.push(rng.gen());
        }

        result
    }
}

/// Returns the storage address of a Starknet storage variable given its name and arguments.
pub(crate) fn get_storage_var_address(
    storage_var_name: &str,
    args: &[Felt],
) -> DevnetResult<StorageKey> {
    let storage_var_address =
        starknet_rs_core::utils::get_storage_var_address(storage_var_name, args)
            .map_err(|err| crate::error::Error::UnexpectedInternalError { msg: err.to_string() })?;

    Ok(PatriciaKey::new(storage_var_address)?)
}

pub(crate) fn get_versioned_constants() -> VersionedConstants {
    #[allow(clippy::unwrap_used)] // TODO
    VersionedConstants::get(&StarknetVersion::V0_13_2).unwrap().clone()
}

/// Values not present here: https://docs.starknet.io/tools/limits-and-triggers/
/// Asked the blockifier team about the values, they provided them here:
/// https://spaceshard.slack.com/archives/C029F9AN8LX/p1721657837687799?thread_ts=1721400009.781699&cid=C029F9AN8LX
pub(crate) fn custom_bouncer_config() -> BouncerConfig {
    BouncerConfig {
        block_max_capacity: BouncerWeights {
            n_steps: 40_000_000,
            l1_gas: 4_950_000, // TODO
            sierra_gas: starknet_api::execution_resources::GasAmount(4_950_000),
            state_diff_size: 4_000,
            n_events: 5_000,
            builtin_count: BuiltinCount {
                pedersen: 1_250_000,
                poseidon: 1_250_000,
                range_check: 250_000,
                range_check96: 250_000,
                add_mod: 250_000,
                mul_mod: 250_000,
                ecdsa: 19_531,
                bitwise: 625_000,
                ec_op: 39_062,
                keccak: 19_531,
            },
            ..BouncerWeights::max()
        },
    }
}

/// Returns the hash of a compiled class.
/// # Arguments
/// * `casm_json` - The compiled class in JSON format.
pub fn calculate_casm_hash(casm_json: Value) -> DevnetResult<Felt> {
    serde_json::from_value::<CompiledClass>(casm_json)
        .map_err(|err| Error::DeserializationError { origin: err.to_string() })?
        .class_hash()
        .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })
}

#[macro_export]
macro_rules! nonzero_gas_price {
    ($value:expr) => {{
        let gas_price = starknet_api::block::GasPrice(($value).get());
        starknet_api::block::NonzeroGasPrice::new(gas_price).unwrap()
    }};
}

#[cfg(test)]
pub(crate) mod test_utils {
    use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
    use starknet_api::transaction::fields::Fee;
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::deprecated::json_contract_class::Cairo0Json;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
    use starknet_types::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
    use starknet_types::rpc::transactions::{
        BroadcastedTransactionCommonV3, DeclareTransaction, ResourceBoundsWrapper, Transaction,
        TransactionWithHash,
    };
    use starknet_types::traits::HashProducer;

    use super::calculate_casm_hash;
    use crate::constants::DEVNET_DEFAULT_CHAIN_ID;
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;

    pub(crate) fn dummy_felt() -> Felt {
        Felt::from_hex_unchecked("0xDD10")
    }

    pub(crate) fn dummy_cairo_1_contract_class() -> SierraContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo1/cairo_1_test.sierra")
                .unwrap();

        ContractClass::cairo_1_from_sierra_json_str(&json_str).unwrap()
    }

    /// casm hash of dummy_cairo_1_contract_class
    pub static DUMMY_CAIRO_1_COMPILED_CLASS_HASH: &str =
        "0x3faafcc98742a29a5ca809bda3c827b2d2c73759c64f695e33106009e7e9fef";

    pub(crate) fn dummy_contract_address() -> ContractAddress {
        ContractAddress::new(Felt::from_hex_unchecked("0xADD4E55")).unwrap()
    }

    pub(crate) fn dummy_declare_transaction_v1() -> TransactionWithHash {
        let chain_id = DEVNET_DEFAULT_CHAIN_ID.to_felt();
        let contract_class = dummy_cairo_0_contract_class();
        let broadcasted_tx = BroadcastedDeclareTransactionV1::new(
            dummy_contract_address(),
            Fee(100),
            &vec![],
            dummy_felt(),
            &contract_class.clone(),
            Felt::ONE,
        );
        let class_hash = contract_class.generate_hash().unwrap();
        let transaction_hash =
            broadcasted_tx.calculate_transaction_hash(&chain_id, &class_hash).unwrap();

        TransactionWithHash::new(
            transaction_hash,
            Transaction::Declare(DeclareTransaction::V1(DeclareTransactionV0V1::new(
                &broadcasted_tx,
                class_hash,
            ))),
        )
    }

    pub(crate) fn dummy_broadcasted_declare_transaction_v2(
        sender_address: &ContractAddress,
    ) -> BroadcastedDeclareTransactionV2 {
        let contract_class = dummy_cairo_1_contract_class();

        let casm_contract_class_json =
            usc::compile_contract(serde_json::to_value(contract_class.clone()).unwrap()).unwrap();

        let compiled_class_hash = calculate_casm_hash(casm_contract_class_json).unwrap();

        BroadcastedDeclareTransactionV2::new(
            &contract_class,
            compiled_class_hash,
            *sender_address,
            Fee(400000),
            &Vec::new(),
            Felt::ZERO,
            Felt::TWO,
        )
    }

    pub(crate) fn cairo_0_account_without_validations() -> Cairo0ContractClass {
        let account_json_path =
            "../../contracts/test_artifacts/cairo0/account_without_validations/account.json";

        Cairo0Json::raw_json_from_path(account_json_path).unwrap().into()
    }

    pub(crate) fn convert_broadcasted_declare_v2_to_v3(
        declare_v2: BroadcastedDeclareTransactionV2,
    ) -> BroadcastedDeclareTransactionV3 {
        BroadcastedDeclareTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: declare_v2.common.signature,
                nonce: declare_v2.common.nonce,
                resource_bounds: ResourceBoundsWrapper::new(
                    declare_v2.common.max_fee.0 as u64,
                    1,
                    0,
                    0,
                ),
                tip: Default::default(),
                paymaster_data: vec![],
                nonce_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            contract_class: declare_v2.contract_class,
            sender_address: declare_v2.sender_address,
            compiled_class_hash: declare_v2.compiled_class_hash,
            account_deployment_data: vec![],
        }
    }
}

#[cfg(any(test, feature = "test_utils"))]
#[allow(clippy::unwrap_used)]
pub mod exported_test_utils {
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_types::contract_class::deprecated::json_contract_class::Cairo0Json;
    use starknet_types::contract_class::Cairo0ContractClass;

    pub fn dummy_cairo_l1l2_contract() -> Cairo0ContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/l1l2.json").unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap().into()
    }

    pub fn dummy_cairo_l1l2_contract_codegen() -> LegacyContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/l1l2.json").unwrap();

        serde_json::from_str(&json_str).unwrap()
    }

    pub fn dummy_cairo_0_contract_class() -> Cairo0ContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/simple_contract.json")
                .unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap().into()
    }

    pub fn dummy_cairo_0_contract_class_codegen() -> LegacyContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/simple_contract.json")
                .unwrap();
        serde_json::from_str(&json_str).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::get_storage_var_address;
    use super::test_utils::{self};

    #[test]
    fn correct_simple_storage_var_address_generated() {
        let expected_storage_var_address =
            starknet_api::abi::abi_utils::get_storage_var_address("simple", &[]);
        let generated_storage_var_address = get_storage_var_address("simple", &[]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().to_bytes_be(),
            generated_storage_var_address.to_felt().to_bytes_be()
        );
    }

    #[test]
    fn correct_complex_storage_var_address_generated() {
        let expected_storage_var_address = starknet_api::abi::abi_utils::get_storage_var_address(
            "complex",
            &[test_utils::dummy_felt()],
        );

        let generated_storage_var_address =
            get_storage_var_address("complex", &[test_utils::dummy_felt()]).unwrap();

        assert_eq!(
            expected_storage_var_address.0.key().to_bytes_be(),
            generated_storage_var_address.to_felt().to_bytes_be()
        );
    }
}
