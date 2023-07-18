use starknet_in_rust::transaction::{verify_version, Declare, DeclareV2};
use starknet_types::felt::{ClassHash, TransactionHash};
use starknet_types::traits::HashProducer;

use crate::error::{Error, Result};
use crate::transactions::declare_transaction::DeclareTransactionV1;
use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
use crate::transactions::{StarknetTransaction, Transaction};
use crate::Starknet;

impl Starknet {
    pub fn add_declare_transaction_v2(
        &mut self,
        declare_transaction: DeclareTransactionV2,
    ) -> Result<(TransactionHash, ClassHash)> {
        let mut declare_transaction = declare_transaction;

        let class_hash = declare_transaction.sierra_contract_class.generate_hash()?;
        declare_transaction.class_hash = Some(class_hash);

        let transaction_hash = declare_transaction.generate_hash()?;
        declare_transaction.transaction_hash = Some(transaction_hash);

        // TODO compile the contract and check the hash
        let transaction = DeclareV2::new(
            &declare_transaction.sierra_contract_class.clone().try_into()?,
            None,
            declare_transaction.compiled_class_hash.into(),
            declare_transaction.chain_id.into(),
            declare_transaction.sender_address.try_into()?,
            declare_transaction.max_fee,
            declare_transaction.version().into(),
            declare_transaction.signature.iter().map(|felt| felt.into()).collect(),
            declare_transaction.nonce.into(),
        )?;

        if transaction.max_fee == 0 {
            return Err(Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(
                    "For declare transaction version 2, max fee cannot be 0".to_string(),
                ),
            ));
        }

        let state_before_txn = self.state.pending_state.clone();

        match transaction.execute(&mut self.state.pending_state, &self.block_context) {
            Ok(tx_info) => {
                declare_transaction.class_hash = Some(class_hash);

                self.handle_successful_transaction(
                    &transaction_hash,
                    Transaction::DeclareV2(declare_transaction),
                    tx_info,
                )?;
            }
            Err(tx_err) => {
                let transaction_to_add = StarknetTransaction::create_rejected(
                    Transaction::DeclareV2(declare_transaction),
                    tx_err,
                );

                self.transactions.insert(&transaction_hash, transaction_to_add);
                // Revert to previous pending state
                self.state.pending_state = state_before_txn;
            }
        }

