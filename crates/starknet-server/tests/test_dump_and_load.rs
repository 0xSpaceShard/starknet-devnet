pub mod common;

// Important! Use unique file names for dump files, tests can be run in parallel.
mod dump_and_load_tests {
    use std::fs::{self};
    use std::path::Path;
    use std::process::Command;

    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_rs_providers::Provider;
    use starknet_types::felt::Felt;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: &str = "0x1";
    static DUMMY_AMOUNT: u128 = 1;

    use std::sync::Arc;

    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement};
    use starknet_rs_signers::{LocalWallet, SigningKey};

    use crate::common::utils::get_events_contract_in_sierra_and_compiled_class_hash;

    #[tokio::test]
    async fn check_dump_path_with_dump_on() {
        let devnet_dump =
            BackgroundDevnet::spawn_with_additional_args(Some(["--dump-on", "exit"].to_vec()))
                .await;
        assert!(devnet_dump.is_err());
    }

    #[tokio::test]
    async fn mint_dump_on_transaction_and_load() {
        // dump after transaction
        let dump_file_name = "dump_on_transaction";
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name, "--dump-on", "transaction"].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": DUMMY_ADDRESS,
                "amount": DUMMY_AMOUNT
            })
            .to_string(),
        );
        let resp = devnet_dump.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let resp_body = get_json_body(resp).await;
        let tx_hash_value = resp_body["tx_hash"].as_str().unwrap();

        // load transaction from file and check transaction hash
        let devnet_load = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");
        let loaded_transaction = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(
                FieldElement::from_hex_be(tx_hash_value).unwrap(),
            )
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(tx_hash_value).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }

        // remove dump file after test
        let file_path = Path::new(dump_file_name);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Could not remove file");
        }
    }

    #[tokio::test]
    async fn mint_dump_on_exit_and_load() {
        // dump after transaction
        let dump_file_name = "dump_on_exit";
        let devnet_dump = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name, "--dump-on", "exit"].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");
        let devnet_dump_pid = devnet_dump.process.id();
        let req_body = Body::from(
            json!({
                "address": DUMMY_ADDRESS,
                "amount": DUMMY_AMOUNT
            })
            .to_string(),
        );
        let resp = devnet_dump.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        let mut resp_body = get_json_body(resp).await;
        let tx_hash_value = resp_body["tx_hash"].as_str().unwrap();

        // Although dump and load is a multiplatform feature and works on all systems is troublesome
        // on the test level. This test is recommended to run on Linux. There are some problems with
        // the termination process on Windows from the test level. It's required to send a signal
        // twice on MacOS.
        for _i in 0..2 {
            let mut kill = Command::new("kill")
                .args(["-s", "SIGINT", &devnet_dump.process.id().to_string()])
                .spawn()
                .unwrap();
            let _result = kill.wait().unwrap();
        }

        // load transaction from file and check transaction hash
        let devnet_load = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");
        let devnet_load_pid = devnet_load.process.id();
        assert_ne!(devnet_dump_pid, devnet_load_pid); // if PID's are different SIGINT signal worked
        let loaded_transaction = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(
                FieldElement::from_hex_be(tx_hash_value).unwrap(),
            )
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(tx_hash_value).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }

        // remove dump file after test
        let file_path = Path::new(dump_file_name);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Could not remove file");
        }
    }

    #[tokio::test]
    async fn declare_deploy() {
        let dump_file_name = "dump_declare_deploy";
        let devnet = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name, "--dump-on", "transaction"].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");

        // get first predeployed account data
        let predeployed_accounts_response =
            devnet.get("/predeployed_accounts", None).await.unwrap();

        let predeployed_accounts_json = get_json_body(predeployed_accounts_response).await;
        let first_account = predeployed_accounts_json.as_array().unwrap().get(0).unwrap();

        let account_address =
            Felt::from_prefixed_hex_str(first_account["address"].as_str().unwrap()).unwrap();
        let private_key =
            Felt::from_prefixed_hex_str(first_account["private_key"].as_str().unwrap()).unwrap();

        // constructs starknet-rs account
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key.into()));
        let address = FieldElement::from(account_address);

        let mut predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        );

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

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
        let devnet_load = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name].to_vec(),
        ))
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

        // remove dump file after test
        let file_path = Path::new(dump_file_name);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Could not remove file");
        }
    }
}
