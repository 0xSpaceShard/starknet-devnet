use starknet_in_rust::core::{
    transaction_hash::{
        calculate_transaction_hash_common,
        TransactionHashPrefix,
    },
};
use starknet_types::{
    contract_address::ContractAddress,
    contract_class::ContractClass,
    error::Error,
    felt::{ClassHash, Felt, TransactionHash},
    traits::HashProducer,
};

use crate::constants;

#[derive(Clone, PartialEq, Eq)]
pub struct DeclareTransactionV2 {
    pub sierra_contract_class: ContractClass,
    pub compiled_class_hash: ClassHash,
    pub sender_address: ContractAddress,
    pub max_fee: u128,
    pub version: Felt,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub class_hash: Option<ClassHash>,
    pub transaction_hash: Option<TransactionHash>,
}

impl HashProducer for DeclareTransactionV2 {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        let class_hash = self.class_hash.unwrap_or(self.sierra_contract_class.generate_hash()?);

        let calldata = [class_hash.into()].to_vec();
        let additional_data = [self.nonce.into(), self.compiled_class_hash.into()].to_vec();

        let transaction_hash: Felt = calculate_transaction_hash_common(
            TransactionHashPrefix::Declare,
            self.version.into(),
            &self.sender_address.try_into()?,
            Felt::from(0).into(),
            &calldata,
            self.max_fee,
            constants::CHAIN_ID.to_felt(),
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

    #[ignore]
    #[test]
    fn correct_declare_transaction_hash_computation() {
        panic!("Transaction hash computation should be checked")
    }
}
