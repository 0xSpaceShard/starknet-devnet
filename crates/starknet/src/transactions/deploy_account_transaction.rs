use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::transaction::DeployAccount;
use starknet_types::error::Error;
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::traits::HashProducer;

use crate::error::{self, Result};

#[derive(Clone)]
pub struct DeployAccountTransaction {
    pub inner: DeployAccount,
    pub chain_id: Felt,
    pub signature: Vec<Felt>,
    pub max_fee: u128,
    pub nonce: Felt,
    pub version: Felt,
}

impl Eq for DeployAccountTransaction {}

impl PartialEq for DeployAccountTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.inner.signature() == other.inner.signature()
            && self.inner.constructor_calldata() == other.inner.constructor_calldata()
            && self.inner.class_hash() == other.inner.class_hash()
            && self.inner.contract_address_salt() == other.inner.contract_address_salt()
    }
}

impl DeployAccountTransaction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        constructor_calldata: Vec<Felt>,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        class_hash: ClassHash,
        contract_address_salt: Felt,
        chain_id: Felt,
        version: Felt,
    ) -> Result<Self> {
        if max_fee == 0 {
            return Err(error::Error::TransactionError(TransactionError::FeeError(
                "For deploy account transaction, max fee cannot be 0".to_string(),
            )));
        }

        let starknet_in_rust_deploy_account = DeployAccount::new(
            class_hash.bytes(),
            max_fee,
            version.into(),
            nonce.into(),
            constructor_calldata.iter().map(|h| h.into()).collect(),
            signature.iter().map(|h| h.into()).collect(),
            contract_address_salt.into(),
            chain_id.into(),
        )
        .map_err(|err| Error::TransactionError(TransactionError::Syscall(err)))?;

        Ok(Self {
            inner: starknet_in_rust_deploy_account,
            chain_id,
            signature,
            nonce,
            max_fee,
            version,
        })
    }

    pub fn class_hash(&self) -> Result<Felt> {
        Felt::new(*self.inner.class_hash()).map_err(error::Error::from)
    }

    pub fn contract_address_salt(&self) -> Felt {
        (self.inner.contract_address_salt().clone()).into()
    }

    pub fn constructor_calldata(&self) -> Vec<Felt> {
        self.inner.constructor_calldata().clone().into_iter().map(|felt| felt.into()).collect()
    }
}

impl HashProducer for DeployAccountTransaction {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        Ok(self.inner.hash_value().clone().into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn correct_transaction_hash_computation() {
        todo!("Transaction hash computation should be checked")
    }

    #[test]
    fn account_deploy_transaction_with_max_fee_zero_should_return_an_error() {
        let result = super::DeployAccountTransaction::new(
            vec![0.into(), 1.into()],
            0,
            vec![0.into(), 1.into()],
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
        );

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::error::Error::TransactionError(
                starknet_in_rust::transaction::error::TransactionError::FeeError(msg),
            ) => assert_eq!(msg, "For deploy account transaction, max fee cannot be 0"),
            _ => panic!("Wrong error type"),
        }
    }
}