        Ok((transaction_hash, class_hash))
    }
    pub fn add_declare_transaction_v1(
        &mut self,
        declare_transaction: DeclareTransactionV1,
    ) -> Result<(TransactionHash, ClassHash)> {
        let mut declare_transaction = declare_transaction;

        let class_hash = declare_transaction.contract_class.generate_hash()?;
        let transaction_hash = declare_transaction.generate_hash()?;
        declare_transaction.transaction_hash = Some(transaction_hash);

        let transaction = Declare {
            class_hash: class_hash.into(),
            sender_address: declare_transaction.sender_address.try_into()?,
            tx_type: starknet_in_rust::definitions::transaction_type::TransactionType::Declare,
            validate_entry_point_selector:
                starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR
                    .clone(),
            version: declare_transaction.version().into(),
            max_fee: declare_transaction.max_fee,
            signature: declare_transaction.signature.iter().map(|felt| felt.into()).collect(),
            nonce: declare_transaction.nonce.into(),
            hash_value: transaction_hash.into(),
            contract_class: declare_transaction.contract_class.clone().try_into()?,
            skip_execute: false,
            skip_fee_transfer: false,
            skip_validate: false,
        };

        verify_version(
            &transaction.version,
            transaction.max_fee,
            &transaction.nonce,
            &transaction.signature,
        )?;

        if transaction.max_fee == 0 {
            return Err(Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(
                    "For declare transaction version 1, max fee cannot be 0".to_string(),
                ),
            ));
        }

        let state_before_txn = self.state.pending_state.clone();

        match transaction.execute(&mut self.state.pending_state, &self.block_context) {
            Ok(tx_info) => {
                declare_transaction.class_hash = Some(class_hash);

                self.handle_successful_transaction(
                    &transaction_hash,
                    Transaction::Declare(declare_transaction),
                    tx_info,
                )?;
            }
            Err(tx_err) => {
                let transaction_to_add = StarknetTransaction::create_rejected(
                    Transaction::Declare(declare_transaction),
                    tx_err,
                );

                self.transactions.insert(&transaction_hash, transaction_to_add);
                // Revert to previous pending state
                self.state.pending_state = state_before_txn;
            }
        }

        Ok((transaction_hash, class_hash))
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::BlockNumber;
    use starknet_in_rust::core::contract_address::compute_casm_class_hash;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_in_rust::transaction::error::TransactionError;
    use starknet_in_rust::CasmContractClass;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::felt::Felt;
    use starknet_types::traits::HashProducer;

    use crate::account::Account;
    use crate::constants::{self};
    use crate::error::Error;
    use crate::traits::{Accounted, HashIdentifiedMut, StateChanger};
    use crate::transactions::declare_transaction::DeclareTransactionV1;
    use crate::transactions::declare_transaction_v2::DeclareTransactionV2;
    use crate::utils::load_cairo_0_contract_class;
    use crate::utils::test_utils::dummy_felt;
    use crate::Starknet;

    fn test_declare_transaction_v2(sender_address: ContractAddress) -> DeclareTransactionV2 {
        let contract_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/declare/declare_cairo_1_test.json"
        );
        let json_str = std::fs::read_to_string(contract_json_path).unwrap();
        let contract_class = ContractClass::cairo_1_from_sierra_json_str(&json_str).unwrap();

        let compiled_class_hash =
            compute_casm_class_hash(&CasmContractClass::try_from(contract_class.clone()).unwrap())
                .unwrap();

        DeclareTransactionV2 {
            sierra_contract_class: contract_class,
            compiled_class_hash: compiled_class_hash.into(),
            sender_address,
            max_fee: 100,
            signature: Vec::new(),
            nonce: Felt::from(0),
            class_hash: None,
            transaction_hash: None,
            chain_id: StarknetChainId::TestNet.to_felt().into(),
        }
    }

    fn test_declare_transaction_v1(sender_address: ContractAddress) -> DeclareTransactionV1 {
        let contract_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/declare/declare_cairo_0_test.json"
        );

        let contract_class = load_cairo_0_contract_class(contract_json_path).unwrap();
        DeclareTransactionV1 {
            sender_address,
            max_fee: 10000,
            signature: Vec::new(),
            nonce: Felt::from(0),
            contract_class,
            class_hash: None,
            transaction_hash: None,
            chain_id: StarknetChainId::TestNet.to_felt().into(),
        }
    }

    #[test]
    fn add_declare_v2_transaction_with_zero_max_fee_should_be_errored() {
        let (mut starknet, sender) = setup(None);
        let mut declare_txn = test_declare_transaction_v2(sender);
        declare_txn.max_fee = 0;
        let expected_error = TransactionError::FeeError(String::from(
            "For declare transaction version 2, max fee cannot be 0",
        ));

        match starknet.add_declare_transaction_v2(declare_txn).err().unwrap() {
            Error::TransactionError(generated_error) => {
                assert_eq!(generated_error.to_string(), expected_error.to_string());
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn add_declare_v2_transaction_should_return_rejected_txn_and_not_be_part_of_pending_state() {
        let (mut starknet, sender) = setup(Some(1));
        let initial_cached_state =
            starknet.state.pending_state.casm_contract_classes().as_ref().unwrap().len();
        let declare_txn = test_declare_transaction_v2(sender);
        let (txn_hash, _) = starknet.add_declare_transaction_v2(declare_txn).unwrap();
        let txn = starknet.transactions.get_by_hash_mut(&txn_hash).unwrap();

        assert_eq!(txn.status, TransactionStatus::Rejected);
        assert_eq!(
            initial_cached_state,
            starknet.state.pending_state.casm_contract_classes().as_ref().unwrap().len()
        );
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
    fn add_declare_v1_transaction_with_zero_max_fee_should_be_errored() {
        let (mut starknet, sender) = setup(None);
        let mut declare_txn = test_declare_transaction_v1(sender);
        declare_txn.max_fee = 0;
        let expected_error = TransactionError::FeeError(String::from(
            "For declare transaction version 1, max fee cannot be 0",
        ));

        match starknet.add_declare_transaction_v1(declare_txn).err().unwrap() {
            Error::TransactionError(generated_error) => {
                assert_eq!(generated_error.to_string(), expected_error.to_string());
            }
            _ => panic!("Wrong error type"),
        }
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
        assert!(starknet.state.is_contract_declared(&class_hash).unwrap());
        // check if pending block is resetted
        assert!(starknet.pending_block().get_transactions().is_empty());
        // check if there is generated block
        assert_eq!(starknet.blocks.num_to_block.len(), 1);
        // check if transaction is in generated block
        assert_eq!(
            starknet
                .blocks
                .num_to_block
                .get(&BlockNumber(0))
                .unwrap()
                .get_transactions()
                .first()
                .unwrap()
                .get_hash()
                .unwrap(),
            tx_hash
        );
    }

    /// Initializes starknet with 1 account - account without validations
    fn setup(acc_balance: Option<u128>) -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/account_without_validations/account.json"
        );
        let contract_class = load_cairo_0_contract_class(account_json_path).unwrap();

        let erc_20_contract = Starknet::create_erc20().unwrap();
        erc_20_contract.deploy(&mut starknet.state).unwrap();

        let acc = Account::new(
            Felt::from(acc_balance.unwrap_or(100)),
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
