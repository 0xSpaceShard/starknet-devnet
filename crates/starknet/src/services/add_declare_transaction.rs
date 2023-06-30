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
    pub fn add_declare_transaction(
        &mut self,
        declare_transaction: DeclareTransactionV1,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        let mut declare_transaction = declare_transaction;
        let class_hash = declare_transaction.contract_class.clone().generate_hash()?;
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
                    Transaction::Declare(declare_transaction.clone()),
                    tx_err,
                );

                self.transactions.insert(&transaction_hash, transaction_to_add);
            }
        }

        Ok((class_hash, class_hash))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_declare_transaction_successful_execution() {
        assert!(false)
    }
}
