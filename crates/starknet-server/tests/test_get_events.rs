pub mod common;

mod get_events_integration_tests {
    use std::sync::Arc;

    use hyper::{Body, StatusCode};
    use serde_json::json;
    use starknet_in_rust::core::contract_address::compute_casm_class_hash;
    use starknet_in_rust::core::transaction_hash::calculate_deploy_account_transaction_hash;
    use starknet_in_rust::felt::Felt252;
    use starknet_in_rust::hash_utils::calculate_contract_address;
    use starknet_in_rust::utils::Address;
    use starknet_in_rust::{CasmContractClass, ContractClass};
    use starknet_rs_accounts::{Account, Call, RawExecution, SingleOwnerAccount};
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::crypto::ecdsa_sign;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeployAccountTransaction, EventFilter, FieldElement,
        FlattenedSierraClass,
    };
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{AnyProvider, JsonRpcClient, Provider, SequencerGatewayProvider};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;
    use starknet_types::traits::ToHexString;
    use tracing::field::Field;
    use url::Url;

    use crate::common::util::{get_json_body, BackgroundDevnet};

    ///
    fn get_events_contract_in_sierra_and_compiled_class_hash()
    -> (FlattenedSierraClass, FieldElement) {
        let sierra_artifact = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/cairo1/events/events.sierra"
        ))
        .unwrap();
        let mut sierra_json: serde_json::Value = serde_json::from_str(&sierra_artifact).unwrap();
        let contract_class: starknet_in_rust::ContractClass =
            serde_json::from_value(sierra_json.clone()).unwrap();

        let casm_contract_class =
            CasmContractClass::from_contract_class(contract_class, true).unwrap();
        let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

        // convert abi from array to string
        sierra_json["abi"] = serde_json::Value::String(sierra_json["abi"].to_string());
        let flattened_sierra: FlattenedSierraClass = serde_json::from_value(sierra_json).unwrap();

        (flattened_sierra, Felt::from(compiled_class_hash).into())
    }

    fn generate_transaction_hash_on_deploy_account_transaction(
        txn: BroadcastedDeployAccountTransaction,
    ) -> FieldElement {
        let calldata: Vec<Felt252> =
            txn.constructor_calldata.iter().map(|x| Felt::from(*x).into()).collect();
        let class_hash: Felt252 = Felt::from(txn.class_hash).into();
        let address_salt: Felt252 = Felt::from(txn.contract_address_salt).into();

        let contract_address = Address(
            calculate_contract_address(
                &address_salt,
                &class_hash,
                &calldata,
                Address(Felt252::from(0)),
            )
            .unwrap(),
        );

        let hash_value = calculate_deploy_account_transaction_hash(
            Felt252::from(1),
            &contract_address,
            class_hash,
            &calldata,
            u128::from_str_radix(&txn.max_fee.to_string(), 10).unwrap(),
            Felt::from(txn.nonce).into(),
            address_salt.clone(),
            Felt::from(chain_id::TESTNET).into(),
        )
        .unwrap();

        Felt::from(hash_value).into()
    }

    #[test]
    fn test_invoke_transaction_hash() {
        let provider =
            JsonRpcClient::new(HttpTransport::new(Url::parse("http://localhost:5000").unwrap()));

        let test_account = SingleOwnerAccount::new(
            provider,
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(FieldElement::ZERO)),
            FieldElement::from_hex_be(
                "0x6f19b187aabb71473c27e01719fc33d53377703e7063c3151cd2481bee1c94c",
            )
            .unwrap(),
            chain_id::TESTNET,
        );

        let execution = test_account
            .execute(vec![Call {
                to: FieldElement::from_hex_be(
                    "0x64bae94dbae2bb29d9be1b8e4e37ca3020f4b2f4ced6f7148800b7be3374640",
                )
                .unwrap(),
                selector: FieldElement::from_hex_be(
                    "0xe654a0a9b2953a6fd9084842d9b9abc308341e6cd2ab57856441c542e51525",
                )
                .unwrap(),
                calldata: vec![FieldElement::from(1u8)],
            }])
            .max_fee(FieldElement::from_hex_be("0x24fd5a988532").unwrap())
            .nonce(FieldElement::from_hex_be("0x33").unwrap())
            .prepared()
            .unwrap();

        assert!(
            Felt::from_prefixed_hex_str(
                "0x068fbb499e59af504491b801b694cb5b7450a2efc338f7480cb1887ea2c9bd01"
            )
            .unwrap()
                == Felt::from(execution.transaction_hash())
        );

        println!("{}", Felt::from(execution.transaction_hash()).to_prefixed_hex_str());
    }

    #[tokio::test]
    async fn get_events() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let predeployed_accounts_response =
            devnet.get("/predeployed_accounts", None).await.unwrap();

        let predeployed_accounts_json = get_json_body(predeployed_accounts_response).await;
        let first_account = predeployed_accounts_json.as_array().unwrap().get(0).unwrap();

        let account_address =
            Felt::from_prefixed_hex_str(first_account["address"].as_str().unwrap()).unwrap();
        let private_key =
            Felt::from_prefixed_hex_str(first_account["private_key"].as_str().unwrap()).unwrap();

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key.into()));
        let address = FieldElement::from(account_address);

        let mut predeployed_account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, address, chain_id::TESTNET);

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        predeployed_account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(100000000000000u128))
            .nonce(FieldElement::from(1u128))
            .send()
            .await
            .unwrap();

        println!(
            "{:?}",
            devnet
                .json_rpc_client
                .get_transaction_by_hash(declaration_result.transaction_hash)
                .await
        );

        // new account created from dummy values for address and private key
        let new_contract = SingleOwnerAccount::new(
            devnet.clone_provider(),
            LocalWallet::from(SigningKey::from_secret_scalar(FieldElement::from(1023u64))),
            FieldElement::from(10u64),
            chain_id::TESTNET,
        );

        let req_body = Body::from(
            json!({
                "address": Felt::from(new_contract.address()).to_prefixed_hex_str(),
                "amount": 100000000
            })
            .to_string(),
        );

        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");

        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, Arc::new(new_contract));

        let mut broadcasted_deploy_account_transaction = BroadcastedDeployAccountTransaction {
            max_fee: FieldElement::from(10000000000u128),
            signature: vec![],
            nonce: FieldElement::from(1u8),
            contract_address_salt: FieldElement::ZERO,
            constructor_calldata: vec![],
            class_hash: declaration_result.class_hash,
        };

        let txn_hash = generate_transaction_hash_on_deploy_account_transaction(
            broadcasted_deploy_account_transaction.clone(),
        );
        let signature = ecdsa_sign(&private_key.into(), &txn_hash).unwrap();
        broadcasted_deploy_account_transaction.signature =
            vec![signature.r, signature.s, signature.v];

        let deployment_result = devnet
            .json_rpc_client
            .add_deploy_account_transaction(broadcasted_deploy_account_transaction)
            .await;
        println!("{:?}", deployment_result);

        println!("{}", Felt::from(get_selector_from_name("deploy").unwrap()).to_prefixed_hex_str());

        let execution = predeployed_account
            .execute(vec![Call {
                to: FieldElement::from_hex_be(
                    "0x64bae94dbae2bb29d9be1b8e4e37ca3020f4b2f4ced6f7148800b7be3374640",
                )
                .unwrap(),
                selector: FieldElement::from_hex_be(
                    "0xe654a0a9b2953a6fd9084842d9b9abc308341e6cd2ab57856441c542e51525",
                )
                .unwrap(),
                calldata: vec![FieldElement::from(1u8)],
            }])
            .max_fee(FieldElement::from_hex_be("0x24fd5a988532").unwrap())
            .nonce(FieldElement::from_hex_be("0x33").unwrap())
            .prepared()
            .unwrap();

        // deploy new account to the already funded account address
        // invoke via the predeployed account

        // let txn =
        //     devnet.json_rpc_client.get_transaction_by_hash(result.transaction_hash).await.
        // unwrap(); let events = devnet
        //     .json_rpc_client
        //     .get_events(
        //         EventFilter { from_block: None, to_block: None, address: None, keys: None },
        //         None,
        //         10000,
        //     )
        //     .await
        //     .unwrap();
        // println!("{:?}", events);
    }
}
