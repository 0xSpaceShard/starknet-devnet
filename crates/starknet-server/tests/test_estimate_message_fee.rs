pub mod common;

mod test_estimate_message_fee {
    use std::sync::Arc;

    use starknet_core::utils::exported_test_utils::dummy_cairo_l1l2_contract;
    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, EthAddress, FieldElement, MsgFromL1};
    use starknet_rs_core::utils::{get_udc_deployed_address, UdcUniqueness};
    use starknet_rs_providers::Provider;

    use crate::common::constants::CHAIN_ID;
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_predeployed_account_props;

    #[tokio::test]
    async fn estimate_message_fee() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
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
                    from_address: EthAddress::from_hex(
                        "0x8359E4B0152ed5A731162D3c7B0D8D56edB165A0",
                    )
                    .unwrap(),
                    to_address: contract_address,
                    entry_point_selector: FieldElement::from_hex_be(
                        "0xc73f681176fc7b3f9693986fd7b14581e8d540519e27400e88b8713932be01",
                    )
                    .unwrap(),
                    payload: [(1_u32).into(), (10_u32).into()].to_vec(),
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();

        println!("{}", res.gas_consumed);
    }
}
