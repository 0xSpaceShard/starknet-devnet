use starknet_in_rust::transaction::{verify_version, Declare, DeclareV2};
use starknet_types::felt::{ClassHash, TransactionHash};

use crate::error::Result;
use crate::starknet::Starknet;
use crate::transactions::declare_transaction::DeclareTransactionV1;
use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
use crate::transactions::{StarknetTransaction, Transaction};

pub fn add_declare_transaction_v2(
    starknet: &mut Starknet,
    declare_transaction: DeclareTransactionV2,
) -> Result<(TransactionHash, ClassHash)> {
    let state_before_txn = starknet.state.pending_state.clone();
    let transaction_hash = declare_transaction.transaction_hash.unwrap();
    let class_hash = declare_transaction.class_hash.unwrap();

    match declare_transaction.inner.execute(&mut starknet.state.pending_state, &starknet.block_context) {
        Ok(tx_info) => {
            // Add sierra contract
            starknet.sierra_contracts.insert(class_hash, declare_transaction.inner.sierra_contract_class.clone());
            starknet.handle_successful_transaction(
                &transaction_hash,
                Transaction::DeclareV2(Box::new(declare_transaction)),
                tx_info,
            )?;
        }
        Err(tx_err) => {
            let transaction_to_add = StarknetTransaction::create_rejected(
                Transaction::DeclareV2(Box::new(declare_transaction)),
                tx_err,
            );

            starknet.transactions.insert(&transaction_hash, transaction_to_add);
            // Revert to previous pending state
            starknet.state.pending_state = state_before_txn;
        }
    }

    Ok((transaction_hash, class_hash))
}
pub fn add_declare_transaction_v1(
    starknet: &mut Starknet,
    declare_transaction: DeclareTransactionV1,
) -> Result<(TransactionHash, ClassHash)> {
    let state_before_txn = starknet.state.pending_state.clone();
    let transaction_hash = declare_transaction.transaction_hash.unwrap();
    let class_hash = declare_transaction.class_hash.unwrap();

    match declare_transaction.inner.execute(&mut starknet.state.pending_state, &starknet.block_context) {
        Ok(tx_info) => {
            starknet.handle_successful_transaction(
                &transaction_hash,
                Transaction::Declare(Box::new(declare_transaction)),
                tx_info,
            )?;
        }
        Err(tx_err) => {
            let transaction_to_add = StarknetTransaction::create_rejected(
                Transaction::Declare(Box::new(declare_transaction)),
                tx_err,
            );

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
    use starknet_in_rust::core::contract_address::compute_casm_class_hash;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::CasmContractClass;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self};
    use crate::starknet::{predeployed, Starknet};
    use crate::traits::{Accounted, Deployed, HashIdentifiedMut, StateExtractor};
    use crate::transactions::declare_transaction::DeclareTransactionV1;
    use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
    use crate::utils::load_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_cairo_0_contract_class, dummy_cairo_1_contract_class, dummy_felt,
    };

    fn test_declare_transaction_v2(sender_address: ContractAddress) -> DeclareTransactionV2 {
        let contract_class = dummy_cairo_1_contract_class();

        let compiled_class_hash =
            compute_casm_class_hash(&CasmContractClass::try_from(contract_class.clone()).unwrap())
                .unwrap();

        DeclareTransactionV2::new(
            contract_class,
            compiled_class_hash.into(),
            sender_address,
            100,
            Vec::new(),
            Felt::from(0),
            StarknetChainId::TestNet.to_felt().into(),
        )
        .unwrap()
    }

    fn test_declare_transaction_v1(sender_address: ContractAddress) -> DeclareTransactionV1 {
        let contract_class = dummy_cairo_0_contract_class();

        DeclareTransactionV1::new(
            sender_address,
            10000,
            Vec::new(),
            Felt::from(0),
            contract_class,
            StarknetChainId::TestNet.to_felt().into(),
        )
        .unwrap()
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let initial_cached_state =
            starknet.state.pending_state.casm_contract_classes().as_ref().unwrap().len();
        let declare_txn = test_declare_transaction_v2(sender);
        let (txn_hash, class_hash) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.status, TransactionStatus::Rejected);
        assert_eq!(
            initial_cached_state,
            starknet.state.pending_state.casm_contract_classes().as_ref().unwrap().len()
        );
        assert!(starknet.sierra_contracts.get(&class_hash).is_none())
    }

    #[test]
    fn add_declare_v2_transaction_successful_execution() {
        let (mut starknet, sender) = setup(Some(100000000));
        let declare_txn = test_declare_transaction_v2(sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v2(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(class_hash, declare_txn.sierra_contract_class.generate_hash().unwrap());
        // check if txn is with status accepted
        assert_eq!(tx.status, TransactionStatus::AcceptedOnL2);
        assert!(starknet.sierra_contracts.get(&class_hash).is_some())
    }

    #[test]
    fn declare_v2_transaction_successful_storage_change() {
        let (mut starknet, sender) = setup(Some(100000000));
        let declare_txn = test_declare_transaction_v2(sender);
        let expected_class_hash = declare_txn.sierra_contract_class.generate_hash().unwrap();
        let expected_compiled_class_hash = declare_txn.compiled_class_hash;

        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(&expected_class_hash));
        assert!(
            !starknet
                .state
                .state
                .casm_contract_classes_mut()
                .contains_key(&expected_compiled_class_hash.bytes())
        );

        let (tx_hash, retrieved_class_hash) =
            starknet.add_declare_transaction_v2(declare_txn).unwrap();

        let retrieved_txn = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(retrieved_class_hash, expected_class_hash);
        // check if txn is with status accepted
        assert_eq!(retrieved_txn.status, TransactionStatus::AcceptedOnL2);
        assert!(starknet.state.is_contract_declared(&expected_class_hash));
    }

    #[test]
    fn add_declare_v1_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let initial_cached_state =
            starknet.state.pending_state.contract_classes().as_ref().unwrap().len();
        let declare_txn = test_declare_transaction_v1(sender);
        let (txn_hash, _) = starknet.add_declare_transaction_v1(declare_txn).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.status, TransactionStatus::Rejected);
        assert_eq!(
            initial_cached_state,
            starknet.state.pending_state.contract_classes().as_ref().unwrap().len()
        );
    }

    #[test]
    fn add_declare_v1_transaction_successful_execution() {
        let (mut starknet, sender) = setup(None);

        let declare_txn = test_declare_transaction_v1(sender);
        let (tx_hash, class_hash) =
            starknet.add_declare_transaction_v1(declare_txn.clone()).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if generated class hash is expected one
        assert_eq!(class_hash, declare_txn.contract_class.generate_hash().unwrap());
        // check if txn is with status accepted
        assert_eq!(tx.status, TransactionStatus::AcceptedOnL2);
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
        let declare_txn = test_declare_transaction_v1(sender);

        let expected_class_hash = declare_txn.contract_class.generate_hash().unwrap();
        // check if contract is not declared
        assert!(!starknet.state.is_contract_declared(&expected_class_hash));

        let (tx_hash, class_hash) = starknet.add_declare_transaction_v1(declare_txn).unwrap();

        let tx = starknet.transactions.get_by_hash_mut(&tx_hash).unwrap();

        // check if txn is with status accepted
        assert_eq!(tx.status, TransactionStatus::AcceptedOnL2);
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
        let contract_class = load_cairo_0_contract_class(account_json_path).unwrap();

        let erc_20_contract = predeployed::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(acc_balance.unwrap_or(10000)),
            dummy_felt(),
            dummy_felt(),
            contract_class.generate_hash().unwrap(),
            contract_class,
            erc_20_contract.get_address(),
        )
        .unwrap();

        acc.deploy(&mut starknet.state).unwrap();
        acc.set_initial_balance(&mut starknet.state).unwrap();

        starknet.state.synchronize_states();
        starknet.block_context = Starknet::get_block_context(
            1,
            constants::ERC20_CONTRACT_ADDRESS,
            StarknetChainId::TestNet,
        )
        .unwrap();

        starknet.restart_pending_block().unwrap();

        (starknet, acc.get_address())
    }
}
