use starknet_in_rust::transaction::Declare;
use starknet_types::{
    felt::{ClassHash, TransactionHash},
    traits::HashProducer,
    DevnetResult,
};

use crate::{
    traits::StateChanger,
    transactions::{declare_transaction::DeclareTransactionV1, StarknetTransaction, Transaction},
    Starknet,
};

impl Starknet {
    pub fn add_declare_transaction_v1(
        &mut self,
        declare_transaction: DeclareTransactionV1,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        let mut declare_transaction = declare_transaction;

        let class_hash = declare_transaction.contract_class.generate_hash()?;
        declare_transaction.class_hash = Some(class_hash);

        let transaction_hash = declare_transaction.generate_hash()?;
        declare_transaction.transaction_hash = Some(transaction_hash);

        let transaction = Declare {
            class_hash: class_hash.into(),
            sender_address: declare_transaction.sender_address.try_into()?,
            tx_type: starknet_in_rust::definitions::transaction_type::TransactionType::Declare,
            validate_entry_point_selector:
                starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR
                    .clone(),
            version: declare_transaction.version.into(),
            max_fee: declare_transaction.max_fee,
            signature: declare_transaction.signature.iter().map(|felt| felt.into()).collect(),
            nonce: declare_transaction.nonce.into(),
            hash_value: transaction_hash.into(),
            contract_class: declare_transaction.contract_class.clone().try_into()?,
        };

        transaction.verify_version()?;

        match transaction.execute(&mut self.state.pending_state, &self.block_context) {
            Ok(tx_info) => {
                let transaction_to_add = StarknetTransaction::create_successful(
                    Transaction::Declare(declare_transaction.clone()),
                    tx_info,
                );

                // add accepted transaction to pending block
                self.blocks
                    .pending_block
                    .add_transaction(Transaction::Declare(declare_transaction));

                // add transaction to transactions
                self.transactions.insert(&transaction_hash, transaction_to_add);

                // create new block from pending one
                self.generate_new_block()?;
                // apply state changes from cached state
                self.state.apply_cached_state()?;
                // make cached state part of "persistent" state
                self.state.equalize_states();
                // clear pending block information
                self.generate_pending_block()?;
            }
            Err(tx_err) => {
                let transaction_to_add = StarknetTransaction::create_rejected(
                    Transaction::Declare(declare_transaction),
                    tx_err,
                );

                self.transactions.insert(&transaction_hash, transaction_to_add);
            }
        }

        Ok((transaction_hash, class_hash))
    }
}

#[cfg(test)]
mod tests {
    use starknet_api::block::BlockNumber;
    use starknet_rs_core::types::TransactionStatus;
    use starknet_types::{
        contract_address::ContractAddress, contract_class::ContractClass, felt::Felt,
        traits::HashProducer,
    };

    use crate::{
        constants,
        traits::{HashIdentifiedMut, StateChanger},
        transactions::declare_transaction::DeclareTransactionV1,
        utils::test_utils::dummy_contract_address,
        Starknet,
    };

    fn test_declare_transaction_v1(sender_address: ContractAddress) -> DeclareTransactionV1 {
        let contract_json_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/declare/declare_test.json");
        let contract_class =
            ContractClass::from_json_str(&std::fs::read_to_string(contract_json_path).unwrap())
                .unwrap();
        DeclareTransactionV1 {
            sender_address,
            version: Felt::from(1),
            max_fee: 0,
            signature: Vec::new(),
            nonce: Felt::from(0),
            contract_class,
            class_hash: None,
            transaction_hash: None,
        }
    }

    #[test]
    fn add_declare_transaction_successful_execution() {
        let (mut starknet, sender) = setup();

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
    fn setup() -> (Starknet, ContractAddress) {
        let mut starknet = Starknet::default();
        let account_json_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/accounts_artifacts/simple_account/account.json");
        let contract_class =
            ContractClass::from_json_str(&std::fs::read_to_string(account_json_path).unwrap())
                .unwrap();

        let class_hash = contract_class.generate_hash().unwrap();
        let address = dummy_contract_address();

        starknet.state.declare_contract_class(class_hash, contract_class).unwrap();
        starknet.state.deploy_contract(address, class_hash).unwrap();

        starknet.state.equalize_states();
        starknet.block_context =
            Starknet::get_block_context(0, constants::ERC20_CONTRACT_ADDRESS).unwrap();

        starknet.empty_pending_block().unwrap();

        (starknet, address)
    }
}
