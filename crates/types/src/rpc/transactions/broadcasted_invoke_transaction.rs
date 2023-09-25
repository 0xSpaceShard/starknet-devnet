use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::definitions::constants::EXECUTE_ENTRY_POINT_SELECTOR;
use starknet_in_rust::transaction::InvokeFunction as SirInvokeFunction;

use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{
    Calldata, Felt, Nonce, TransactionHash, TransactionSignature, TransactionVersion,
};
use crate::rpc::transactions::invoke_transaction_v1::InvokeTransactionV1;
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedInvokeTransaction {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl BroadcastedInvokeTransaction {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        calldata: &Calldata,
        version: TransactionVersion,
    ) -> Self {
        Self {
            sender_address,
            calldata: calldata.clone(),
            common: BroadcastedTransactionCommon {
                max_fee,
                signature: signature.clone(),
                nonce,
                version,
            },
        }
    }

    pub fn create_sir_invoke_function(&self, chain_id: Felt) -> DevnetResult<SirInvokeFunction> {
        Ok(SirInvokeFunction::new(
            self.sender_address.into(),
            EXECUTE_ENTRY_POINT_SELECTOR.clone(),
            self.common.max_fee.0,
            self.common.version.into(),
            self.calldata.iter().map(|f| f.into()).collect(),
            self.common.signature.iter().map(|f| f.into()).collect(),
            chain_id.into(),
            Some(self.common.nonce.into()),
        )?)
    }

    pub fn create_invoke_transaction(
        &self,
        transaction_hash: TransactionHash,
    ) -> InvokeTransactionV1 {
        InvokeTransactionV1 {
            transaction_hash,
            max_fee: self.common.max_fee,
            version: self.common.version,
            signature: self.common.signature.clone(),
            nonce: self.common.nonce,
            sender_address: self.sender_address,
            calldata: self.calldata.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::Fee;
    use starknet_in_rust::definitions::block_context::StarknetChainId;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_invoke_transaction::BroadcastedInvokeTransaction;
    use crate::traits::ToHexString;

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
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/invoke_v1_testnet_0x068fbb499e59af504491b801b694cb5b7450a2efc338f7480cb1887ea2c9bd01.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayInvokeTransaction =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let transaction = BroadcastedInvokeTransaction::new(
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            Fee(u128::from_str_radix(
                &feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(),
                16,
            )
            .unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            &feeder_gateway_transaction.calldata,
            feeder_gateway_transaction.version,
        );

        let transaction = transaction
            .create_sir_invoke_function(ChainId::TestNet.to_felt())
            .unwrap();

        assert_eq!(feeder_gateway_transaction.transaction_hash, transaction.hash_value().into());
    }
}
