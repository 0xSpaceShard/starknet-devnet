pub mod common;

mod get_class_hash_at_integration_tests {
    use std::sync::Arc;

    use starknet_core::constants::CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH;
    use starknet_core::utils::exported_test_utils::{replaceable_class, replacing_class};
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BlockId, BlockTag, FieldElement, FunctionCall, StarknetError, TransactionExecutionStatus,
        TransactionStatus,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_rs_signers::Signer;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{CHAIN_ID, PREDEPLOYED_ACCOUNT_ADDRESS};

    async fn assert_hash_at_address(
        devnet: &BackgroundDevnet,
        expected_hash: FieldElement,
        address: FieldElement,
    ) {
        let retrieved_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), address)
            .await
            .unwrap();

        assert_eq!(retrieved_hash, expected_hash);
    }

    #[tokio::test]
    async fn get_class_hash_at_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let expected_hash =
            FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap();
        assert_hash_at_address(&devnet, expected_hash, contract_address).await;
    }

    #[tokio::test]
    async fn get_class_hash_at_for_undeployed_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let undeployed_address = "0x1234";
        let contract_address = FieldElement::from_hex_be(undeployed_address).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_number() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Number(0), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_hash() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(
                BlockId::Hash(FieldElement::from_hex_be("0x1").unwrap()),
                contract_address,
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    pub async fn deploy_replaceable_contract<P: Provider + Send + Sync, S: Signer + Send + Sync>(
        account: &SingleOwnerAccount<P, S>,
    ) -> (FieldElement, FieldElement) {
        // declare
        let contract_class = Arc::new(serde_json::from_value(replaceable_class().inner).unwrap());
        let declaration_result = account
            .declare_legacy(contract_class)
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        // deploy
        let salt = FieldElement::ZERO;
        let ctor_calldata = vec![];
        let contract_factory = ContractFactory::new(declaration_result.class_hash, account);
        contract_factory
            .deploy(ctor_calldata.clone(), salt, false)
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();

        let deployment_address = get_udc_deployed_address(
            salt,
            declaration_result.class_hash,
            &starknet_rs_core::utils::UdcUniqueness::NotUnique,
            &ctor_calldata,
        );
        (deployment_address, declaration_result.class_hash)
    }

    pub async fn declare_replacing_contract<P: Provider + Send + Sync, S: Signer + Send + Sync>(
        account: &SingleOwnerAccount<P, S>,
    ) -> FieldElement {
        let contract_class = Arc::new(serde_json::from_value(replacing_class().inner).unwrap());
        let declaration_result = account
            .declare_legacy(contract_class)
            .max_fee(FieldElement::from(100000000000000000000u128))
            .send()
            .await
            .unwrap();
        declaration_result.class_hash
    }

    #[tokio::test]
    async fn test_class_replacement() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (replaceable_address, replaceable_hash) = deploy_replaceable_contract(&account).await;
        println!("DEBUG test: address {replaceable_address:#x} holding {replaceable_hash:#x}");

        // assert current class is `replaceable` by checking class hash and call result
        assert_hash_at_address(&devnet, replaceable_hash, replaceable_address).await;
        let call = FunctionCall {
            contract_address: replaceable_address,
            entry_point_selector: get_selector_from_name("foo").unwrap(),
            calldata: vec![],
        };
        let initial_call_result = devnet
            .json_rpc_client
            .call(call.clone(), BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        assert_eq!(initial_call_result, [FieldElement::from(42_u32)]);

        // replace class
        let replacing_hash = declare_replacing_contract(&account).await;
        println!("DEBUG test: replacing_hash: {replacing_hash:#x}");
        let invoke_calls = vec![Call {
            to: replaceable_address,
            selector: get_selector_from_name("replace").unwrap(),
            calldata: vec![replacing_hash],
        }];
        let replacement_tx = account
            .execute(invoke_calls)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();
        match devnet.json_rpc_client.get_transaction_status(replacement_tx.transaction_hash).await {
            Ok(TransactionStatus::AcceptedOnL2(TransactionExecutionStatus::Succeeded)) => (),
            other => panic!("Invalid response: {other:?}"),
        };

        // TODO assert diff content, currently getting empty array
        // let update =
        //     devnet.json_rpc_client.get_state_update(BlockId::Tag(BlockTag::Latest)).await.
        // unwrap(); match update {
        //     MaybePendingStateUpdate::Update(StateUpdate { state_diff, .. }) => {
        //         assert_eq!(
        //             state_diff.replaced_classes,
        //             [ReplacedClassItem {
        //                 contract_address: replaceable_address,
        //                 class_hash: replacing_hash
        //             }]
        //         )
        //     }
        //     other => panic!("Invalid: {other:?}"),
        // }

        // assert current class is `replacing` by checking class hash and call result
        assert_hash_at_address(&devnet, replacing_hash, replaceable_address).await;
        let initial_call_result =
            devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
        assert_eq!(initial_call_result, [FieldElement::from(43_u32)]);
    }
}
