pub mod common;

mod blocks_on_demand_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockStatus, BlockTag, FieldElement, MaybePendingBlockWithTxHashes,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_contract_balance,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn assert_pending_state_update(devnet: &BackgroundDevnet) {
        let pending_state_update = &devnet
            .send_custom_rpc(
                "starknet_getStateUpdate",
                json!(    {
                    "block_id": "pending"
                }),
            )
            .await["result"];

        assert!(pending_state_update["old_root"].is_string());
        assert!(pending_state_update["state_diff"].is_object());
        assert!(pending_state_update["block_hash"].is_null());
        assert!(pending_state_update["new_root"].is_null());
    }

    async fn assert_latest_state_update(devnet: &BackgroundDevnet, block_id: &str) {
        let latest_state_update = &devnet
            .send_custom_rpc(
                "starknet_getStateUpdate",
                json!(    {
                    "block_id": block_id
                }),
            )
            .await["result"];

        assert!(latest_state_update["block_hash"].is_string());
        assert!(latest_state_update["new_root"].is_string());
        assert!(latest_state_update["old_root"].is_string());
        assert!(latest_state_update["state_diff"].is_object());
    }

    async fn assert_latest_block_with_transactions(
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

    async fn assert_pending_block_with_transactions(
        devnet: &BackgroundDevnet,
        transactions_count: u128,
    ) {
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();

        match pending_block {
            MaybePendingBlockWithTxHashes::PendingBlock(block) => {
                assert_eq!(block.transactions.len() as u128, transactions_count);

                for tx_hash in block.transactions {
                    assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
                }
            }
            _ => panic!("Invalid block type {:?}", pending_block),
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

        assert_latest_block_with_transactions(&devnet, 1, tx_hashes).await;
        assert_pending_block_with_transactions(&devnet, 0).await;

        assert_pending_state_update(&devnet).await;
        assert_latest_state_update(&devnet, "latest").await;
    }

    #[tokio::test]
    async fn pending_block_and_state_in_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        devnet.create_block().await.unwrap();

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        // querying pending block in normal mode should default to the latest block
        match pending_block {
            MaybePendingBlockWithTxHashes::Block(block) => {
                assert_eq!(block, latest_block);
            }
            _ => panic!("Invalid block type {:?}", pending_block),
        }

        // querying state update in normal mode should default to the latest state update
        assert_latest_state_update(&devnet, "pending").await;
        assert_latest_state_update(&devnet, "latest").await;
    }

    #[tokio::test]
    async fn blocks_on_demand_invoke_and_call() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"]).await.unwrap();

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
            .send()
            .await
            .unwrap();

        devnet.create_block().await.unwrap();

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let initial_value = FieldElement::from(10_u32);
        let ctor_args = vec![initial_value];
        contract_factory
            .deploy(ctor_args.clone(), FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        devnet.create_block().await.unwrap();

        // generate the address of the newly deployed contract
        let contract_address = get_udc_deployed_address(
            FieldElement::ZERO,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_args,
        );

        let mut tx_hashes = Vec::new();
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

        devnet.create_block().await.unwrap();

        assert_latest_block_with_transactions(&devnet, 3, tx_hashes).await;
        assert_eq!(
            get_contract_balance(&devnet, contract_address).await,
            initial_value + (increment * FieldElement::from(tx_count as u128))
        );
    }
}
