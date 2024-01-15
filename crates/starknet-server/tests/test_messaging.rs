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
    use serde_json::{json, Value};
    use starknet_rs_accounts::{
        Account, AccountError, Call, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockTag, FieldElement, FunctionCall, InvokeTransactionResult,
        MaybePendingTransactionReceipt, TransactionExecutionStatus, TransactionReceipt,
    };
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider};
    use starknet_rs_signers::LocalWallet;

    use crate::common::background_anvil::BackgroundAnvil;
    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CHAIN_ID, L1_HANDLER_SELECTOR, MESSAGING_L1_CONTRACT_ADDRESS,
        MESSAGING_L2_CONTRACT_ADDRESS, MESSAGING_WHITELISTED_L1_CONTRACT,
    };
    use crate::common::utils::{
        get_json_body, get_messaging_contract_in_sierra_and_compiled_class_hash,
        get_messaging_lib_in_sierra_and_compiled_class_hash, send_ctrl_c_signal, to_hex_felt,
        UniqueAutoDeletableFile,
    };

    const DUMMY_L1_ADDRESS: &str = "0xc662c410c0ecf747543f5ba90660f6abebd9c8c4";
    const MESSAGE_WITHDRAW_OPCODE: &str = "0x0";

    const MAX_FEE: u128 = 1e18 as u128;

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

        account.execute(invoke_calls).max_fee(FieldElement::from(MAX_FEE)).send().await.unwrap();
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

        account.execute(invoke_calls).max_fee(FieldElement::from(MAX_FEE)).send().await.unwrap();
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

    /// Withdraws the given amount from a user and send this amount in a l2->l1 message
    /// using a library syscall instead of the contract with storage directly.
    async fn withdraw_from_lib<A: ConnectedAccount + Send + Sync + 'static>(
        account: A,
        contract_address: FieldElement,
        user: FieldElement,
        amount: FieldElement,
        l1_address: FieldElement,
        lib_class_hash: FieldElement,
    ) -> Result<InvokeTransactionResult, AccountError<<A as Account>::SignError>> {
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("withdraw_from_lib").unwrap(),
            calldata: vec![user, amount, l1_address, lib_class_hash],
        }];

        account.execute(invoke_calls).max_fee(FieldElement::from(MAX_FEE)).send().await
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

        // Declare l1l2 contract with storage (meant to be deployed).
        let (sierra_class, casm_class_hash) =
            get_messaging_contract_in_sierra_and_compiled_class_hash();

        let sierra_class_hash = sierra_class.class_hash();
        let declaration = account.declare(Arc::new(sierra_class), casm_class_hash);
        declaration.max_fee(FieldElement::from(MAX_FEE)).send().await.unwrap();

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
            .max_fee(FieldElement::from(MAX_FEE))
            .send()
            .await
            .expect("Cannot deploy");

        (devnet, account, contract_address)
    }

    fn assert_traces(traces: &Value) {
        assert_eq!(traces["type"], "L1_HANDLER");
        assert_eq!(
            traces["function_invocation"]["contract_address"],
            MESSAGING_L2_CONTRACT_ADDRESS
        );
        assert_eq!(traces["function_invocation"]["entry_point_selector"], L1_HANDLER_SELECTOR);
        assert_eq!(traces["function_invocation"]["calldata"][0], MESSAGING_L1_CONTRACT_ADDRESS);
        assert!(traces["state_diff"].is_null());
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

        // Flush messages to check the presence of the withdraw
        let req_body = Body::from(json!({ "dry_run": true }).to_string());
        let resp = devnet.post_json("/postman/flush".into(), req_body).await.expect("flush failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let resp_body = get_json_body(resp).await;

        assert_eq!(
            resp_body,
            json!({
                "messages_to_l1": [
                    {
                        "from_address": to_hex_felt(&l1l2_contract_address),
                        "to_address": DUMMY_L1_ADDRESS,
                        "payload": [MESSAGE_WITHDRAW_OPCODE, &to_hex_felt(&user), &to_hex_felt(&user_balance)]
                    }
                ],
                "messages_to_l2": [],
                "generated_l2_transactions": [],
                "l1_provider": "dry run"
            })
        );
    }

    #[tokio::test]
    async fn can_send_message_to_l1_from_library_syscall() {
        let (devnet, account, l1l2_contract_address) =
            setup_devnet(&["--account-class", "cairo1"]).await;

        // Declare l1l2 lib with only one function to send messages.
        // It's class hash can then be ignored, it's hardcoded in the contract.
        let (sierra_class, casm_class_hash) = get_messaging_lib_in_sierra_and_compiled_class_hash();
        let lib_sierra_class_hash = sierra_class.class_hash();

        account
            .declare(Arc::new(sierra_class), casm_class_hash)
            .max_fee(FieldElement::from(MAX_FEE))
            .send()
            .await
            .unwrap();

        let user = FieldElement::ONE;

        // Set balance to 1 for user.
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [user_balance]);

        // We don't actually send a message to the L1 in this test.
        let l1_address = FieldElement::from_hex_be(DUMMY_L1_ADDRESS).unwrap();

        // Withdraw the 1 amount in an l2->l1 message.
        withdraw_from_lib(
            Arc::clone(&account),
            l1l2_contract_address,
            user,
            user_balance,
            l1_address,
            lib_sierra_class_hash,
        )
        .await
        .unwrap();
        assert_eq!(get_balance(&devnet, l1l2_contract_address, user).await, [FieldElement::ZERO]);

        // Flush messages to check the presence of the withdraw
        let req_body = Body::from(json!({ "dry_run": true }).to_string());
        let resp = devnet.post_json("/postman/flush".into(), req_body).await.expect("flush failed");
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let resp_body = get_json_body(resp).await;

        assert_eq!(
            resp_body,
            json!({
                "messages_to_l1": [
                    {
                        "from_address": to_hex_felt(&l1l2_contract_address),
                        "to_address": DUMMY_L1_ADDRESS,
                        "payload": [MESSAGE_WITHDRAW_OPCODE, &to_hex_felt(&user), &to_hex_felt(&user_balance)]
                    }
                ],
                "messages_to_l2": [],
                "generated_l2_transactions": [],
                "l1_provider": "dry run"
            })
        );
    }

    #[tokio::test]
    #[ignore = "Starknet-rs doesnt support receipt with actual_fee as object"]
    async fn mock_message_to_l2_creates_a_tx_with_desired_effect() {
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
        let tx_hash = body.get("transaction_hash").unwrap().as_str().unwrap();
        assert_eq!(tx_hash, "0x7723a4247725834f72abe4d52768db6a2c5a39dac747a7d207250e0a583a31a");

        // assert state changed
        assert_eq!(
            get_balance(&devnet, l1l2_contract_address, user).await,
            [user_balance + increment_amount]
        );

        // assert tx and receipt retrievable and correct
        let tx_hash_felt = FieldElement::from_hex_be(tx_hash).unwrap();
        let expected_calldata =
            vec![FieldElement::from_hex_be(MESSAGING_L1_ADDRESS).unwrap(), user, increment_amount];
        match devnet.json_rpc_client.get_transaction_by_hash(tx_hash_felt).await {
            Ok(starknet_rs_core::types::Transaction::L1Handler(tx)) => {
                assert_eq!(tx.transaction_hash, tx_hash_felt);
                assert_eq!(tx.calldata, expected_calldata);
            }
            other => panic!("Error in fetching tx: {other:?}"),
        }
        match devnet.json_rpc_client.get_transaction_receipt(tx_hash_felt).await {
            Ok(MaybePendingTransactionReceipt::Receipt(TransactionReceipt::L1Handler(receipt))) => {
                assert_eq!(receipt.transaction_hash, tx_hash_felt);
                assert_eq!(
                    receipt.execution_result.status(),
                    TransactionExecutionStatus::Succeeded
                );
            }
            other => panic!("Error in fetching receipt: {other:?}"),
        }
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
                "from_address": format!("0x{:64x}", l1l2_contract_address),
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
            "0xac5356289f98e5012f8efcabb9f7ad3adaa43f81ac47b6311d2c23ca01590bd0"
        );
    }

    #[tokio::test]
    async fn can_interact_with_l1() {
        let anvil = BackgroundAnvil::spawn().await.unwrap();

        // check with --dump-on exit or transaction
        // TODO: copy this test to dump/load tests and revert changes here
        let (devnet, sn_account, sn_l1l2_contract) = setup_devnet(&[
            "--account-class",
            "cairo1",
            "--dump-path",
            "can_interact_with_l1",
            "--dump-on",
            "exit",
        ])
        .await;

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

        // Consume the message to increase the balance.
        anvil
            .withdraw_l1l2(eth_l1l2_address, sn_l1l2_contract_u256, user_eth, 1.into())
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

        // todo: test this with dump scenario
        // println!("devnet.process {:?}", devnet.);
        // println!("anvil.process {:?}", anvil.process);
        // Assert traces of L1Handler transaction with custom rpc call,
        // json_rpc_client.trace_transaction() is not supported
        let flush_body = get_json_body(resp).await;
        let l1_handler_tx_trace = &devnet
            .send_custom_rpc(
                "starknet_traceTransaction",
                json!({ "transaction_hash": flush_body.get("generated_l2_transactions").unwrap()[0] }),
            )
            .await["result"];
        assert_traces(l1_handler_tx_trace);
    }

    #[tokio::test]
    async fn assert_l1_handler_tx_can_be_dumped_and_loaded() {
        let dump_file = UniqueAutoDeletableFile::new("dump-with-l1-handler");
        let (dumping_devnet, account, l1l2_contract_address) = setup_devnet(&[
            "--account-class",
            "cairo1",
            "--dump-on",
            "exit",
            "--dump-path",
            &dump_file.path,
        ])
        .await;

        // Set balance for user
        let user = FieldElement::ONE;
        let user_balance = FieldElement::ONE;
        increase_balance(Arc::clone(&account), l1l2_contract_address, user, user_balance).await;
        assert_eq!(get_balance(&dumping_devnet, l1l2_contract_address, user).await, [user_balance]);

        // Use postman to send a message to l2 without l1 - the message increments user balance
        let increment_amount = FieldElement::from_hex_be("0xff").unwrap();
        let req_body = Body::from(json!({
            "l1_contract_address": MESSAGING_WHITELISTED_L1_CONTRACT,
            "l2_contract_address": format!("0x{:64x}", l1l2_contract_address),
            "entry_point_selector": format!("0x{:64x}", get_selector_from_name("deposit").unwrap()),
            "payload": [to_hex_felt(&user), to_hex_felt(&increment_amount)],
            "paid_fee_on_l1": "0x1234",
            "nonce": "0x1"
        }).to_string());

        dumping_devnet.post_json("/postman/send_message_to_l2".into(), req_body).await.unwrap();

        assert_eq!(
            get_balance(&dumping_devnet, l1l2_contract_address, user).await,
            [user_balance + increment_amount]
        );

        send_ctrl_c_signal(&dumping_devnet.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1));

        let loading_devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--account-class",
            "cairo1",
        ])
        .await
        .unwrap();

        assert_eq!(
            get_balance(&loading_devnet, l1l2_contract_address, user).await,
            [user_balance + increment_amount]
        );
    }
}
