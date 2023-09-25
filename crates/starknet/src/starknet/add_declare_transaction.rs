use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::{DeclareTransaction, Transaction};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;
use crate::transactions::StarknetTransaction;

pub fn add_declare_transaction_v2(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV2,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::TransactionError(
            starknet_in_rust::transaction::error::TransactionError::FeeError(
                "For declare transaction version 2, max fee cannot be 0".to_string(),
            ),
        ));
    }

    let sir_declare_transaction =
        broadcasted_declare_transaction.create_sir_declare(starknet.config.chain_id.to_felt())?;

    let transaction_hash = sir_declare_transaction.hash_value.clone().into();
    let class_hash: ClassHash = sir_declare_transaction.sierra_class_hash.clone().into();

    let state_before_txn = starknet.state.pending_state.clone();
    let transaction = Transaction::Declare(DeclareTransaction::Version2(
        sir_declare_transaction.clone().try_into()?,
    ));

    match sir_declare_transaction
        .execute(&mut starknet.state.pending_state, &starknet.block_context)
    {
        Ok(tx_info) => match tx_info.revert_error {
            // Add sierra contract
            Some(error) => {
                let transaction_to_add =
                    StarknetTransaction::create_rejected(&transaction, None, &error);

                starknet.transactions.insert(&transaction_hash, transaction_to_add);
                // Revert to previous pending state
                starknet.state.pending_state = state_before_txn;
            }
            None => {
                starknet.state.contract_classes.insert(
                    class_hash,
                    ContractClass::Cairo1(broadcasted_declare_transaction.contract_class),
                );
                starknet.handle_successful_transaction(
                    &transaction_hash,
                    &transaction,
                    &tx_info,
                )?;
            }
        },
        Err(tx_err) => {
            let transaction_to_add =
                StarknetTransaction::create_rejected(&transaction, None, &tx_err.to_string());

            starknet.transactions.insert(&transaction_hash, transaction_to_add);
            // Revert to previous pending state
            starknet.state.pending_state = state_before_txn;
        }
    }

    Ok((transaction_hash, class_hash))
}

pub fn add_declare_transaction_v1(
    starknet: &mut Starknet,
    broadcasted_declare_transaction: BroadcastedDeclareTransactionV1,
) -> DevnetResult<(TransactionHash, ClassHash)> {
    if broadcasted_declare_transaction.common.max_fee.0 == 0 {
        return Err(Error::TransactionError(
            starknet_in_rust::transaction::error::TransactionError::FeeError(
                "For declare transaction version 1, max fee cannot be 0".to_string(),
            ),
        ));
    }

    let state_before_txn = starknet.state.pending_state.clone();

    let class_hash = broadcasted_declare_transaction.generate_class_hash()?;
    let transaction_hash = broadcasted_declare_transaction
        .calculate_transaction_hash(&starknet.config.chain_id.to_felt(), &class_hash)?;

    let declare_transaction =
        broadcasted_declare_transaction.create_declare(class_hash, transaction_hash);
    let transaction = Transaction::Declare(DeclareTransaction::Version1(declare_transaction));

    let sir_declare_transaction =
        broadcasted_declare_transaction.create_sir_declare(class_hash, transaction_hash)?;

    match sir_declare_transaction
        .execute(&mut starknet.state.pending_state, &starknet.block_context)
    {
        Ok(tx_info) => match tx_info.revert_error {
            Some(error) => {
                let transaction_to_add =
                    StarknetTransaction::create_rejected(&transaction, None, &error);

                starknet.transactions.insert(&transaction_hash, transaction_to_add);
                // Revert to previous pending state
                starknet.state.pending_state = state_before_txn;
            }
            None => {
                starknet
                    .state
                    .contract_classes
                    .insert(class_hash, broadcasted_declare_transaction.contract_class.into());
                starknet.handle_successful_transaction(
                    &transaction_hash,
                    &transaction,
                    &tx_info,
                )?;
            }
        },
        Err(tx_err) => {
            let transaction_to_add =
                StarknetTransaction::create_rejected(&transaction, None, &tx_err.to_string());

            starknet.transactions.insert(&transaction_hash, transaction_to_add);
            // Revert to previous pending state
            starknet.state.pending_state = state_before_txn;
        }
    }

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
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For declare transaction version 2, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let initial_cached_state = starknet.state.pending_state.contract_classes().len();
        let declare_txn = dummy_broadcasted_declare_transaction_v2(&sender);
        let (txn_hash, class_hash) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, None);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Reverted);
        assert_eq!(initial_cached_state, starknet.state.pending_state.contract_classes().len());
        assert!(starknet.state.contract_classes.get(&class_hash).is_none())
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
        assert_eq!(tx.finality_status, Some(TransactionFinalityStatus::AcceptedOnL2));
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
                .class_hash_to_compiled_class
                .contains_key(&expected_compiled_class_hash.bytes())
        );

        let (tx_hash, retrieved_class_hash) =
            starknet.add_declare_transaction_v2(declare_txn).unwrap();

        let retrieved_txn = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(retrieved_class_hash, expected_class_hash);
        // check if txn is with status accepted
        assert_eq!(retrieved_txn.finality_status, Some(TransactionFinalityStatus::AcceptedOnL2));
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
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For declare transaction version 1, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v1_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let initial_cached_state = starknet.state.pending_state.contract_classes().len();
        let declare_txn = broadcasted_declare_transaction_v1(sender);
        let (txn_hash, _) = starknet.add_declare_transaction_v1(declare_txn).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.finality_status, None);
        assert_eq!(txn.execution_result.status(), TransactionExecutionStatus::Reverted);
        assert_eq!(initial_cached_state, starknet.state.pending_state.contract_classes().len());
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
        assert_eq!(tx.finality_status, Some(TransactionFinalityStatus::AcceptedOnL2));
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
        assert_eq!(tx.finality_status, Some(TransactionFinalityStatus::AcceptedOnL2));
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

        starknet.state.synchronize_states();
        starknet.block_context = Starknet::get_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            DEVNET_DEFAULT_CHAIN_ID,
        )
        .unwrap();

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
