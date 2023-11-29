use std::sync::Arc;

use blockifier::transaction::transactions::InvokeTransaction;
use serde::{Deserialize, Serialize};
use starknet_api::hash::StarkFelt;
use starknet_rs_crypto::poseidon_hash_many;
use starknet_rs_ff::FieldElement;

use super::broadcasted_invoke_transaction_v1::PREFIX_INVOKE;
use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{Calldata, Felt};
use crate::utils::into_vec;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedInvokeTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub sender_address: ContractAddress,
    pub calldata: Calldata,
    pub account_deployment_data: Vec<Felt>,
}

impl BroadcastedInvokeTransactionV3 {
    /// Computes the transaction hash as follows:
    /// h(common_tx_fields, h(account_deployment_data),h(calldata)) with poseidon hash
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    fn calculate_transaction_hash(&self, chain_id: Felt) -> DevnetResult<Felt> {
        let common_fields = self.common.common_fields_for_hash(
            PREFIX_INVOKE,
            chain_id.into(),
            self.sender_address.into(),
        )?;

        let account_deployment_data_hash = poseidon_hash_many(
            &self
                .account_deployment_data
                .iter()
                .map(|f| FieldElement::from(*f))
                .collect::<Vec<FieldElement>>(),
        );

        let call_data_hash = poseidon_hash_many(
            &self.calldata.iter().map(|f| FieldElement::from(*f)).collect::<Vec<FieldElement>>(),
        );

        let fields_to_hash =
            [common_fields.as_slice(), &[account_deployment_data_hash], &[call_data_hash]].concat();

        let txn_hash = poseidon_hash_many(fields_to_hash.as_slice());

        Ok(txn_hash.into())
    }

    /// Creates a blockifier invoke transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    /// `only_query` - whether the transaction is a query or not
    pub fn create_blockifier_invoke_transaction(
        &self,
        chain_id: Felt,
        only_query: bool,
    ) -> DevnetResult<InvokeTransaction> {
        let txn_hash = self.calculate_transaction_hash(chain_id)?;

        let sn_api_transaction = starknet_api::transaction::InvokeTransactionV3 {
            resource_bounds: self.common.resource_bounds.clone(),
            tip: self.common.tip,
            signature: starknet_api::transaction::TransactionSignature(into_vec(
                &self.common.signature,
            )),
            nonce: starknet_api::core::Nonce(self.common.nonce.into()),
            sender_address: self.sender_address.try_into()?,
            calldata: starknet_api::transaction::Calldata(Arc::new(
                self.calldata.iter().map(StarkFelt::from).collect::<Vec<StarkFelt>>(),
            )),
            nonce_data_availability_mode: self.common.nonce_data_availability_mode,
            fee_data_availability_mode: self.common.fee_data_availability_mode,
            paymaster_data: starknet_api::transaction::PaymasterData(
                self.common.paymaster_data.iter().map(|f| f.into()).collect(),
            ),
            account_deployment_data: starknet_api::transaction::AccountDeploymentData(
                self.account_deployment_data.iter().map(|f| f.into()).collect(),
            ),
        };

        Ok(InvokeTransaction {
            tx: starknet_api::transaction::InvokeTransaction::V3(sn_api_transaction),
            tx_hash: starknet_api::transaction::TransactionHash(txn_hash.into()),
            only_query,
        })
    }
}

// impl BroadcastedInvokeTransactionV1 {
//     pub fn new(
//         sender_address: ContractAddress,
//         max_fee: Fee,
//         signature: &TransactionSignature,
//         nonce: Nonce,
//         calldata: &Calldata,
//         version: TransactionVersion,
//     ) -> Self {
//         Self {
//             sender_address,
//             calldata: calldata.clone(),
//             common: BroadcastedTransactionCommon {
//                 max_fee,
//                 signature: signature.clone(),
//                 nonce,
//                 version,
//             },
//         }
//     }

//     pub fn create_blockifier_invoke_transaction(
//         &self,
//         chain_id: Felt,
//         only_query: bool,
//     ) -> DevnetResult<InvokeTransaction> {
//         let txn_hash: Felt = compute_hash_on_elements(&[
//             PREFIX_INVOKE,
//             self.common.version.into(), // version
//             self.sender_address.into(),
//             FieldElement::ZERO, // entry_point_selector
//             compute_hash_on_elements(
//                 &self
//                     .calldata
//                     .iter()
//                     .map(|felt| FieldElement::from(*felt))
//                     .collect::<Vec<FieldElement>>(),
//             ),
//             self.common.max_fee.0.into(),
//             chain_id.into(),
//             self.common.nonce.into(),
//         ])
//         .into();

