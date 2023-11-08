use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use self::data_generator::generate_schema_value;
use self::spec_schemas::Schema;

mod data_generator;
mod spec_modifier;
mod spec_schemas;

#[derive(Serialize, Deserialize)]
pub(crate) struct Spec {
    openrpc: String,
    methods: Vec<Method>,
    components: Components,
}

#[derive(Serialize, Deserialize)]
struct Method {
    name: String,
    params: Vec<Param>,
    result: Result,
}

#[derive(Serialize, Deserialize)]
struct Param {
    name: String,
    required: bool,
    schema: Schema,
}

#[derive(Serialize, Deserialize)]
struct Result {
    name: String,
    schema: Schema,
}

#[derive(Serialize, Deserialize)]
struct Components {
    schemas: HashMap<String, Schema>,
}

impl Spec {
    fn load_from_dir(dir_path: &str) -> Vec<Self> {
        let mut specs: Vec<Spec> = Vec::new();

        let mut instructions = Option::None;

        for path in fs::read_dir(dir_path).unwrap() {
            let path = path.unwrap().path();

            if let Some("yaml") = path.as_path().extension().and_then(OsStr::to_str) {
                instructions = Some(spec_modifier::SpecModifier::load_from_path(
                    path.as_path().to_str().unwrap(),
                ));
                break;
            }
        }

        for path in fs::read_dir(dir_path).unwrap() {
            let path = path.unwrap().path();

            if let Some("yaml") = path.as_path().extension().and_then(OsStr::to_str) {
                continue;
            }

            let spec = Self::load_from_path(path.as_path().to_str().unwrap(), &instructions);

            specs.push(spec);
        }

        specs
    }

    fn load_from_path(
        path: &str,
        modify_spec_instructions: &Option<spec_modifier::SpecModifier>,
    ) -> Self {
        let spec_str = fs::read_to_string(path).expect("Could not read the JSON-RPC spec file");

        if let Some(instructions) = modify_spec_instructions {
            // Remove some parts of the spec which were added due to some mistake
            let json_obj_spec: serde_json::Value = serde_json::from_str(&spec_str)
                .expect("Could not parse the JSON-RPC spec file to JSON object");

            instructions.generate_spec(json_obj_spec)
        } else {
            let spec: Spec =
                serde_json::from_str(&spec_str).expect("Could not parse the JSON-RPC spec");

            spec
        }
    }
}

fn generate_combined_schema(specs: &Vec<Spec>) -> HashMap<String, Schema> {
    let mut combined_schema = HashMap::<String, Schema>::new();

    for spec in specs {
        for schema in &spec.components.schemas {
            match schema.1.clone() {
                Schema::Ref(reference) => {
                    // if reference to external file, then dont add it
                    let schema_parts = reference
                        .ref_field
                        .trim_start_matches("./")
                        .split("#/components/schemas/")
                        .filter(|entry| !entry.is_empty())
                        .collect::<Vec<&str>>();

                    if schema_parts.len() == 1 {
                        // then it is not reference to external file
                        // only references to external files are not added
                        combined_schema.insert(schema.0.clone(), schema.1.clone());
                    }
                }
                _ => {
                    combined_schema.insert(schema.0.clone(), schema.1.clone());
                }
            }
        }
    }

    combined_schema
}

fn generate_json_rpc_request(
    method: &Method,
    schemas: &HashMap<String, Schema>,
) -> core::result::Result<serde_json::Value, String> {
    let mut request = HashMap::new();
    request.insert("jsonrpc", Value::String("2.0".to_string()));
    request.insert("method", Value::String(method.name.clone()));
    request.insert("id", Value::Number(serde_json::Number::from(1)));

    // Add the parameters to the request
    let mut params = Map::new();

    for param in method.params.iter() {
        let param_value = generate_schema_value(&param.schema, schemas, 0)?;
        params.insert(param.name.clone(), param_value);
    }
    if params.len() > 0 {
        request.insert("params", Value::Object(params));
    } else {
        request.insert("params", Value::Array(vec![]));
    }

    serde_json::to_value(&request)
        .map_err(|err| format!("Could not serialize the JSON-RPC request: {}", err.to_string()))
}

