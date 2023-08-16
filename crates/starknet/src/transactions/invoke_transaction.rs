use starknet_in_rust::definitions::constants::EXECUTE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::transaction::InvokeFunction;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::traits::HashProducer;

use crate::error::{self, Result};

#[derive(Clone)]
pub struct InvokeTransactionV1 {
    pub inner: InvokeFunction,
    pub chain_id: Felt,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub calldata: Vec<Felt>,
    pub max_fee: u128,
    pub version: Felt,
}

impl Eq for InvokeTransactionV1 {}

impl PartialEq for InvokeTransactionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.inner.signature() == other.inner.signature()
            && self.inner.contract_address() == other.inner.contract_address()
            && self.inner.hash_value() == other.inner.hash_value()
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
        version: Felt,
    ) -> Result<Self> {
        Ok(Self {
            inner: starknet_in_rust::transaction::InvokeFunction::new(
                sender_address.try_into()?,
                EXECUTE_ENTRY_POINT_SELECTOR.clone(),
                max_fee,
                version.into(),
                calldata.iter().map(|f| f.into()).collect(),
                signature.iter().map(|f| f.into()).collect(),
                chain_id.into(),
                Some(nonce.into()),
            )?,
            chain_id,
            signature,
            nonce,
            calldata,
            max_fee,
            version,
        })
    }

    pub fn sender_address(&self) -> Result<ContractAddress> {
        self.inner.contract_address().clone().try_into().map_err(error::Error::from)
    }

    pub fn calldata(&self) -> &Vec<Felt> {
        &self.calldata
    }
}

impl HashProducer for InvokeTransactionV1 {
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
    struct FeederGatewayInvokeTransaction {
        transaction_hash: Felt,
        sender_address: Felt,
        max_fee: Felt,
        nonce: Felt,
        calldata: Vec<Felt>,
        version: Felt,
    }

    /// Get transaction from feeder gateway by hash and then using the same parameters compute the
    /// transaction hash
    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/sequencer_response/invoke_v1_testnet_0x068fbb499e59af504491b801b694cb5b7450a2efc338f7480cb1887ea2c9bd01.json"
        )).unwrap()).unwrap();

        let feeder_gateway_transaction: FeederGatewayInvokeTransaction =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let transaction = super::InvokeTransactionV1::new(
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            u128::from_str_radix(&feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(), 16)
                .unwrap(),
            vec![],
            feeder_gateway_transaction.nonce,
            feeder_gateway_transaction.calldata,
            StarknetChainId::TestNet.to_felt().into(),
            feeder_gateway_transaction.version,
        )
        .unwrap();

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            transaction.generate_hash().unwrap()
        );
    }
}
