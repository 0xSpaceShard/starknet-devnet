use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::compile_sierra_contract;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, CompiledClassHash, TransactionHash};
use starknet_types::rpc::transactions::declare_transaction_v3::DeclareTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, DeclareTransaction, Transaction, TransactionWithHash,
};

use crate::constants::{
    MAXIMUM_CONTRACT_BYTECODE_SIZE, MAXIMUM_CONTRACT_CLASS_SIZE, MAXIMUM_SIERRA_LENGTH,
};
use crate::error::{DevnetResult, Error, TransactionValidationError};
use crate::starknet::Starknet;
use crate::state::CustomState;

fn check_class_size(
    executable_tx: &starknet_api::executable_transaction::DeclareTransaction,
) -> DevnetResult<()> {
    let serialized_class = serde_json::to_vec(&executable_tx.contract_class()).map_err(|e| {
        Error::UnexpectedInternalError {
            msg: format!("Could not determine class size via serialization: {e}"),
        }
    })?;

    let sierra_length = executable_tx.class_info.sierra_program_length();
    let casm_length = executable_tx.class_info.bytecode_length();
    tracing::info!(
        "Declaring class: serialized size: {} bytes, sierra: {} felts, casm: {} felts",
        serialized_class.len(),
        sierra_length,
        casm_length,
    );

    if serialized_class.len() > MAXIMUM_CONTRACT_CLASS_SIZE
        || sierra_length > MAXIMUM_SIERRA_LENGTH
        || casm_length > MAXIMUM_CONTRACT_BYTECODE_SIZE
    {
        return Err(Error::ContractClassSizeIsTooLarge);
    }

    Ok(())
}

pub fn add_declare_transaction(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransaction,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if !broadcasted_declare_transaction.are_gas_bounds_valid() {
        return Err(TransactionValidationError::InsufficientResourcesForValidate.into());
    }

    if broadcasted_declare_transaction.is_only_query() {
        return Err(Error::UnsupportedAction {
            msg: "only-query transactions are not supported".to_string(),
        });
    }

    let executable_tx =
        broadcasted_declare_transaction.create_sn_api_declare(&starknet.chain_id().to_felt())?;

    check_class_size(&executable_tx)?;

    let transaction_hash = executable_tx.tx_hash.0;
    let class_hash = executable_tx.class_hash().0;

    let (declare_transaction, contract_class, casm_hash, sender_address) =
        match broadcasted_declare_transaction {
            BroadcastedDeclareTransaction::V3(ref v3) => {
                let declare_transaction = Transaction::Declare(DeclareTransaction::V3(
                    DeclareTransactionV3::new(v3, class_hash),
                ));

                (
                    declare_transaction,
                    v3.contract_class.clone().into(),
                    Some(v3.compiled_class_hash),
                    &v3.sender_address,
                )
            }
        };

    assert_casm_hash_is_valid(&contract_class, casm_hash)?;

    let validate = !(Starknet::is_account_impersonated(
        &mut starknet.pending_state,
        &starknet.cheats,
        sender_address,
    )?);

    let transaction = TransactionWithHash::new(transaction_hash, declare_transaction);
    let execution_info = blockifier::transaction::account_transaction::AccountTransaction {
        tx: starknet_api::executable_transaction::AccountTransaction::Declare(executable_tx),
        execution_flags: ExecutionFlags { only_query: false, charge_fee: true, validate },
    }
    .execute(&mut starknet.pending_state.state, &starknet.block_context)?;

    // if tx successful, store the class
    if !execution_info.is_reverted() {
        let state = starknet.get_state();
        state.declare_contract_class(class_hash, casm_hash, contract_class)?;
    }

    starknet.handle_accepted_transaction(transaction, execution_info)?;

    Ok((transaction_hash, class_hash))
}

