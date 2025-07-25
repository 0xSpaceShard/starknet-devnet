use std::sync::Arc;

use serde_json::json;
use starknet_core::constants::STRK_ERC20_CONTRACT_ADDRESS;
use starknet_rs_accounts::{
    Account, AccountError, AccountFactory, ConnectedAccount, ExecutionEncoder, ExecutionEncoding,
    OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_contract::ContractFactory;
use starknet_rs_core::types::{
    BlockId, BlockTag, BroadcastedDeclareTransactionV3, BroadcastedDeployAccountTransactionV3,
    BroadcastedInvokeTransactionV3, BroadcastedTransaction, Call, ContractExecutionError,
    DataAvailabilityMode, ExecuteInvocation, Felt, FunctionCall, InvokeTransactionTrace,
    MaybePreConfirmedBlockWithTxHashes, ResourceBounds, ResourceBoundsMapping,
    SimulatedTransaction, SimulationFlag, StarknetError, TransactionExecutionErrorData,
    TransactionTrace,
};
use starknet_rs_core::utils::{
    UdcUniqueness, cairo_short_string_to_felt, get_selector_from_name, get_udc_deployed_address,
};
use starknet_rs_providers::{Provider, ProviderError};
use starknet_rs_signers::{LocalWallet, Signer, SigningKey};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, CAIRO_1_CONTRACT_PATH,
    CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH, CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID,
    ETH_ERC20_CONTRACT_ADDRESS, QUERY_VERSION_OFFSET, UDC_LEGACY_CONTRACT_ADDRESS,
};
use crate::common::fees::{assert_difference_if_validation, assert_fee_in_resp_at_least_equal};
use crate::common::utils::{
    LocalFee, assert_contains, declare_v3_deploy_v3, get_deployable_account_signer,
    get_flattened_sierra_contract_and_casm_hash, get_simple_contract_artifacts, iter_to_hex_felt,
    to_hex_felt, to_num_as_hex,
};

#[tokio::test]
async fn simulate_declare_v3() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    );

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

    let nonce = Felt::ZERO;
    let declaration = account.declare_v3(Arc::new(flattened_contract_artifact.clone()), casm_hash);

    let fee = LocalFee::from(declaration.estimate_fee().await.unwrap());
    let declaration = declaration
        .l1_gas(fee.l1_gas)
        .l1_gas_price(fee.l1_gas_price)
        .l2_gas(fee.l2_gas)
        .l2_gas_price(fee.l2_gas_price)
        .l1_data_gas(fee.l1_data_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .nonce(nonce)
        .prepared()
        .unwrap();

    let declaration_hash = declaration.transaction_hash(false);
    let signature = signer.sign_hash(&declaration_hash).await.unwrap();

    let sender_address_hex = to_hex_felt(&account_address);

    let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
        json!({
            "block_id": "latest",
            "simulation_flags": simulation_flags,
            "transactions": [
                {
                    "type": "DECLARE",
                    "sender_address": sender_address_hex,
                    "compiled_class_hash": to_hex_felt(&casm_hash),
                    "version": "0x3",
                    "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                    "nonce": to_num_as_hex(&nonce),
                    "contract_class": flattened_contract_artifact,
                    "resource_bounds": ResourceBoundsMapping::from(fee),
                    "tip": "0x0",
                    "paymaster_data": [],
                    "account_deployment_data":[],
                    "nonce_data_availability_mode":"L1",
                    "fee_data_availability_mode":"L1",
                }
            ]
        })
    };

    let resp_no_flags = &devnet
        .send_custom_rpc("starknet_simulateTransactions", get_params(&["SKIP_FEE_CHARGE"]))
        .await
        .unwrap()[0];

    let resp_skip_validation = &devnet
        .send_custom_rpc(
            "starknet_simulateTransactions",
            get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]),
        )
        .await
        .unwrap()[0];

    assert_difference_if_validation(resp_no_flags, resp_skip_validation, &sender_address_hex, true);
}

