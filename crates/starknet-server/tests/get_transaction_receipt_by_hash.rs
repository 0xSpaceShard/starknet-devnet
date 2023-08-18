pub mod common;

mod get_transaction_receipt_by_hash_integration_tests {

    use std::sync::Arc;

    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, FieldElement,
        MaybePendingTransactionReceipt, StarknetError, TransactionReceipt, TransactionStatus,
    };
    use starknet_rs_core::utils::get_udc_deployed_address;
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_events_contract_in_sierra_and_compiled_class_hash, get_json_body,
    };

    #[tokio::test]
    async fn deploy_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get first predeployed account data
        let predeployed_accounts_response =
            devnet.get("/predeployed_accounts", None).await.unwrap();

        let predeployed_accounts_json = get_json_body(predeployed_accounts_response).await;
        let first_account = predeployed_accounts_json.as_array().unwrap().get(0).unwrap();

        let account_address =
            Felt::from_prefixed_hex_str(first_account["address"].as_str().unwrap()).unwrap();
        let private_key =
            Felt::from_prefixed_hex_str(first_account["private_key"].as_str().unwrap()).unwrap();

        // constructs starknet-rs account
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key.into()));
        let address = FieldElement::from(account_address);

        let mut predeployed_account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, address, chain_id::TESTNET);

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1000000000000000000000000000u128))
            .send()
            .await
            .unwrap();

        let predeployed_account = Arc::new(predeployed_account);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let deployment_result = contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1000000000000000000000000000u128))
            .send()
            .await
            .unwrap();

        // generate the address of the newly deployed contract
        let new_contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &[],
        );

        let deployment_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deployment_result.transaction_hash)
            .await
            .unwrap();

        match deployment_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Deploy(receipt)) => {
                assert_eq!(receipt.contract_address, new_contract_address);
            }
            _ => panic!("Invalid receipt {:?}", deployment_receipt),
        };
    }

    #[tokio::test]
    async fn get_declare_v1_transaction_receipt_by_hash_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await
            .unwrap();

        let result: starknet_rs_core::types::MaybePendingTransactionReceipt = devnet
            .json_rpc_client
            .get_transaction_receipt(declare_transaction.transaction_hash)
            .await
            .unwrap();

        match result {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Declare(declare)) => {
                assert_eq!(declare.status, TransactionStatus::Rejected);
            }
            _ => panic!("Invalid result: {result:?}"),
        }
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_receipt(FieldElement::from_hex_be("0x0").unwrap())
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
