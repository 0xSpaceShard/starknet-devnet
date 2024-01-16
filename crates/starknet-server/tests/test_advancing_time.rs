pub mod common;

mod advancing_time_tests {

    use std::sync::Arc;
    use std::{thread, time};

    use hyper::Body;
    use serde_json::json;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall};
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_json_body, get_timestamp_contract_in_sierra_and_compiled_class_hash,
        get_unix_timestamp_as_seconds,
    };
    use crate::common::utils::{send_ctrl_c_signal, UniqueAutoDeletableFile};

    const DUMMY_ADDRESS: u128 = 1;
    const DUMMY_AMOUNT: u128 = 1;
    // buffer should be always lower than the time change that we are testing
    const BUFFER_TIME_SECONDS: u64 = 30;

    pub fn assert_ge_with_buffer(val1: Option<u64>, val2: Option<u64>) {
        assert!(val1 >= val2, "Failed inequation: {val1:?} >= {val2:?}");
        let upper_limit = Some(val2.unwrap() + BUFFER_TIME_SECONDS);
        assert!(val1 <= upper_limit, "Failed inequation: {val1:?} <= {upper_limit:?}");
    }

    pub fn assert_gt_with_buffer(val1: Option<u64>, val2: Option<u64>) {
        assert!(val1 > val2, "Failed inequation: {val1:?} > {val2:?}");
        let upper_limit = Some(val2.unwrap() + BUFFER_TIME_SECONDS);
        assert!(val1 <= upper_limit, "Failed inequation: {val1:?} <= {upper_limit:?}");
    }

    pub async fn setup_timestamp_contract(devnet: &BackgroundDevnet) -> FieldElement {
        let (signer, address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        );

        // declare
        let (cairo_1_contract, casm_class_hash) =
            get_timestamp_contract_in_sierra_and_compiled_class_hash();
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();
        let predeployed_account = Arc::new(predeployed_account);

        // deploy
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &[],
        )
    }

    pub async fn get_current_timestamp(
        devnet: &BackgroundDevnet,
        timestamp_contract_address: FieldElement,
    ) -> Option<u64> {
        let call_current_timestamp = FunctionCall {
            contract_address: timestamp_contract_address,
            entry_point_selector: get_selector_from_name("get_timestamp").unwrap(),
            calldata: vec![],
        };
        let call_result = devnet
            .json_rpc_client
            .call(call_current_timestamp, BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(call_result.len(), 1);

        call_result[0].to_string().parse::<u64>().ok()
    }

    pub async fn mine_new_block(devnet: &BackgroundDevnet) {
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
    }

    #[tokio::test]
    async fn timestamp_syscall_set_in_past() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        // set time in past
        let past_time = 1;
        let set_time_body = Body::from(json!({ "time": past_time }).to_string());
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], past_time);

        mine_new_block(&devnet).await;

        // check if timestamp is greater/equal
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_ge_with_buffer(current_timestamp, Some(past_time));
    }

    #[tokio::test]
    async fn timestamp_syscall_set_in_future() {
        let now = get_unix_timestamp_as_seconds();
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        // set time in future
        let future_time = now + 1000;
        let set_time_body = Body::from(json!({ "time": future_time }).to_string());
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], future_time);

        mine_new_block(&devnet).await;

        // check if timestamp is greater/equal
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_ge_with_buffer(current_timestamp, Some(future_time));
    }

    #[tokio::test]
    async fn timestamp_syscall_increase_time() {
        let now = get_unix_timestamp_as_seconds();
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        // increase time
        let increase_time: u64 = 1000;
        let increase_time_body = Body::from(json!({ "time": increase_time }).to_string());
        devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();

        // check if timestamp is greater/equal
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_ge_with_buffer(current_timestamp, Some(now + increase_time));

        // wait 1 second, mine block
        thread::sleep(time::Duration::from_secs(1));
        mine_new_block(&devnet).await;

        // check if timestamp is greater
        let timestamp_after_new_block =
            get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_gt_with_buffer(timestamp_after_new_block, Some(now + increase_time));
        assert_gt_with_buffer(timestamp_after_new_block, current_timestamp);
    }

    #[tokio::test]
    async fn timestamp_syscall_contract_constructor() {
        let now = get_unix_timestamp_as_seconds();
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check constructor set of timestamp
        let call_storage_timestamp = FunctionCall {
            contract_address: timestamp_contract_address,
            entry_point_selector: get_selector_from_name("get_storage_timestamp").unwrap(),
            calldata: vec![],
        };
        let storage_timestamp = devnet
            .json_rpc_client
            .call(call_storage_timestamp, BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap()[0]
            .to_string()
            .parse::<u64>()
            .ok();
        assert_gt_with_buffer(storage_timestamp, Some(now));

        // wait 1 second and mine block
        thread::sleep(time::Duration::from_secs(1));
        mine_new_block(&devnet).await;

        // check if current timestamp is greater than storage timestamp and now
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_gt_with_buffer(current_timestamp, Some(now));
        assert_gt_with_buffer(current_timestamp, storage_timestamp);
    }

    #[tokio::test]
    async fn start_time_in_past_syscall() {
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            past_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        mine_new_block(&devnet).await;

        // check if timestamp is greater/equal
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_ge_with_buffer(current_timestamp, Some(past_time));
    }

    #[tokio::test]
    async fn start_time_in_future_syscall() {
        let now: u64 = get_unix_timestamp_as_seconds();
        let future_time = now + 1000;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            future_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");
        let timestamp_contract_address = setup_timestamp_contract(&devnet).await;

        mine_new_block(&devnet).await;

        // check if timestamp is greater/equal
        let current_timestamp = get_current_timestamp(&devnet, timestamp_contract_address).await;
        assert_ge_with_buffer(current_timestamp, Some(future_time));
    }

    #[tokio::test]
    async fn set_time_in_past() {
        // set time and assert if it's greater/equal than past_time, check if it's inside buffer
        // limit
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(json!({ "time": past_time }).to_string());
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], past_time);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than past_time, check if it's inside
        // buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(empty_block["timestamp"].as_u64(), Some(past_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block, check if it's inside buffer
        // limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(mint_block["timestamp"].as_u64(), empty_block["timestamp"].as_u64());
    }

    #[tokio::test]
    async fn set_time_in_future() {
        // set time and assert if it's greater/equal than future_time, check if it's inside buffer
        // limit
        let now = get_unix_timestamp_as_seconds();
        let future_time = now + 1000;
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(json!({ "time": future_time }).to_string());
        let resp = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body = get_json_body(resp).await;
        assert_eq!(resp_body["block_timestamp"], future_time);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check if block_timestamp is greater than future_time, check if it's
        // inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(empty_block["timestamp"].as_u64(), Some(future_time));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last empty block, check if it's inside
        // buffer limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(mint_block["timestamp"].as_u64(), Some(future_time));
    }

    #[tokio::test]
    async fn set_time_empty_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(json!({}).to_string());
        let result = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn set_time_wrong_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let set_time_body = Body::from(
            json!({
                "test": 0
            })
            .to_string(),
        );
        let result = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn increase_time() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let now = get_unix_timestamp_as_seconds();

        // increase time and assert if it's greater than now, check if it's inside buffer limit
        let first_increase_time: u64 = 10000;
        let first_increase_time_body =
            Body::from(json!({ "time": first_increase_time }).to_string());
        devnet.post_json("/increase_time".into(), first_increase_time_body).await.unwrap();
        let first_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            first_increase_time_block["timestamp"].as_u64(),
            Some(now + first_increase_time),
        );

        // second increase time, check if it's inside buffer limit
        let second_increase_time: u64 = 1000;
        let second_increase_time_body =
            Body::from(json!({ "time": second_increase_time }).to_string());
        devnet.post_json("/increase_time".into(), second_increase_time_body).await.unwrap();
        let second_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            second_increase_time_block["timestamp"].as_u64(),
            Some(now + first_increase_time + second_increase_time),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create block and check again if block_timestamp is greater than last block, check if it's
        // inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(
            empty_block["timestamp"].as_u64(),
            second_increase_time_block["timestamp"].as_u64(),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if after mint timestamp is greater than last block, check if it's inside buffer
        // limit
        devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(last_block["timestamp"].as_u64(), empty_block["timestamp"].as_u64());
    }

    #[tokio::test]
    async fn increase_time_empty_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let increase_time_body = Body::from(json!({}).to_string());
        let result = devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn increase_time_wrong_body() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let increase_time_body = Body::from(
            json!({
                "test": 0
            })
            .to_string(),
        );
        let result = devnet.post_json("/increase_time".into(), increase_time_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn wrong_start_time() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--start-time", "wrong"]).await;
        assert!(devnet.is_err());
    }

    #[tokio::test]
    async fn start_time_in_past() {
        let past_time = 1;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            past_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");

        // create block and check if block timestamp is greater/equal 1, check if it's inside buffer
        // limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(empty_block["timestamp"].as_u64(), Some(past_time));
    }

    #[tokio::test]
    async fn start_time_in_future() {
        let now = get_unix_timestamp_as_seconds();
        let future_time = now + 100;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            future_time.to_string().as_str(),
        ])
        .await
        .expect("Could not start Devnet");

        // create block and check if block timestamp is greater than now, check if it's inside
        // buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(empty_block["timestamp"].as_u64(), Some(future_time));
    }

    #[tokio::test]
    async fn advance_time_combination_test_with_dump_and_load() {
        let now = get_unix_timestamp_as_seconds();
        let past_time = 1;
        let dump_file = UniqueAutoDeletableFile::new("time_combination");
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--start-time",
            past_time.to_string().as_str(),
            "--dump-path",
            dump_file.path.as_str(),
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        // increase time and assert if it's greater/equal than start-time argument +
        // first_increase_time, check if it's inside buffer limit
        let first_increase_time: u64 = 1000;
        let first_increase_time_body =
            Body::from(json!({ "time": first_increase_time }).to_string());
        devnet.post_json("/increase_time".into(), first_increase_time_body).await.unwrap();
        let first_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            first_increase_time_block["timestamp"].as_u64(),
            Some(past_time + first_increase_time),
        );

        // increase the time a second time and assert if it's greater/equal than past_time +
        // first_increase_time + second_increase_time, check if it's inside buffer limit
        let second_increase_time: u64 = 100;
        let second_increase_time_body =
            Body::from(json!({ "time": second_increase_time }).to_string());
        devnet.post_json("/increase_time".into(), second_increase_time_body).await.unwrap();
        let second_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            second_increase_time_block["timestamp"].as_u64(),
            Some(past_time + first_increase_time + second_increase_time),
        );

        // set time to be now and check if the latest block timestamp is greater/equal now, check if
        // it's inside buffer limit
        let set_time_body = Body::from(json!({ "time": now }).to_string());
        let resp_set_time = devnet.post_json("/set_time".into(), set_time_body).await.unwrap();
        let resp_body_set_time = get_json_body(resp_set_time).await;
        assert_eq!(resp_body_set_time["block_timestamp"], now);
        let set_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(set_time_block["timestamp"].as_u64(), Some(now));

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // create a new empty block and check again if block timestamp is greater than
        // set_time_block, check if it's inside buffer limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let empty_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_gt_with_buffer(
            empty_block["timestamp"].as_u64(),
            set_time_block["timestamp"].as_u64(),
        );

        // increase the time a third time and assert if it's greater/equal than last empty block
        // timestamp + third_increase_time, check if it's inside buffer limit
        let third_increase_time: u64 = 10000;
        let third_increase_time_body =
            Body::from(json!({ "time": third_increase_time }).to_string());
        devnet.post_json("/increase_time".into(), third_increase_time_body).await.unwrap();
        let third_increase_time_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            third_increase_time_block["timestamp"].as_u64(),
            Some(empty_block["timestamp"].as_u64().unwrap() + third_increase_time),
        );

        // wait 1 second
        thread::sleep(time::Duration::from_secs(1));

        // check if the last block timestamp is greater previous block, check if it's inside buffer
        // limit
        devnet.post_json("/create_block".into(), Body::from(json!({}).to_string())).await.unwrap();
        let last_block = &devnet
            .send_custom_rpc("starknet_getBlockWithTxHashes", json!({ "block_id": "latest" }))
            .await["result"];
        assert_ge_with_buffer(
            last_block["timestamp"].as_u64(),
            third_increase_time_block["timestamp"].as_u64(),
        );

        // ctrl c
        send_ctrl_c_signal(&devnet.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1));
        
    }
}
