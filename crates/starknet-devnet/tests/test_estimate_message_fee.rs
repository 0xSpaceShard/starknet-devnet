pub mod common;

mod test_estimate_message_fee {

    use std::sync::Arc;

    use starknet_core::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, EthAddress, Felt, MsgFromL1, StarknetError};
    use starknet_rs_core::utils::{get_udc_deployed_address, UdcUniqueness};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CHAIN_ID, L1_HANDLER_SELECTOR, MESSAGING_WHITELISTED_L1_CONTRACT,
    };

    #[tokio::test]
    async fn estimate_message_fee() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        ));

        // get class
        let contract_json = dummy_cairo_l1l2_contract();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        account
            .declare_legacy(contract_artifact.clone())
            .nonce(Felt::ZERO)
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy instance of class
        let contract_factory = ContractFactory::new(class_hash, account.clone());
        let salt = Felt::from_hex("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory
            .deploy_v1(constructor_calldata, salt, false)
            .nonce(Felt::ONE)
            // max fee implicitly estimated
            .send()
            .await.expect("Cannot deploy");

        let res = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: contract_address,
                    entry_point_selector: Felt::from_hex(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();

        assert_eq!(res.gas_consumed, Felt::from(16027u32));
    }

    #[tokio::test]
    async fn estimate_message_fee_contract_not_found() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let err = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: Felt::from_hex("0x1").unwrap(),
                    entry_point_selector: Felt::from_hex(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Error expected");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn estimate_message_fee_block_not_found() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");

        let err = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: Felt::from_hex("0x1").unwrap(),
                    entry_point_selector: Felt::from_hex(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Number(101),
            )
            .await
            .expect_err("Error expected");

        match err {
            ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
