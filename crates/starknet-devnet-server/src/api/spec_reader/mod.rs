use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;

use serde::Deserialize;
use serde_json::{Map, Value};

use self::data_generator::generate_schema_value;
use self::spec_schemas::Schema;

mod data_generator;
mod spec_modifier;
mod spec_schemas;

#[derive(Deserialize)]
pub struct Spec {
    methods: Vec<ApiMethod>,
    components: Components,
}

#[derive(Deserialize)]
struct ApiMethod {
    name: String,
    params: Vec<Param>,
    result: Option<Result>,
}

#[derive(Deserialize)]
struct Param {
    name: String,
    // TODO: improve generation logic to use this field
    // #[serde(default)]
    // required: bool,
    schema: Schema,
}

#[derive(Deserialize)]
struct Result {
    schema: Schema,
}

#[derive(Deserialize)]
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
    method: &ApiMethod,
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
    response_schema: &Schema,
    schemas: &HashMap<String, Schema>,
) -> core::result::Result<serde_json::Value, String> {
    generate_schema_value(response_schema, schemas, 0)
}

mod tests {
    use std::fs::File;

    use serde::Deserialize;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    use super::{ApiMethod, Spec, generate_combined_schema, generate_json_rpc_response};
    use crate::api::RPC_SPEC_VERSION;
    use crate::api::models::{
        JsonRpcRequest, JsonRpcSubscriptionRequest, StarknetResponse, StarknetSpecRequest,
    };
    use crate::api::spec_reader::generate_json_rpc_request;
    use crate::subscribe::{SubscriptionConfirmation, SubscriptionResponse};

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

