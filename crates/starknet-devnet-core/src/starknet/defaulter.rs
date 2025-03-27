use std::io::Read;

use blockifier::execution::contract_class::RunnableCompiledClass;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::StateResult;
use starknet_api::core::{ClassHash, ContractAddress, Nonce, PatriciaKey};
use starknet_api::state::StorageKey;
use starknet_rs_core::types::Felt;
use starknet_types::contract_class::convert_codegen_to_blockifier_compiled_class;
use tracing::debug;

use super::starknet_config::ForkConfig;

#[derive(thiserror::Error, Debug)]
enum OriginError {
    #[error("Error in communication with forking origin: {0}")]
    CommunicationError(String),
    #[error("Received invalid response from forking origin: {0}")]
    FormatError(String),
    #[error("Received JSON response from forking origin, but no result property in it")]
    NoResult,
}

impl OriginError {
    fn from_status_code(status_code: reqwest::StatusCode) -> Self {
        let additional_info = match status_code.as_u16() {
            429 => {
                "This means your program is making Devnet send too many requests to the forking \
                 origin. 1) It could be a temporary issue, so try re-running your program. 2) If \
                 forking is not crucial for your use-case, disable it. 3) Try changing the forking \
                 URL 4) Consider adding short sleeps to the program from which you are interacting \
                 with Devnet."
            }
            _ => "",
        };

        OriginError::CommunicationError(format!("{status_code}. {additional_info}"))
    }
}

/// Used for interacting with the origin in forking mode. The calls are blocking. Only handles the
/// basic state reading necessary for contract interaction. For other RPC methods, see
/// `OriginForwarder`
#[derive(Debug, Clone)]
struct BlockingOriginReader {
    url: url::Url,
    block_number: u64,
    client: reqwest::blocking::Client,
}

impl BlockingOriginReader {
    fn new(url: url::Url, block_number: u64) -> Self {
        Self { url, block_number, client: reqwest::blocking::Client::new() }
    }

    fn send_body(
        &self,
        method: &str,
        mut params: serde_json::Value,
    ) -> Result<serde_json::Value, OriginError> {
        params["block_id"] = serde_json::json!({
            "block_number": self.block_number
        });
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 0,
        });

        match self
            .client
            .post(self.url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
        {
            Ok(mut resp) => {
                let resp_status = resp.status();
                if resp_status != reqwest::StatusCode::OK {
                    return Err(OriginError::from_status_code(resp_status));
                }

                // load json
                let mut buff = vec![];
                resp.read_to_end(&mut buff).map_err(|e| OriginError::FormatError(e.to_string()))?;
                let resp_json_value: serde_json::Value = serde_json::from_slice(&buff)
                    .map_err(|e| OriginError::FormatError(e.to_string()))?;

                let result = &resp_json_value["result"];
                if result.is_null() {
                    // the received response is assumed to mean that the origin doesn't contain the
                    // requested resource
                    debug!("Origin response contains no 'result': {resp_json_value}");
                    Err(OriginError::NoResult)
                } else {
                    Ok(result.clone())
                }
            }
            Err(other_err) => Err(OriginError::CommunicationError(other_err.to_string())),
        }
    }
}

/// Used for forking - reads from the origin if Some(origin_reader); otherwise returns the default
/// or Err, depending on the method
#[derive(Clone, Debug, Default)]
pub struct StarknetDefaulter {
    origin_reader: Option<BlockingOriginReader>,
}

impl StarknetDefaulter {
    pub fn new(fork_config: ForkConfig) -> Self {
        let origin_reader =
            if let (Some(fork_url), Some(block)) = (fork_config.url, fork_config.block_number) {
                Some(BlockingOriginReader::new(fork_url, block))
            } else {
                None
            };
        Self { origin_reader }
    }

    pub fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<Felt> {
        if let Some(origin) = &self.origin_reader {
            origin.get_storage_at(contract_address, key)
        } else {
            Ok(Default::default())
        }
    }

    pub fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        if let Some(origin) = &self.origin_reader {
            origin.get_nonce_at(contract_address)
        } else {
            Ok(Default::default())
        }
    }

    pub fn get_compiled_class(&self, class_hash: ClassHash) -> StateResult<RunnableCompiledClass> {
        if let Some(origin) = &self.origin_reader {
            origin.get_compiled_class(class_hash)
        } else {
            Err(StateError::UndeclaredClassHash(class_hash))
        }
    }

    pub fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        if let Some(origin) = &self.origin_reader {
            origin.get_class_hash_at(contract_address)
        } else {
            Ok(Default::default())
        }
    }
}

fn convert_json_value_to_felt(json_value: serde_json::Value) -> StateResult<Felt> {
    serde_json::from_value(json_value).map_err(|e| StateError::StateReadError(e.to_string()))
}

fn convert_patricia_key_to_hex(key: PatriciaKey) -> String {
    key.key().to_hex_string()
}

// Same as StateReader, but with &self instead of &mut self
impl BlockingOriginReader {
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<Felt> {
        let storage = match self.send_body(
            "starknet_getStorageAt",
            serde_json::json!({
                "contract_address": convert_patricia_key_to_hex(contract_address.0),
                "key": convert_patricia_key_to_hex(key.0),
            }),
        ) {
            Err(OriginError::NoResult) => Default::default(),
            Err(other_error) => return Err(StateError::StateReadError(other_error.to_string())),
            Ok(value) => convert_json_value_to_felt(value)?,
        };
        Ok(storage)
    }

    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        let nonce = match self.send_body(
            "starknet_getNonce",
            serde_json::json!({
                "contract_address": convert_patricia_key_to_hex(contract_address.0),
            }),
        ) {
            Err(OriginError::NoResult) => Default::default(),
            Err(other_error) => return Err(StateError::StateReadError(other_error.to_string())),
            Ok(value) => Nonce(convert_json_value_to_felt(value)?),
        };
        Ok(nonce)
    }

    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        let class_hash = match self.send_body(
            "starknet_getClassHashAt",
            serde_json::json!({
                "contract_address": convert_patricia_key_to_hex(contract_address.0),
            }),
        ) {
            Err(OriginError::NoResult) => Default::default(),
            Err(other_error) => return Err(StateError::StateReadError(other_error.to_string())),
            Ok(value) => ClassHash(convert_json_value_to_felt(value)?),
        };
        Ok(class_hash)
    }

    fn get_compiled_class(&self, class_hash: ClassHash) -> StateResult<RunnableCompiledClass> {
        match self.send_body(
            "starknet_getClass",
            serde_json::json!({
                "class_hash": class_hash.0.to_hex_string(),
            }),
        ) {
            Err(OriginError::NoResult) => Err(StateError::UndeclaredClassHash(class_hash)),
            Err(other_error) => Err(StateError::StateReadError(other_error.to_string())),
            Ok(value) => {
                let contract_class: starknet_rs_core::types::ContractClass =
                    serde_json::from_value(value)
                        .map_err(|e| StateError::StateReadError(e.to_string()))?;

                convert_codegen_to_blockifier_compiled_class(contract_class)
                    .map_err(|e| StateError::StateReadError(e.to_string()))
            }
        }
    }
}