#[tokio::test]
async fn simulate_deploy_account() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // define the key of the new account - dummy value
    let new_account_signer = get_deployable_account_signer();
    let account_factory = OpenZeppelinAccountFactory::new(
        Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH),
        CHAIN_ID,
        new_account_signer.clone(),
        devnet.clone_provider(),
    )
    .await
    .unwrap();

    let nonce = Felt::ZERO;
    let salt_hex = "0x123";
    let deployment = account_factory.deploy_v3(Felt::from_hex_unchecked(salt_hex));
    let fee = LocalFee::from(deployment.estimate_fee().await.unwrap());

    let deployment = deployment
        .nonce(nonce)
        .l1_gas(fee.l1_gas)
        .l1_gas_price(fee.l1_gas_price)
        .l2_gas_price(fee.l2_gas_price)
        .l2_gas(fee.l2_gas)
        .l1_data_gas(fee.l1_data_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .prepared()
        .unwrap();

    let deployment_tx_hash = deployment.transaction_hash(false);

    let signature = new_account_signer.sign_hash(&deployment_tx_hash).await.unwrap();
    let signature_hex: Vec<String> = iter_to_hex_felt(&[signature.r, signature.s]);
    let account_public_key = new_account_signer.get_public_key().await.unwrap().scalar();

    let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
        json!({
            "block_id": "latest",
            "simulation_flags": simulation_flags,
            "transactions": [
                {
                    "type": "DEPLOY_ACCOUNT",
                    "version": "0x3",
                    "signature": signature_hex,
                    "nonce": to_num_as_hex(&nonce),
                    "contract_address_salt": salt_hex,
                    "constructor_calldata": [to_hex_felt(&account_public_key)],
                    "class_hash": CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
                    "resource_bounds": ResourceBoundsMapping::from(fee),
                    "tip": "0x0",
                    "paymaster_data": [],
                    "nonce_data_availability_mode":"L1",
                    "fee_data_availability_mode":"L1",
                }
            ]
        })
    };

    let account_address = deployment.address();
    let account_address_hex = to_hex_felt(&account_address);
    devnet.mint(account_address, 1e18 as u128).await;

    // no flags
    let params_no_flags = get_params(&[]);
    let resp_no_flags =
        &devnet.send_custom_rpc("starknet_simulateTransactions", params_no_flags).await.unwrap()[0];

    let no_flags_trace = &resp_no_flags["transaction_trace"];
    assert_eq!(
        no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
        account_address_hex
    );
    assert_eq!(
        Felt::from_hex_unchecked(
            no_flags_trace["fee_transfer_invocation"]["contract_address"].as_str().unwrap()
        ),
        STRK_ERC20_CONTRACT_ADDRESS
    );
    assert_eq!(
        no_flags_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
        account_address_hex
    );

    // skipped validation
    let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
    let resp_skip_validation = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
        .await
        .unwrap()[0];
    let skip_validation_trace = &resp_skip_validation["transaction_trace"];
    assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
    assert_eq!(
        Felt::from_hex_unchecked(
            skip_validation_trace["fee_transfer_invocation"]["contract_address"].as_str().unwrap()
        ),
        STRK_ERC20_CONTRACT_ADDRESS
    );
    assert_eq!(
        skip_validation_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
        account_address_hex
    );

    assert_fee_in_resp_at_least_equal(resp_no_flags, resp_skip_validation);

    // skipped validation and fee charging (everything)
    let params_skip_everything = get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]);
    let resp_skip_everything = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_everything)
        .await
        .unwrap()[0];
    let skip_everything_trace = &resp_skip_everything["transaction_trace"];
    assert!(skip_everything_trace["validate_invocation"].as_object().is_none());
    assert!(skip_everything_trace["fee_transfer_invocation"].as_object().is_none());
    assert_eq!(
        skip_everything_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
        account_address_hex
    );
}

