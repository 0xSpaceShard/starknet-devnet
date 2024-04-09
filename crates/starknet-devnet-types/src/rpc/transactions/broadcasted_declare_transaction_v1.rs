use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_rs_core::crypto::compute_hash_on_elements;
use starknet_rs_ff::FieldElement;

use crate::constants::PREFIX_DECLARE;
use crate::contract_address::ContractAddress;
use crate::contract_class::Cairo0ContractClass;
use crate::error::DevnetResult;
use crate::felt::{ClassHash, Felt, Nonce, TransactionSignature, TransactionVersion};
use crate::rpc::transactions::BroadcastedTransactionCommon;
use crate::traits::HashProducer;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_class: Cairo0ContractClass,
    pub sender_address: ContractAddress,
}

impl BroadcastedDeclareTransactionV1 {
    pub fn new(
        sender_address: ContractAddress,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        contract_class: &Cairo0ContractClass,
        version: TransactionVersion,
    ) -> Self {
        Self {
            sender_address,
            contract_class: contract_class.clone(),
            common: BroadcastedTransactionCommon {
                max_fee,
                nonce,
                version,
                signature: signature.clone(),
            },
        }
    }

    pub fn generate_class_hash(&self) -> DevnetResult<Felt> {
        self.contract_class.generate_hash()
    }

    pub fn calculate_transaction_hash(
        &self,
        chain_id: &Felt,
        class_hash: &ClassHash,
    ) -> DevnetResult<ClassHash> {
        Ok(compute_hash_on_elements(&[
            PREFIX_DECLARE,
            self.common.version.into(),
            self.sender_address.into(),
            FieldElement::ZERO, // entry_point_selector
            compute_hash_on_elements(&[FieldElement::from(*class_hash)]),
            self.common.max_fee.0.into(),
            FieldElement::from(*chain_id),
            self.common.nonce.into(),
        ])
        .into())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::Fee;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::contract_class::Cairo0Json;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use crate::rpc::transactions::BroadcastedDeclareTransaction;
    use crate::traits::{HashProducer, ToHexString};

    #[derive(Deserialize)]
    struct FeederGatewayDeclareTransactionV1 {
        transaction_hash: Felt,
        max_fee: Felt,
        nonce: Felt,
        class_hash: Felt,
        sender_address: Felt,
        version: Felt,
    }

    #[test]
    /// test_artifact is taken from starknet-rs. https://github.com/xJonathanLEI/starknet-rs/blob/starknet-core/v0.5.1/starknet-core/test-data/contracts/cairo0/artifacts/event_example.txt
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_str = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/events_cairo0.casm"
        ))
        .unwrap();
        let cairo0 = Cairo0Json::raw_json_from_json_str(&json_str).unwrap();

        // this is declare v1 transaction send with starknet-rs
        let json_obj: serde_json::Value = serde_json::from_reader(std::fs::File::open(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/sequencer_response/declare_v1_testnet_0x04f3480733852ec616431fd89a5e3127b49cef0ac7a71440ebdec40b1322ca9d.json"
        )).unwrap()).unwrap();

        let feeder_gateway_transaction: FeederGatewayDeclareTransactionV1 =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        assert_eq!(feeder_gateway_transaction.class_hash, cairo0.generate_hash().unwrap());

        let broadcasted_tx = BroadcastedDeclareTransactionV1::new(
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            Fee(u128::from_str_radix(
                &feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(),
                16,
            )
            .unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            &cairo0.into(),
            feeder_gateway_transaction.version,
        );

        let blockifier_declare_transaction =
            BroadcastedDeclareTransaction::V1(Box::new(broadcasted_tx))
                .create_blockifier_declare(&ChainId::goerli_legacy_id())
                .unwrap();

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            blockifier_declare_transaction.tx_hash().0.into()
        );
        assert_eq!(
            feeder_gateway_transaction.class_hash,
            blockifier_declare_transaction.class_hash().0.into()
        );
    }
}
