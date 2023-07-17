use starknet_in_rust::transaction::error::TransactionError;
use starknet_in_rust::transaction::DeployAccount;
use starknet_types::error::Error;
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::traits::HashProducer;

use crate::error::Result;

#[derive(Clone)]
pub struct DeployAccountTransaction(pub(crate) DeployAccount);

impl Eq for DeployAccountTransaction {}

impl PartialEq for DeployAccountTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.0.signature() == other.0.signature()
            && self.0.constructor_calldata() == other.0.constructor_calldata()
            && self.0.class_hash() == other.0.class_hash()
            && self.0.contract_address_salt() == other.0.contract_address_salt()
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
        let starknet_in_rust_deploy_account = DeployAccount::new(
            class_hash.bytes(),
            max_fee,
            version.into(),
            nonce.into(),
            constructor_calldata.iter().map(|h| h.into()).collect(),
            signature.iter().map(|h| h.into()).collect(),
            contract_address_salt.into(),
            chain_id.into(),
            None,
        )
        .map_err(|err| Error::TransactionError(TransactionError::Syscall(err)))?;

        Ok(Self(starknet_in_rust_deploy_account))
    }
}

impl HashProducer for DeployAccountTransaction {
    fn generate_hash(&self) -> starknet_types::DevnetResult<Felt> {
        Ok(self.0.hash_value().clone().into())
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