#[tokio::test]
async fn simulate_invoke_v3() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer.clone(),
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    ));

    // get class
    let (contract_artifact, casm_hash) = get_simple_contract_artifacts();
    let contract_artifact = Arc::new(contract_artifact);
    let class_hash = contract_artifact.class_hash();

    // declare class
    let declaration_result = account.declare_v3(contract_artifact, casm_hash).send().await.unwrap();
    assert_eq!(declaration_result.class_hash, class_hash);

    // deploy instance of class
    let contract_factory = ContractFactory::new(class_hash, account.clone());
    let salt = Felt::from_hex_unchecked("0x123");
    let constructor_calldata = vec![Felt::ZERO];
    let contract_address = get_udc_deployed_address(
        salt,
        class_hash,
        &UdcUniqueness::NotUnique,
        &constructor_calldata,
    );
    contract_factory.deploy_v3(constructor_calldata, salt, false).send().await.unwrap();

    // prepare the call used in simulation
    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("increase_balance").unwrap(),
        calldata: vec![Felt::from(100u128), Felt::ZERO], // increase amount
    }];

    // TODO fails if gas bounds too low, can be used to test reverted case
    let nonce = Felt::TWO; // after declare+deploy
    let execution = account.execute_v3(calls.clone());
    let fee = LocalFee::from(execution.estimate_fee().await.unwrap());
    let invoke_request = execution
        .l1_gas(fee.l1_gas)
        .l1_gas_price(fee.l1_gas_price)
        .l1_data_gas(fee.l1_data_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .l2_gas(fee.l2_gas)
        .l2_gas_price(fee.l2_gas_price)
        .nonce(nonce)
        .prepared()
        .unwrap();

    let signature = signer.sign_hash(&invoke_request.transaction_hash(false)).await.unwrap();
    let sender_address_hex = to_hex_felt(&account.address());

    let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
        json!({
            "block_id": "latest",
            "simulation_flags": simulation_flags,
            "transactions": [
                {
                    "type": "INVOKE",
                    "version": "0x3",
                    "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                    "nonce": nonce,
                    "calldata": iter_to_hex_felt(&account.encode_calls(&calls)),
                    "sender_address": sender_address_hex,
                    "resource_bounds": ResourceBoundsMapping::from(fee),
                    "tip":"0x0",
                    "paymaster_data":[],
                    "account_deployment_data":[],
                    "nonce_data_availability_mode":"L1",
                    "fee_data_availability_mode":"L1"
                }
            ]
        })
    };

    let params_no_flags = get_params(&[]);

    let resp_no_flags =
        &devnet.send_custom_rpc("starknet_simulateTransactions", params_no_flags).await.unwrap()[0];
    assert_eq!(
        resp_no_flags["transaction_trace"]["execute_invocation"]["contract_address"],
        sender_address_hex
    );

    let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
    let resp_skip_validation = &devnet
        .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
        .await
        .unwrap()[0];
    assert_eq!(
        resp_skip_validation["transaction_trace"]["execute_invocation"]["contract_address"],
        sender_address_hex
    );

    assert_difference_if_validation(
        resp_no_flags,
        resp_skip_validation,
        &sender_address_hex,
        false, // skip fee
    );

    // assert simulations haven't changed the balance property
    let final_balance = devnet
        .json_rpc_client
        .call(
            FunctionCall {
                contract_address,
                entry_point_selector: get_selector_from_name("get_balance").unwrap(),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .unwrap();
    assert_eq!(final_balance, vec![Felt::ZERO]);
}

#[tokio::test]
async fn using_query_version_if_simulating() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        CHAIN_ID,
        ExecutionEncoding::New,
    );

    // get class
    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_VERSION_ASSERTER_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    let (generated_class_hash, contract_address) =
        declare_v3_deploy_v3(&account, flattened_contract_artifact, casm_hash, &[]).await.unwrap();
    assert_eq!(generated_class_hash, class_hash);

    let calls = vec![Call {
        to: contract_address,
        selector: get_selector_from_name("assert_version").unwrap(),
        calldata: vec![QUERY_VERSION_OFFSET + Felt::THREE], // expected version
    }];

    let execution = account.execute_v3(calls.clone());
    let fee = LocalFee::from(execution.estimate_fee().await.unwrap());
    let nonce = Felt::TWO; // after declare+deploy
    let invoke_request = execution
        .l1_data_gas(fee.l1_data_gas)
        .l1_gas(fee.l1_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .l1_gas_price(fee.l1_gas_price)
        .l2_gas(fee.l2_gas)
        .l2_gas_price(fee.l2_gas_price)
        .nonce(nonce)
        .prepared()
        .unwrap();

    let signature = signer.sign_hash(&invoke_request.transaction_hash(false)).await.unwrap();
    let invoke_simulation_body = json!({
        "block_id": "latest",
        "simulation_flags": [],
        "transactions": [
            {
                "type": "INVOKE",
                "version": "0x3",
                "signature": [signature.r, signature.s],
                "nonce": nonce,
                "calldata": account.encode_calls(&calls),
                "sender_address": account.address(),
                "resource_bounds": ResourceBoundsMapping::from(fee),
                "tip":"0x0",
                "paymaster_data":[],
                "account_deployment_data":[],
                "nonce_data_availability_mode":"L1",
                "fee_data_availability_mode":"L1"
            }
        ]
    });

    devnet.send_custom_rpc("starknet_simulateTransactions", invoke_simulation_body).await.unwrap();
}

