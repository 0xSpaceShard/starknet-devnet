pub mod common;

mod blocks_on_demand_tests {
    use std::sync::Arc;

    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockStatus, BlockTag, FieldElement, MaybePendingStateUpdate,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_contract_balance,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    async fn assert_latest_block_with_transactions(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions_count: u128,
    ) {
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();
        assert_eq!(latest_block.block_number, block_number);
        assert_eq!(latest_block.transactions.len() as u128, transactions_count);
        assert_eq!(latest_block.status, BlockStatus::AcceptedOnL2);

        for tx_hash in latest_block.transactions {
            assert_tx_successful(&tx_hash, &devnet.json_rpc_client).await;
        }
    }

    async fn assert_pending_block_with_transactions(
        devnet: &BackgroundDevnet,
        block_number: u64,
        transactions_count: u128,
    ) {
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(pending_block.block_number, block_number);
        assert_eq!(pending_block.transactions.len() as u128, transactions_count);
        assert_eq!(pending_block.status, BlockStatus::Pending);
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
        for _ in 0..tx_count {
            devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        }

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(0_u128), BlockTag::Latest).await;

        devnet.create_block().await.unwrap();

        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Pending)
            .await;
        assert_balance(&devnet, FieldElement::from(tx_count * DUMMY_AMOUNT), BlockTag::Latest)
            .await;

        assert_latest_block_with_transactions(&devnet, 1, tx_count).await;

        // check if pending_block was restarted
        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        assert_eq!(pending_block.block_number, 2);
        assert_eq!(pending_block.transactions.len(), 0);

        assert_pending_block_with_transactions(&devnet, 2, 0).await;
    }

    #[tokio::test]
    async fn pending_block_in_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        devnet.create_block().await.unwrap();

        let pending_block = devnet.get_pending_block_with_tx_hashes().await.unwrap();
        let latest_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

        // querying pending block in normal mode should default to the latest block
        assert_eq!(pending_block, latest_block);
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

        let increment = FieldElement::from(5_u32);
        let contract_invoke = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increment, FieldElement::ZERO],
        }];

        let invoke_result = predeployed_account
            .execute(contract_invoke.clone())
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        devnet.create_block().await.unwrap();

        assert_tx_successful(&invoke_result.transaction_hash, &devnet.json_rpc_client).await;
        assert_eq!(
            get_contract_balance(&devnet, contract_address).await,
            initial_value + increment
        );
    }

    #[tokio::test]
    async fn get_state_update() {
        let devnet: BackgroundDevnet =
            BackgroundDevnet::spawn_with_additional_args(&["--blocks-on-demand"])
                .await
                .expect("Could not start Devnet");

        devnet.create_block().await.unwrap();

        let state_update_pending_block =
            devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Pending)).await.unwrap();
        match state_update_pending_block {
            MaybePendingStateUpdate::PendingUpdate(_) => (),
            other => panic!("Unexpected result: {other:?}"),
        }

        let state_update_latest_block =
            devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Latest)).await.unwrap();
        match state_update_latest_block {
            MaybePendingStateUpdate::Update(_) => (),
            other => panic!("Unexpected result: {other:?}"),
        }
    }
}
