use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::{DeclareTransaction, Transaction};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;

pub fn add_declare_transaction_v2(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV2,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "declare transaction v2".into() });
    }

    let blockifier_declare_transaction =
        broadcasted_declare_transaction.create_blockifier_declare(starknet.chain_id().to_felt())?;

    let transaction_hash = blockifier_declare_transaction.tx_hash().0.into();
    let class_hash = blockifier_declare_transaction.class_hash().0.into();

    let transaction = Transaction::Declare(DeclareTransaction::Version2(
        broadcasted_declare_transaction.create_declare(class_hash, transaction_hash),
    ));

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Declare(
            blockifier_declare_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, blockifier_execution_result)?;

    Ok((transaction_hash, class_hash))
}

pub fn add_declare_transaction_v1(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV1,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::MaxFeeZeroError { tx_type: "declare transaction v1".into() });
    }

    let class_hash = broadcasted_declare_transaction.generate_class_hash()?;
    let transaction_hash = broadcasted_declare_transaction
        .calculate_transaction_hash(&starknet.config.chain_id.to_felt(), &class_hash)?;

    let declare_transaction =
        broadcasted_declare_transaction.create_declare(class_hash, transaction_hash);
    let transaction = Transaction::Declare(DeclareTransaction::Version1(declare_transaction));

    let blockifier_declare_transaction =
        broadcasted_declare_transaction.create_blockifier_declare(class_hash, transaction_hash)?;

    let blockifier_execution_result =
        blockifier::transaction::account_transaction::AccountTransaction::Declare(
            blockifier_declare_transaction,
        )
        .execute(&mut starknet.state.state, &starknet.block_context, true, true);

    starknet.handle_transaction_result(transaction, blockifier_execution_result)?;

    Ok((transaction_hash, class_hash))
}

#[cfg(test)]
mod tests {
    use starknet_api::block::BlockNumber;
    use starknet_api::transaction::Fee;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0Json, ContractClass};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self, DEVNET_DEFAULT_CHAIN_ID};
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut, StateExtractor};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_broadcasted_declare_transaction_v2, dummy_cairo_1_contract_class,
        dummy_contract_address, dummy_felt,
    };

    fn broadcasted_declare_transaction_v1(
        sender_address: ContractAddress,
    ) -> BroadcastedDeclareTransactionV1 {
        let contract_class = dummy_cairo_0_contract_class();

        BroadcastedDeclareTransactionV1::new(
            sender_address,
            Fee(10000),
            &Vec::new(),
            Felt::from(0),
            &contract_class.into(),
            Felt::from(1),
        )
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

        let result = Starknet::default().add_declare_transaction_v2(declare_transaction_v2);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "declare transaction v2: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);

        match starknet.add_declare_transaction_v2(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientAccountBalance,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v2_transaction_successful_execution() {
        let (mut starknet, sender) = setup(Some(100000000));

        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v2(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(
            class_hash,
            ContractClass::Cairo1(declare_txn.contract_class).generate_hash().unwrap()
        );
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(starknet.state.contract_classes.get(&class_hash).is_some());
    }

    #[test]
    fn declare_v2_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup(Some(100000000));
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);
        let expected_class_hash =
            ContractClass::Cairo1(declare_txn.contract_class.clone()).generate_hash().unwrap();
        let expected_compiled_class_hash = declare_txn.compiled_class_hash;

        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(&expected_class_hash));
        assert!(
            !starknet
                .state
                .state
                .state
                .class_hash_to_compiled_class
                .contains_key(&expected_compiled_class_hash)
        );

        let (tx_hash, retrieved_class_hash) =
            starknet.add_declare_transaction_v2(declare_txn).unwrap();

        let retrieved_txn = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(retrieved_class_hash, expected_class_hash);
        // check if txn is with status accepted
        assert_eq!(retrieved_txn.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(retrieved_txn.execution_result.status(), TransactionExecutionStatus::Succeeded);
        assert!(starknet.state.is_contract_declared(&expected_class_hash));
    }

    #[test]
    fn declare_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let declare_transaction = BroadcastedDeclareTransactionV1::new(
            dummy_contract_address(),
            Fee(0),
            &vec![],
            dummy_felt(),
            &dummy_cairo_0_contract_class().into(),
            Felt::from(1),
        );

        let result = Starknet::default().add_declare_transaction_v1(declare_transaction);

        assert!(result.is_err());
        match result.err().unwrap() {
            err @ crate::error::Error::MaxFeeZeroError { .. } => {
                assert_eq!(err.to_string(), "declare transaction v1: max_fee cannot be zero")
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_low_max_fee() {
        let (mut starknet, sender) = setup(Some(20000));

        let mut declare_txn = broadcasted_declare_transaction_v1(sender);
        declare_txn.common.max_fee = Fee(declare_txn.common.max_fee.0 / 10);

        match starknet.add_declare_transaction_v1(declare_txn).unwrap_err() {
            crate::error::Error::TransactionValidationError(
                crate::error::TransactionValidationError::InsufficientMaxFee,
            ) => {}
            err => {
                panic!("Wrong error type received {:?}", err);
            }
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_an_error_due_to_not_enough_balance_on_account() {
        let (mut starknet, sender) = setup(Some(1));

        let declare_txn = broadcasted_declare_transaction_v1(sender);
        match starknet.add_declare_transaction_v1(declare_txn).unwrap_err() {
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
        let (mut starknet, sender) = setup(None);

        let declare_txn = broadcasted_declare_transaction_v1(sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v1(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(class_hash, declare_txn.contract_class.generate_hash().unwrap());
        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);
        // check if contract is successfully declared
        assert!(starknet.state.is_contract_declared(&class_hash));
        // check if pending block is resetted
        assert!(starknet.pending_block().get_transactions().is_empty());
        // check if there is generated block
        assert_eq!(starknet.blocks.num_to_block.len(), 1);
        // check if transaction is in generated block
        assert_eq!(
            *starknet
                .blocks
                .num_to_block
                .get(&BlockNumber(0))
                .unwrap()
                .get_transactions()
                .first()
                .unwrap(),
            tx_hash
        );
    }

    #[test]
    fn declare_v1_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup(None);
        let declare_txn = broadcasted_declare_transaction_v1(sender);

        let expected_class_hash = declare_txn.contract_class.generate_hash().unwrap();
        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(&expected_class_hash));

        let (tx_hash, class_hash) = starknet.add_declare_transaction_v1(declare_txn).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if txn is with status accepted
        assert_eq!(tx.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(tx.execution_result.status(), TransactionExecutionStatus::Succeeded);

        // check if contract is declared
        assert!(starknet.state.is_contract_declared(&class_hash));
    }

    /// Initializes starknet with 1 account - account without validations
    fn setup(acc_balance: Option<u128>) -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = Cairo0Json::raw_json_from_path(account_json_path).unwrap();

        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(acc_balance.unwrap_or(10000)),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class.into(),
            erc_20_contract.get_address(),
        )
        .unwrap();

        acc.deploy(&mut starknet.state).unwrap();
        acc.set_initial_balance(&mut starknet.state).unwrap();

        starknet.state.clear_dirty_state();
        starknet.block_context = Starknet::init_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        );

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