#[tokio::test]
async fn test_simulation_of_panicking_invoke() {
    let devnet = BackgroundDevnet::spawn().await.unwrap();

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        starknet_rs_accounts::ExecutionEncoding::New,
    );

    let (contract_class, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (_, contract_address) =
        declare_v3_deploy_v3(&account, contract_class, casm_hash, &[]).await.unwrap();

    let top_selector = get_selector_from_name("create_panic").unwrap();
    let panic_message_text = "funny_text";
    let panic_message = cairo_short_string_to_felt(panic_message_text).unwrap();

    let calls =
        vec![Call { to: contract_address, selector: top_selector, calldata: vec![panic_message] }];
    let block = devnet
        .json_rpc_client
        .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
        .await
        .unwrap();

    let gas_price = match block {
        starknet_rs_core::types::MaybePreConfirmedBlockWithTxHashes::Block(latest) => {
            latest.l2_gas_price.price_in_fri
        }
        MaybePreConfirmedBlockWithTxHashes::PreConfirmedBlock(pre_confirmed) => {
            pre_confirmed.l2_gas_price.price_in_fri
        }
    };
    let gas_price = u128::from_le_bytes(gas_price.to_bytes_le()[..16].try_into().unwrap());
    let nonce = Felt::TWO; // after declare + deploy
    let simulation = account
        .execute_v3(calls)
        .nonce(nonce)
        .l1_data_gas_price(gas_price)
        .l1_gas_price(gas_price)
        .l2_gas_price(gas_price)
        .l1_gas(0)
        .l1_data_gas(1e3 as u64)
        .l2_gas(1e8 as u64)
        .simulate(false, false)
        .await
        .unwrap();

    match simulation.transaction_trace {
        TransactionTrace::Invoke(InvokeTransactionTrace {
            execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
            ..
        }) => {
            assert_contains(&reverted_invocation.revert_reason, panic_message_text);
        }
        other_trace => panic!("Unexpected trace {other_trace:?}"),
    }
}

