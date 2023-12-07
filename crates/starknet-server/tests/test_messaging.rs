//! This module tests the messaging feature from the starknet-server perspective.
//!
//! The message contract `cairo_0_l1l2_contract` associates a balance to a user (ContractAddress)
//! and contains the following entrypoints:
//! * increase_balance -> increases the balance of a user (contract address) for the given amount.
//! * get_balance -> gets the balance of a user.
//! * withdraw -> withdraw from a user the given amount, sending the amount to a l2->l1 message.
//! * deposit -> deposit the given amount from a l1->l2 message (l1 handler function).

// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_messaging {
    use std::str::FromStr;
    use std::sync::Arc;

    use ethers::prelude::*;
    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_rs_accounts::{
        Account, Call, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall};
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider};
    use starknet_rs_signers::LocalWallet;

    use crate::common::background_anvil::BackgroundAnvil;
    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::CHAIN_ID;
    use crate::common::utils::{
        get_json_body, get_messaging_contract_in_sierra_and_compiled_class_hash, to_hex_felt,
    };

    const DUMMY_L1_ADDRESS: &str = "0xc662c410c0ecf747543f5ba90660f6abebd9c8c4";

    /// Differs from MESSAGING_WHITELISTED_L1_CONTRACT: that address is hardcoded in the cairo0 l1l2
    /// contract relying on it This address is provided as an argument to the cairo1 l1l2
    /// contract
    const MESSAGING_L1_ADDRESS: &str = "0x5fbdb2315678afecb367f032d93f642f64180aa3";

    /// Withdraws the given amount from a user and send this amount in a l2->l1 message.
    async fn withdraw<A: ConnectedAccount + Send + Sync + 'static>(
        account: A,
        contract_address: FieldElement,
        user: FieldElement,
        amount: FieldElement,
        l1_address: FieldElement,
    ) {
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("withdraw").unwrap(),
            calldata: vec![user, amount, l1_address],
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

    /// Sets up a `BackgroundDevnet` with the message l1-l2 contract deployed.
    /// Returns (devnet instance, account used for deployment, l1-l2 contract address).
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

        let (sierra_class, casm_class_hash) =
            get_messaging_contract_in_sierra_and_compiled_class_hash();

        let sierra_class_hash = sierra_class.class_hash();
        let declaration = account.declare(Arc::new(sierra_class), casm_class_hash);

        declaration.send().await.unwrap();

        // deploy instance of class
        let contract_factory = ContractFactory::new(sierra_class_hash, account.clone());
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            sierra_class_hash,
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
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

        // We don't actually send a message to the L1 in this test.
        let l1_address = FieldElement::from_hex_be(DUMMY_L1_ADDRESS).unwrap();

        // Withdraw the 1 amount in a l2->l1 message.
        withdraw(Arc::clone(&account), l1l2_contract_address, user, user_balance, l1_address).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ZERO]);

        // Flush messages to check the presence of the withdraw where use expected to be 1 and
        // amount to be 1. The message always starts with a magic value MESSAGE_WITHDRAW
        // which is 0.
        let req_body = Body::from(json!({ "dry_run": true }).to_string());
        let resp = devnet.post_json("/postman/flush".into(), req_body).await.expect("flush failed");
        // assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let body = get_json_body(resp).await;

        let messages_to_l1 = body.get("messages_to_l1").unwrap().as_array().unwrap();
        assert_eq!(messages_to_l1.len(), 1);

        let l1_contract_address = messages_to_l1[0].get("to_address").unwrap().as_str().unwrap();
        let l2_contract_address = messages_to_l1[0].get("from_address").unwrap().as_str().unwrap();
        assert_eq!(l2_contract_address, format!("0x{:64x}", account.address()));
        assert_eq!(l1_contract_address, DUMMY_L1_ADDRESS);

        let payload = messages_to_l1[0].get("payload").unwrap().as_array().unwrap();
        // MESSAGE_WITHDRAW opcode, equal to 0, first element of the payload.
        assert_eq!(payload, &["0x0", &to_hex_felt(&user), &to_hex_felt(&user_balance)]);

        assert!(body.get("messages_to_l2").unwrap().as_array().unwrap().is_empty());
        assert!(body.get("generated_l2_transactions").unwrap().as_array().unwrap().is_empty());
        assert_eq!(body.get("l1_provider").unwrap().as_str().unwrap(), "dry run");
    }

    #[tokio::test]
    async fn can_receive_mock_message_to_l2() {
        let (devnet, account, l1l2_contract_address) =
            setup_devnet(&["--account-class", "cairo1"]).await;
        let user = FieldElement::ONE;

        // Set balance to 1 for user.
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

        // Use postman to send a message to l2 without l1 - the message increments user balance
        let increment_amount = FieldElement::from_hex_be("0xff").unwrap();
        let req_body = Body::from(
            json!({
                "l1_contract_address": MESSAGING_L1_ADDRESS,
                "l2_contract_address": format!("0x{:64x}", l1l2_contract_address),
                "entry_point_selector": format!("0x{:64x}", get_selector_from_name("deposit").unwrap()),
                "payload": [to_hex_felt(&user), to_hex_felt(&increment_amount)],
                "paid_fee_on_l1": "0x1234",
                "nonce": "0x1"
            }).to_string(),
        );

        let resp = devnet
            .post_json("/postman/send_message_to_l2".into(), req_body)
            .await
            .expect("send message to l2 failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let body = get_json_body(resp).await;
        assert_eq!(
            body.get("transaction_hash").unwrap().as_str().unwrap(),
            "0x7723a4247725834f72abe4d52768db6a2c5a39dac747a7d207250e0a583a31a"
        );

        assert_eq!(
            get_balance(&devnet, l1l2_contract_address, user).await,
            [user_balance + increment_amount]
        );
    }

    #[tokio::test]
    async fn can_deploy_l1_messaging_contract() {
        let anvil = BackgroundAnvil::spawn().await.unwrap();

        let (devnet, _, _) = setup_devnet(&["--account-class", "cairo1"]).await;

        let req_body = Body::from(json!({ "network_url": anvil.url }).to_string());
        let resp = devnet
            .post_json("/postman/load_l1_messaging_contract".into(), req_body)
            .await
            .expect("deploy l1 messaging contract failed");

        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let body = get_json_body(resp).await;
        assert_eq!(
            body.get("messaging_contract_address").unwrap().as_str().unwrap(),
            MESSAGING_L1_ADDRESS
        );
    }

    #[tokio::test]
    async fn can_consume_from_l2() {
        let (devnet, account, l1l2_contract_address) =
            setup_devnet(&["--account-class", "cairo1"]).await;
        let user = FieldElement::ONE;

        // Set balance to 1 for user.
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

        // We don't need valid l1 address as we don't use L1 node.
        let l1_address = FieldElement::from_hex_be(DUMMY_L1_ADDRESS).unwrap();

        // Withdraw the 1 amount in a l2->l1 message.
        withdraw(Arc::clone(&account), l1l2_contract_address, user, user_balance, l1_address).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ZERO]);

        let req_body = Body::from(
            json!({
                "from_address": "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
                "to_address": DUMMY_L1_ADDRESS,
                "payload": ["0x0","0x1","0x1"],
            })
            .to_string(),
        );

        let resp = devnet
            .post_json("/postman/consume_message_from_l2".into(), req_body)
            .await
            .expect("consume message from l2 failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let body = get_json_body(resp).await;
        assert_eq!(
            body.get("message_hash").unwrap().as_str().unwrap(),
            "0x9192040596a7dab9117c2882e4ea05364bee50a91bc84731d0eb6abc4b712398"
        );
    }

    #[tokio::test]
    async fn can_interact_with_l1() {
        let anvil = BackgroundAnvil::spawn().await.unwrap();

        let (devnet, sn_account, sn_l1l2_contract) =
            setup_devnet(&["--account-class", "cairo1"]).await;

        // Load l1 messaging contract.
        let req_body = Body::from(json!({ "network_url": anvil.url }).to_string());
        let resp = devnet
            .post_json("/postman/load_l1_messaging_contract".into(), req_body)
            .await
            .expect("deploy l1 messaging contract failed");

        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let body = get_json_body(resp).await;
        assert_eq!(
            body.get("messaging_contract_address").unwrap().as_str().unwrap(),
            MESSAGING_L1_ADDRESS
        );

        // Deploy the L1L2 testing contract on L1 (on L2 it's already pre-deployed).
        let l1_messaging_address = H160::from_str(MESSAGING_L1_ADDRESS).unwrap();
        let eth_l1l2_address = anvil.deploy_l1l2_contract(l1_messaging_address).await.unwrap();
        let eth_l1l2_address_hex = format!("{eth_l1l2_address:#x}");

        let eth_l1l2_address_felt = FieldElement::from_hex_be(&eth_l1l2_address_hex).unwrap();
        let user_sn = FieldElement::ONE;
        let user_eth: U256 = 1.into();

        // Set balance to 1 for the user 1 on L2.
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&sn_account), sn_l1l2_contract, user_sn, user_balance).await;
        assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [user_balance]);

        // Withdraw the amount 1 from user 1 balance on L2 to send it on L1 with a l2->l1 message.
        withdraw(
            Arc::clone(&sn_account),
            sn_l1l2_contract,
            user_sn,
            user_balance,
            eth_l1l2_address_felt,
        )
        .await;
        assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [FieldElement::ZERO]);

        // Flush to send the messages.
        let resp =
            devnet.post_json("/postman/flush".into(), "".into()).await.expect("flush failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        // Check that the balance is 0 on L1 before consuming the message.
        let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
        assert_eq!(user_balance_eth, 0.into());

        let sn_l1l2_contract_u256 =
            U256::from_str_radix(&format!("0x{:64x}", sn_l1l2_contract), 16).unwrap();

        let account_address_u256 =
            U256::from_str_radix(&format!("0x{:64x}", sn_account.address()), 16).unwrap();

        // Consume the message to increase the balance.
        anvil
            .withdraw_l1l2(eth_l1l2_address, account_address_u256, user_eth, 1.into())
            .await
            .unwrap();

        let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();
        assert_eq!(user_balance_eth, 1.into());

        // Send back the amount 1 to the user 1 on L2.
        anvil
            .deposit_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into())
            .await
            .unwrap();

        let user_balance_eth = anvil.get_balance_l1l2(eth_l1l2_address, user_eth).await.unwrap();

        // Balances on both layers is 0 at this point.
        assert_eq!(user_balance_eth, 0.into());
        assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [FieldElement::ZERO]);

        // Flush messages to have MessageToL2 executed.
        let resp =
            devnet.post_json("/postman/flush".into(), "".into()).await.expect("flush failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        // Ensure the balance is back to 1 on L2.
        assert_eq!(get_balance(&devnet, sn_l1l2_contract, user_sn).await, [FieldElement::ONE]);
    }
}
