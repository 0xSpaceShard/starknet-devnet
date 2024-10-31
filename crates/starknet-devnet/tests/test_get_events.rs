#![cfg(test)]
pub mod common;

mod get_events_integration_tests {
    use std::sync::Arc;

    use starknet_accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount};
    use starknet_contract::ContractFactory;
    use starknet_core::types::{BlockId, BlockTag, Call, EventFilter, Felt, StarknetError};
    use starknet_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

    /// The test verifies that the `get_events` RPC method returns the correct events.
    /// The test starts a devnet, gets the first predeployed account, using it declares and deploys
    /// a contract that emits events.
    /// Then the events are being fetched first all of them then in chunks
    async fn get_events_correct_chunking(devnet: &BackgroundDevnet, block_on_demand: bool) {
        let (signer, address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(Felt::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        let predeployed_account = Arc::new(predeployed_account);

        if block_on_demand {
            devnet.create_block().await.unwrap();
        }

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        contract_factory
            .deploy_v1(vec![], Felt::ZERO, false)
            .max_fee(Felt::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        if block_on_demand {
            devnet.create_block().await.unwrap();
        }

        // generate the address of the newly deployed contract
        let new_contract_address = get_udc_deployed_address(
            Felt::ZERO,
            declaration_result.class_hash,
            &starknet_core::utils::UdcUniqueness::NotUnique,
            &[],
        );

        let events_contract_call = vec![Call {
            to: new_contract_address,
            selector: get_selector_from_name("emit_event").unwrap(),
            calldata: vec![Felt::ONE],
        }];

        // invoke 10 times the contract to emit event, it should produce 10 events
        let n_events_contract_invocations = 10;
        let nonce = predeployed_account.get_nonce().await.unwrap();
        for n in 0..n_events_contract_invocations {
            predeployed_account
                .execute_v1(events_contract_call.clone())
                .nonce(nonce + Felt::from(n))
                .max_fee(Felt::from(100000000000000000000u128))
                .send()
                .await
                .unwrap();
        }

        if block_on_demand {
            devnet.create_block().await.unwrap();
        }

        // get all the events from the contract, the chunk size is large enough so we are sure
        // we get all the events in one call
        let event_filter = EventFilter {
            from_block: None,
            to_block: Some(BlockId::Tag(BlockTag::Latest)),
            address: Some(new_contract_address),
            keys: None,
        };

        let events =
            devnet.json_rpc_client.get_events(event_filter.clone(), None, 100000000).await.unwrap();

        let generated_events_count = events.events.len();
        assert_eq!(generated_events_count, n_events_contract_invocations);

        // divide the events by a group of 3
        // and iterate over with continuation token
        // on the last iteration the continuation token should be None
        let chunk_size = 3;
        let mut continuation_token: Option<String> = None;
        let mut total_extracted_events = 0;
        loop {
            let events = devnet
                .json_rpc_client
                .get_events(event_filter.clone(), continuation_token, chunk_size as u64)
                .await
                .unwrap();
            total_extracted_events += events.events.len();

            if events.continuation_token.is_some() {
                assert_eq!(events.events.len(), chunk_size);
            } else {
                assert!(events.events.len() <= chunk_size);
            }

            continuation_token = events.continuation_token;
            if continuation_token.is_none() {
                break;
            }
        }

        assert_eq!(total_extracted_events, generated_events_count);
    }

    #[tokio::test]
    async fn get_events_correct_chunking_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        get_events_correct_chunking(&devnet, false).await
    }

    #[tokio::test]
    async fn get_events_correct_chunking_blocks_generation_on_demand() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
                .await
                .expect("Could not start Devnet");

        get_events_correct_chunking(&devnet, true).await
    }

    #[tokio::test]
    async fn get_events_errors() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        {
            let chunk_size: u64 = 3;
            let continuation_token: Option<String> = None;
            let event_filter = EventFilter {
                from_block: Some(BlockId::Number(90000000)),
                to_block: Some(BlockId::Tag(BlockTag::Latest)),
                address: None,
                keys: None,
            };
            match devnet
                .json_rpc_client
                .get_events(event_filter, continuation_token, chunk_size)
                .await
            {
                Err(ProviderError::StarknetError(StarknetError::BlockNotFound)) => (),
                resp => panic!("Unexpected response: {resp:?}"),
            }
        }
        {
            let chunk_size: u64 = 3;
            let continuation_token = Some(String::from("invalid token"));
            let event_filter = EventFilter {
                from_block: Some(BlockId::Number(0)),
                to_block: Some(BlockId::Tag(BlockTag::Latest)),
                address: None,
                keys: None,
            };
            match devnet
                .json_rpc_client
                .get_events(event_filter, continuation_token, chunk_size)
                .await
            {
                Err(ProviderError::StarknetError(StarknetError::InvalidContinuationToken)) => (),
                resp => panic!("Unexpected response: {resp:?}"),
            }
        }
    }
}
