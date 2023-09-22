pub mod common;

mod get_events_integration_tests {
    use std::sync::Arc;

    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{BlockId, BlockTag, EventFilter, FieldElement};
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::Provider;
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_events_contract_in_sierra_and_compiled_class_hash, get_json_body,
    };

    #[tokio::test]
    /// The test verifies that the `get_events` RPC method returns the correct events.
    /// The test starts a devnet, gets the first predeployed account, using it declares and deploys
    /// a contract that emits events.
    /// Then the events are being fetched first all of them then in chunks
    async fn get_events_correct_chunking() {
        let devnet = BackgroundDevnet::spawn(None).await.expect("Could not start Devnet");

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
        contract_factory
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

        let events_contract_call = vec![Call {
            to: new_contract_address,
            selector: get_selector_from_name("emit_event").unwrap(),
            calldata: vec![FieldElement::from(1u8)],
        }];

        // invoke 10 times the contract to emit event, it should produce 10 events
        let n_events_contract_invokations = 10;
        for _ in 0..n_events_contract_invokations {
            predeployed_account
                .execute(events_contract_call.clone())
                .max_fee(FieldElement::from(1e18 as u128))
                .send()
                .await
                .unwrap();
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
        assert_eq!(generated_events_count, n_events_contract_invokations);

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
}
