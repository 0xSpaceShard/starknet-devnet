mod trace_tests {
    use std::sync::Arc;

    use starknet_core::constants::{
        CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, CHARGEABLE_ACCOUNT_ADDRESS,
        ETH_ERC20_CONTRACT_ADDRESS,
    };
    use starknet_rs_accounts::{
        Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockTag, DeployedContractItem, ExecuteInvocation, Felt, InvokeTransactionTrace,
        StarknetError, TransactionTrace,
    };
    use starknet_rs_core::utils::{get_udc_deployed_address, UdcUniqueness};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        get_deployable_account_signer, get_events_contract_in_sierra_and_compiled_class_hash,
    };

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    fn assert_mint_invocation(trace: TransactionTrace) {
        match trace {
            TransactionTrace::Invoke(InvokeTransactionTrace {
                validate_invocation,
                execute_invocation: ExecuteInvocation::Success(execute_invocation),
                fee_transfer_invocation,
                ..
            }) => {
                for invocation in [validate_invocation.unwrap(), execute_invocation] {
                    assert_eq!(
                        invocation.caller_address,
                        Felt::from_hex_unchecked(CHARGEABLE_ACCOUNT_ADDRESS)
                    );
                    assert_eq!(invocation.calldata[6], Felt::from(DUMMY_ADDRESS));
                    assert_eq!(invocation.calldata[7], Felt::from(DUMMY_AMOUNT));
                }

                assert_eq!(
                    fee_transfer_invocation.unwrap().caller_address,
                    ETH_ERC20_CONTRACT_ADDRESS
                );
            }
            other => panic!("Invalid trace: {other:?}"),
        };
    }

    async fn get_invoke_trace(devnet: &BackgroundDevnet) {
        let mint_tx_hash = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        devnet.create_block().await.unwrap();

        let mint_tx_trace = devnet.json_rpc_client.trace_transaction(mint_tx_hash).await.unwrap();
        assert_mint_invocation(mint_tx_trace);
    }

    #[tokio::test]
    async fn get_trace_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        match devnet.json_rpc_client.trace_transaction(Felt::ZERO).await {
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => (),
            other => panic!("Should fail with error; got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_invoke_trace_normal_mode() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        get_invoke_trace(&devnet).await
    }

    #[tokio::test]
    async fn get_invoke_trace_block_generation_on_demand() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--block-generation-on", "demand"])
                .await
                .expect("Could not start Devnet");

        get_invoke_trace(&devnet).await
    }

    #[tokio::test]
    async fn get_declare_trace() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        let declare_tx_trace = devnet
            .json_rpc_client
            .trace_transaction(declaration_result.transaction_hash)
            .await
            .unwrap();

        if let TransactionTrace::Declare(declare_trace) = declare_tx_trace {
            let validate_invocation = declare_trace.validate_invocation.unwrap();

            assert_eq!(validate_invocation.contract_address, account_address);
            assert_eq!(
                validate_invocation.class_hash,
                Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)
            );
            assert_eq!(
                validate_invocation.calldata[0],
                Felt::from_hex_unchecked(
                    "0x113bf26d112a164297e04381212c9bd7409f07591f0a04f539bdf56693eaaf3"
                )
            );

            assert_eq!(
                declare_trace.fee_transfer_invocation.unwrap().contract_address,
                ETH_ERC20_CONTRACT_ADDRESS
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
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = account
            .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy twice - should result in only 1 instance in deployed_contracts and no declares
        let contract_factory = ContractFactory::new(declaration_result.class_hash, account.clone());
        for salt in (0_u32..2).map(Felt::from) {
            let ctor_data = vec![];
            let deployment_tx = contract_factory
                .deploy_v1(ctor_data.clone(), salt, false)
                .max_fee(Felt::from(1e18 as u128))
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
            Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH),
            constants::CHAIN_ID,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        // deploy account
        let deployment = account_factory
            .deploy_v1(Felt::from_hex_unchecked("0x123"))
            .max_fee(Felt::from(1e18 as u128))
            .nonce(Felt::ZERO)
            .prepared()
            .unwrap();
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, 1e18 as u128).await;
        deployment.send().await.unwrap();

        let deployment_hash = deployment.transaction_hash(false);
        let deployment_trace =
            devnet.json_rpc_client.trace_transaction(deployment_hash).await.unwrap();

        if let starknet_rs_core::types::TransactionTrace::DeployAccount(deployment_trace) =
            deployment_trace
        {
            let validate_invocation = deployment_trace.validate_invocation.unwrap();
            assert_eq!(
                validate_invocation.class_hash,
                Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)
            );
            assert_eq!(
                validate_invocation.calldata[0],
                Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)
            );

            assert_eq!(
                deployment_trace.constructor_invocation.class_hash,
                Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH)
            );

            assert_eq!(
                deployment_trace.fee_transfer_invocation.unwrap().contract_address,
                ETH_ERC20_CONTRACT_ADDRESS
            );
        } else {
            panic!("Could not unpack the transaction trace from {deployment_trace:?}");
        }
    }

    #[tokio::test]
    async fn get_traces_from_block() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let mint_tx_hash: Felt = devnet.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        let traces = devnet
            .json_rpc_client
            .trace_block_transactions(BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(traces.len(), 1);
        let trace = traces[0];
        assert_eq!(trace.transaction_hash, mint_tx_hash);

        assert_mint_invocation(trace.trace_root);
    }
}
