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
    use serde::Deserialize;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;
    use starknet_types::traits::{HashProducer, ToHexString};

    #[derive(Deserialize)]
    struct FeederGatewayDeployAccountTransaction {
        transaction_hash: Felt,
        version: Felt,
        max_fee: Felt,
        nonce: Felt,
        constructor_calldata: Vec<Felt>,
        contract_address: Felt,
        contract_address_salt: Felt,
        class_hash: Felt,
    }

    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/sequencer_response/deploy_account_testnet_0x23a872d966d4f6091cc3725604fdaa1b39cef76ebf38b9a06a0b71e9ed700ea.json"
        )).unwrap()).unwrap();

        let feeder_gateway_transaction: FeederGatewayDeployAccountTransaction =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let deploy_account_transaction = super::DeployAccountTransaction::new(
            feeder_gateway_transaction.constructor_calldata,
            u128::from_str_radix(&feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(), 16)
                .unwrap(),
            vec![],
            feeder_gateway_transaction.nonce,
            feeder_gateway_transaction.class_hash,
            feeder_gateway_transaction.contract_address_salt,
            StarknetChainId::TestNet.to_felt().into(),
            feeder_gateway_transaction.version,
        )
        .unwrap();

        assert_eq!(
            ContractAddress::new(feeder_gateway_transaction.contract_address).unwrap(),
            ContractAddress::try_from(deploy_account_transaction.inner.contract_address().clone())
                .unwrap()
        );
        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            deploy_account_transaction.generate_hash().unwrap()
        );
    }
}
