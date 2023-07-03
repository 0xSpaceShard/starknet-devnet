use starknet_api::transaction::TransactionHash;
use starknet_in_rust::{
    core::transaction_hash::calculate_declare_v2_transaction_hash, SierraContractClass,
};
use starknet_types::{
    contract_address::ContractAddress,
    error::Error,
    felt::{ClassHash, Felt},
    traits::HashProducer,
};

use crate::constants;

#[derive(Clone, PartialEq, Eq)]
pub struct DeclareTransactionV2 {
    sierra_contract_class: SierraContractClass,
    compiled_class_hash: ClassHash,
    sender_address: ContractAddress,
    max_fee: u128,
    version: Felt,
    signature: Vec<Felt>,
    nonce: Felt,
    class_hash: Option<ClassHash>,
    transaction_hash: Option<TransactionHash>,
}

impl HashProducer for DeclareTransactionV2 {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        let felt_252 = calculate_declare_v2_transaction_hash(
            &self.sierra_contract_class,
            self.compiled_class_hash.into(),
            constants::CHAIN_ID.to_felt(),
            &self.sender_address.try_into()?,
            self.max_fee,
            self.version.into(),
            self.nonce.into(),
        )
        .map_err(|err| {
            Error::StarknetInRustTransactionError(
                starknet_in_rust::transaction::error::TransactionError::Syscall(err),
            )
        })?;

        Ok(Felt::from(felt_252))
    }
}
