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
pub struct Spec {
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
        for (schema_name, schema) in &spec.components.schemas {
            match schema.clone() {
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
                        combined_schema.insert(schema_name.clone(), schema.clone());
                    }
                }
                _ => {
                    combined_schema.insert(schema_name.clone(), schema.clone());
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
    if !params.is_empty() {
        request.insert("params", Value::Object(params));
    } else {
        request.insert("params", Value::Array(vec![]));
    }

    serde_json::to_value(&request)
        .map_err(|err| format!("Could not serialize the JSON-RPC request: {}", err))
}

fn generate_json_rpc_response(
    method: &Method,
    schemas: &HashMap<String, Schema>,
) -> core::result::Result<serde_json::Value, String> {
    generate_schema_value(&method.result.schema, schemas, 0)
}

mod tests {
    use std::fs::File;

    use super::{generate_combined_schema, generate_json_rpc_response, Spec};
    use crate::api::json_rpc::spec_reader::generate_json_rpc_request;
    use crate::api::json_rpc::{StarknetRequest, StarknetResponse, RPC_SPEC_VERSION};

    #[test]
    /// This test asserts that the spec files used in testing indeed match the expected version
    fn rpc_spec_using_correct_version() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path_to_spec_dir = format!("{manifest_dir}/test_data/spec/{RPC_SPEC_VERSION}");
        let spec_files = std::fs::read_dir(path_to_spec_dir).unwrap();

        // traverse all json files in the rpc spec dir and assert they all have the expected version
        for spec_file in
            spec_files.filter(|f| f.as_ref().unwrap().path().extension().unwrap() == "json")
        {
            let spec_file_path = spec_file.unwrap().path();
            let spec_file_path = spec_file_path.to_str().unwrap();
            let spec_reader = std::fs::File::open(spec_file_path).unwrap();
            let spec_content: serde_json::Value = serde_json::from_reader(spec_reader).unwrap();
            match spec_content
                .get("info")
                .and_then(|info| info.get("version"))
                .and_then(|ver| ver.as_str())
            {
                Some(RPC_SPEC_VERSION) => (),
                other => panic!("Invalid version in {spec_file_path}: {other:?}"),
            }
        }
    }

    #[test]
    fn test_spec_methods() {
        let specs_folder = concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/spec/");
        let specs = Spec::load_from_dir(format!("{specs_folder}/{RPC_SPEC_VERSION}",).as_str());
        let combined_schema = generate_combined_schema(&specs);

        for _ in 0..1000 {
            for spec in specs.iter() {
                // Iterate over the methods in the spec
                for method in
                    spec.methods.iter().filter(|m| m.name != "starknet_getBlockWithReceipts")
                {
                    // Create a JSON-RPC request for each method
                    let request = generate_json_rpc_request(method, &combined_schema)
                        .expect("Could not generate the JSON-RPC request");

                    let sn_request = serde_json::from_value::<StarknetRequest>(request.clone());

                    if sn_request.is_err() {
                        serde_json::to_writer_pretty(
                            File::create("failed_request.json").unwrap(),
                            &request,
                        )
                        .unwrap();
                        panic!("Failed method request: {}", method.name);
                    }

                    let response = generate_json_rpc_response(method, &combined_schema)
                        .expect("Could not generate the JSON-RPC response");

                    let sn_response = serde_json::from_value::<StarknetResponse>(response.clone());

                    if sn_response.is_err() {
                        serde_json::to_writer_pretty(
                            File::create("failed_response.json").unwrap(),
                            &response,
                        )
                        .unwrap();
                        panic!("Failed method response: {}", method.name);
                    }

                    let sn_response = sn_response.unwrap();
                    let sn_request = sn_request.unwrap();

                    match sn_request {
                        StarknetRequest::TransactionReceiptByTransactionHash(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::TransactionReceiptByTransactionHash(_)
                            ));
                        }
                        StarknetRequest::BlockWithTransactionHashes(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::BlockWithTransactionHashes(_)
                            ));
                        }
                        StarknetRequest::BlockHashAndNumber => {
                            assert!(matches!(sn_response, StarknetResponse::BlockHashAndNumber(_)));
                        }
                        StarknetRequest::BlockNumber
                        | StarknetRequest::BlockTransactionCount(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::BlockNumber(_)
                                    | StarknetResponse::BlockTransactionCount(_)
                            ));
                        }
                        StarknetRequest::Call(_) => {
                            assert!(matches!(sn_response, StarknetResponse::Call(_)));
                        }
                        StarknetRequest::ClassAtContractAddress(_)
                        | StarknetRequest::ClassByHash(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::ClassAtContractAddress(_)
                                    | StarknetResponse::ClassByHash(_)
                            ));
                        }
                        StarknetRequest::EsimateFee(_) => {
                            assert!(matches!(sn_response, StarknetResponse::EstimateFee(_)));
                        }
                        StarknetRequest::EstimateMessageFee(_) => {
                            assert!(matches!(sn_response, StarknetResponse::EstimateMessageFee(_)));
                        }
                        StarknetRequest::Events(_) => {
                            assert!(matches!(sn_response, StarknetResponse::Events(_)));
                        }
                        StarknetRequest::SimulateTransactions(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::SimulateTransactions(_)
                            ));
                        }
                        StarknetRequest::StateUpdate(_) => {
                            assert!(matches!(sn_response, StarknetResponse::StateUpdate(_)));
                        }
                        StarknetRequest::Syncing => {
                            assert!(matches!(sn_response, StarknetResponse::Syncing(_)));
                        }
                        StarknetRequest::TransactionStatusByHash(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::TransactionStatusByHash(_)
                            ));
                        }
                        StarknetRequest::AddDeclareTransaction(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::AddDeclareTransaction(_)
                            ));
                        }
                        StarknetRequest::AddDeployAccountTransaction(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::AddDeployAccountTransaction(_)
                            ));
                        }
                        StarknetRequest::AddInvokeTransaction(_) => {
                            assert!(matches!(
                                sn_response,
                                StarknetResponse::AddInvokeTransaction(_)
                            ));
                        }
                        _ => {
                            // Remaining responses are not implemented, because
                            // multiple requests return the same response format either u64, Felt,
                            // etc. so its impossible to know which
                            // response variant is generated based on
                            // serde untagged deserialization. This is due to the fact that the
                            // first variant which complies with the response format is returned
                        }
                    }
                }
            }
        }
    }
}
