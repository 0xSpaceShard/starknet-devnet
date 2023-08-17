use starknet_in_rust::transaction::DeclareV2;
use starknet_in_rust::SierraContractClass;
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::felt::{ClassHash, Felt, TransactionHash};
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;

use crate::error::Result;

#[derive(Clone)]
pub struct DeclareTransactionV2 {
    pub(crate) inner: DeclareV2,
    pub sierra_contract_class: SierraContractClass,
    pub compiled_class_hash: ClassHash,
    pub sender_address: ContractAddress,
    pub max_fee: u128,
    pub signature: Vec<Felt>,
    pub nonce: Felt,
    pub class_hash: ClassHash,
    pub transaction_hash: TransactionHash,
    pub chain_id: Felt,
    pub version: Felt,
}

impl PartialEq for DeclareTransactionV2 {
    fn eq(&self, other: &Self) -> bool {
        self.sierra_contract_class == other.sierra_contract_class
            && self.compiled_class_hash == other.compiled_class_hash
            && self.sender_address == other.sender_address
            && self.max_fee == other.max_fee
            && self.signature == other.signature
            && self.nonce == other.nonce
            && self.class_hash == other.class_hash
            && self.transaction_hash == other.transaction_hash
            && self.chain_id == other.chain_id
            && self.version == other.version
    }
}

impl Eq for DeclareTransactionV2 {}

impl DeclareTransactionV2 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sierra_contract_class: SierraContractClass,
        compiled_class_hash: ClassHash,
        sender_address: ContractAddress,
        max_fee: u128,
        signature: Vec<Felt>,
        nonce: Felt,
        chain_id: Felt,
        version: Felt,
    ) -> Result<Self> {
        let transaction = DeclareV2::new(
            &sierra_contract_class,
            None,
            compiled_class_hash.into(),
            chain_id.into(),
            sender_address.try_into()?,
            max_fee,
            version.into(),
            signature.iter().map(|felt| felt.into()).collect(),
            nonce.into(),
        )?;
        let class_hash = transaction.sierra_class_hash.clone().into();
        let transaction_hash = transaction.hash_value.clone().into();

        Ok(Self {
            inner: transaction,
            sierra_contract_class,
            compiled_class_hash,
            sender_address,
            max_fee,
            signature,
            nonce,
            class_hash,
            transaction_hash,
            chain_id,
            version,
        })
    }

    pub fn sender_address(&self) -> &ContractAddress {
        &self.sender_address
    }

    pub fn class_hash(&self) -> &ClassHash {
        &self.class_hash
    }

    pub fn compiled_class_hash(&self) -> &ClassHash {
        &self.compiled_class_hash
    }
}

impl HashProducer for DeclareTransactionV2 {
    fn generate_hash(&self) -> DevnetResult<Felt> {
        Ok(self.inner.hash_value.clone().into())
    }
}

#[cfg(test)]
mod tests {

    use serde::Deserialize;
    use starknet_in_rust::core::contract_address::compute_sierra_class_hash;
    use starknet_in_rust::definitions::block_context::StarknetChainId;
    use starknet_rs_core::types::contract::SierraClass;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::felt::Felt;
    use starknet_types::traits::{HashProducer, ToHexString};

    use super::DeclareTransactionV2;

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
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_artifacts/events_cairo1.sierra");

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
        let json_obj: serde_json::Value = serde_json::from_reader(std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_artifacts/sequencer_response/declare_v2_testnet_0x01b852f1fe2b13db21a44f8884bc4b7760dc277bb3820b970dba929860275617.json"
        )).unwrap()).unwrap();

        let feeder_gateway_transaction: FeederGatewayDeclareTransactionV2 =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let sierra_contract_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_artifacts/events_cairo1.sierra");

        let cairo_1_contract = ContractClass::cairo_1_from_sierra_json_str(
            &std::fs::read_to_string(sierra_contract_path).unwrap(),
        )
        .unwrap();
        let declare_transaction = DeclareTransactionV2::new(
            cairo_1_contract,
            feeder_gateway_transaction.compiled_class_hash,
            ContractAddress::new(feeder_gateway_transaction.sender_address).unwrap(),
            u128::from_str_radix(&feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(), 16)
                .unwrap(),
            vec![],
            feeder_gateway_transaction.nonce,
            StarknetChainId::TestNet.to_felt().into(),
            feeder_gateway_transaction.version,
        )
        .unwrap();

        assert_eq!(feeder_gateway_transaction.class_hash, declare_transaction.class_hash);
        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            declare_transaction.generate_hash().unwrap()
        );
    }
}
