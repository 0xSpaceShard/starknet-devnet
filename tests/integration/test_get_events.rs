use std::sync::Arc;

use starknet_rs_accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    BlockId, BlockTag, Call, EmittedEvent, EventFilter, Felt, StarknetError,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{self, MAINNET_URL, STRK_ERC20_CONTRACT_ADDRESS};
use crate::common::utils::get_events_contract_artifacts;

async fn get_events_follow_continuation_token(
    devnet: &BackgroundDevnet,
    event_filter: EventFilter,
    chunk_size: u64,
) -> Result<Vec<EmittedEvent>, ProviderError> {
    let mut events = vec![];
    let mut continuation_token: Option<String> = None;
    loop {
        let events_page = devnet
            .json_rpc_client
            .get_events(event_filter.clone(), continuation_token, chunk_size)
            .await?;

        events.extend(events_page.events);

        continuation_token = events_page.continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }

    Ok(events)
}

/// A helper function which asserts that the `starknet_getEvents` RPC method returns the correct
/// events. It expects a running Devnet, gets the first predeployed account and uses it to declare
/// and deploy a contract that emits events. Then the events are fetched: first all in a single
/// chunk, then in multiple chunks.
async fn get_events_correct_chunking(devnet: &BackgroundDevnet, block_on_demand: bool) {
    let (signer, address) = devnet.get_first_predeployed_account().await;
    let mut predeployed_account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

    let (cairo_1_contract, casm_class_hash) = get_events_contract_artifacts();

    // declare the contract
    let declaration_result: starknet_rs_core::types::DeclareTransactionResult = predeployed_account
        .declare_v3(Arc::new(cairo_1_contract), casm_class_hash)
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
    contract_factory.deploy_v3(vec![], Felt::ZERO, false).send().await.unwrap();

    if block_on_demand {
        devnet.create_block().await.unwrap();
    }

    // generate the address of the newly deployed contract
    let new_contract_address = get_udc_deployed_address(
        Felt::ZERO,
        declaration_result.class_hash,
        &starknet_rs_core::utils::UdcUniqueness::NotUnique,
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
            .execute_v3(events_contract_call.clone())
            .nonce(nonce + Felt::from(n))
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
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
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
            from_block: Some(BlockId::Number(90_000_000)),
            to_block: Some(BlockId::Tag(BlockTag::Latest)),
            address: None,
            keys: None,
        };
        match devnet.json_rpc_client.get_events(event_filter, continuation_token, chunk_size).await
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
        match devnet.json_rpc_client.get_events(event_filter, continuation_token, chunk_size).await
        {
            Err(ProviderError::StarknetError(StarknetError::InvalidContinuationToken)) => (),
            resp => panic!("Unexpected response: {resp:?}"),
        }
    }
}

/// Spawns Devnet which forks mainnet at block number `block`.
async fn fork_mainnet_at(block: u64) -> Result<BackgroundDevnet, anyhow::Error> {
    let cli_args = ["--fork-network", MAINNET_URL, "--fork-block", &block.to_string()];
    Ok(BackgroundDevnet::spawn_with_additional_args(&cli_args).await?)
}

const FORK_BLOCK: u64 = 1374700;
const EVENTS_IN_FORK_BLOCK: usize = 330;

#[tokio::test]
async fn get_events_from_forked_devnet_when_last_queried_block_on_origin() {
    let fork_devnet = fork_mainnet_at(FORK_BLOCK).await.unwrap();

    assert_eq!(
        FORK_BLOCK + 1,
        fork_devnet.get_latest_block_with_tx_hashes().await.unwrap().block_number
    );

    let chunk_size = 100; // to force pagination
    let events = get_events_follow_continuation_token(
        &fork_devnet,
        EventFilter {
            from_block: Some(BlockId::Number(FORK_BLOCK)),
            to_block: Some(BlockId::Number(FORK_BLOCK)),
            address: Some(STRK_ERC20_CONTRACT_ADDRESS),
            keys: Some(vec![vec![get_selector_from_name("Transfer").unwrap()]]),
        },
        chunk_size,
    )
    .await
    .unwrap();

    assert_eq!(events.len(), EVENTS_IN_FORK_BLOCK);
}

#[tokio::test]
async fn get_events_from_forked_devnet_when_first_queried_block_on_devnet() {
    let fork_devnet = fork_mainnet_at(FORK_BLOCK).await.unwrap();

    assert_eq!(
        FORK_BLOCK + 1,
        fork_devnet.get_latest_block_with_tx_hashes().await.unwrap().block_number
    );

    let dummy_address = Felt::ONE;
    let mint_amount = 10;
    let n_mints = 3;
    for _ in 0..n_mints {
        fork_devnet.mint(dummy_address, mint_amount).await;
    }

    let chunk_size = 100; // to force pagination
    let events = get_events_follow_continuation_token(
        &fork_devnet,
        EventFilter {
            from_block: Some(BlockId::Number(FORK_BLOCK + 1)),
            to_block: None,
            address: Some(STRK_ERC20_CONTRACT_ADDRESS),
            keys: Some(vec![vec![get_selector_from_name("Transfer").unwrap()]]),
        },
        chunk_size,
    )
    .await
    .unwrap();

    // Each minting creates 2 transfers: one to charge the chargeable contract, one to give funds
    // to the target address.
    assert_eq!(events.len(), n_mints * 2);
}

#[tokio::test]
async fn get_events_from_forked_devnet_when_first_queried_block_on_origin_and_last_on_devnet() {
    let fork_devnet = fork_mainnet_at(FORK_BLOCK).await.unwrap();

    assert_eq!(
        FORK_BLOCK + 1,
        fork_devnet.get_latest_block_with_tx_hashes().await.unwrap().block_number
    );

    let dummy_address = Felt::ONE;
    let mint_amount = 10;
    let n_mints = 3;
    for _ in 0..n_mints {
        fork_devnet.mint(dummy_address, mint_amount).await;
    }

    let chunk_size = 100; // to force pagination
    let events = get_events_follow_continuation_token(
        &fork_devnet,
        EventFilter {
            from_block: Some(BlockId::Number(FORK_BLOCK)),
            to_block: None,
            address: Some(STRK_ERC20_CONTRACT_ADDRESS),
            keys: Some(vec![vec![get_selector_from_name("Transfer").unwrap()]]),
        },
        chunk_size,
    )
    .await
    .unwrap();

    // Each minting creates 2 transfers: one to charge the chargeable contract, one to give funds
    // to the target address.
    let fork_events = n_mints * 2;
    assert_eq!(events.len(), EVENTS_IN_FORK_BLOCK + fork_events);
}
