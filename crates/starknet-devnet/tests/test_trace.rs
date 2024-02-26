pub mod common;

mod trace_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::constants::{
        CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, CHARGEABLE_ACCOUNT_ADDRESS,
        ETH_ERC20_CONTRACT_ADDRESS,
    };
    use starknet_rs_accounts::{
        Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{
        DeployedContractItem, FieldElement, FunctionInvocation, StarknetError, TransactionTrace,
    };
    use starknet_rs_core::utils::{get_udc_deployed_address, UdcUniqueness};
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::rpc::transactions::BlockTransactionTrace;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_deployable_account_signer, get_events_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    fn assert_mint_invocation(invocation: FunctionInvocation) {
        assert_eq!(
            invocation.contract_address,
            FieldElement::from_hex_be(CHARGEABLE_ACCOUNT_ADDRESS).unwrap()
        );
        assert_eq!(invocation.calldata[6], FieldElement::from(DUMMY_ADDRESS));
        assert_eq!(invocation.calldata[7], FieldElement::from(DUMMY_AMOUNT));
    }

    #[tokio::test]
    async fn get_trace_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let err = devnet
            .json_rpc_client
            .trace_transaction(FieldElement::ZERO)
            .await
            .expect_err("Should fail");

        match err {
            ProviderError::StarknetError(StarknetError::TransactionHashNotFound) => (),
            _ => panic!("Should fail with error"),
        }
    }

    #[tokio::test]
    async fn get_invoke_trace() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        let mint_tx_trace = devnet.json_rpc_client.trace_transaction(mint_tx_hash).await.unwrap();

        if let starknet_rs_core::types::TransactionTrace::Invoke(invoke_trace) = mint_tx_trace {
            assert_mint_invocation(invoke_trace.validate_invocation.unwrap());

            if let starknet_rs_core::types::ExecuteInvocation::Success(execute_invocation) =
                invoke_trace.execute_invocation
            {
                assert_mint_invocation(execute_invocation);
            }

            assert_eq!(
                invoke_trace.fee_transfer_invocation.unwrap().contract_address,
                FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction trace from {mint_tx_trace:?}");
        }
    }

    #[tokio::test]
    async fn get_declare_trace() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        );

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let declare_tx_trace = devnet
            .json_rpc_client
            .trace_transaction(declaration_result.transaction_hash)
            .await
            .unwrap();

        if let starknet_rs_core::types::TransactionTrace::Declare(declare_trace) = declare_tx_trace
        {
            let validate_invocation = declare_trace.validate_invocation.unwrap();

            assert_eq!(validate_invocation.contract_address, account_address);
            assert_eq!(
                validate_invocation.class_hash,
                FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
            );
            assert_eq!(
                validate_invocation.calldata[0],
                FieldElement::from_hex_be(
                    "0x113bf26d112a164297e04381212c9bd7409f07591f0a04f539bdf56693eaaf3"
                )
                .unwrap()
            );

            assert_eq!(
                declare_trace.fee_transfer_invocation.unwrap().contract_address,
                FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction trace from {declare_tx_trace:?}");
        }
    }

    #[tokio::test]
    async fn test_contract_deployment_trace() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::New,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy twice - should result in only 1 instance in deployed_contracts and no declares
        let contract_factory = ContractFactory::new(declaration_result.class_hash, account.clone());
        for salt in (0_u32..2).map(FieldElement::from) {
            let ctor_data = vec![];
            let deployment_tx = contract_factory
                .deploy(ctor_data.clone(), salt, false)
                .max_fee(FieldElement::from(1e18 as u128))
                .send()
                .await
                .expect("Cannot deploy");

            let deployment_address = get_udc_deployed_address(
                salt,
                declaration_result.class_hash,
                &UdcUniqueness::NotUnique,
                &ctor_data,
            );
            let deployment_trace = devnet
                .json_rpc_client
                .trace_transaction(deployment_tx.transaction_hash)
                .await
                .unwrap();

            match deployment_trace {
                TransactionTrace::Invoke(tx) => {
                    let state_diff = tx.state_diff.unwrap();
                    assert_eq!(state_diff.declared_classes, []);
                    assert_eq!(state_diff.deprecated_declared_classes, []);
                    assert_eq!(
                        state_diff.deployed_contracts,
                        [DeployedContractItem {
                            address: deployment_address,
                            class_hash: declaration_result.class_hash
                        }]
                    );
                }
                other => panic!("Invalid trace: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn get_deploy_account_trace() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap(),
            chain_id::TESTNET,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        // deploy account
        let deployment = account_factory
            .deploy(FieldElement::from_hex_be("0x123").unwrap())
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ZERO)
            .prepared()
            .unwrap();
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, 1e18 as u128).await;
        deployment.send().await.unwrap();

        let deploy_account_tx_trace =
            devnet.json_rpc_client.trace_transaction(deployment.transaction_hash()).await.unwrap();

        if let starknet_rs_core::types::TransactionTrace::DeployAccount(deployment_trace) =
            deploy_account_tx_trace
        {
            let validate_invocation = deployment_trace.validate_invocation.unwrap();
            assert_eq!(
                validate_invocation.class_hash,
                FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
            );
            assert_eq!(
                validate_invocation.calldata[0],
                FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
            );

            assert_eq!(
                deployment_trace.constructor_invocation.class_hash,
                FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
            );

            assert_eq!(
                deployment_trace.fee_transfer_invocation.unwrap().contract_address,
                FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction trace from {deploy_account_tx_trace:?}");
        }
    }

    #[tokio::test]
    async fn get_traces_from_block() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let mint_tx_hash: FieldElement = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        // Currently, we support only one transaction per block, so if this changes in the future
        // this test also needs to be updated
        let block_traces = &devnet
            .send_custom_rpc("starknet_traceBlockTransactions", json!({ "block_id": "latest" }))
            .await["result"]["traces"];
        let traces = &block_traces[0];

        // assert if there is only one transaction trace
        assert_eq!(
            serde_json::from_value::<Vec<BlockTransactionTrace>>(block_traces.clone(),)
                .unwrap()
                .len(),
            1
        );

        // assert transaction hash
        assert_eq!(
            mint_tx_hash,
            FieldElement::from_hex_be(traces["transaction_hash"].as_str().unwrap()).unwrap()
        );

        // assert validate invocation
        assert_mint_invocation(
            serde_json::from_value::<FunctionInvocation>(
                traces["trace_root"]["validate_invocation"].clone(),
            )
            .unwrap(),
        );

        // assert execute invocation
        assert_mint_invocation(
            serde_json::from_value::<FunctionInvocation>(
                traces["trace_root"]["execute_invocation"].clone(),
            )
            .unwrap(),
        );

        // assert fee transfer invocation
        assert_eq!(
            FieldElement::from_hex_be(
                traces["trace_root"]["fee_transfer_invocation"]["contract_address"]
                    .as_str()
                    .unwrap()
            )
            .unwrap(),
            FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS).unwrap()
        );
    }
}
