pub mod common;

mod dump_and_load_tests {
    use std::path::Path;

    use hyper::Body;
    use serde_json::json;
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{send_ctrl_c_signal, UniqueAutoDeletableFile};

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    use std::sync::Arc;

    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::FieldElement;

    use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

    #[tokio::test]
    async fn dump_wrong_cli_parameters_no_path() {
        let devnet_dump =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-on", "exit"]).await;
        assert!(devnet_dump.is_err());
    }

    #[tokio::test]
    async fn dump_wrong_cli_parameters_path() {
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            "///",
            "--dump-on",
            "transaction",
        ])
        .await;

        assert!(devnet_dump.is_err());
    }

    #[tokio::test]
    async fn dump_wrong_cli_parameters_mode() {
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            "dump_wrong_cli_mode",
            "--dump-on",
            "e",
        ])
        .await;

        assert!(devnet_dump.is_err());
    }

    #[tokio::test]
    async fn mint_dump_on_transaction_and_load() {
        // dump after transaction
        let dump_file = UniqueAutoDeletableFile::new("dump_on_transaction");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "transaction",
        ])
        .await
        .expect("Could not start Devnet");
        let mint_tx_hash_1 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_tx_hash_2 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        // load transaction from file and check transaction hash
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");
        let loaded_transaction_1 =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_1).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction_1
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash_1);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction_1:?}");
        }

        let loaded_transaction_2 =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash_2).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction_2
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash_2);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction_2:?}");
        }
    }

    #[tokio::test]
    async fn mint_dump_on_exit_and_load() {
        // dump on exit
        let dump_file = UniqueAutoDeletableFile::new("dump_on_exit");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            dump_file.path.as_str(),
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");
        let devnet_dump_pid = devnet_dump.process.id();
        let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        send_ctrl_c_signal(&devnet_dump.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1));

        // load transaction from file and check transaction hash
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");
        let devnet_load_pid = devnet_load.process.id();
        assert_ne!(devnet_dump_pid, devnet_load_pid); // if PID's are different SIGINT signal worked
        let loaded_transaction =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
    }

    #[tokio::test]
    async fn declare_deploy() {
        let dump_file = UniqueAutoDeletableFile::new("dump_declare_deploy");
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "transaction",
        ])
        .await
        .expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
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

        let predeployed_account = Arc::new(predeployed_account);

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());
        let deploy_result = contract_factory
            .deploy(vec![], FieldElement::ZERO, false)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // load transaction from file and check transactions hashes
        let devnet_load =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");

        // check declare transaction
        let loaded_declare_v2 = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(declaration_result.transaction_hash)
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Declare(
            starknet_rs_core::types::DeclareTransaction::V2(declare_v2),
        ) = loaded_declare_v2
        {
            assert_eq!(declare_v2.transaction_hash, declaration_result.transaction_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_declare_v2:?}");
        }

        // check deploy transaction
        let loaded_deploy_v2 = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(deploy_result.transaction_hash)
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(deploy_v2),
        ) = loaded_deploy_v2
        {
            assert_eq!(deploy_v2.transaction_hash, deploy_result.transaction_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_deploy_v2:?}");
        }
    }

    #[tokio::test]
    async fn dump_without_transaction() {
        // dump on exit
        let dump_file = UniqueAutoDeletableFile::new("dump_without_transaction");
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            &dump_file.path,
            "--dump-on",
            "exit",
        ])
        .await
        .expect("Could not start Devnet");

        send_ctrl_c_signal(&devnet_dump.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1));

        // file should not be created if there are no transactions
        if Path::new(&dump_file.path).exists() {
            panic!(
                "Could find the dump file but there were no transactions to dump {}",
                &dump_file.path
            );
        }
    }

    #[tokio::test]
    async fn dump_endpoint_fail_with_wrong_request() {
        let devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let dump_body = Body::from(
            json!({
                "test": ""
            })
            .to_string(),
        );
        let result = devnet_dump.post_json("/dump".into(), dump_body).await.unwrap();
        assert_eq!(result.status(), 400);
    }

    #[tokio::test]
    async fn dump_endpoint_fail_with_wrong_file_name() {
        let devnet_dump = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let dump_body = Body::from(
            json!({
                "path": "///"
            })
            .to_string(),
        );
        let result = devnet_dump.post_json("/dump".into(), dump_body).await.unwrap();
        assert_eq!(result.status(), 400);
    }

    #[tokio::test]
    async fn load_endpoint_fail_with_wrong_request() {
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let load_body = Body::from(
            json!({
                "test": ""
            })
            .to_string(),
        );
        let result = devnet_load.post_json("/load".into(), load_body).await.unwrap();
        assert_eq!(result.status(), 422);
    }

    #[tokio::test]
    async fn load_endpoint_fail_with_wrong_path() {
        let load_file_name = "load_file_name";
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let load_body = Body::from(json!({ "path": load_file_name }).to_string());
        let result = devnet_load.post_json("/load".into(), load_body).await.unwrap();
        assert_eq!(result.status(), 400);
    }

    #[tokio::test]
    async fn dump_load_endpoints_transaction_and_state_after_load_is_valid() {
        // check if the dump with the default path "dump_endpoint" works as expected when json body
        // is empty, later check if the dump with the custom path "dump_endpoint_custom_path"
        // works
        let dump_file = UniqueAutoDeletableFile::new("dump_endpoint");
        let devnet_dump =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", &dump_file.path])
                .await
                .expect("Could not start Devnet");
        let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let dump_body = Body::from(json!({}).to_string());
        devnet_dump.post_json("/dump".into(), dump_body).await.unwrap();
        assert!(Path::new(&dump_file.path).exists());

        let dump_file_custom = UniqueAutoDeletableFile::new("dump_endpoint_custom_path");
        let dump_body_custom_path =
            Body::from(json!({ "path": dump_file_custom.path }).to_string());
        devnet_dump.post_json("/dump".into(), dump_body_custom_path).await.unwrap();
        assert!(Path::new(&dump_file_custom.path).exists());

        // load and re-execute from "dump_endpoint" file and check if transaction and state of the
        // blockchain is valid
        let devnet_load = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let load_body = Body::from(json!({ "path": dump_file.path }).to_string());
        devnet_load.post_json("/load".into(), load_body).await.unwrap();

        let balance_result =
            devnet_load.get_balance(&FieldElement::from(DUMMY_ADDRESS)).await.unwrap();
        assert_eq!(balance_result, DUMMY_AMOUNT.into());

        let loaded_transaction =
            devnet_load.json_rpc_client.get_transaction_by_hash(mint_tx_hash).await.unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(invoke_v1.transaction_hash, mint_tx_hash);
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
    }
}
