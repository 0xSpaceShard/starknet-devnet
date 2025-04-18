use blockifier::bouncer::{BouncerConfig, BouncerWeights};
use blockifier::transaction::objects::TransactionExecutionInfo;
use blockifier::versioned_constants::VersionedConstants;
use starknet_api::block::StarknetVersion;
use starknet_rs_core::types::Felt;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};

use crate::error::DevnetResult;

pub mod random_number_generator {
    use rand::{Rng, SeedableRng, thread_rng};
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

/// This should be modified when updating to the version after 0.13.5
pub(crate) fn get_versioned_constants() -> VersionedConstants {
    #[allow(clippy::unwrap_used)]
    VersionedConstants::get(&StarknetVersion::V0_13_5).unwrap().clone()
}

/// Values not present here: https://docs.starknet.io/tools/limits-and-triggers/
/// Asked the blockifier team about the values, they provided them in these threads:
/// https://spaceshard.slack.com/archives/C029F9AN8LX/p1721657837687799?thread_ts=1721400009.781699&cid=C029F9AN8LX
/// https://spaceshard.slack.com/archives/C029F9AN8LX/p1739259794326519?thread_ts=1738840494.497479&cid=C029F9AN8LX
pub(crate) fn custom_bouncer_config() -> BouncerConfig {
    BouncerConfig {
        block_max_capacity: BouncerWeights {
            l1_gas: 4_950_000,
            sierra_gas: starknet_api::execution_resources::GasAmount(5_000_000_000),
            state_diff_size: 4_000,
            n_events: 5_000,
            ..BouncerWeights::max()
        },
    }
}

#[macro_export]
macro_rules! nonzero_gas_price {
    ($value:expr) => {{
        let gas_price = starknet_api::block::GasPrice(($value).get());
        starknet_api::block::NonzeroGasPrice::new(gas_price).unwrap()
    }};
}

pub(crate) fn maybe_extract_failure_reason(
    execution_info: &TransactionExecutionInfo,
) -> Option<String> {
    execution_info.revert_error.as_ref().map(|err| err.to_string())
}

#[cfg(test)]
pub(crate) mod test_utils {
    use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
    use starknet_api::data_availability::DataAvailabilityMode;
    use starknet_api::transaction::fields::Tip;
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::deprecated::json_contract_class::Cairo0Json;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
    use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
    use starknet_types::rpc::transactions::declare_transaction_v3::DeclareTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, BroadcastedInvokeTransaction,
        BroadcastedTransactionCommonV3, DeclareTransaction, ResourceBoundsWrapper, Transaction,
        TransactionWithHash,
    };
    use starknet_types::traits::HashProducer;

    use crate::account::KeyPair;
    use crate::constants::DEVNET_DEFAULT_CHAIN_ID;

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
    pub static DUMMY_CAIRO_1_COMPILED_CLASS_HASH: Felt = Felt::from_hex_unchecked(
        "0x3faafcc98742a29a5ca809bda3c827b2d2c73759c64f695e33106009e7e9fef",
    );

    pub(crate) fn dummy_contract_address() -> ContractAddress {
        ContractAddress::new(Felt::from_hex_unchecked("0xADD4E55")).unwrap()
    }

    /// Returns a gas bounds object with max amounts set to input params and prices set to 1
    pub(crate) fn resource_bounds_with_price_1(
        l1_gas: u64,
        l1_data_gas: u64,
        l2_gas: u64,
    ) -> ResourceBoundsWrapper {
        ResourceBoundsWrapper::new(l1_gas, 1, l1_data_gas, 1, l2_gas, 1)
    }

    /// Returns an unsigned tx representing a v3 declaration of a predefined dummy
    /// class. The tx can be sent to Starknet.
    pub(crate) fn broadcasted_declare_tx_v3_of_dummy_class(
        sender_address: ContractAddress,
        nonce: Felt,
        resource_bounds: ResourceBoundsWrapper,
    ) -> BroadcastedDeclareTransactionV3 {
        broadcasted_declare_tx_v3(
            sender_address,
            nonce,
            dummy_cairo_1_contract_class(),
            DUMMY_CAIRO_1_COMPILED_CLASS_HASH,
            resource_bounds,
        )
    }

