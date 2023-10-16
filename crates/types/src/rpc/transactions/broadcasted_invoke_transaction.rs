use std::sync::Arc;

use blockifier::transaction::transactions::InvokeTransaction;
use cairo_felt::Felt252;
use serde::{Deserialize, Serialize};
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::Fee;
use starknet_in_rust::core::transaction_hash::{
    calculate_transaction_hash_common, TransactionHashPrefix,
};

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

    pub fn create_blockifier_invoke_transaction(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<InvokeTransaction> {
        let entry_point_selector_field = Felt252::from(0u8);
        let additional_data = vec![self.common.nonce];

        let txn_hash: Felt = calculate_transaction_hash_common(
            TransactionHashPrefix::Invoke,
            self.common.version.into(),
            &self.sender_address.into(),
            entry_point_selector_field,
            self.calldata.iter().map(Felt252::from).collect::<Vec<Felt252>>().as_slice(),
            self.common.max_fee.0,
            chain_id.into(),
            additional_data.iter().map(Felt252::from).collect::<Vec<Felt252>>().as_slice(),
        )?
        .into();

        let sn_api_transaction = starknet_api::transaction::InvokeTransactionV1 {
            max_fee: self.common.max_fee,
            signature: starknet_api::transaction::TransactionSignature(
                self.common.signature.iter().map(|f| f.into()).collect(),
            ),
            nonce: starknet_api::core::Nonce(self.common.nonce.into()),
            sender_address: self.sender_address.try_into()?,
            calldata: starknet_api::transaction::Calldata(Arc::new(
                self.calldata.iter().map(StarkFelt::from).collect::<Vec<StarkFelt>>(),
            )),
        };

        Ok(InvokeTransaction {
            tx: starknet_api::transaction::InvokeTransaction::V1(sn_api_transaction),
            tx_hash: starknet_api::transaction::TransactionHash(txn_hash.into()),
        })
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

        let chain_id = ChainId::TestNet.to_felt();
        let blockifier_transaction =
            transaction.create_blockifier_invoke_transaction(chain_id).unwrap();

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            blockifier_transaction.tx_hash.0.into()
        );
    }
}