#[tokio::test]
async fn simulate_of_multiple_txs_shouldnt_return_an_error_if_invoke_transaction_reverts() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::New,
    );

    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    let estimate_fee_resource_bounds = ResourceBoundsMapping {
        l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
    };

    // call non existent method in UDC
    let calls = vec![Call {
        to: UDC_LEGACY_CONTRACT_ADDRESS,
        selector: get_selector_from_name("no_such_method").unwrap(),
        calldata: vec![
            class_hash,
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,
            Felt::ZERO,
        ],
    }];

    let calldata = account.encode_calls(&calls);

    let is_query = true;
    let nonce_data_availability_mode = DataAvailabilityMode::L1;
    let fee_data_availability_mode = DataAvailabilityMode::L1;

    let simulation_result = devnet
        .json_rpc_client
        .simulate_transactions(
            account.block_id(),
            [
                BroadcastedTransaction::Declare(BroadcastedDeclareTransactionV3 {
                    sender_address: account_address,
                    compiled_class_hash: casm_hash,
                    signature: vec![],
                    nonce: Felt::ZERO,
                    contract_class: Arc::new(flattened_contract_artifact.clone()),
                    resource_bounds: estimate_fee_resource_bounds.clone(),
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
                    sender_address: account_address,
                    calldata,
                    signature: vec![],
                    nonce: Felt::ONE,
                    resource_bounds: estimate_fee_resource_bounds,
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
            ],
            [SimulationFlag::SkipValidate, SimulationFlag::SkipFeeCharge],
        )
        .await
        .unwrap();

    match &simulation_result[1].transaction_trace {
        TransactionTrace::Invoke(InvokeTransactionTrace {
            execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
            ..
        }) => assert_contains(&reverted_invocation.revert_reason, "ENTRYPOINT_NOT_FOUND"),
        other_trace => panic!("Unexpected trace {:?}", other_trace),
    }
}

#[tokio::test]
async fn simulate_of_multiple_txs_should_return_index_of_first_failing_transaction() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start devnet");

    // get account
    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer.clone(),
        account_address,
        devnet.json_rpc_client.chain_id().await.unwrap(),
        ExecutionEncoding::New,
    );

    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let (flattened_contract_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
    let class_hash = flattened_contract_artifact.class_hash();

    let estimate_fee_resource_bounds = ResourceBoundsMapping {
        l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
    };

    // call non existent method in UDC
    let calls = vec![Call {
        to: UDC_LEGACY_CONTRACT_ADDRESS,
        selector: get_selector_from_name("no_such_method").unwrap(),
        calldata: vec![
            class_hash,
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,
            Felt::ZERO,
        ],
    }];

    let calldata = account.encode_calls(&calls);

    let is_query = true;
    let nonce_data_availability_mode = DataAvailabilityMode::L1;
    let fee_data_availability_mode = DataAvailabilityMode::L1;

    let simulation_err = devnet
        .json_rpc_client
        .simulate_transactions(
            account.block_id(),
            [
                BroadcastedTransaction::Declare(BroadcastedDeclareTransactionV3 {
                    sender_address: account_address,
                    compiled_class_hash: casm_hash,
                    signature: vec![],
                    nonce: Felt::ZERO,
                    contract_class: Arc::new(flattened_contract_artifact.clone()),
                    resource_bounds: estimate_fee_resource_bounds.clone(),
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransactionV3 {
                    sender_address: account_address,
                    calldata,
                    signature: vec![],
                    nonce: Felt::ONE,
                    resource_bounds: estimate_fee_resource_bounds,
                    tip: 0,
                    paymaster_data: vec![],
                    account_deployment_data: vec![],
                    nonce_data_availability_mode,
                    fee_data_availability_mode,
                    is_query,
                }),
            ],
            [],
        )
        .await
        .unwrap_err();

    match simulation_err {
        ProviderError::StarknetError(StarknetError::TransactionExecutionError(
            TransactionExecutionErrorData { transaction_index, .. },
        )) => {
            assert_eq!(transaction_index, 0);
        }
        other_error => panic!("Unexpected error {:?}", other_error),
    }
}

#[tokio::test]
async fn simulate_with_gas_bounds_exceeding_balance_returns_error_if_charging_not_skipped() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let gas = 1e11 as u64;
    let gas_price = 1e11 as u128;

    let declaration = account
        .declare_v3(Arc::new(sierra_artifact), casm_hash)
        .l1_data_gas(gas)
        .l2_gas(gas)
        .l1_gas(gas)
        .l1_gas_price(gas_price)
        .l2_gas_price(gas_price)
        .l1_data_gas_price(gas_price);

    match declaration.simulate(false, false).await.unwrap_err() {
        AccountError::Provider(ProviderError::StarknetError(
            StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                execution_error,
                ..
            }),
        )) => assert_contains(&format!("{:?}", execution_error), "exceed balance"),
        other => panic!("Unexpected error {other:?}"),
    }

    // should not fail because fee transfer is skipped
    declaration.simulate(false, true).await.unwrap();
}

