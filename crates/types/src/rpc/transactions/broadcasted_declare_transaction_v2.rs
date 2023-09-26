use blockifier::transaction::transactions::DeclareTransaction;
use serde::{Deserialize, Serialize};
use starknet_api::transaction::Fee;
use starknet_in_rust::core::contract_address::compute_sierra_class_hash;
use starknet_in_rust::core::transaction_hash::calculate_declare_v2_transaction_hash;
use starknet_in_rust::transaction::DeclareV2 as SirDeclareV2;
use starknet_in_rust::SierraContractClass;

use crate::contract_address::ContractAddress;
use crate::contract_class::ContractClass;
use crate::error::DevnetResult;
use crate::felt::{
    ClassHash, CompiledClassHash, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transactions::declare_transaction_v2::DeclareTransactionV2;
use crate::rpc::transactions::BroadcastedTransactionCommon;
use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeclareTransactionV2 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    #[serde(deserialize_with = "deserialize_to_sierra_contract_class")]
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddress,
    pub compiled_class_hash: CompiledClassHash,
}

impl BroadcastedDeclareTransactionV2 {
    pub fn new(
        contract_class: &SierraContractClass,
        compiled_class_hash: CompiledClassHash,
        sender_address: ContractAddress,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        version: TransactionVersion,
    ) -> Self {
        Self {
            contract_class: contract_class.clone(),
            sender_address,
            compiled_class_hash,
            common: BroadcastedTransactionCommon {
                max_fee,
                version,
                signature: signature.clone(),
                nonce,
            },
        }
    }

    pub fn create_declare(
        &self,
        class_hash: ClassHash,
        transaction_hash: TransactionHash,
    ) -> DeclareTransactionV2 {
        DeclareTransactionV2 {
            class_hash,
            compiled_class_hash: self.compiled_class_hash,
            sender_address: self.sender_address,
            nonce: self.common.nonce,
            max_fee: self.common.max_fee,
            version: self.common.version,
            transaction_hash,
            signature: self.common.signature.clone(),
        }
    }

    pub fn create_sir_declare(&self, chain_id: Felt) -> DevnetResult<SirDeclareV2> {
        Ok(SirDeclareV2::new(
            &self.contract_class,
            None,
            self.compiled_class_hash.into(),
            chain_id.into(),
            self.sender_address.into(),
            self.common.max_fee.0,
            self.common.version.into(),
            self.common.signature.iter().map(|felt| felt.into()).collect(),
            self.common.nonce.into(),
        )?)
    }

    pub fn create_blockifier_declare(&self, chain_id: Felt) -> DevnetResult<DeclareTransaction> {
        let sierra_class_hash: Felt = compute_sierra_class_hash(&self.contract_class)?.into();

        let sn_api_declare = starknet_api::transaction::DeclareTransaction::V2(
            starknet_api::transaction::DeclareTransactionV2 {
                max_fee: self.common.max_fee,
                signature: starknet_api::transaction::TransactionSignature(
                    self.common.signature.iter().map(|&felt| felt.into()).collect(),
                ),
                nonce: starknet_api::core::Nonce(self.common.nonce.into()),
                class_hash: sierra_class_hash.into(),
                compiled_class_hash: self.compiled_class_hash.into(),
                sender_address: self.sender_address.try_into()?,
            },
        );

        let txn_hash: Felt = calculate_declare_v2_transaction_hash(
            sierra_class_hash.into(),
            self.compiled_class_hash.into(),
            chain_id.into(),
            &self.sender_address.into(),
            self.common.max_fee.0,
            self.common.version.into(),
            self.common.nonce.into(),
        )?
        .into();

        Ok(DeclareTransaction::new(
            sn_api_declare,
            starknet_api::transaction::TransactionHash(txn_hash.into()),
            blockifier::execution::contract_class::ContractClass::V1(
                ContractClass::Cairo1(self.contract_class.clone()).try_into()?,
            ),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::Fee;
    use starknet_in_rust::core::contract_address::compute_sierra_class_hash;
    use starknet_rs_core::types::contract::SierraClass;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::contract_class::ContractClass;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
    use crate::traits::ToHexString;

    #[derive(Deserialize)]
    struct FeederGatewayDeclareTransactionV2 {
        transaction_hash: Felt,
        sender_address: Felt,
        max_fee: Felt,
        nonce: Felt,
        class_hash: Felt,
        compiled_class_hash: Felt,
        version: Felt,
    }

    #[test]
    fn sierra_hash_from_events_sierra_artifact() {
        let sierra_contract_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/events_cairo1.sierra");

        let cairo_1_contract = ContractClass::cairo_1_from_sierra_json_str(
            &std::fs::read_to_string(sierra_contract_path).unwrap(),
        )
        .unwrap();
        let sierra_class: SierraClass =
            serde_json::from_value(serde_json::to_value(cairo_1_contract.clone()).unwrap())
                .unwrap();
        println!("{}", Felt::from(sierra_class.class_hash().unwrap()).to_prefixed_hex_str());

        println!(
            "{}",
            Felt::from(compute_sierra_class_hash(&cairo_1_contract).unwrap()).to_prefixed_hex_str()
        );
    }

    /// Data for the contract artifact is taken from
    /// test_data/cairo1/events/events_2.0.1_compiler.sierra Which in turn is taken from cairo package https://github.com/starkware-libs/cairo/blob/98eb937c6e7e12b16c0471f087309c10bffe5013/crates/cairo-lang-starknet/cairo_level_tests/events.cairo
    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/declare_v2_testnet_0x01b852f1fe2b13db21a44f8884bc4b7760dc277bb3820b970dba929860275617.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayDeclareTransactionV2 =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let sierra_contract_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/events_cairo1.sierra");

        let cairo_1_contract = ContractClass::cairo_1_from_sierra_json_str(
            &std::fs::read_to_string(sierra_contract_path).unwrap(),
        )
        .unwrap();
        let broadcasted_declare_transaction = BroadcastedDeclareTransactionV2::new(
            &cairo_1_contract,
            feeder_gateway_transaction.compiled_class_hash,
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            Fee(u128::from_str_radix(
                &feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(),
                16,
            )
            .unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            feeder_gateway_transaction.version,
        );
        let sir_declare_transaction =
            broadcasted_declare_transaction.create_sir_declare(ChainId::TestNet.to_felt()).unwrap();

        let blockifier_declare_transaction = broadcasted_declare_transaction
            .create_blockifier_declare(ChainId::TestNet.to_felt())
            .unwrap();

        assert_eq!(
            feeder_gateway_transaction.class_hash,
            sir_declare_transaction.sierra_class_hash.into()
        );
        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            sir_declare_transaction.hash_value.into()
        );

        assert_eq!(
            feeder_gateway_transaction.class_hash,
            blockifier_declare_transaction.class_hash().into()
        );
        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            blockifier_declare_transaction.tx_hash().0.into()
        );
    }
}
