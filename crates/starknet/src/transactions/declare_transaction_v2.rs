use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix,
};
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;

use crate::error::{Error, Result};

#[derive(Clone, PartialEq, Eq)]
pub struct DeclareTransactionV2 {
    pub(crate) sierra_contract_class: ContractClass,
    pub(crate) compiled_class_hash: ClassHash,
    pub(crate) sender_address: ContractAddress,
    pub(crate) max_fee: u128,
    pub(crate) signature: Vec<Felt>,
    pub(crate) nonce: Felt,
    pub(crate) class_hash: Option<ClassHash>,
    pub(crate) transaction_hash: Option<TransactionHash>,
    pub(crate) chain_id: Felt,
    pub(crate) version: Felt,
}

impl DeclareTransactionV2 {
    pub fn new(
        sierra_contract_class: ContractClass,
        compiled_class_hash: ClassHash,
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        chain_id: Felt,
    ) -> Result<Self> {
        if max_fee == 0 {
            return Err(Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(
                    "For declare transaction version 2, max fee cannot be 0".to_string(),
                ),
            ));
        }

        Ok(Self {
            sierra_contract_class,
            compiled_class_hash,
            sender_address,
            max_fee,
            signature,
            nonce,
            class_hash: None,
            transaction_hash: None,
            chain_id,
            version: Felt::from(2),
        })
    }

    pub fn sender_address(&self) -> &ContractAddress {
        &self.sender_address
    }

    pub fn class_hash(&self) -> Option<&ClassHash> {
        self.class_hash.as_ref()
    }

    pub fn compiled_class_hash(&self) -> &ClassHash {
        &self.compiled_class_hash
    }
}

impl HashProducer for DeclareTransactionV2 {
    fn generate_hash(&self) -> DevnetResult<Felt> {
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
            self.chain_id.into(),
            &additional_data,
        )
        .map_err(|err| {
            starknet_types::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::Syscall(err),
            )
        })?
        .into();

        Ok(transaction_hash)
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::test_utils::{
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
    };

    #[ignore]
    #[test]
    fn correct_declare_transaction_hash_computation() {
        todo!("Transaction hash computation should be checked")
    }

    #[test]
    fn declare_transaction_v2_with_max_fee_zero_should_return_an_error() {
        let result = super::DeclareTransactionV2::new(
            dummy_cairo_1_contract_class(),
            dummy_felt(),
            dummy_contract_address(),
            0,
            vec![],
            dummy_felt(),
            dummy_felt(),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For declare transaction version 2, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }
}
