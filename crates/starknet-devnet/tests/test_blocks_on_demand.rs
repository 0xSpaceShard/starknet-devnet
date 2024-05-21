pub mod common;

mod blocks_on_demand_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockStatus, BlockTag, FieldElement, MaybePendingBlockWithTxHashes,
        MaybePendingStateUpdate,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_contract_balance, get_contract_balance_by_block_id,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn assert_pending_state_update(devnet: &BackgroundDevnet) {
        let pending_state_update = &devnet
            .json_rpc_client
            .get_state_update(BlockId::Tag(BlockTag::Pending))
            .await
            .unwrap();

        assert!(matches!(pending_state_update, MaybePendingStateUpdate::PendingUpdate(_)));
    }

    async fn assert_latest_state_update(devnet: &BackgroundDevnet, block_tag: BlockId) {
        let latest_state_update =
            &devnet.json_rpc_client.get_state_update(block_tag).await.unwrap();

        assert!(matches!(latest_state_update, MaybePendingStateUpdate::Update(_)));
    }

    async fn assert_latest_block_with_tx_hashes(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions: Vec<FieldElement>,
    ) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(transactions, latest_block.transactions);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);

        for tx_hash in latest_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_tx_hashes(devnet: &BackgroundDevnet) {
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();

        for tx_hash in pending_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_latest_block_with_txs(devnet: &BackgroundDevnet, block_number: u64) {
        let latest_block = devnet.get_latest_block_with_txs().await.unwrap();

        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);

        for tx in latest_block.transactions {
            assert_tx_successful(tx.transaction_hash(), &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_txs(devnet: &BackgroundDevnet) {
        let pending_block = devnet.get_pending_block_with_txs().await.unwrap();

        for tx in pending_block.transactions {
            assert_tx_successful(tx.transaction_hash(), &devnet.json_rpc_client).await;
        }
    }

    async fn assert_latest_block_with_receipts(devnet: &BackgroundDevnet, block_number: u64) {
        let latest_block = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithReceipts",
                json!(    {
                    "block_id": "latest"
                }),
            )
            .await["result"];

        assert_eq!(latest_block["block_number"], block_number);
        assert_eq!(latest_block["status"], "ACCEPTED_ON_L2");

        for tx in latest_block["transactions"].as_array().unwrap() {
            assert_tx_successful(
                &FieldElement::from_hex_be(tx["receipt"]["transaction_hash"].as_str().unwrap())
                    .unwrap(),
                &devnet.json_rpc_client,
            )
            .await;
        }
    }

    async fn assert_pending_block_with_receipts(devnet: &BackgroundDevnet) {
        let pending_block = &devnet
            .send_custom_rpc(
                "starknet_getBlockWithReceipts",
                json!(    {
                    "block_id": "pending"
                }),
            )
            .await["result"];

        assert!(pending_block["status"].is_null());

        for tx in pending_block["transactions"].as_array().unwrap() {
            assert_tx_successful(
                &FieldElement::from_hex_be(tx["receipt"]["transaction_hash"].as_str().unwrap())
                    .unwrap(),
                &devnet.json_rpc_client,
            )
            .await;
        }
    }

    async fn assert_balance(devnet: &BackgroundDevnet, expected: FieldElement, tag: BlockTag) {
        let balance = devnet
            .get_balance_by_tag(
                &FieldElement::from_hex_be(DUMMY_ADDRESS.to_string().as_str()).unwrap(),
                FeeUnit::WEI,
                tag,
            )
            .await
            .unwrap();
        assert_eq!(balance, expected);
    }

    #[tokio::test]
    async fn blocks_on_demand_states_and_blocks() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        let tx_count = 5;
        let mut tx_hashes = Vec::new();
        for _ in 0..tx_count {
            let mint_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
            tx_hashes.push(mint_hash);
        }

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(0_u128), BlockTag::Latest).await;

        devnet.create_block().await.unwrap();

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Latest)
            .await;

        assert_latest_block_with_tx_hashes(&devnet, 1, tx_hashes).await;
        assert_latest_block_with_txs(&devnet, 1).await;
        assert_latest_block_with_receipts(&devnet, 1).await;

        assert_pending_block_with_tx_hashes(&devnet).await;
        assert_pending_block_with_txs(&devnet).await;
        assert_pending_block_with_receipts(&devnet).await;

        assert_pending_state_update(&devnet).await;
        assert_latest_state_update(&devnet, BlockId::Tag(BlockTag::Latest)).await;
    }

    #[tokio::test]
    async fn pending_block_and_state_in_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        devnet.create_block().await.unwrap();

        let pending_block = devnet
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Pending))
            .await
            .unwrap();
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        // querying pending block in normal mode should default to the latest block
        match pending_block {
            MaybePendingBlockWithTxHashes::Block(block) => {
                assert_eq!(block, latest_block);
            }
            _ => panic!("Invalid block type {:?}", pending_block),
        }

        // querying state update in normal mode should default to the latest state update
        assert_latest_state_update(&devnet, BlockId::Tag(BlockTag::Pending)).await;
        assert_latest_state_update(&devnet, BlockId::Tag(BlockTag::Latest)).await;
    }

    #[tokio::test]
    async fn blocks_on_demand_invoke_and_call() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

        let mut tx_hashes = Vec::new();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (contract_class, casm_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(contract_class), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ZERO)
            .send()
            .await
            .unwrap();

        tx_hashes.push(declaration_result.transaction_hash);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        let deploy_result = contract_factory
            .deploy(ctor_args.clone(), FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ZERO)
            .send()
            .await
            .unwrap();

        tx_hashes.push(deploy_result.transaction_hash);

        // generate the address of the newly deployed contract
        let contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_args,
        );

        assert_eq!(
            get_contract_balance_by_block_id(
                &devnet,
                contract_address,
                BlockId::Tag(BlockTag::Pending)
            )
            .await,
            initial_value
        );

        let increment = FieldElement::from(5_u32);
        let contract_invoke = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increment, FieldElement::ZERO],
        }];
        let tx_count = 2;
        for n in 1..=tx_count {
            let invoke_result = predeployed_account
                .execute(contract_invoke.clone())
                .max_fee(FieldElement::from(1e18 as u128))
                .nonce(FieldElement::from(n as u128))
                .send()
                .await
                .unwrap();

            assert_tx_successful(&invoke_result.transaction_hash, &devnet.json_rpc_client).await;

            tx_hashes.push(invoke_result.transaction_hash);
        }

        assert_eq!(
            get_contract_balance_by_block_id(
                &devnet,
                contract_address,
                BlockId::Tag(BlockTag::Pending)
            )
            .await,
            initial_value + (increment * FieldElement::from(tx_count as u128))
        );

        devnet.create_block().await.unwrap();

        assert_latest_block_with_tx_hashes(&devnet, 1, tx_hashes).await;

        println!("TODO: This call should query the latest state not pending");
        assert_eq!(
            get_contract_balance_by_block_id(
                &devnet,
                contract_address,
                BlockId::Tag(BlockTag::Pending)
            )
            .await,
            initial_value + (increment * FieldElement::from(tx_count as u128))
        );
        assert_eq!(
            get_contract_balance(&devnet, contract_address).await,
            initial_value + (increment * FieldElement::from(tx_count as u128))
        );
    }
}
