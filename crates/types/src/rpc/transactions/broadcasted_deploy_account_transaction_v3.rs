use std::sync::Arc;

use serde::{Deserialize, Serialize};
use starknet_api::core::calculate_contract_address;
use starknet_api::transaction::DeployAccountTransactionV3;
use starknet_rs_crypto::poseidon_hash_many;

use super::broadcasted_deploy_account_transaction_v1::PREFIX_DEPLOY_ACCOUNT;
use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{Calldata, ClassHash, ContractAddressSalt, Felt};
use crate::utils::into_vec;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}

impl BroadcastedDeployAccountTransactionV3 {
    fn calculate_transaction_hash(
        &self,
        chain_id: Felt,
        contract_address: ContractAddress,
    ) -> DevnetResult<Felt> {
        let common_fields = self.common.common_fields_for_hash(
            PREFIX_DEPLOY_ACCOUNT,
            chain_id.into(),
            contract_address.into(),
        )?;

        let constructor_calldata_hash = poseidon_hash_many(&into_vec(&self.constructor_calldata));

        let fields_to_hash = [
            common_fields.as_slice(),
            &[constructor_calldata_hash],
            &[self.class_hash.into()],
            &[self.contract_address_salt.into()],
        ]
        .concat();

        let txn_hash = poseidon_hash_many(fields_to_hash.as_slice());

        Ok(txn_hash.into())
    }

    fn calculate_contract_address(
        contract_address_salt: &Felt,
        class_hash: &ClassHash,
        constructor_calldata: &Calldata,
    ) -> DevnetResult<ContractAddress> {
        let contract_address = calculate_contract_address(
            starknet_api::transaction::ContractAddressSalt(contract_address_salt.into()),
            starknet_api::core::ClassHash(class_hash.into()),
            &starknet_api::transaction::Calldata(Arc::new(
                constructor_calldata.iter().map(|felt| felt.into()).collect(),
            )),
            starknet_api::core::ContractAddress::from(0u8),
        )?;

        Ok(ContractAddress::from(contract_address))
    }

    pub fn create_blockifier_deploy_account(
        &self,
        chain_id: Felt,
        only_query: bool,
    ) -> DevnetResult<blockifier::transaction::transactions::DeployAccountTransaction> {
        let contract_address = Self::calculate_contract_address(
            &self.contract_address_salt,
            &self.class_hash,
            &self.constructor_calldata,
        )?;

        let transaction_hash = self.calculate_transaction_hash(chain_id, contract_address)?;

        let sn_api_deploy_account =
            starknet_api::transaction::DeployAccountTransaction::V3(DeployAccountTransactionV3 {
                resource_bounds: self.common.resource_bounds.clone(),
                tip: self.common.tip,
                signature: starknet_api::transaction::TransactionSignature(into_vec(
                    &self.common.signature,
                )),
                nonce: starknet_api::core::Nonce(self.common.nonce.into()),
                class_hash: starknet_api::core::ClassHash(self.class_hash.into()),
                nonce_data_availability_mode: self.common.nonce_data_availability_mode,
                fee_data_availability_mode: self.common.fee_data_availability_mode,
                paymaster_data: starknet_api::transaction::PaymasterData(into_vec(
                    &self.common.paymaster_data,
                )),
                contract_address_salt: starknet_api::transaction::ContractAddressSalt(
                    self.contract_address_salt.into(),
                ),
                constructor_calldata: starknet_api::transaction::Calldata(Arc::new(into_vec(
                    &self.constructor_calldata,
                ))),
            });

        Ok(blockifier::transaction::transactions::DeployAccountTransaction {
            tx: sn_api_deploy_account,
            tx_hash: starknet_api::transaction::TransactionHash(transaction_hash.into()),
            contract_address: contract_address.try_into()?,
            only_query,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::{ResourceBoundsMapping, Tip};

    use crate::chain_id::ChainId;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
    use crate::rpc::transactions::BroadcastedTransactionCommonV3;
    use crate::utils::test_utils::from_u8_to_da_mode;

    #[derive(Deserialize)]
    struct FeederGatewayDeployAccountTransactionV3 {
        // common
        transaction_hash: Felt,
        nonce: Felt,
        version: Felt,
        nonce_data_availability_mode: u8,
        fee_data_availability_mode: u8,
        resource_bounds: ResourceBoundsMapping,
        tip: Tip,
        paymaster_data: Vec<Felt>,
        // specific
        class_hash: Felt,
        constructor_calldata: Vec<Felt>,
        contract_address_salt: Felt,
    }

    /// Data for test case is taken from https://spaceshard.slack.com/archives/C05FAMWQ8JE/p1700501793152349?thread_ts=1700058492.284919&cid=C05FAMWQ8JE
    /// The transaction was taken from https://external.integration.starknet.io/feeder_gateway/get_transaction?transactionHash=0x29fd7881f14380842414cdfdd8d6c0b1f2174f8916edcfeb1ede1eb26ac3ef0
    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/deploy_account_v3_integration_0x29fd7881f14380842414cdfdd8d6c0b1f2174f8916edcfeb1ede1eb26ac3ef0.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayDeployAccountTransactionV3 =
            serde_json::from_str(
                &serde_json::to_string_pretty(&json_obj.get("transaction").unwrap().clone())
                    .unwrap(),
            )
            .unwrap();

        let broadcasted_txn = BroadcastedDeployAccountTransactionV3 {
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
            constructor_calldata: feeder_gateway_transaction.constructor_calldata,
            contract_address_salt: feeder_gateway_transaction.contract_address_salt,
            class_hash: feeder_gateway_transaction.class_hash,
        };

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            broadcasted_txn
                .calculate_transaction_hash(
                    ChainId::Testnet.to_felt(),
                    BroadcastedDeployAccountTransactionV3::calculate_contract_address(
                        &broadcasted_txn.contract_address_salt,
                        &broadcasted_txn.class_hash,
                        &broadcasted_txn.constructor_calldata
                    )
                    .unwrap()
                )
                .unwrap()
        );
    }
}
