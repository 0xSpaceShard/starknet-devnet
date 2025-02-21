use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::compile_sierra_contract;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, CompiledClassHash, TransactionHash};
use starknet_types::rpc::transactions::declare_transaction_v0v1::DeclareTransactionV0V1;
use starknet_types::rpc::transactions::declare_transaction_v2::DeclareTransactionV2;
use starknet_types::rpc::transactions::declare_transaction_v3::DeclareTransactionV3;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, DeclareTransaction, Transaction, TransactionWithHash,
};

use crate::error::{DevnetResult, Error, TransactionValidationError};
use crate::starknet::Starknet;
use crate::state::CustomState;

pub fn add_declare_transaction(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransaction,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if !broadcasted_declare_transaction.is_max_fee_valid() {
        return Err(TransactionValidationError::InsufficientResourcesForValidate.into());
    }

    if broadcasted_declare_transaction.is_only_query() {
        return Err(Error::UnsupportedAction {
            msg: "only-query transactions are not supported".to_string(),
        });
    }

    let executable_tx =
        broadcasted_declare_transaction.create_sn_api_declare(&starknet.chain_id().to_felt())?;

    let transaction_hash = executable_tx.tx_hash.0;
    let class_hash = executable_tx.class_hash().0;

    let (declare_transaction, contract_class, casm_hash, sender_address) =
        match broadcasted_declare_transaction {
            BroadcastedDeclareTransaction::V1(ref v1) => {
                let declare_transaction = Transaction::Declare(DeclareTransaction::V1(
                    DeclareTransactionV0V1::new(v1, class_hash),
                ));

                (declare_transaction, v1.contract_class.clone().into(), None, &v1.sender_address)
            }
            BroadcastedDeclareTransaction::V2(ref v2) => {
                let declare_transaction = Transaction::Declare(DeclareTransaction::V2(
                    DeclareTransactionV2::new(v2, class_hash),
                ));

                (
                    declare_transaction,
                    v2.contract_class.clone().into(),
                    Some(v2.compiled_class_hash),
                    &v2.sender_address,
                )
            }
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
    use blockifier::state::state_api::StateReader;
    use starknet_api::core::CompiledClassHash;
    use starknet_api::transaction::fields::Fee;
    use starknet_rs_core::types::{
        BlockId, BlockTag, Felt, TransactionExecutionStatus, TransactionFinalityStatus,
    };
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::rpc::transactions::BroadcastedDeclareTransaction;
    use starknet_types::traits::HashProducer;

    use crate::error::{Error, TransactionValidationError};
    use crate::starknet::tests::setup_starknet_with_no_signature_check_account;
    use crate::starknet::Starknet;
    use crate::state::{BlockNumberOrPending, CustomStateReader};
    use crate::traits::{HashIdentified, HashIdentifiedMut};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        convert_broadcasted_declare_v2_to_v3, dummy_broadcasted_declare_transaction_v2,
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
    };

    fn broadcasted_declare_transaction_v1(
        sender_address: ContractAddress,
    ) -> BroadcastedDeclareTransaction {
        let contract_class = dummy_cairo_0_contract_class();

        BroadcastedDeclareTransaction::V1(Box::new(BroadcastedDeclareTransactionV1::new(
            sender_address,
            Fee(10000),
            &Vec::new(),
            Felt::ZERO,
            &contract_class,
            Felt::ONE,
        )))
    }

    #[test]
    fn declare_transaction_v3_with_query_version_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV2::new(
            &dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            Fee(10000),
            &vec![],
            dummy_felt(),
            dummy_felt(),
        );

        let mut declare_transaction = convert_broadcasted_declare_v2_to_v3(declare_transaction);
        declare_transaction.common.version = Felt::THREE + QUERY_VERSION_OFFSET;

        let result = Starknet::default().add_declare_transaction(
            BroadcastedDeclareTransaction::V3(Box::new(declare_transaction)),
        );

        match result {
            Err(crate::error::Error::UnsupportedAction { msg }) => {
                assert_eq!(msg, "only-query transactions are not supported")
            }
            other => panic!("Unexpected result: {other:?}"),
        };
    }

    #[test]
    fn declare_transaction_v3_with_max_fee_zero_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV2::new(
            &dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            dummy_felt(),
        );

        let declare_transaction = convert_broadcasted_declare_v2_to_v3(declare_transaction);

        let result = Starknet::default().add_declare_transaction(
            BroadcastedDeclareTransaction::V3(Box::new(declare_transaction)),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn declare_transaction_v2_with_max_fee_zero_should_return_an_error() {
        let declare_transaction_v2 = BroadcastedDeclareTransactionV2::new(
            &dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            dummy_felt(),
        );

        let result = Starknet::default().add_declare_transaction(
            BroadcastedDeclareTransaction::V2(Box::new(declare_transaction_v2)),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1);
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender.account_address);

        match starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V2(Box::new(declare_txn)))
            .unwrap_err()
        {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v3_transaction_successful_execution() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e18 as u128);

        let declare_txn = convert_broadcasted_declare_v2_to_v3(
            dummy_broadcasted_declare_transaction_v2(&sender.account_address),
        );

        let (tx_hash, class_hash) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(
                declare_txn.clone(),
            )))
            .unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(
            class_hash,
            ContractClass::Cairo1(declare_txn.contract_class).generate_hash().unwrap()
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
    fn add_declare_v2_transaction_successful_execution() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e8 as u128);

        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender.account_address);
        let (tx_hash, class_hash) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V2(Box::new(
                declare_txn.clone(),
            )))
            .unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        let generated_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();
        assert_eq!(class_hash, generated_hash);

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert_eq!(
            starknet.get_class(&BlockId::Tag(BlockTag::Latest), class_hash).unwrap(),
            declare_txn.contract_class.into()
        );
    }

    #[test]
    fn declare_v2_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1e8 as u128);
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender.account_address);
        let expected_class_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();
        let expected_compiled_class_hash = declare_txn.compiled_class_hash;

        // check if contract is not declared
        assert!(!starknet.pending_state.is_contract_declared(expected_class_hash));
        assert_eq!(
            starknet
                .pending_state
                .get_compiled_class_hash(starknet_api::core::ClassHash(expected_class_hash))
                .unwrap(),
            CompiledClassHash(Felt::ZERO)
        );
        assert!(starknet.get_class(&BlockId::Tag(BlockTag::Latest), expected_class_hash).is_err());

        let (tx_hash, retrieved_class_hash) = starknet
            .add_declare_transaction(BroadcastedDeclareTransaction::V2(Box::new(declare_txn)))
            .unwrap();

        let retrieved_txn = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(retrieved_class_hash, expected_class_hash);
        // check if txn is with status accepted
        assert_eq!(retrieved_txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(retrieved_txn.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(starknet.pending_state.is_contract_declared(expected_class_hash));
        assert_eq!(
            starknet
                .pending_state
                .get_compiled_class_hash(starknet_api::core::ClassHash(expected_class_hash))
                .unwrap()
                .0,
            expected_compiled_class_hash
        );
    }

    #[test]
    fn declare_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV1::new(
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            &dummy_cairo_0_contract_class(),
            Felt::ONE,
        );

        let result = Starknet::default().add_declare_transaction(
            starknet_types::rpc::transactions::BroadcastedDeclareTransaction::V1(Box::new(
                declare_transaction,
            )),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            Error::TransactionValidationError(
                TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_low_max_fee() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(20000);

        let mut declare_txn = broadcasted_declare_transaction_v1(sender.account_address);
        match declare_txn {
            BroadcastedDeclareTransaction::V1(ref mut v1) => {
                v1.common.max_fee = Fee(10);
            }
            _ => panic!("Wrong transaction type"),
        }

        match starknet.add_declare_transaction(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientResourcesForValidate,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_not_enough_balance_on_account() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(1);

        let declare_txn = broadcasted_declare_transaction_v1(sender.account_address);
        match starknet.add_declare_transaction(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v1_transaction_successful_execution() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(10000);

        let initial_block_count = starknet.blocks.hash_to_block.len();
        let declare_txn = broadcasted_declare_transaction_v1(sender.account_address);
        let (tx_hash, class_hash) = starknet.add_declare_transaction(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();
        match declare_txn {
            BroadcastedDeclareTransaction::V1(ref v1) => {
                // check if generated class hash is expected one
                assert_eq!(class_hash, v1.contract_class.generate_hash().unwrap());
            }
            _ => panic!("Wrong transaction type"),
        }
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        // check if contract is successfully declared
        assert!(starknet.pending_state.is_contract_declared(class_hash));
        // check if pending block is reset
        assert!(starknet.pending_block().get_transactions().is_empty());
        // check if there is one new generated block
        assert_eq!(starknet.blocks.hash_to_block.len(), initial_block_count + 1);
        // check if transaction is in generated block
        assert_eq!(
            *starknet
                .blocks
                .get_by_hash(starknet.blocks.last_block_hash.unwrap())
                .unwrap()
                .get_transactions()
                .first()
                .unwrap(),
            tx_hash
        );
    }

    #[test]
    fn declare_v1_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup_starknet_with_no_signature_check_account(10000);
        let declare_txn = broadcasted_declare_transaction_v1(sender.account_address);

        match declare_txn {
            BroadcastedDeclareTransaction::V1(ref v1) => {
                let expected_class_hash = v1.contract_class.generate_hash().unwrap();
                // check if contract is not declared
                assert!(!starknet.pending_state.is_contract_declared(expected_class_hash));
            }
            _ => panic!("Wrong transaction type"),
        }

        let (tx_hash, class_hash) = starknet.add_declare_transaction(declare_txn).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check if contract is declared
        assert!(starknet.pending_state.is_contract_declared(class_hash));
    }
}
