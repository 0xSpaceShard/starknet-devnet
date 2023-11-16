//! This module tests the messaging feature from the starknet-server perspective.
//!
//! The message contract `cairo_0_l1l2_contract` associated a balance to a user (ContractAddress)
//! and contains the following entrypoints:
//! * increase_balance -> increases the balance of a user (contract address) for the given amount.
//! * get_balance -> gets the balance of a user.
//! * withdraw -> withdraw from a user the given amount, sending the amount to a l2->l1 message.
//! * deposit -> deposit the given amount from a l1->l2 message (l1 handler function).

// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_messaging {
    use std::sync::Arc;

    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_core::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use starknet_rs_accounts::{
        Account, Call, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall};
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider};
    use starknet_rs_signers::LocalWallet;

    use crate::common::constants::{CHAIN_ID, MESSAGING_L1_ALLOWED_CONTRACT};
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    /// Withdraws the given amount from a user and send this amount in a l2->l1 message.
    async fn withdraw<A: ConnectedAccount + Send + Sync + 'static>(
        account: A,
        contract_address: FieldElement,
        user: FieldElement,
        amount: FieldElement,
    ) {
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("withdraw").unwrap(),
            calldata: vec![user, amount],
        }];

        account.execute(invoke_calls).send().await.unwrap();
    }

    /// Increases the balance for the given user.
    async fn increase_balance<A: ConnectedAccount + Send + Sync + 'static>(
        account: A,
        contract_address: FieldElement,
        user: FieldElement,
        amount: FieldElement,
    ) {
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![user, amount],
        }];

        account.execute(invoke_calls).send().await.unwrap();
    }

    /// Gets the balance for the given user.
    async fn get_balance(
        devnet: &BackgroundDevnet,
        contract_address: FieldElement,
        user: FieldElement,
    ) -> Vec<FieldElement> {
        let call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_balance").unwrap(),
            calldata: vec![user],
        };

        devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap()
    }

    /// Setups a `BackgroundDevnet` with the message l1 l2 contract deployed.
    /// Returns the devnet instance and the account used for deployment.
    async fn setup_devnet(
        devnet_args: &[&str],
    ) -> (
        BackgroundDevnet,
        Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
        FieldElement,
    ) {
        let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let contract_json = dummy_cairo_l1l2_contract();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        let declaration_result = account
            .declare_legacy(contract_artifact.clone())
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();
        assert_eq!(declaration_result.class_hash, class_hash);

        // deploy instance of class
        let contract_factory = ContractFactory::new(class_hash, account.clone());
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory
            .deploy(constructor_calldata, salt, false)
            .nonce(FieldElement::ONE)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .expect("Cannot deploy");

        (devnet, account, contract_address)
    }

    #[tokio::test]
    async fn can_send_message_to_l1() {
        let (devnet, account, l1l2_contract_address) =
            setup_devnet(&["--account-class", "cairo1"]).await;
        let user = FieldElement::ONE;

        // Set balance to 1 for user.
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, FieldElement::ONE)
            .await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ONE]);

        // Withdraw the 1 amount in a l2->l1 message.
        withdraw(Arc::clone(&account), l1l2_contract_address, user, FieldElement::ONE).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ZERO]);

        // Flush messages to check the presence of the withdraw where use expected to be 1 and
        // amount to be 1. The message always starts with a magic value MESSAGE_WITHDRAW
        // which is 0.
        let req_body = Body::from(
            json!({
                "dryRun": true,
            })
            .to_string(),
        );

        let resp = devnet.post_json("/postman/flush".into(), req_body).await.expect("flush failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let body = get_json_body(resp).await;

        let messages_to_l1 = body.get("messagesToL1").unwrap().as_array().unwrap();
        assert_eq!(messages_to_l1.len(), 1);

        let l1_contract_address =
            messages_to_l1[0].get("l1_contract_address").unwrap().as_str().unwrap();
        let l2_contract_address =
            messages_to_l1[0].get("l2_contract_address").unwrap().as_str().unwrap();
        let payload = serde_json::from_value::<Vec<String>>(
            messages_to_l1[0].get("payload").unwrap().clone(),
        )
        .unwrap();
        assert_eq!(l2_contract_address, format!("0x{:64x}", account.address()));
        assert_eq!(l1_contract_address, MESSAGING_L1_ALLOWED_CONTRACT);
        assert_eq!(payload.len(), 3);
        // MESSAGE_WITHDRAW opcode, equal to 0, first element of the payload.
        assert_eq!(payload[0], "0x0");
        // User.
        assert_eq!(payload[1], "0x1");
        // Amount.
        assert_eq!(payload[2], "0x1");

        assert_eq!(body.get("messagesToL2").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(body.get("l1Provider").unwrap().as_str().unwrap(), "dry run");
    }

    #[tokio::test]
    async fn can_receive_mock_message_to_l2() {
        let (devnet, account, l1l2_contract_address) =
            setup_devnet(&["--account-class", "cairo1"]).await;
        let user = FieldElement::ONE;

        // Set balance to 1 for user.
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, FieldElement::ONE)
            .await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ONE]);

        // Use postman to send a message to l2 without l1.
        let req_body = Body::from(
            json!({
                "l1ContractAddress": MESSAGING_L1_ALLOWED_CONTRACT,
                "l2ContractAddress": format!("0x{:64x}", l1l2_contract_address),
                "entryPointSelector": format!("0x{:64x}", get_selector_from_name("deposit").unwrap()),
                "payload": ["0x1", "0xff"],
                "paidFeeOnL1": "0x1234",
                "nonce": "0x1"
            })
                .to_string(),
        );

        let resp = devnet
            .post_json("/postman/send_message_to_l2".into(), req_body)
            .await
            .expect("send message to l2 failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let body = get_json_body(resp).await;
        assert_eq!(
            body.get("transactionHash").unwrap().as_str().unwrap(),
            "0x1468183dc780231ea033f2aef5a7fa172daba80f53e2360e787ed1988ed670c"
        );
    }
}
