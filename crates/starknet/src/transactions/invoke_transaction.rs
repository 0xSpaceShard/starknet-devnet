use starknet_in_rust::definitions::constants::EXECUTE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::transaction::InvokeFunction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

use crate::error::{self, Result};

#[derive(Clone)]
pub struct InvokeTransactionV1(pub(crate) InvokeFunction);

impl Eq for InvokeTransactionV1 {}

impl PartialEq for InvokeTransactionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.0.signature() == other.0.signature()
            && self.0.contract_address() == other.0.contract_address()
            && self.0.hash_value() == other.0.hash_value()
    }
}

impl InvokeTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        calldata: Vec<Felt>,
        chain_id: Felt,
    ) -> Result<Self> {
        if max_fee == 0 {
            return Err(error::Error::TransactionError(TransactionError::FeeError(
                "For invoke transaction, max fee cannot be 0".to_string(),
            )));
        }

        Ok(Self(starknet_in_rust::transaction::InvokeFunction::new(
            sender_address.try_into()?,
            EXECUTE_ENTRY_POINT_SELECTOR.clone(),
            max_fee,
            Felt::from(1).into(),
            calldata.iter().map(|f| f.into()).collect(),
            signature.iter().map(|f| f.into()).collect(),
            chain_id.into(),
            Some(nonce.into()),
        )?))
    }
}

impl HashProducer for InvokeTransactionV1 {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        Ok(self.0.hash_value().clone().into())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::test_utils::{dummy_contract_address, dummy_felt};

    #[test]
    #[ignore]
    fn correct_transaction_hash_computation() {
        todo!("Transaction hash computation should be checked")
    }

    #[test]
    fn invoke_transaction_with_max_fee_zero_should_return_error() {
        let result = super::InvokeTransactionV1::new(
            dummy_contract_address(),
            0,
            vec![],
            dummy_felt(),
            vec![],
            dummy_felt(),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For invoke transaction, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }
}