#[tokio::test]
async fn simulate_v3_with_skip_fee_charge_deploy_account_declare_deploy_via_invoke_to_udc_happy_path()
 {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
        .await
        .expect("Could not start Devnet");

    let new_account_private_key = Felt::from(7777);
    let signer =
        LocalWallet::from_signing_key(SigningKey::from_secret_scalar(new_account_private_key));

    let latest = BlockId::Tag(BlockTag::Latest);

    let public_key = signer.get_public_key().await.unwrap();
    let salt = Felt::from_hex_unchecked("0x123");
    let account_class_hash = Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH);

    let resource_bounds = ResourceBoundsMapping {
        l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
    };

    let nonce_data_availability_mode = DataAvailabilityMode::L1;
    let fee_data_availability_mode = DataAvailabilityMode::L1;
    let is_query = true;
    let chain_id = devnet.json_rpc_client.chain_id().await.unwrap();
    let paymaster_data = vec![];
    let tip = 0;

    let account_factory = OpenZeppelinAccountFactory::new(
        account_class_hash,
        chain_id,
        &signer,
        &devnet.json_rpc_client,
    )
    .await
    .unwrap();

    let nonce = Felt::ZERO;
    let account_deployment = account_factory
        .deploy_v3(salt)
        .nonce(nonce)
        .l1_data_gas(0)
        .l1_gas(0)
        .l2_gas(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .l1_gas_price(0);

    let account_address = account_deployment.address();
    let txn_hash = account_deployment.prepared().unwrap().transaction_hash(is_query);
    let signature = signer.sign_hash(&txn_hash).await.unwrap();

    let deploy_account_transaction = BroadcastedDeployAccountTransactionV3 {
        nonce,
        signature: vec![signature.r, signature.s],
        contract_address_salt: salt,
        constructor_calldata: vec![public_key.scalar()],
        class_hash: account_class_hash,
        resource_bounds: resource_bounds.clone(),
        tip,
        paymaster_data: paymaster_data.clone(),
        nonce_data_availability_mode,
        fee_data_availability_mode,
        is_query,
    };

    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        &signer,
        account_address,
        chain_id,
        ExecutionEncoding::New,
    );

    let (sierra_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let contract_class_hash = sierra_artifact.class_hash();
    let nonce = Felt::ONE;
    let declare_txn_hash = account
        .declare_v3(Arc::new(sierra_artifact.clone()), casm_hash)
        .nonce(nonce)
        .l1_data_gas(0)
        .l1_gas(0)
        .l2_gas(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .l1_gas_price(0)
        .prepared()
        .unwrap()
        .transaction_hash(is_query);

    let declare_signature = signer.sign_hash(&declare_txn_hash).await.unwrap();

    let declare_transaction = BroadcastedDeclareTransactionV3 {
        sender_address: account_address,
        compiled_class_hash: casm_hash,
        signature: vec![declare_signature.r, declare_signature.s],
        nonce,
        contract_class: Arc::new(sierra_artifact),
        resource_bounds: resource_bounds.clone(),
        tip,
        paymaster_data: paymaster_data.clone(),
        account_deployment_data: vec![],
        nonce_data_availability_mode,
        fee_data_availability_mode,
        is_query,
    };

    // call non existent method in UDC
    let calls = vec![Call {
        to: UDC_LEGACY_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            contract_class_hash,
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,
            Felt::ZERO,
        ],
    }];

    let calldata = account.encode_calls(&calls);
    let nonce = Felt::TWO;
    let invoke_transaction_hash = account
        .execute_v3(calls)
        .l1_data_gas(0)
        .l1_gas(0)
        .l2_gas(0)
        .l1_data_gas_price(0)
        .l2_gas_price(0)
        .l1_gas_price(0)
        .nonce(nonce)
        .prepared()
        .unwrap()
        .transaction_hash(is_query);

    let invoke_signature = signer.sign_hash(&invoke_transaction_hash).await.unwrap();

    let invoke_transaction = BroadcastedInvokeTransactionV3 {
        sender_address: account_address,
        calldata,
        signature: vec![invoke_signature.r, invoke_signature.s],
        nonce,
        resource_bounds: resource_bounds.clone(),
        tip,
        paymaster_data: paymaster_data.clone(),
        account_deployment_data: vec![],
        nonce_data_availability_mode,
        fee_data_availability_mode,
        is_query,
    };

    devnet
        .json_rpc_client
        .simulate_transactions(
            latest,
            [
                BroadcastedTransaction::DeployAccount(deploy_account_transaction),
                BroadcastedTransaction::Declare(declare_transaction),
                BroadcastedTransaction::Invoke(invoke_transaction),
            ],
            [SimulationFlag::SkipFeeCharge],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn simulate_invoke_v3_with_fee_just_below_estimated_should_return_a_trace_of_reverted_transaction()
 {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let mut account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );
    account.set_block_id(BlockId::Tag(BlockTag::Latest));

    let declare_result =
        account.declare_v3(Arc::new(sierra_artifact.clone()), casm_hash).send().await.unwrap();

    let salt = Felt::from_hex_unchecked("0x123");
    let execution = account.execute_v3(vec![Call {
        to: UDC_LEGACY_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            declare_result.class_hash,
            salt,
            Felt::ZERO, // is_unique
            Felt::ZERO, // constructor data length
        ],
    }]);
    let fee = execution.estimate_fee().await.unwrap();
    let fee = LocalFee::from(fee);

    let SimulatedTransaction { transaction_trace, .. } = execution
        .l1_gas(fee.l1_gas)
        .l1_gas_price(fee.l1_gas_price)
        .l2_gas(fee.l2_gas - 1)
        .l2_gas_price(fee.l2_gas_price)
        .l1_data_gas(fee.l1_data_gas)
        .l1_data_gas_price(fee.l1_data_gas_price)
        .simulate(false, false)
        .await
        .unwrap();

    match transaction_trace {
        TransactionTrace::Invoke(InvokeTransactionTrace {
            execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
            ..
        }) => assert_contains(&reverted_invocation.revert_reason, "Insufficient"),
        other => panic!("Unexpected trace {other:?}"),
    }
}

