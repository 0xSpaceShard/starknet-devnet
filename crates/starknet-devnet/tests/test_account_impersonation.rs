pub mod common;

mod impersonated_account_tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use serde_json::json;
    use server::test_utils::exported_test_utils::assert_contains;
    use starknet_core::constants::STRK_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_accounts::{
        Account, AccountFactory, AccountFactoryError, Call, ExecutionEncoding,
        OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedInvokeTransaction, ContractClass, ExecutionResult,
        FieldElement, FunctionCall, MaybePendingBlockWithTxHashes, StarknetError,
    };
    use starknet_rs_core::utils::{
        get_selector_from_name, get_storage_var_address, get_udc_deployed_address,
    };
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_rs_signers::Signer;
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use starknet_types::traits::ToDecimalString;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_cairo1_classes_equal, assert_tx_successful, declare_deploy,
        get_block_reader_contract_in_sierra_and_compiled_class_hash, get_json_body,
        get_simple_contract_in_sierra_and_compiled_class_hash, resolve_path,
        send_ctrl_c_signal_and_wait, spawn_forkable_devnet,
    };

    const SEPOLIA_URL: &str = "http://rpc.pathfinder.equilibrium.co/integration-sepolia/rpc/v0_7";
    const SEPOLIA_GENESIS_BLOCK_HASH: &str =
        "0x19f675d3fb226821493a6ab9a1955e384bba80f130de625621a418e9a7c0ca3";

    #[tokio::test]
    async fn test_impersonated_account_of_a_predeployed_account_can_create_transfer() {
        println!("Origin devnet is being spawned");
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let (signer, account_address) = origin_devnet.get_first_predeployed_account().await;

        println!("Forked devnet is being spawned");
        let forked_devnet = origin_devnet.fork().await.unwrap();
        forked_devnet.impersonate_account(account_address).await.unwrap();

        let forked_account_initial_balance =
            forked_devnet.get_balance(&account_address, FeeUnit::FRI).await.unwrap();

        let amount_to_transfer = FieldElement::from_dec_str("100000000000").unwrap();

        let account = SingleOwnerAccount::new(
            &origin_devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let invoke_call = Call {
            to: FieldElement::from_hex_be(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,  // recipient
                amount_to_transfer, // low part of uint256
                FieldElement::ZERO, // high part of uint256
            ],
        };

        let max_fee = account
            .execute(vec![invoke_call.clone()])
            .fee_estimate_multiplier(2.0)
            .estimate_fee()
            .await
            .unwrap()
            .overall_fee;

        let account_nonce = forked_devnet
            .json_rpc_client
            .get_nonce(BlockId::Tag(BlockTag::Latest), account.address())
            .await
            .unwrap();

        let invoke_request = account
            .execute(vec![invoke_call])
            .max_fee(max_fee)
            .nonce(account_nonce)
            .prepared()
            .unwrap()
            .get_invoke_request(false)
            .await
            .unwrap();

        let mut invoke_request_json = serde_json::to_value(invoke_request).unwrap();
        invoke_request_json["signature"] = json!(["0x1"]);

        let broadcasted_invoke_transacton =
            serde_json::from_value::<BroadcastedInvokeTransaction>(invoke_request_json).unwrap();

        let result = forked_devnet
            .json_rpc_client
            .add_invoke_transaction(broadcasted_invoke_transacton)
            .await
            .unwrap();

        let receipt = forked_devnet
            .json_rpc_client
            .get_transaction_receipt(result.transaction_hash)
            .await
            .unwrap();

        assert_eq!(receipt.execution_result(), &ExecutionResult::Succeeded);

        let forked_account_balance =
            forked_devnet.get_balance(&account_address, FeeUnit::FRI).await.unwrap();
        assert!(forked_account_initial_balance >= amount_to_transfer + forked_account_balance);
    }
}
