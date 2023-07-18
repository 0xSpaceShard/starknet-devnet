use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix,
};
use starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::error::Error;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;

#[derive(Clone, PartialEq, Eq)]
pub struct DeclareTransactionV1 {
    pub sender_address: ContractAddress,
    pub max_fee: u128,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub contract_class: ContractClass,
    pub class_hash: Option<ClassHash>,
    pub transaction_hash: Option<TransactionHash>,
    pub chain_id: Felt,
}

impl DeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        contract_class: ContractClass,
        chain_id: Felt,
    ) -> Self {
        Self {
            sender_address,
            max_fee,
            signature,
            nonce,
            contract_class,
            class_hash: None,
            transaction_hash: None,
            chain_id,
        }
    }

    pub(crate) fn version(&self) -> Felt {
        Felt::from(1)
    }
}

impl HashProducer for DeclareTransactionV1 {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        let class_hash = self.class_hash.unwrap_or(self.contract_class.generate_hash()?);

        let (calldata, additional_data) = (Vec::new(), vec![class_hash.into()]);

        let transaction_hash: Felt = calculate_transaction_hash_common(
            TransactionHashPrefix::Declare,
            self.version().into(),
            &self.sender_address.try_into()?,
            VALIDATE_DECLARE_ENTRY_POINT_SELECTOR.clone(),
            &calldata,
            self.max_fee,
            self.chain_id.into(),
            &additional_data,
        )
        .map_err(|err| {
            Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::Syscall(err),
            )
        })?
        .into();

        Ok(transaction_hash)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn correct_transaction_hash_computation() {
        panic!("Transaction hash computation should be checked")
    }
}