fn generate_json_rpc_response(
    method: &Method,
    schemas: &HashMap<String, Schema>,
) -> core::result::Result<serde_json::Value, String> {
    generate_schema_value(&method.result.schema, schemas, 0)
}

mod tests {
    use serde::Deserialize;
    use serde::de::DeserializeOwned;
    use starknet_types::contract_class::DeprecatedContractClass;
    use starknet_types::contract_class::deprecated::rpc_contract_class::ContractClassAbiEntryWithType;
    use starknet_types::felt::Felt;
    use starknet_types::num_bigint::BigUint;
    use starknet_types::rpc::estimate_message_fee::FeeEstimateWrapper;
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::{
        BroadcastedDeclareTransaction, FunctionInvocation, SimulatedTransaction, TransactionTrace, BroadcastedTransaction,
    };
    use starknet_types::starknet_api::deprecated_contract_class::StructAbiEntry;
    use starknet_types::starknet_api::serde_utils::PrefixedBytesAsHex;
    use starknet_types::starknet_api::transaction::Fee;

    use super::data_generator::generate_schema_value;
    use super::{generate_combined_schema, generate_json_rpc_response, Spec};
    use crate::api::json_rpc::models::{TransactionStatusOutput, BroadcastedDeclareTransactionEnumWrapper};
    use crate::api::json_rpc::spec_reader::generate_json_rpc_request;
    use crate::api::json_rpc::{StarknetRequest, StarknetResponse};

    fn assert_type_with_schema_value<T: DeserializeOwned>(schema_name: &str, check_count: u16) {
        let specs =
            Spec::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/spec/0.5.0"));
            let combined_schema = generate_combined_schema(&specs);
        let schema = combined_schema.get(schema_name).unwrap();
        for _ in 0..check_count {
            let json_str = serde_json::to_string(&generate_schema_value(schema, &combined_schema, 0).unwrap()).unwrap();
            let possible_error = serde_json::from_str::<T>(&json_str);
            if possible_error.is_err() {
                println!("{json_str}");
                possible_error.unwrap();
            }
        }
    }

    #[test]
    fn broadcasted_txn_request() {
        let json_str = r#"{
            "type": "DECLARE",
            "contract_class": {
                "program": "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",
                "entry_points_by_type": {
                    "CONSTRUCTOR": [],
                    "EXTERNAL": [
                        {
                            "offset": "0x3a",
                            "selector": "0x362398bec32bc0ebb411203221a35a0301193a96f317ebe5e40be9f60d15320"
                        },
                        {
                            "offset": "0x5b",
                            "selector": "0x39e11d48192e4333233c7eb19d10ad67c362bb28580c604d67884c85da39695"
                        }
                    ],
                    "L1_HANDLER": []
                },
                "abi": [
                    {
                        "type": "function",
                        "name": "increase_balance",
                        "inputs": [
                            {
                                "name": "amount",
                                "type": "felt"
                            }
                        ],
                        "outputs": []
                    },
                    {
                        "type": "function",
                        "name": "get_balance",
                        "inputs": [],
                        "outputs": [
                            {
                                "name": "res",
                                "type": "felt"
                            }
                        ],
                        "stateMutability": "view"
                    }
                ]
            },
            "sender_address": "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba",
            "max_fee": "0xf7cf5d8fdc00",
            "version": "0x1",
            "signature": [
                "0x59a872e715504380f4246a20b7554de2f5956f6399c8d04482924b92c291297",
                "0x3491dd26ddbd71b4bcb4fcf22b94b129c61088ef0db4e55e741ddaf2a5cd033"
            ],
            "nonce": "0x0"
        }"#;

