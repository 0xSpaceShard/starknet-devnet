use std::sync::Arc;

use server::test_utils::assert_contains;
use starknet_core::constants::{
    DEVNET_DEFAULT_L1_DATA_GAS_PRICE, DEVNET_DEFAULT_L1_GAS_PRICE, DEVNET_DEFAULT_L2_GAS_PRICE,
    UDC_CONTRACT_CLASS_HASH,
};
use starknet_rs_accounts::{Account, ExecutionEncoder, ExecutionEncoding, SingleOwnerAccount};
use starknet_rs_core::chain_id::SEPOLIA;
use starknet_rs_core::types::{
    BlockHashAndNumber, BlockId, BlockTag, BroadcastedInvokeTransactionV3, BroadcastedTransaction,
    Call, ContractClass, ContractExecutionError, DataAvailabilityMode, ExecuteInvocation, Felt,
    InvokeTransactionTrace, ResourceBounds, ResourceBoundsMapping, SimulatedTransaction,
    SimulationFlag, SimulationFlagForEstimateFee, StarknetError, TransactionExecutionErrorData,
    TransactionTrace,
};
use starknet_rs_core::utils::{get_selector_from_name, get_storage_var_address, starknet_keccak};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID, ETH_ERC20_CONTRACT_ADDRESS,
    UDC_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    FeeUnit, assert_cairo1_classes_equal, get_events_contract_in_sierra_and_compiled_class_hash,
    get_flattened_sierra_contract_and_casm_hash,
};

#[tokio::test]
async fn get_storage_from_an_old_state() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .expect("Could not start Devnet");
    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    devnet.create_block().await.unwrap();
    let BlockHashAndNumber { block_hash, .. } =
        devnet.json_rpc_client.block_hash_and_number().await.unwrap();

    let amount = Felt::from(1_000_000_000);

    account
        .execute_v3(vec![Call {
            to: ETH_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,  // recipient
                amount,     // low part of uint256
                Felt::ZERO, // high part of uint256
            ],
        }])
        .send()
        .await
        .unwrap();

    let storage_address = get_storage_var_address("ERC20_balances", &[account_address]).unwrap();

    let latest_balance = devnet
        .json_rpc_client
        .get_storage_at(
            ETH_ERC20_CONTRACT_ADDRESS,
            storage_address,
            BlockId::Tag(starknet_rs_core::types::BlockTag::Latest),
        )
        .await
        .unwrap();
    let previous_balance = devnet
        .json_rpc_client
        .get_storage_at(ETH_ERC20_CONTRACT_ADDRESS, storage_address, BlockId::Hash(block_hash))
        .await
        .unwrap();

    assert!(latest_balance + amount <= previous_balance); // due to fee
}

#[tokio::test]
async fn minting_in_multiple_steps_and_getting_balance_at_each_block() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .unwrap();

    // create a block if there are no blocks before starting minting
    devnet.create_block().await.unwrap();

    let address = Felt::ONE;

    let mint_amount = 1e18 as u128;
    let unit = FeeUnit::Fri;

    for _ in 0..3 {
        let BlockHashAndNumber { block_hash, .. } =
            devnet.json_rpc_client.block_hash_and_number().await.unwrap();

        devnet.mint(address, mint_amount).await;
        let block_id = BlockId::Hash(block_hash);
        let balance_at_block = devnet.get_balance_at_block(&address, block_id).await.unwrap();

        let latest_balance = devnet.get_balance_latest(&address, unit).await.unwrap();

        assert_eq!(balance_at_block + Felt::from(mint_amount), latest_balance);
    }
}

