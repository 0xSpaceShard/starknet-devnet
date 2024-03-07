use std::collections::HashMap;
use std::io::Read;

use blockifier::execution::contract_class::ContractClass;
use blockifier::state::cached_state::StorageEntry;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::{StateReader, StateResult};
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_types::contract_class::convert_codegen_to_blockifier_class;
use starknet_types::felt::Felt;
use starknet_types::traits::ToHexString;

use crate::starknet::starknet_config::ForkConfig;

/// A simple implementation of `StateReader` using `HashMap`s as storage.
/// Copied from blockifier test_utils, added `impl State`
#[derive(Debug, Default, Clone)]
pub struct DictState {
    pub storage_view: HashMap<StorageEntry, StarkFelt>,
    pub address_to_nonce: HashMap<ContractAddress, Nonce>,
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub class_hash_to_class: HashMap<ClassHash, ContractClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
    origin_client: Option<BlockingOriginReader>,
}

impl DictState {
    pub fn new(fork_config: ForkConfig) -> Self {
        let origin_client = if let Some(fork_url) = fork_config.url {
            Some(BlockingOriginReader::new(fork_url, fork_config.block.unwrap()))
        } else {
            None
        };
        Self { origin_client, ..Self::default() }
    }
}

#[derive(Debug, Clone)]
struct BlockingOriginReader {
    url: url::Url,
    block_number: u64,
    client: reqwest::blocking::Client,
}

fn convert_json_value_to_stark_felt(json_value: serde_json::Value) -> StarkFelt {
    StarkFelt::try_from(json_value.as_str().unwrap()).unwrap()
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
                let result = &value["result"];
                Ok(result.clone())
            }
            Err(err) => Err(err),
        }
    }
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
                "starknet_getStorage",
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
                convert_codegen_to_blockifier_class(contract_class)
            }
        }
    }
}

impl StateReader for DictState {
    fn get_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt> {
        let contract_storage_key = (contract_address, key);
        match self.storage_view.get(&contract_storage_key) {
            Some(value) => Ok(*value),
            None => {
                if let Some(origin) = &self.origin_client {
                    origin.get_storage_at(contract_address, key)
                } else {
                    Ok(Default::default())
                }
            }
        }
    }

    fn get_nonce_at(&mut self, contract_address: ContractAddress) -> StateResult<Nonce> {
        match self.address_to_nonce.get(&contract_address) {
            Some(value) => Ok(*value),
            None => {
                if let Some(origin) = &self.origin_client {
                    origin.get_nonce_at(contract_address)
                } else {
                    Ok(Default::default())
                }
            }
        }
    }

    fn get_compiled_contract_class(&mut self, class_hash: ClassHash) -> StateResult<ContractClass> {
        match self.class_hash_to_class.get(&class_hash) {
            Some(contract_class) => Ok(contract_class.clone()),
            None => {
                if let Some(origin) = &self.origin_client {
                    origin.get_compiled_contract_class(class_hash)
                } else {
                    Err(StateError::UndeclaredClassHash(class_hash))
                }
            }
        }
    }

    fn get_class_hash_at(&mut self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        match self.address_to_class_hash.get(&contract_address) {
            Some(class_hash) => Ok(*class_hash),
            None => {
                if let Some(origin) = &self.origin_client {
                    origin.get_class_hash_at(contract_address)
                } else {
                    Ok(Default::default())
                }
            }
        }
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
    ) -> StateResult<starknet_api::core::CompiledClassHash> {
        // TODO can't ask origin for this - insufficient API - probably not important
        let compiled_class_hash =
            self.class_hash_to_compiled_class_hash.get(&class_hash).copied().unwrap_or_default();
        Ok(compiled_class_hash)
    }
}

// Basing the methods on blockifier's `State` interface, without those that would never be used
// (add_visited_pcs, to_state_diff)
impl DictState {
    pub fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
        value: StarkFelt,
    ) -> std::result::Result<(), blockifier::state::errors::StateError> {
        self.storage_view.insert((contract_address, key), value);
        Ok(())
    }

    pub fn increment_nonce(&mut self, contract_address: ContractAddress) -> StateResult<()> {
        let current_nonce = self.get_nonce_at(contract_address)?;
        let current_nonce_as_u64 = usize::try_from(current_nonce.0)? as u64;
        let next_nonce_val = 1_u64 + current_nonce_as_u64;
        let next_nonce = Nonce(StarkFelt::from(next_nonce_val));
        self.address_to_nonce.insert(contract_address, next_nonce);

        Ok(())
    }

    pub fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()> {
        if contract_address == ContractAddress::default() {
            return Err(StateError::OutOfRangeContractAddress);
        }

        self.address_to_class_hash.insert(contract_address, class_hash);
        Ok(())
    }

    pub fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> StateResult<()> {
        self.class_hash_to_class.insert(class_hash, contract_class);
        Ok(())
    }

    pub fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        self.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
        Ok(())
    }
}
