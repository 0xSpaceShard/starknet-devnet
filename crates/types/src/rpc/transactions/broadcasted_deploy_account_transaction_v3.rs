use std::sync::Arc;

use serde::{Deserialize, Serialize};
use starknet_api::core::calculate_contract_address;
use starknet_rs_crypto::poseidon_hash_many;

use super::broadcasted_deploy_account_transaction_v1::PREFIX_DEPLOY_ACCOUNT;
use super::deploy_account_transaction::DeployAccountTransactionV1;
use super::BroadcastedTransactionCommonV3;
use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Felt, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::utils::into_vec;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionV3 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommonV3,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}

impl BroadcastedDeployAccountTransactionV3 {
    fn calculate_transaction_hash(&self, chain_id: Felt) -> DevnetResult<Felt> {
        let contract_address = calculate_contract_address(
            starknet_api::transaction::ContractAddressSalt(self.contract_address_salt.into()),
            starknet_api::core::ClassHash(self.class_hash.into()),
            &starknet_api::transaction::Calldata(Arc::new(
                self.constructor_calldata.iter().map(|felt| felt.into()).collect(),
            )),
            starknet_api::core::ContractAddress::from(0u8),
        )?;

        let common_fields = self.common.common_fields_for_hash(
            PREFIX_DEPLOY_ACCOUNT,
            chain_id.into(),
            ContractAddress::from(contract_address).into(),
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
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::{ResourceBoundsMapping, Tip};

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::felt::Felt;
    use crate::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
    use crate::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
    use crate::rpc::transactions::BroadcastedTransactionCommonV3;
    use crate::traits::ToHexString;
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
            broadcasted_txn.calculate_transaction_hash(ChainId::Testnet.to_felt()).unwrap()
        );
    }
}
