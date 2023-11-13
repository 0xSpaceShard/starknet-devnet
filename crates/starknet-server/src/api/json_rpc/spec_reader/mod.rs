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

#[ignore]
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
        BroadcastedDeclareTransaction, FunctionInvocation, SimulatedTransaction, TransactionTrace, BroadcastedTransaction, InvokeTransactionTrace,
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
                std::fs::write("./output.json", json_str);
                possible_error.unwrap();
            }
        }
    }

    #[test]
    #[ignore]
    fn test_abi_entry() {
        //let x: FunctionInvocation = serde_json::from_str(&std::fs::read_to_string("output.json").unwrap()).unwrap();
        assert_type_with_schema_value::<TransactionTrace>("TRANSACTION_TRACE", 1)
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
                    
                    if sn_response.is_err() {
                        println!("{}", serde_json::to_string(&request).unwrap());
                        sn_response.unwrap();
                    }
                }
            }
        }
    }

    #[test]
    fn test_call_method_request() {
        assert_method_requests(Some("starknet_getStateUpdate"), 1);
        assert_method_response(Some("starknet_getStateUpdate"), 100);
    }

    #[test]
    #[ignore]
    fn test_simulate_transaction_responses() {
        assert_method_response(Some("starknet_simulateTransactions"), 1);
    }

    #[test]
    fn test_for_all_methods_requests() {
        assert_method_requests(None, 100);
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
