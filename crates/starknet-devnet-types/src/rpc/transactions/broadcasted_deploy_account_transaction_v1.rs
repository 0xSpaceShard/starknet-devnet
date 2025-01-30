use serde::Deserialize;
use starknet_api::transaction::fields::Fee;

use crate::felt::{
    Calldata, ClassHash, ContractAddressSalt, Nonce, TransactionSignature, TransactionVersion,
};
use crate::rpc::transactions::BroadcastedTransactionCommon;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BroadcastedDeployAccountTransactionV1 {
    #[serde(flatten)]
    pub common: BroadcastedTransactionCommon,
    pub contract_address_salt: ContractAddressSalt,
    pub constructor_calldata: Calldata,
    pub class_hash: ClassHash,
}

impl BroadcastedDeployAccountTransactionV1 {
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
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use starknet_api::transaction::fields::Fee;
    use starknet_rs_core::types::Felt;

    use crate::chain_id::ChainId;
    use crate::contract_address::ContractAddress;
    use crate::felt::try_felt_to_num;
    use crate::rpc::transactions::BroadcastedDeployAccountTransaction;
    use crate::rpc::transactions::broadcasted_deploy_account_transaction_v1::BroadcastedDeployAccountTransactionV1;

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

        let broadcasted_tx = BroadcastedDeployAccountTransactionV1::new(
            &feeder_gateway_transaction.constructor_calldata,
            Fee(try_felt_to_num(feeder_gateway_transaction.max_fee).unwrap()),
            &vec![],
            feeder_gateway_transaction.nonce,
            feeder_gateway_transaction.class_hash,
            feeder_gateway_transaction.contract_address_salt,
            feeder_gateway_transaction.version,
        );

        let chain_id = ChainId::goerli_legacy_id();

        let blockifier_deploy_account_transaction =
            BroadcastedDeployAccountTransaction::V1(broadcasted_tx)
                .create_sn_api_deploy_account(&chain_id)
                .unwrap();

        assert_eq!(
            ContractAddress::new(feeder_gateway_transaction.contract_address).unwrap(),
            ContractAddress::from(blockifier_deploy_account_transaction.contract_address)
        );

        assert_eq!(
            feeder_gateway_transaction.transaction_hash,
            blockifier_deploy_account_transaction.tx_hash.0
        );
    }
}