        for i in 1..=1000 {
            for spec in specs.iter() {
                // Iterate over the methods in the spec
                for method in spec.methods.iter() {
                    // Create a JSON-RPC request for each method
                    let request = generate_json_rpc_request(method, &combined_schema)
                        .unwrap_or_else(|e| {
                            panic!(
                                "i={i} Failed generating request for RPC method {}: {e}",
                                method.name
                            )
                        });

                    let response = method.result.as_ref().map(|result_schema| {
                        generate_json_rpc_response(&result_schema.schema, &combined_schema)
                            .unwrap_or_else(|e| {
                                panic!(
                                    "i={i} Failed generating response for RPC method {}: {e}",
                                    method.name
                                )
                            })
                    });

                    #[derive(Deserialize)]
                    #[serde(untagged)]
                    enum ApiOrWsRequest {
                        Api(Box<JsonRpcRequest>),
                        SubscribeWs(JsonRpcSubscriptionRequest),
                        WsNotification(Box<SubscriptionResponse>),
                    }
                    let sn_request =
                        deserialize_to_type_or_panic::<ApiOrWsRequest>(request, &method.name);

                    match sn_request {
                        ApiOrWsRequest::Api(json_rpc_request) => {
                            let response = response.unwrap();
                            let sn_response: StarknetResponse =
                                deserialize_to_type_or_panic(response, &method.name);

                            assert_api_request_and_response_are_related(
                                &json_rpc_request,
                                sn_response,
                                method,
                            );
                        }
                        ApiOrWsRequest::SubscribeWs(_json_rpc_subscription_request) => {
                            let response = response.unwrap();

                            deserialize_to_type_or_panic::<SubscriptionConfirmation>(
                                response,
                                &method.name,
                            );
                        }
                        ApiOrWsRequest::WsNotification(subscription_response) => {
                            match *subscription_response {
                                SubscriptionResponse::Confirmation { .. } => {
                                    panic!("Unexpected data")
                                }
                                SubscriptionResponse::Notification(_) => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn deserialize_to_type_or_panic<T: DeserializeOwned>(
        json_value: Value,
        method_name: &String,
    ) -> T {
        let failed_file_name = if json_value.get("method").is_some() {
            "failed_request.json"
        } else {
            "failed_response.json"
        };

        let deserialized = serde_json::from_value::<T>(json_value.clone());

        if let Some(err) = deserialized.as_ref().err() {
            serde_json::to_writer_pretty(File::create(failed_file_name).unwrap(), &json_value)
                .unwrap();
            panic!("{} method {} with {:?}.", failed_file_name, method_name, err);
        }

        deserialized.unwrap()
    }

    fn assert_api_request_and_response_are_related(
        sn_request: &JsonRpcRequest,
        sn_response: StarknetResponse,
        method: &ApiMethod,
    ) {
        match sn_request {
            JsonRpcRequest::StarknetSpecRequest(req) => match req {
                StarknetSpecRequest::BlockWithTransactionHashes(_)
                | StarknetSpecRequest::BlockWithFullTransactions(_)
                | StarknetSpecRequest::BlockWithReceipts(_) => {
                    assert!(matches!(
                        sn_response,
                        StarknetResponse::Block(_) | StarknetResponse::PreConfirmedBlock(_)
                    ));
                }
                StarknetSpecRequest::TransactionReceiptByTransactionHash(_) => {
                    assert!(matches!(
                        sn_response,
                        StarknetResponse::TransactionReceiptByTransactionHash(_)
                    ));
                }
                StarknetSpecRequest::ClassAtContractAddress(_)
                | StarknetSpecRequest::ClassByHash(_) => {
                    assert!(matches!(sn_response, StarknetResponse::ContractClass(_)));
                }
                StarknetSpecRequest::BlockTransactionCount(_)
                | StarknetSpecRequest::BlockNumber => {
                    assert!(matches!(
                        sn_response,
                        StarknetResponse::BlockTransactionCount(_)
                            | StarknetResponse::BlockNumber(_)
                    ));
                }
                StarknetSpecRequest::Call(_) => {
                    assert!(matches!(sn_response, StarknetResponse::Call(_)))
                }
                StarknetSpecRequest::EstimateFee(_) => {
                    assert!(matches!(sn_response, StarknetResponse::EstimateFee(_)))
                }
                StarknetSpecRequest::BlockHashAndNumber => {
                    assert!(matches!(sn_response, StarknetResponse::BlockHashAndNumber(_)));
                }
                StarknetSpecRequest::EstimateMessageFee(_) => {
                    assert!(matches!(sn_response, StarknetResponse::EstimateMessageFee(_)));
                }
                StarknetSpecRequest::Events(_) => {
                    assert!(matches!(sn_response, StarknetResponse::Events(_)));
                }
                StarknetSpecRequest::SimulateTransactions(_) => {
                    assert!(matches!(sn_response, StarknetResponse::SimulateTransactions(_)));
                }
                StarknetSpecRequest::StateUpdate(_) => {
                    assert!(matches!(
                        sn_response,
                        StarknetResponse::StateUpdate(_)
                            | StarknetResponse::PreConfirmedStateUpdate(_)
                    ));
                }
                StarknetSpecRequest::Syncing => {
                    assert!(matches!(sn_response, StarknetResponse::Syncing(_)));
                }
                StarknetSpecRequest::TransactionStatusByHash(_) => {
                    assert!(matches!(sn_response, StarknetResponse::TransactionStatusByHash(_)));
                }
                StarknetSpecRequest::AddDeclareTransaction(_) => {
                    assert!(matches!(sn_response, StarknetResponse::AddDeclareTransaction(_)));
                }
                StarknetSpecRequest::AddDeployAccountTransaction(_) => {
                    assert!(matches!(
                        sn_response,
                        StarknetResponse::AddDeployAccountTransaction(_)
                    ));
                }
                StarknetSpecRequest::AddInvokeTransaction(_) => {
                    assert!(matches!(sn_response, StarknetResponse::TransactionHash(_)));
                }
                StarknetSpecRequest::SpecVersion => {
                    assert!(matches!(sn_response, StarknetResponse::String(_)));
                }
                StarknetSpecRequest::TransactionByHash(_)
                | StarknetSpecRequest::TransactionByBlockAndIndex(_) => {
                    assert!(matches!(sn_response, StarknetResponse::Transaction(_)));
                }
                StarknetSpecRequest::ContractNonce(_)
                | StarknetSpecRequest::ChainId
                | StarknetSpecRequest::ClassHashAtContractAddress(_)
                | StarknetSpecRequest::StorageAt(_) => {
                    assert!(matches!(sn_response, StarknetResponse::Felt(_)));
                }
                StarknetSpecRequest::TraceTransaction(_) => {
                    assert!(matches!(sn_response, StarknetResponse::TraceTransaction(_)));
                }
                StarknetSpecRequest::BlockTransactionTraces(_) => {
                    assert!(matches!(sn_response, StarknetResponse::BlockTransactionTraces(_)));
                }
                StarknetSpecRequest::MessagesStatusByL1Hash(_) => {
                    assert!(matches!(sn_response, StarknetResponse::MessagesStatusByL1Hash(_)));
                }
                StarknetSpecRequest::CompiledCasmByClassHash(_) => {
                    assert!(matches!(sn_response, StarknetResponse::CompiledCasm(_)));
                }
                StarknetSpecRequest::StorageProof(_) => {
                    panic!("Should never be matched, unused by Devnet")
                }
            },
            _ => panic!(
                "Devnet methods not asserted. This match case must not be reached, because this \
                 method covers starknet RPC method (starknet_.....) {:?} {}",
                sn_request, method.name
            ),
        }
    }
}
