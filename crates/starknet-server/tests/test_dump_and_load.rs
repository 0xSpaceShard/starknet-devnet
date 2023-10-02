pub mod common;

// Important! Use unique file names for dump files, tests can be run in parallel.
mod dump_and_load_tests {
    use std::process::Command;

    use starknet_rs_providers::Provider;

    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::remove_file;

    static DUMMY_ADDRESS: u128 = 1;
    static DUMMY_AMOUNT: u128 = 1;

    use std::sync::Arc;

    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement};

    use crate::common::utils::{
        get_events_contract_in_sierra_and_compiled_class_hash, get_predeployed_account_props,
    };

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
        let mint_tx_hash_1 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;
        let mint_tx_hash_2 = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        // load transaction from file and check transaction hash
        let devnet_load = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name].to_vec(),
        ))
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

        remove_file(dump_file_name);
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
        let mint_tx_hash = devnet_dump.mint(DUMMY_ADDRESS, DUMMY_AMOUNT).await;

        #[cfg(windows)]
        {
            // To send SIGINT signal on windows, windows-kill is needed
            let mut kill = Command::new("windows-kill")
                .args(["-SIGINT", &devnet_dump.process.id().to_string()])
                .spawn()
                .unwrap();
            kill.wait().unwrap();
        }

        #[cfg(unix)]
        {
            let mut kill = Command::new("kill")
                .args(["-s", "SIGINT", &devnet_dump.process.id().to_string()])
                .spawn()
                .unwrap();
            kill.wait().unwrap();
        }

        // load transaction from file and check transaction hash
        let devnet_load = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name].to_vec(),
        ))
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

        remove_file(dump_file_name);
    }

    #[tokio::test]
    async fn declare_deploy() {
        let dump_file_name = "dump_declare_deploy";
        let devnet = BackgroundDevnet::spawn_with_additional_args(Some(
            ["--dump-path", dump_file_name, "--dump-on", "transaction"].to_vec(),
        ))
        .await
        .expect("Could not start Devnet");

        // constructs starknet-rs account
        let (signer, account_address) = get_predeployed_account_props();
        let mut predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
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

        remove_file(dump_file_name);
    }
}