// estimate fee of invoke transaction that reverts must fail, but simulating the same invoke
// transaction have to produce trace of a reverted transaction
#[tokio::test]
async fn estimate_fee_and_simulate_transaction_for_contract_deployment_in_an_old_block_should_not_produce_the_same_error()
 {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .unwrap();

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    devnet.create_block().await.unwrap();

    let BlockHashAndNumber { block_hash, .. } =
        devnet.json_rpc_client.block_hash_and_number().await.unwrap();

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_VERSION_ASSERTER_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    // declare class
    let declaration_result =
        account.declare_v3(Arc::new(flattened_contract_artifact), casm_hash).send().await.unwrap();
    assert_eq!(declaration_result.class_hash, class_hash);

    let invoked_selector = get_selector_from_name("deployContract").unwrap();
    let calls = vec![Call {
        to: UDC_CONTRACT_ADDRESS,
        selector: invoked_selector,
        calldata: vec![
            class_hash,
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,
            Felt::ZERO,
        ],
    }];

    let block_id = BlockId::Hash(block_hash);
    let estimate_fee_error = devnet
        .json_rpc_client
        .estimate_fee(
            [BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
                signature: vec![],
                nonce: Felt::ZERO,
                sender_address: account_address,
                calldata: account.encode_calls(&calls),
                is_query: true,
                resource_bounds: ResourceBoundsMapping {
                    l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                    l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                    l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
                },
                tip: 0,
                paymaster_data: vec![],
                account_deployment_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            })],
            [SimulationFlagForEstimateFee::SkipValidate],
            block_id,
        )
        .await
        .unwrap_err();

    let SimulatedTransaction { transaction_trace, .. } = devnet
        .json_rpc_client
        .simulate_transaction(
            block_id,
            BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
                signature: vec![],
                nonce: Felt::ZERO,
                sender_address: account_address,
                calldata: account.encode_calls(&calls),
                is_query: true,
                resource_bounds: ResourceBoundsMapping {
                    l1_gas: ResourceBounds {
                        max_amount: 0,
                        max_price_per_unit: DEVNET_DEFAULT_L1_GAS_PRICE.into(),
                    },
                    l1_data_gas: ResourceBounds {
                        max_amount: 1000,
                        max_price_per_unit: DEVNET_DEFAULT_L1_DATA_GAS_PRICE.into(),
                    },
                    l2_gas: ResourceBounds {
                        max_amount: 1e7 as u64,
                        max_price_per_unit: DEVNET_DEFAULT_L2_GAS_PRICE.into(),
                    },
                },
                tip: 0,
                paymaster_data: vec![],
                account_deployment_data: vec![],
                nonce_data_availability_mode: DataAvailabilityMode::L1,
                fee_data_availability_mode: DataAvailabilityMode::L1,
            }),
            [SimulationFlag::SkipValidate],
        )
        .await
        .unwrap();

    // The error is expected to be like this:
    // __execute__ of account contract -> deployContract of UDC contract -> constructor at computed address
    let execution_error_msg = match estimate_fee_error {
        ProviderError::StarknetError(StarknetError::TransactionExecutionError(
            TransactionExecutionErrorData {
                execution_error: ContractExecutionError::Nested(account_contract_error),
                ..
            },
        )) => {
            assert_eq!(account_contract_error.selector, starknet_keccak("__execute__".as_bytes()));
            assert_eq!(account_contract_error.contract_address, account.address());

            let account_class_hash = devnet
                .json_rpc_client
                .get_class_hash_at(
                    BlockId::Tag(BlockTag::Latest),
                    account_contract_error.contract_address,
                )
                .await
                .unwrap();
            assert_eq!(account_contract_error.class_hash, account_class_hash);

            match account_contract_error.error.as_ref() {
                ContractExecutionError::Nested(udc_contract_error) => {
                    assert_eq!(udc_contract_error.contract_address, UDC_CONTRACT_ADDRESS);
                    assert_eq!(udc_contract_error.selector, invoked_selector);
                    assert_eq!(udc_contract_error.class_hash, UDC_CONTRACT_CLASS_HASH);

                    match udc_contract_error.error.as_ref() {
                        ContractExecutionError::Nested(error_at_to_be_deployed_address) => {
                            assert_eq!(error_at_to_be_deployed_address.class_hash, class_hash);
                            match error_at_to_be_deployed_address.error.as_ref() {
                                ContractExecutionError::Message(msg) => {
                                    assert_contains(msg, &format!("{class_hash:x}"));
                                    assert_contains(msg, "is not declared");

                                    msg.clone()
                                }
                                other => panic!("Unexpected error: {other:?}"),
                            }
                        }
                        other => panic!("Unexpected error: {other:?}"),
                    }
                }
                other => panic!("Unexpected error: {other:?}"),
            }
        }
        other => panic!("Unexpected error: {other:?}"),
    };

    match transaction_trace {
        TransactionTrace::Invoke(InvokeTransactionTrace {
            execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
            ..
        }) => {
            assert_contains(&reverted_invocation.revert_reason, &execution_error_msg);
        }
        other => panic!("Unexpected trace {other:?}"),
    }
}

