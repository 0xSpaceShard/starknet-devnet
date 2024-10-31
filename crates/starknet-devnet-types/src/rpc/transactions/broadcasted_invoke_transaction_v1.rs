use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;

use crate::contract_address::ContractAddress;
use crate::felt::{Calldata, Nonce, TransactionSignature, TransactionVersion};
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
}

impl BroadcastedInvokeTransactionV1 {
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
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::Fee;
    use starknet_core::types::Felt;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::felt::try_felt_to_num;
    use crate::rpc::transactions::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
    use crate::rpc::transactions::BroadcastedInvokeTransaction;

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

        let transaction = BroadcastedInvokeTransactionV1::new(
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            Fee(try_felt_to_num(feeder_gateway_transaction.max_fee).unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            &feeder_gateway_transaction.calldata,
            feeder_gateway_transaction.version,
        );

        let chain_id = ChainId::goerli_legacy_id();
        let blockifier_transaction = BroadcastedInvokeTransaction::V1(transaction)
            .create_blockifier_invoke_transaction(&chain_id, false)
            .unwrap();

        assert_eq!(feeder_gateway_transaction.transaction_hash, blockifier_transaction.tx_hash.0);
    }
}
