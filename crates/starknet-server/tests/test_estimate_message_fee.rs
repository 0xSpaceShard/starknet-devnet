pub mod common;

mod test_estimate_message_fee {
    const L1_HANDLER_SELECTOR: &str =
        "0xc73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01";

    use std::sync::Arc;

    use starknet_core::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, EthAddress, FieldElement, MsgFromL1, StarknetError,
    };
    use starknet_rs_core::utils::{get_udc_deployed_address, UdcUniqueness};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{CHAIN_ID, MESSAGING_WHITELISTED_L1_CONTRACT};

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
            ExecutionEncoding::Legacy,
        ));

        // get class
        let contract_json = dummy_cairo_l1l2_contract();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        account
            .declare_legacy(contract_artifact.clone())
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy instance of class
        let contract_factory = ContractFactory::new(class_hash, account.clone());
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory
            .deploy(constructor_calldata, salt, false)
            .nonce(FieldElement::ONE)
            // max fee implicitly estimated
            .send()
            .await.expect("Cannot deploy");

        let res = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: contract_address,
                    entry_point_selector: FieldElement::from_hex_be(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();

        assert_eq!(res.gas_consumed, 18485);
    }

    #[tokio::test]
    async fn estimate_message_fee_contract_not_found() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let err = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: FieldElement::from_hex_be("0x1").unwrap(),
                    entry_point_selector: FieldElement::from_hex_be(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .expect_err("Error expected");

        match err {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn estimate_message_fee_block_not_found() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let err = devnet
            .json_rpc_client
            .estimate_message_fee(
                MsgFromL1 {
                    from_address: EthAddress::from_hex(MESSAGING_WHITELISTED_L1_CONTRACT).unwrap(),
                    to_address: FieldElement::from_hex_be("0x1").unwrap(),
                    entry_point_selector: FieldElement::from_hex_be(L1_HANDLER_SELECTOR).unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Number(101),
            )
            .await
            .expect_err("Error expected");

        match err {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
