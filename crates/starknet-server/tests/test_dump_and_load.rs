pub mod common;

mod dump_and_load_tests {
    use std::fs::{File, self};
    use std::path::Path;
    use std::process::Command;

    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_providers::Provider;
    use tokio::signal;
    use tokio::signal::unix::Signal;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    static DUMMY_ADDRESS: &str = "0x1";
    static DUMMY_AMOUNT: u128 = 1;

    #[tokio::test]
    async fn mint_dump_on_transaction_and_load() {
        // dump after transaction 
        let dump_file_name = "dump";
        let devnet_dump = BackgroundDevnet::spawn(Some(["--dump-path".to_string(), dump_file_name.to_string(), "--dump-on".to_string(), "transaction".to_string()].to_vec())).await.expect("Could not start Devnet");
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
        let tx_hash_value = resp_body["tx_hash"].take();

        // load transaction from file and check hashes
        let devnet_load = BackgroundDevnet::spawn(Some(["--dump-path".to_string(), dump_file_name.to_string()].to_vec())).await.expect("Could not start Devnet");
        let loaded_transaction = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(FieldElement::from_hex_be(tx_hash_value.as_str().unwrap()).unwrap())
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(tx_hash_value.as_str().unwrap()).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
        
        let file_path = Path::new(dump_file_name);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Could not remove file");
        }
    }

    #[tokio::test]
    async fn mint_dump_on_exit_and_load() {
        // dump after transaction 
        let dump_file_name = "dump";
        let devnet_dump = BackgroundDevnet::spawn(Some(["--dump-path".to_string(), dump_file_name.to_string(), "--dump-on".to_string(), "exit".to_string()].to_vec())).await.expect("Could not start Devnet");
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
        let tx_hash_value = resp_body["tx_hash"].take();
        // let kill = Command::new("kill")
        //     .args(["-s", "TERM", &devnet_dump.process.id().to_string()])
        //     .spawn();
        // kill.expect("kill").wait();
        // signal::kill(Pid::from_raw(devnet_dump.process.id()), Signal::SIGTERM).unwrap();
        // devnet_dump.process.kill().expect("Failed to kill child process");

        // load transaction from file and check hashes
        let devnet_load = BackgroundDevnet::spawn(Some(["--dump-path".to_string(), dump_file_name.to_string()].to_vec())).await.expect("Could not start Devnet");
        let loaded_transaction = devnet_load
            .json_rpc_client
            .get_transaction_by_hash(FieldElement::from_hex_be(tx_hash_value.as_str().unwrap()).unwrap())
            .await
            .unwrap();
        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = loaded_transaction
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(tx_hash_value.as_str().unwrap()).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {loaded_transaction:?}");
        }
        
        let file_path = Path::new(dump_file_name);
        if file_path.exists() {
            fs::remove_file(file_path).expect("Could not remove file");
        }
    }
}


