pub mod common;

mod get_transaction_receipt_by_hash_integration_tests {

    use std::sync::Arc;

    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::{
        Account, AccountFactory, Call, ExecutionEncoding, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, FieldElement,
        MaybePendingTransactionReceipt, StarknetError, TransactionReceipt,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;

    use crate::common::constants::CHAIN_ID;
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_deployable_account_signer, get_events_contract_in_sierra_and_compiled_class_hash,
        get_json_body,
    };

    #[tokio::test]
    async fn deploy_account_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // constructs starknet-rs account
        let signer = get_deployable_account_signer();

        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;
        let salt = FieldElement::THREE;
        let deployment = account_factory.deploy(salt);
        let fee = deployment.estimate_fee().await.unwrap();
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, (fee.overall_fee * 2) as u128).await;

        let deploy_account_result = deployment.nonce(new_account_nonce).send().await.unwrap();

        let deploy_account_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deploy_account_result.transaction_hash)
            .await
            .unwrap();

        match deploy_account_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::DeployAccount(receipt)) => {
                assert_eq!(receipt.contract_address, new_account_address);
            }
            _ => {
                panic!("Invalid receipt {:?}", deploy_account_receipt);
            }
        }
    }

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

        let mut predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        );

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let predeployed_account = Arc::new(predeployed_account);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let deployment_result = contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
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
    async fn reverted_invoke_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (private_key, account_address) = devnet.get_first_predeployed_account().await;

        // constructs starknet-rs account
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        let mut predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        );

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let transfer_execution = predeployed_account.execute(vec![Call {
            to: FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,
                FieldElement::from_dec_str("1000000000").unwrap(),
                FieldElement::ZERO,
            ],
        }]);

        let fee = transfer_execution.estimate_fee().await.unwrap();

        // send transaction with lower than estimated fee
        // should revert
        let transfer_result = transfer_execution
            .max_fee(FieldElement::from(fee.overall_fee - 1))
            .send()
            .await
            .unwrap();

        let transfer_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(transfer_result.transaction_hash)
            .await
            .unwrap();

        match transfer_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
                match receipt.execution_result {
                    starknet_rs_core::types::ExecutionResult::Reverted { .. } => (),
                    _ => panic!("Invalid receipt {:?}", receipt),
                }
            }
            _ => panic!("Invalid receipt {:?}", transfer_receipt),
        };
    }

    #[tokio::test]
    async fn declare_v1_transaction_fails_with_insufficient_max_fee() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction_result = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await;

        match declare_transaction_result {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage { code, message: _ })) => {
                match code {
                    MaybeUnknownErrorCode::Known(StarknetError::InsufficientMaxFee) => (),
                    _ => panic!("Invalid error: {:?}", code),
                }
            }
            _ => {
                panic!("Invalid result: {:?}", declare_transaction_result);
            }
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