/// If cairo1, convert `contract_class` to casm, calculate its hash and assert it's equal to
/// `received_casm_hash`. If cairo0, assert no `received_casm_hash`.
fn assert_casm_hash_is_valid(
    contract_class: &ContractClass,
    received_casm_hash: Option<CompiledClassHash>,
) -> DevnetResult<()> {
    match (contract_class, received_casm_hash) {
        (ContractClass::Cairo0(_), None) => Ok(()), // if cairo0, casm_hash expected to be None
        (ContractClass::Cairo1(cairo_lang_contract_class), Some(received_casm_hash)) => {
            let casm = compile_sierra_contract(cairo_lang_contract_class)?;

            let calculated_casm_hash = casm.compiled_class_hash();
            if calculated_casm_hash == received_casm_hash {
                Ok(())
            } else {
                Err(Error::CompiledClassHashMismatch)
            }
        }
        unexpected => Err(Error::UnexpectedInternalError {
            msg: format!("Unexpected class and casm combination: {unexpected:?}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::data_availability::DataAvailabilityMode;
    use starknet_rs_core::types::{Felt, TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, BroadcastedTransactionCommonV3, ResourceBoundsWrapper,
    };
    use starknet_types::traits::HashProducer;

    use crate::error::{Error, TransactionValidationError};
    use crate::starknet::Starknet;
    use crate::starknet::tests::setup_starknet_with_no_signature_check_account;
    use crate::state::{BlockNumberOrPending, CustomStateReader};
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::{
        broadcasted_declare_tx_v3_of_dummy_class, dummy_cairo_1_contract_class,
        dummy_contract_address, dummy_felt, resource_bounds_with_price_1,
    };

    #[test]
    fn declare_transaction_v3_with_query_version_should_return_an_error() {
        let declare_tx = BroadcastedDeclareTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE + QUERY_VERSION_OFFSET,
                signature: vec![],
                nonce: dummy_felt(),
                resource_bounds: resource_bounds_with_price_1(1, 1, 1),
                tip: Default::default(),
                paymaster_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            },
            contract_class: dummy_cairo_1_contract_class(),
            sender_address: dummy_contract_address(),
            compiled_class_hash: dummy_felt(),
            account_deployment_data: vec![],
        };

        let result = Starknet::default()
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(declare_tx)));

        match result {
            Err(Error::UnsupportedAction { msg }) => {
                assert_eq!(msg, "only-query transactions are not supported")
            }
            other => panic!("Unexpected result: {other:?}"),
        };
    }

    #[test]
    fn declare_transaction_v3_with_zero_gas_bounds_should_return_an_error() {
        let declare_tx = BroadcastedDeclareTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: Felt::THREE + QUERY_VERSION_OFFSET,
                signature: vec![],
                nonce: dummy_felt(),
                resource_bounds: ResourceBoundsWrapper::new(0, 0, 0, 0, 0, 0),
                tip: Default::default(),
                paymaster_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            },
            contract_class: dummy_cairo_1_contract_class(),
            sender_address: dummy_contract_address(),
            compiled_class_hash: dummy_felt(),
            account_deployment_data: vec![],
        };

        let result = Starknet::default()
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(declare_tx)));

        match result {
            Err(Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            )) => {}
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn add_declare_v3_transaction_successful_execution() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e18 as u128);

        let declare_tx = broadcasted_declare_tx_v3_of_dummy_class(
            sender.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        let (tx_hash, class_hash) =
            starknet.add_declare_transaction(declare_tx.clone().into()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(
            class_hash,
            ContractClass::Cairo1(declare_tx.contract_class).generate_hash().unwrap()
        );
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        starknet
            .rpc_contract_classes
            .read()
            .get_class(&class_hash, &BlockNumberOrPending::Number(tx.block_number.unwrap().0))
            .unwrap();
    }

    #[test]
    fn add_declare_v3_transaction_should_return_an_error_due_to_low_gas_bounds() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(20000);

        let declare_tx = broadcasted_declare_tx_v3_of_dummy_class(
            sender.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1, 1),
        );

        match starknet.add_declare_transaction(declare_tx.into()) {
            Err(Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            )) => {}
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn add_declare_v3_transaction_should_return_an_error_due_to_not_enough_balance_on_account() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1);

        let declare_tx = broadcasted_declare_tx_v3_of_dummy_class(
            sender.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        match starknet.add_declare_transaction(declare_tx.into()).unwrap_err() {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => panic!("Wrong error type received {:?}", err),
        }
    }

    #[test]
    fn declare_v3_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e18 as u128);

        let declare_tx = broadcasted_declare_tx_v3_of_dummy_class(
            sender.account_address,
            Felt::ZERO,
            resource_bounds_with_price_1(0, 1000, 1e9 as u64),
        );

        // check if contract is not declared
        let expected_class_hash =
            ContractClass::Cairo1(declare_tx.contract_class.clone()).generate_hash().unwrap();
        assert!(!starknet.pending_state.is_contract_declared(expected_class_hash));

        let (tx_hash, class_hash) = starknet.add_declare_transaction(declare_tx.into()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check if contract is declared
        assert!(starknet.pending_state.is_contract_declared(class_hash));
    }
}