//         let sn_api_transaction = starknet_api::transaction::InvokeTransactionV1 {
//             max_fee: self.common.max_fee,
//             signature: starknet_api::transaction::TransactionSignature(
//                 self.common.signature.iter().map(|f| f.into()).collect(),
//             ),
//             nonce: starknet_api::core::Nonce(self.common.nonce.into()),
//             sender_address: self.sender_address.try_into()?,
//             calldata: starknet_api::transaction::Calldata(Arc::new(
//                 self.calldata.iter().map(StarkFelt::from).collect::<Vec<StarkFelt>>(),
//             )),
//         };

//         Ok(InvokeTransaction {
//             tx: starknet_api::transaction::InvokeTransaction::V1(sn_api_transaction),
//             tx_hash: starknet_api::transaction::TransactionHash(txn_hash.into()),
//             only_query,
//         })
//     }

//     pub fn create_invoke_transaction(
//         &self,
//         transaction_hash: TransactionHash,
//     ) -> InvokeTransactionV1 {
//         InvokeTransactionV1 {
//             transaction_hash,
//             max_fee: self.common.max_fee,
//             version: self.common.version,
//             signature: self.common.signature.clone(),
//             nonce: self.common.nonce,
//             sender_address: self.sender_address,
//             calldata: self.calldata.clone(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::{ResourceBoundsMapping, Tip};

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
    use crate::rpc::transactions::BroadcastedTransactionCommonV3;
    use crate::traits::ToHexString;
    use crate::utils::test_utils::from_u8_to_da_mode;

    #[derive(Deserialize)]
    struct FeederGatewayInvokeTransactionV3 {
        // common
        transaction_hash: Felt,
        nonce: Felt,
        version: Felt,
        nonce_data_availability_mode: u8,
        fee_data_availability_mode: u8,
        resource_bounds: ResourceBoundsMapping,
        tip: Tip,
        paymaster_data: Vec<Felt>,
        account_deployment_data: Vec<Felt>,
        // specific
        sender_address: Felt,
        calldata: Vec<Felt>,
    }

    #[test]
    fn test_dummy_transaction_hash_taken_from_papyrus() {
        let txn_json_str = r#"{
            "signature": ["0x3", "0x4"],
            "version": "0x3",
            "nonce": "0x9",
            "sender_address": "0x12fd538",
            "nonce_data_availability_mode": "L1",
            "fee_data_availability_mode": "L1",
            "resource_bounds": {
              "L2_GAS": {
                "max_amount": "0x0",
                "max_price_per_unit": "0x0"
              },
              "L1_GAS": {
                "max_amount": "0x7c9",
                "max_price_per_unit": "0x1"
              }
            },
            "tip": "0x0",
            "paymaster_data": [],
            "account_deployment_data": [],
            "calldata": [
              "0x11",
              "0x26"
            ]
          }"#;

        let transaction =
            serde_json::from_str::<BroadcastedInvokeTransactionV3>(txn_json_str).unwrap();
        let chain_id = b"1";

        let mut padded_chain_id = [0u8; 32];
        padded_chain_id[(32 - chain_id.len())..].copy_from_slice(chain_id);

        println!(
            "{}",
            transaction
                .calculate_transaction_hash(Felt::new(padded_chain_id).unwrap())
                .unwrap()
                .to_prefixed_hex_str()
        );
    }

    /// Data for test case is taken from https://spaceshard.slack.com/archives/C05FAMWQ8JE/p1700501793152349?thread_ts=1700058492.284919&cid=C05FAMWQ8JE
    /// The transaction was taken from https://external.integration.starknet.io/feeder_gateway/get_transaction?transactionHash=0x41906f1c314cca5f43170ea75d3b1904196a10101190d2b12a41cc61cfd17c
    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/invoke_v3_integration_0x41906f1c314cca5f43170ea75d3b1904196a10101190d2b12a41cc61cfd17c.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayInvokeTransactionV3 = serde_json::from_str(
            &serde_json::to_string_pretty(&json_obj.get("transaction").unwrap().clone()).unwrap(),
        )
        .unwrap();

        let broadcasted_txn = BroadcastedInvokeTransactionV3 {
            common: BroadcastedTransactionCommonV3 {
                version: feeder_gateway_transaction.version,
                signature: vec![],
                nonce: feeder_gateway_transaction.nonce,
                resource_bounds: feeder_gateway_transaction.resource_bounds,
                tip: feeder_gateway_transaction.tip,
                paymaster_data: feeder_gateway_transaction.paymaster_data,
                nonce_data_availability_mode: from_u8_to_da_mode(
                    feeder_gateway_transaction.nonce_data_availability_mode,
                ),
                fee_data_availability_mode: from_u8_to_da_mode(
                    feeder_gateway_transaction.fee_data_availability_mode,
                ),
            },
            sender_address: ContractAddress::new(feeder_gateway_transaction.sender_address)
                .unwrap(),
            calldata: feeder_gateway_transaction.calldata,
            account_deployment_data: feeder_gateway_transaction.account_deployment_data,
        };

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            broadcasted_txn.calculate_transaction_hash(ChainId::Testnet.to_felt()).unwrap()
        );
    }
}
