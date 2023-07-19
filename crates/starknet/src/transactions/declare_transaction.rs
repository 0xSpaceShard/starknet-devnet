use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix,
};
use starknet_in_rust::definitions::constants::VALIDATE_DECLARE_ENTRY_POINT_SELECTOR;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;

use crate::error::{Error, Result};

#[derive(Clone, PartialEq, Eq)]
pub struct DeclareTransactionV1 {
    pub(crate) sender_address: ContractAddress,
    pub(crate) max_fee: u128,
    pub(crate) signature: Vec<Felt>,
    pub(crate) nonce: Felt,
    pub(crate) contract_class: ContractClass,
    pub(crate) class_hash: Option<ClassHash>,
    pub(crate) transaction_hash: Option<TransactionHash>,
    pub(crate) chain_id: Felt,
}

impl DeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        contract_class: ContractClass,
        chain_id: Felt,
    ) -> Result<Self> {
        if max_fee == 0 {
            return Err(Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(
                    "For declare transaction version 1, max fee cannot be 0".to_string(),
                ),
            ));
        }

        Ok(Self {
            sender_address,
            max_fee,
            signature,
            nonce,
            contract_class,
            class_hash: None,
            transaction_hash: None,
            chain_id,
        })
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
        dummy_cairo_0_contract_class, dummy_contract_address, dummy_felt,
    };

    #[test]
    #[ignore]
    fn correct_transaction_hash_computation() {
        todo!("Transaction hash computation should be checked")
    }

    #[test]
    fn declare_transaction_v1_with_max_fee_zero_should_return_an_error() {
        let result = super::DeclareTransactionV1::new(
            dummy_contract_address(),
            0,
            vec![],
            dummy_felt(),
            dummy_cairo_0_contract_class(),
            dummy_felt(),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For declare transaction version 1, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }
}