#[tokio::test]
async fn simulate_invoke_declare_deploy_account_with_either_gas_or_gas_price_set_to_zero_or_both_will_revert_if_skip_fee_charge_is_not_set()
 {
    let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
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

    let call = Call {
        to: ETH_ERC20_CONTRACT_ADDRESS,
        selector: get_selector_from_name("transfer").unwrap(),
        calldata: vec![
            Felt::ONE,         // recipient
            Felt::from(1_000), // low part of uint256
            Felt::ZERO,        // high part of uint256
        ],
    };

    let calldata = account.encode_calls(&[call]);

    let new_account_private_key = Felt::from(7777);
    let signer =
        LocalWallet::from_signing_key(SigningKey::from_secret_scalar(new_account_private_key));
    let public_key = signer.get_public_key().await.unwrap().scalar();

    let (sierra_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let account_class_hash = Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH);

    let nonce_data_availability_mode = DataAvailabilityMode::L1;
    let fee_data_availability_mode = DataAvailabilityMode::L1;
    let tip = 0;

    let nonce = Felt::ZERO;
    let sierra_artifact = Arc::new(sierra_artifact);
    let block_id = BlockId::Tag(BlockTag::Latest);
    let is_query = true;

    for (gas_units, gas_price) in [(0, 0), (0, 1e18 as u128), (1e18 as u64, 0)] {
        let resource_bounds = ResourceBoundsMapping {
            l1_gas: ResourceBounds { max_amount: gas_units, max_price_per_unit: gas_price },
            l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
            l1_data_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        };

        let invoke_transaction = BroadcastedInvokeTransactionV3 {
            sender_address: account_address,
            calldata: calldata.clone(),
            signature: vec![],
            nonce,
            resource_bounds: resource_bounds.clone(),
            tip,
            paymaster_data: vec![],
            account_deployment_data: vec![],
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        let invoke_transaction = BroadcastedTransaction::Invoke(invoke_transaction);

        let deploy_account_transaction = BroadcastedDeployAccountTransactionV3 {
            signature: vec![],
            nonce,
            contract_address_salt: Felt::ZERO,
            constructor_calldata: vec![public_key],
            class_hash: account_class_hash,
            resource_bounds: resource_bounds.clone(),
            tip,
            paymaster_data: vec![],
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        let deploy_account_transaction =
            BroadcastedTransaction::DeployAccount(deploy_account_transaction);

        let declare_transaction = BroadcastedDeclareTransactionV3 {
            sender_address: account_address,
            compiled_class_hash: casm_hash,
            signature: vec![],
            nonce,
            contract_class: sierra_artifact.clone(),
            resource_bounds,
            tip,
            paymaster_data: vec![],
            account_deployment_data: vec![],
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        let declare_transaction = BroadcastedTransaction::Declare(declare_transaction);

        for transaction in [deploy_account_transaction, declare_transaction, invoke_transaction] {
            let simulation_error = devnet
                .json_rpc_client
                .simulate_transaction(block_id, &transaction, [SimulationFlag::SkipValidate])
                .await
                .unwrap_err();

            match simulation_error {
                ProviderError::StarknetError(StarknetError::TransactionExecutionError(
                    TransactionExecutionErrorData {
                        execution_error: ContractExecutionError::Message(msg),
                        ..
                    },
                )) => {
                    assert_eq!(
                        &msg,
                        "The transaction's resources don't cover validation or the minimal \
                         transaction fee."
                    );
                }
                other => panic!("Unexpected error: {:?}", other),
            }

            devnet
                .json_rpc_client
                .simulate_transaction(
                    block_id,
                    &transaction,
                    [SimulationFlag::SkipValidate, SimulationFlag::SkipFeeCharge],
                )
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn simulate_invoke_v3_with_failing_execution_should_return_a_trace_of_reverted_transaction() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    let (sierra_artifact, casm_hash) =
        get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

    let (signer, account_address) = devnet.get_first_predeployed_account().await;

    let account = SingleOwnerAccount::new(
        &devnet.json_rpc_client,
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    );

    let (_, contract_address) =
        declare_v3_deploy_v3(&account, sierra_artifact, casm_hash, &[]).await.unwrap();

    let panic_reason = "custom little reason";

    let SimulatedTransaction { transaction_trace, .. } = account
        .execute_v3(vec![Call {
            to: contract_address,
            selector: get_selector_from_name("create_panic").unwrap(),
            calldata: vec![cairo_short_string_to_felt(panic_reason).unwrap()],
        }])
        .simulate(false, true)
        .await
        .unwrap();

    match transaction_trace {
        TransactionTrace::Invoke(InvokeTransactionTrace {
            execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
            ..
        }) => assert_contains(&reverted_invocation.revert_reason, panic_reason),
        other => panic!("Unexpected trace {other:?}"),
    }
}