    /// Returns an unsigned tx representing a v3 declaration of the provided class. The
    /// tx can be sent to Starknet.
    pub(crate) fn broadcasted_declare_tx_v3(
        sender_address: ContractAddress,
        nonce: Felt,
        contract_class: cairo_lang_starknet_classes::contract_class::ContractClass,
        compiled_class_hash: Felt,
        resource_bounds: ResourceBoundsWrapper,
    ) -> BroadcastedDeclareTransactionV3 {
        BroadcastedDeclareTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce,
                resource_bounds,
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            },
            contract_class,
            sender_address,
            compiled_class_hash,
            account_deployment_data: vec![],
        }
    }

    pub(crate) fn dummy_declare_tx_v3_with_hash() -> TransactionWithHash {
        let declare_txn = broadcasted_declare_tx_v3_of_dummy_class(
            dummy_contract_address(),
            Felt::ZERO,
            ResourceBoundsWrapper::new(
                1, 1, // l1_gas
                0, 0, // l1_data_gas
                0, 0, // l2_gas
            ),
        );
        let sierra_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();

        let tx_hash = BroadcastedDeclareTransaction::V3(Box::new(declare_txn.clone()))
            .create_sn_api_declare(&DEVNET_DEFAULT_CHAIN_ID.to_felt())
            .unwrap()
            .tx_hash;

        TransactionWithHash::new(
            *tx_hash,
            Transaction::Declare(DeclareTransaction::V3(DeclareTransactionV3::new(
                &declare_txn,
                sierra_hash,
            ))),
        )
    }

    pub(crate) fn cairo_0_account_without_validations() -> Cairo0ContractClass {
        let account_json_path =
            "../../contracts/test_artifacts/cairo0/account_without_validations/account.json";

        Cairo0Json::raw_json_from_path(account_json_path).unwrap().into()
    }

    pub fn dummy_key_pair() -> KeyPair {
        KeyPair { public_key: dummy_felt(), private_key: dummy_felt() }
    }

    pub fn test_invoke_transaction_v3(
        account_address: ContractAddress,
        contract_address: ContractAddress,
        function_selector: Felt,
        params: &[Felt],
        nonce: u128,
        resource_bounds: ResourceBoundsWrapper,
    ) -> BroadcastedInvokeTransaction {
        // Encode: [addr, selector, params len, *params]
        let mut calldata = vec![Felt::from(contract_address), function_selector];
        calldata.push(params.len().into());
        calldata.extend_from_slice(params);

        BroadcastedInvokeTransaction::V3(BroadcastedInvokeTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE,
                signature: vec![],
                nonce: Felt::from(nonce),
                resource_bounds,
                tip: Tip(0),
                paymaster_data: vec![],
                nonce_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
                fee_data_availability_mode:
                    starknet_api::data_availability::DataAvailabilityMode::L1,
            },
            sender_address: account_address,
            calldata,
            account_deployment_data: vec![],
        })
    }
}

#[cfg(any(test, feature = "test_utils"))]
#[allow(clippy::unwrap_used)]
pub mod exported_test_utils {
    use starknet_types::contract_class::Cairo0ContractClass;
    use starknet_types::contract_class::deprecated::json_contract_class::Cairo0Json;

    pub fn dummy_cairo_l1l2_contract() -> Cairo0ContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/l1l2.json").unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap().into()
    }

    pub fn dummy_cairo_0_contract_class() -> Cairo0ContractClass {
        let json_str =
            std::fs::read_to_string("../../contracts/test_artifacts/cairo0/simple_contract.json")
                .unwrap();

        Cairo0Json::raw_json_from_json_str(&json_str).unwrap().into()
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
