use starknet_in_rust::{
    core::transaction_hash::{calculate_transaction_hash_common, TransactionHashPrefix},
    definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR,
};
use starknet_types::{
    cairo_felt::Felt252,
    contract_address::ContractAddress,
    contract_class::ContractClass,
    error::Error,
    felt::{ClassHash, Felt, TransactionHash},
    traits::HashProducer,
};

use crate::constants;

#[derive(Clone)]
pub struct DeclareTransactionV1 {
    pub sender_address: ContractAddress,
    pub version: Felt,
    pub max_fee: u128,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub contract_class: ContractClass,
    pub class_hash: Option<ClassHash>,
    pub transaction_hash: Option<TransactionHash>,
}

impl DeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        version: Felt,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        contract_class: ContractClass,
    ) -> Self {
        Self {
            sender_address,
            version,
            max_fee,
            signature,
            nonce,
            contract_class,
            class_hash: None,
            transaction_hash: None,
        }
    }
}

impl HashProducer for DeclareTransactionV1 {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        let class_hash = self.contract_class.generate_hash()?;

        let (calldata, additional_data) = if self.version.is_zero() {
            (vec![class_hash.into()], vec![self.nonce.into()])
        } else {
            (Vec::new(), vec![class_hash.into()])
        };

        let transaction_hash = calculate_transaction_hash_common(
            TransactionHashPrefix::Declare,
            self.version.into(),
            &self.sender_address.try_into()?,
            Felt::from(0u128).into(),
            &calldata,
            self.max_fee,
            constants::CHAIN_ID.to_felt(),
            &additional_data,
        )
        .map_err(|err| {
            Error::StarknetInRustTransactionError(
                starknet_in_rust::transaction::error::TransactionError::Syscall(err),
            )
        })?;

        Ok(transaction_hash.into())
    }
}
