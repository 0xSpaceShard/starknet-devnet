use std::io::Read;

use blockifier::execution::contract_class::ContractClass;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::StateResult;
use starknet_api::core::{ClassHash, ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_types::contract_class::convert_codegen_to_blockifier_compiled_class;
use starknet_types::felt::Felt;
use starknet_types::traits::ToHexString;

use super::starknet_config::ForkConfig;

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
    ) -> Result<serde_json::Value, reqwest::Error> {
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
                assert_eq!(resp.status(), reqwest::StatusCode::OK);
                let mut buff = vec![];
                resp.read_to_end(&mut buff).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&buff).unwrap();
                // TODO perhaps deal with error here?
                let result = &value["result"];
                Ok(result.clone())
            }
            Err(err) => Err(err),
        }
    }
}

// TODO rename to StarknetDefaulter
#[derive(Clone, Debug, Default)]
pub struct Defaulter {
    origin_reader: Option<BlockingOriginReader>,
}

impl Defaulter {
    pub fn new(fork_config: ForkConfig) -> Self {
        let origin_reader = if let Some(fork_url) = fork_config.url {
            Some(BlockingOriginReader::new(fork_url, fork_config.block.unwrap()))
        } else {
            None
        };
        Self { origin_reader }
    }

    pub fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt> {
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

    pub fn get_compiled_contract_class(&self, class_hash: ClassHash) -> StateResult<ContractClass> {
        if let Some(origin) = &self.origin_reader {
            origin.get_compiled_contract_class(class_hash)
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

fn convert_json_value_to_stark_felt(json_value: serde_json::Value) -> StarkFelt {
    StarkFelt::try_from(json_value.as_str().unwrap()).unwrap()
}

// Same as Statereader, but with &self instead of &mut self
impl BlockingOriginReader {
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt> {
        let contract_address: Felt = contract_address.0.try_into().unwrap();
        let key: Felt = key.0.try_into().unwrap();
        Ok(
            match self.send_body(
                "starknet_getStorageAt",
                serde_json::json!({
                    "contract_address": contract_address.to_prefixed_hex_str(),
                    "key": key.to_prefixed_hex_str(),
                }),
            ) {
                Ok(serde_json::Value::Null) | Err(_) => Default::default(),
                Ok(value) => convert_json_value_to_stark_felt(value),
            },
        )
    }

    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        let contract_address: Felt = contract_address.0.try_into().unwrap();
        Ok(
            match self.send_body(
                "starknet_getNonce",
                serde_json::json!({
                    "contract_address": contract_address.to_prefixed_hex_str(),
                }),
            ) {
                Ok(serde_json::Value::Null) | Err(_) => Default::default(),
                Ok(value) => Nonce(convert_json_value_to_stark_felt(value)),
            },
        )
    }

    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        let contract_address: Felt = contract_address.0.try_into().unwrap();
        Ok(
            match self.send_body(
                "starknet_getClassHashAt",
                serde_json::json!({
                    "contract_address": contract_address.to_prefixed_hex_str(),
                }),
            ) {
                Ok(serde_json::Value::Null) | Err(_) => Default::default(),
                Ok(value) => ClassHash(convert_json_value_to_stark_felt(value)),
            },
        )
    }

    fn get_compiled_contract_class(&self, class_hash: ClassHash) -> StateResult<ContractClass> {
        match self.send_body(
            "starknet_getClass",
            serde_json::json!({
                "class_hash": Felt::try_from(class_hash).unwrap().to_prefixed_hex_str(),
            }),
        ) {
            // TODO perhaps don't do this null catching here and in other endpoints
            Ok(serde_json::Value::Null) | Err(_) => {
                Err(StateError::UndeclaredClassHash(class_hash))
            }
            Ok(value) => {
                let contract_class: starknet_rs_core::types::ContractClass =
                    serde_json::from_value(value).unwrap();
                convert_codegen_to_blockifier_compiled_class(contract_class)
            }
        }
    }
}