        let json_value = serde_json::from_str::<BroadcastedDeclareTransactionEnumWrapper>(json_str).unwrap();
    }

    #[test]
    fn test_abi_entry() {
        
        assert_type_with_schema_value::<BroadcastedTransaction>("BROADCASTED_DECLARE_TXN_V1", 100)
    }

    fn assert_method_requests(method_name: Option<&str>, check_count: u16) {
        let specs =
            Spec::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/spec/0.5.0"));
            let combined_schema = generate_combined_schema(&specs);

        for _ in 0..check_count {
            for spec in specs.iter() {
                for method in spec.methods.iter().filter(|method| match method_name {
                    Some(name) => method.name == name,
                    None => true,
                }) {
                    let request = generate_json_rpc_request(&method, &combined_schema)
                            .expect("Could not generate the JSON-RPC request");

                    let sn_request = serde_json::from_value::<StarknetRequest>(request.clone());

                    if sn_request.is_err() {
                        println!("{}", serde_json::to_string(&request).unwrap());
                        sn_request.unwrap();
                    }
                }
            }
        }
    }

    fn assert_method_response(method_name: Option<&str>, check_count: u16) {
        let specs =
            Spec::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/spec/0.5.0"));
            let combined_schema = generate_combined_schema(&specs);

        for _ in 0..check_count {
            for spec in specs.iter() {
                for method in spec.methods.iter().filter(|method| match method_name {
                    Some(name) => method.name == name,
                    None => true,
                }) {
                    let request = generate_json_rpc_response(&method, &combined_schema)
                            .expect("Could not generate the JSON-RPC request");

                    let sn_response = serde_json::from_value::<StarknetResponse>(request.clone());
                    println!("{}", serde_json::to_string(&request).unwrap());
                    if sn_response.is_err() {
                        
                        sn_response.unwrap();
                    }
                }
            }
        }
    }

    #[test]
    fn test_call_method_request() {
        assert_method_requests(Some("starknet_call"), 1);
    }

    #[test]
    #[ignore]
    fn test_simulate_transaction_responses() {
        assert_method_response(Some("starknet_simulateTransactions"), 1);
    }

    #[test]
    #[ignore]
    fn test_for_all_methods_requests() {
        assert_method_requests(None, 50);
    }

    #[test]
    #[ignore]
    fn test_simulate_transaction_requests() {
        assert_method_requests(Some("starknet_simulateTransactions"), 100);
    }

    #[test]
    #[ignore]
    fn test_spec_methods() {
        let specs =
            Spec::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/spec/0.5.0"));
        let combined_schema = generate_combined_schema(&specs);
        for _ in 0..100 {
            for spec in specs.iter() {
                // Iterate over the methods in the spec
                for method in spec.methods.iter() {
                    // Create a JSON-RPC request for each method
                    let request = generate_json_rpc_request(&method, &combined_schema)
                        .expect("Could not generate the JSON-RPC request");

                    let sn_request = serde_json::from_value::<StarknetRequest>(request.clone());

                    if sn_request.is_err() {
                        println!("{}", serde_json::to_string(&request).unwrap());
                        panic!("Error in request of method {}", method.name);
                    }

                    let response = generate_json_rpc_response(&method, &combined_schema)
                        .expect("Could not generate the JSON-RPC response");

                    let sn_response = serde_json::from_value::<StarknetResponse>(response.clone());

                    if sn_response.is_err() {
                        println!("Error: {}", serde_json::to_string(&response).unwrap());
                        panic!("Error in response of method {}", method.name);
                    }

                    // match sn_request {
                    //     StarknetRequest::SpecVersion => {
                    //         let sn_response: StarknetResponse =
                    //             serde_json::from_value(response.clone()).unwrap();
                    //         assert!(matches!(sn_response, StarknetResponse::SpecVersion(_)));
                    //     }
                    //     _ => {}
                    // }
                }
            }
        }
    }
}
