use std::sync::Arc;

use cairo_felt::Felt252;
use serde::{Deserialize, Serialize};
use starknet_api::core::calculate_contract_address;
use starknet_api::transaction::Fee;

use crate::contract_address::ContractAddress;
use crate::error::DevnetResult;
use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Felt, Nonce, TransactionHash, TransactionSignature,
    TransactionVersion,
};
use crate::rpc::transactions::deploy_account_transaction::DeployAccountTransaction;
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct BroadcastedDeployAccountTransaction {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}

impl BroadcastedDeployAccountTransaction {
    pub fn new(
        constructor_calldata: &Calldata,
        max_fee: Fee,
        signature: &TransactionSignature,
        nonce: Nonce,
        class_hash: ClassHash,
        contract_address_salt: ContractAddressSalt,
        version: TransactionVersion,
    ) -> Self {
        Self {
            contract_address_salt,
            constructor_calldata: constructor_calldata.clone(),
            class_hash,
            common: BroadcastedTransactionCommon {
                max_fee,
                signature: signature.clone(),
                nonce,
                version,
            },
        }
    }

    pub fn create_blockifier_deploy_account(
        &self,
        chain_id: Felt,
    ) -> DevnetResult<blockifier::transaction::transactions::DeployAccountTransaction> {
        let contract_address = calculate_contract_address(
            starknet_api::transaction::ContractAddressSalt(self.contract_address_salt.into()),
            starknet_api::core::ClassHash(self.class_hash.into()),
            &starknet_api::transaction::Calldata(Arc::new(
                self.constructor_calldata.iter().map(|felt| felt.into()).collect(),
            )),
            starknet_api::core::ContractAddress::from(0u8),
        )?;

        let transaction_hash: Felt =
            starknet_in_rust::core::transaction_hash::calculate_deploy_account_transaction_hash(
                self.common.version.into(),
                &(ContractAddress::from(contract_address).into()),
                self.class_hash.into(),
                &self.constructor_calldata.iter().map(|f| f.into()).collect::<Vec<Felt252>>(),
                self.common.max_fee.0,
                self.common.nonce.into(),
                self.contract_address_salt.into(),
                chain_id.into(),
            )?
            .into();

        let sn_api_transaction = starknet_api::transaction::DeployAccountTransactionV1 {
            max_fee: self.common.max_fee,
            signature: starknet_api::transaction::TransactionSignature(
                self.common.signature.iter().map(|felt| felt.into()).collect(),
            ),
            nonce: starknet_api::core::Nonce(self.common.nonce.into()),
            class_hash: self.class_hash.into(),
            contract_address_salt: starknet_api::transaction::ContractAddressSalt(
                self.contract_address_salt.into(),
            ),
            constructor_calldata: starknet_api::transaction::Calldata(Arc::new(
                self.constructor_calldata.iter().map(|felt| felt.into()).collect(),
            )),
        };

        Ok(blockifier::transaction::transactions::DeployAccountTransaction {
            tx: starknet_api::transaction::DeployAccountTransaction::V1(sn_api_transaction),
            tx_hash: starknet_api::transaction::TransactionHash(transaction_hash.into()),
            contract_address,
        })
    }

    pub fn compile_deploy_account_transaction(
        &self,
        transaction_hash: &TransactionHash,
        contract_address: ContractAddress,
    ) -> DeployAccountTransaction {
        DeployAccountTransaction {
            transaction_hash: *transaction_hash,
            max_fee: self.common.max_fee,
            version: self.common.version,
            signature: self.common.signature.clone(),
            nonce: self.common.nonce,
            class_hash: self.class_hash,
            contract_address_salt: self.contract_address_salt,
            constructor_calldata: self.constructor_calldata.clone(),
            contract_address,
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
    use crate::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
    use crate::traits::ToHexString;

    #[derive(Deserialize)]
    struct FeederGatewayDeployAccountTransaction {
        transaction_hash: Felt,
        version: Felt,
        max_fee: Felt,
        nonce: Felt,
        constructor_calldata: Vec<Felt>,
        contract_address: Felt,
        contract_address_salt: Felt,
        class_hash: Felt,
    }

    #[test]
    fn correct_transaction_hash_computation_compared_to_a_transaction_from_feeder_gateway() {
        let json_obj: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sequencer_response/deploy_account_testnet_0x23a872d966d4f6091cc3725604fdaa1b39cef76ebf38b9a06a0b71e9ed700ea.json"
            ))
            .unwrap(),
        )
        .unwrap();

        let feeder_gateway_transaction: FeederGatewayDeployAccountTransaction =
            serde_json::from_value(json_obj.get("transaction").unwrap().clone()).unwrap();

        let broadcasted_tx = BroadcastedDeployAccountTransaction::new(
            &feeder_gateway_transaction.constructor_calldata,
            Fee(u128::from_str_radix(
                &feeder_gateway_transaction.max_fee.to_nonprefixed_hex_str(),
                16,
            )
            .unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            feeder_gateway_transaction.class_hash,
            feeder_gateway_transaction.contract_address_salt,
            feeder_gateway_transaction.version,
        );

        let chain_id = ChainId::Testnet.to_felt();

        let blockifier_deploy_account_transaction =
            broadcasted_tx.create_blockifier_deploy_account(chain_id).unwrap();

        assert_eq!(
            ContractAddress::new(feeder_gateway_transaction.contract_address).unwrap(),
            ContractAddress::from(blockifier_deploy_account_transaction.contract_address)
        );

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            blockifier_deploy_account_transaction.tx_hash.0.into()
        );
    }
}