#[tokio::test]
async fn test_getting_class_at_various_blocks() {
    let devnet_args = ["--state-archive-capacity", "full"];
    let devnet = BackgroundDevnet::spawn_with_additional_args(&devnet_args).await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        SEPOLIA,
        ExecutionEncoding::New,
    ));

    let (contract_class, casm_class_hash) = get_events_contract_in_sierra_and_compiled_class_hash();

    // declare the contract
    let declaration_result = predeployed_account
        .declare_v3(Arc::new(contract_class.clone()), casm_class_hash)
        .l1_gas(0)
        .l1_data_gas(1000)
        .l2_gas(1e8 as u64)
        .send()
        .await
        .unwrap();

    let declaration_block = devnet.get_latest_block_with_tx_hashes().await.unwrap();

    // create an extra block so the declaration block is no longer the latest
    devnet.create_block().await.unwrap();

    // getting class at the following block IDs should be successful
    let expected_class = ContractClass::Sierra(contract_class);
    for block_id in [
        BlockId::Tag(BlockTag::Latest),
        BlockId::Tag(BlockTag::Pending),
        BlockId::Number(declaration_block.block_number),
        BlockId::Number(declaration_block.block_number + 1),
        BlockId::Hash(declaration_block.block_hash),
    ] {
        let retrieved_class = devnet
            .json_rpc_client
            .get_class(block_id, declaration_result.class_hash)
            .await
            .unwrap();

        assert_cairo1_classes_equal(&retrieved_class, &expected_class).unwrap();
    }

    // getting class at the following block IDs should NOT be successful
    for block_id in [BlockId::Number(declaration_block.block_number - 1)] {
        let retrieved =
            devnet.json_rpc_client.get_class(block_id, declaration_result.class_hash).await;
        match retrieved {
            Err(ProviderError::StarknetError(StarknetError::ClassHashNotFound)) => (),
            other => panic!("Unexpected response: {other:?}"),
        }
    }
}

#[tokio::test]
async fn test_nonce_retrieval_for_an_old_state() {
    let devnet =
        BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
            .await
            .unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        SEPOLIA,
        ExecutionEncoding::New,
    ));
    let BlockHashAndNumber { block_number, .. } =
        devnet.json_rpc_client.block_hash_and_number().await.unwrap();

    let initial_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    account
        .execute_v3(vec![Call {
            to: ETH_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,        // recipient
                Felt::from(1000), // low part of uint256
                Felt::ZERO,       // high part of uint256
            ],
        }])
        .send()
        .await
        .unwrap();

    let latest_nonce = devnet
        .json_rpc_client
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .unwrap();

    assert!(latest_nonce > initial_nonce);
    let nonce_at_old_state = devnet
        .json_rpc_client
        .get_nonce(BlockId::Number(block_number), account_address)
        .await
        .unwrap();

    assert_eq!(initial_nonce, nonce_at_old_state);
}
