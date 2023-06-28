use starknet_in_rust::transaction::Declare;
use starknet_types::{DevnetResult, felt::{TransactionHash, ClassHash}, traits::HashProducer};

use crate::{Starknet, transactions::{declare_transaction::DeclareTransactionV1, StarknetTransaction, Transaction}};

impl Starknet {
    pub(crate) fn add_declare_transaction(
        &mut self,
        declare_transaction: DeclareTransactionV1,
    ) -> DevnetResult<(TransactionHash, ClassHash)> {
        let class_hash = declare_transaction.contract_class.clone().generate_hash()?;
        let transaction_hash = declare_transaction.generate_hash()?;

        let declare_transaction_result = Declare {
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
        }
        .execute(&mut self.state.pending_state, &self.block_context);

        let transaction_to_add = match declare_transaction_result {
            Ok(tx_info) => StarknetTransaction::create_pending(
                Transaction::Declare(declare_transaction),
                tx_info,
            ),
            Err(err) => {
                StarknetTransaction::create_rejected(Transaction::Declare(declare_transaction), err)
            }
        };

        self.transactions.insert(&transaction_hash, transaction_to_add);

        Ok((class_hash, class_hash))
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn add_declare_transaction_successful_execution() {

    }
}