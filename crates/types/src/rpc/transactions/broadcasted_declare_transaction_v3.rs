use blockifier::transaction::transactions::DeclareTransaction;
use cairo_lang_starknet::contract_class::ContractClass as SierraContractClass;
use serde::{Deserialize, Serialize};
use starknet_api::transaction::DeclareTransactionV3;
use starknet_rs_crypto::poseidon_hash_many;
use starknet_rs_ff::FieldElement;

use super::broadcasted_declare_transaction_v1::PREFIX_DECLARE;
use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::contract_class::{compute_sierra_class_hash, ContractClass};
use crate::error::DevnetResult;
use crate::felt::{ClassHash, CompiledClassHash, Felt};
use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
use crate::utils::into_vec;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeclareTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    #[serde(deserialize_with = "deserialize_to_sierra_contract_class")]
    pub contract_class: SierraContractClass,
    pub sender_address: ContractAddress,
    pub compiled_class_hash: CompiledClassHash,
    pub account_deployment_data: Vec<Felt>,
}

impl BroadcastedDeclareTransactionV3 {
    /// Computes the transaction hash as follows:
    /// h(common_tx_fields, h(account_deployment_data), class_hash, compiled_class_hash) with
    /// poseidon hash
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    /// `class_hash` - the class hash to use for the transaction hash computation,
    /// computed from the contract class
    fn calculate_transaction_hash(
        &self,
        chain_id: Felt,
        class_hash: ClassHash,
    ) -> DevnetResult<Felt> {
        let common_fields = self.common.common_fields_for_hash(
            PREFIX_DECLARE,
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

        let fields_to_hash = [
            common_fields.as_slice(),
            &[account_deployment_data_hash],
            &[class_hash.into()],
            &[self.compiled_class_hash.into()],
        ]
        .concat();

        let txn_hash = poseidon_hash_many(fields_to_hash.as_slice());

        Ok(txn_hash.into())
    }

    /// Creates a blockifier declare transaction from the current transaction.
    /// The transaction hash is computed using the given chain id.
    ///
    /// # Arguments
    /// `chain_id` - the chain id to use for the transaction hash computation
    /// `only_query` - whether the transaction is a query or not
    pub fn create_blockifier_declare(
        &self,
        chain_id: Felt,
        only_query: bool,
    ) -> DevnetResult<DeclareTransaction> {
        let sierra_class_hash = compute_sierra_class_hash(&self.contract_class)?;
        let transaction_hash = self.calculate_transaction_hash(chain_id, sierra_class_hash)?;

        let sn_api_declare =
            starknet_api::transaction::DeclareTransaction::V3(DeclareTransactionV3 {
                resource_bounds: (&self.common.resource_bounds).into(),
                tip: self.common.tip,
                signature: starknet_api::transaction::TransactionSignature(into_vec(
                    &self.common.signature,
                )),
                nonce: starknet_api::core::Nonce(self.common.nonce.into()),
                class_hash: starknet_api::core::ClassHash(sierra_class_hash.into()),
                compiled_class_hash: starknet_api::core::CompiledClassHash(
                    self.compiled_class_hash.into(),
                ),
                sender_address: self.sender_address.try_into()?,
                nonce_data_availability_mode: self.common.nonce_data_availability_mode,
                fee_data_availability_mode: self.common.fee_data_availability_mode,
                paymaster_data: starknet_api::transaction::PaymasterData(into_vec(
                    &self.common.paymaster_data,
                )),
                account_deployment_data: starknet_api::transaction::AccountDeploymentData(
                    into_vec(&self.account_deployment_data),
                ),
            });

        let contract_class = ContractClass::Cairo1(self.contract_class.clone()).try_into()?;
        let transaction_hash = starknet_api::transaction::TransactionHash(transaction_hash.into());

        if only_query {
            Ok(DeclareTransaction::new_for_query(sn_api_declare, transaction_hash, contract_class)?)
        } else {
            Ok(DeclareTransaction::new(sn_api_declare, transaction_hash, contract_class)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::Tip;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::contract_class::ContractClass;
    use crate::felt::{ClassHash, CompiledClassHash, Felt};
    use crate::rpc::transactions::broadcasted_declare_transaction_v3::BroadcastedDeclareTransactionV3;
    use crate::rpc::transactions::{BroadcastedTransactionCommonV3, ResourceBoundsWrapper};
    use crate::utils::test_utils::from_u8_to_da_mode;

    #[derive(Deserialize)]
    struct FeederGatewayDeclareTransactionV3 {
        // common
        transaction_hash: Felt,
        nonce: Felt,
        version: Felt,
        nonce_data_availability_mode: u8,
        fee_data_availability_mode: u8,
        resource_bounds: ResourceBoundsWrapper,
        tip: Tip,
        paymaster_data: Vec<Felt>,
        account_deployment_data: Vec<Felt>,
        // specific
        sender_address: Felt,
        compiled_class_hash: CompiledClassHash,
        class_hash: ClassHash,
    }

    /// Data for test case is taken from https://spaceshard.slack.com/archives/C05FAMWQ8JE/p1700501793152349?thread_ts=1700058492.284919&cid=C05FAMWQ8JE
    /// The transaction was taken from https://external.integration.starknet.io/feeder_gateway/get_transaction?transactionHash=0x41d1f5206ef58a443e7d3d1ca073171ec25fa75313394318fc83a074a6631c3
    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/declare_v3_integration_0x41d1f5206ef58a443e7d3d1ca073171ec25fa75313394318fc83a074a6631c3.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayDeclareTransactionV3 =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        // cairo_1_contract is only needed for constructing BroadcastedDeclareTransactionV3
        // the class_hash and compiled_class_hash will be provided from
        // FeederGatewayDeclareTransactionV3
        let sierra_contract_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/events_cairo1.sierra");

        let cairo_1_contract = ContractClass::cairo_1_from_sierra_json_str(
            &std::fs::read_to_string(sierra_contract_path).unwrap(),
        )
        .unwrap();

        let broadcasted_txn = BroadcastedDeclareTransactionV3 {
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
            contract_class: cairo_1_contract,
            sender_address: ContractAddress::new(feeder_gateway_transaction.sender_address)
                .unwrap(),
            compiled_class_hash: feeder_gateway_transaction.compiled_class_hash,
            account_deployment_data: feeder_gateway_transaction.account_deployment_data,
        };

        assert_eq!(
            broadcasted_txn
                .calculate_transaction_hash(
                    ChainId::Testnet.to_felt(),
                    feeder_gateway_transaction.class_hash
                )
                .unwrap(),
            feeder_gateway_transaction.transaction_hash
        );
    }
}
