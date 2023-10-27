use cairo_lang_utils::bigint::BigUintAsHex;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::Read;

use starknet_rs_core::types::{ContractClass as ContractClassStarknet, LegacyEntryPointsByType};
use starknet_in_rust::SierraContractClass;
use starknet_rs_core::types::{BlockId, FlattenedSierraClass,CompressedLegacyContractClass};
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_class::deprecated::rpc_contract_class::DeprecatedContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, CompiledClassHash, Felt};
use starknet_types::num_bigint::BigUint;
use starknet_types::patricia_key::PatriciaKey;

use crate::error::{DevnetResult, Error, StateError};

use starknet_rs_providers::jsonrpc::{HttpTransport, JsonRpcClientError};
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};

use tokio::runtime::Runtime;
use url::Url;

use starknet_rs_ff::FieldElement;

use starknet_types::contract_class::deprecated::Cairo0ContractClass;
use starknet_types::contract_class::deprecated::rpc_contract_class::ContractClassAbiEntryWithType;
use cairo_lang_starknet::contract_class::ContractEntryPoints;
use cairo_lang_starknet::abi::Contract;

// #[derive(Clone)]
pub(crate) struct DevnetState {
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub address_to_nonce: HashMap<ContractAddress, Felt>,
    pub address_to_storage: HashMap<ContractStorageKey, Felt>,
    pub class_hash_to_compiled_class: HashMap<ClassHash, ContractClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
    fork_reader: Option<ForkStateReader>,
}

impl Default for DevnetState {
    fn default() -> Self {
        Self {
            address_to_class_hash: Default::default(),
            address_to_nonce: Default::default(),
            address_to_storage: Default::default(),
            class_hash_to_compiled_class: Default::default(),
            class_hash_to_compiled_class_hash: Default::default(),
            fork_reader: None
        }
     }
}

impl Clone for DevnetState {
    fn clone(&self) -> Self {
        Self {
            address_to_class_hash: self.address_to_class_hash.clone(),
            address_to_nonce: self.address_to_nonce.clone(),
            address_to_storage: self.address_to_storage.clone(),
            class_hash_to_compiled_class: self.class_hash_to_compiled_class.clone(),
            class_hash_to_compiled_class_hash: self.class_hash_to_compiled_class_hash.clone(),
            fork_reader: None // TODO: fix it later
        }
    }
}


impl crate::traits::DevnetStateReader for DevnetState {
    fn compiled_class_hash_at(&self, class_hash: &ClassHash) -> DevnetResult<ClassHash> {
        Ok(self.class_hash_to_compiled_class_hash.get(class_hash).cloned().unwrap_or_default())
    }

    fn storage_at(&self, storage_key: &ContractStorageKey) -> DevnetResult<Felt> {
        self.address_to_storage.get(storage_key).cloned()
        .map_or_else(
            || self.fork_reader.as_ref().map_or(DevnetResult::Ok(Felt::default()), |fork| {
                fork.storage_at(storage_key)
            }),
            |r| DevnetResult::Ok(r)
        )
    }

    fn nonce_at(&self, address: &ContractAddress) -> DevnetResult<Felt> {
        self.address_to_nonce.get(address).cloned()
        .map_or_else(
            || self.fork_reader.as_ref().map_or(DevnetResult::Ok(Felt::default()), |fork| {
                fork.nonce_at(address)
            }),
            |r| DevnetResult::Ok(r)
        )
    }

    fn class_hash_at(&self, address: &ContractAddress) -> DevnetResult<ClassHash> {
        self.address_to_class_hash.get(address).cloned()
        .map_or_else(
            || self.fork_reader.as_ref().map_or(DevnetResult::Ok(Felt::default()), |fork| {
                fork.class_hash_at(address)
            }),
            |r| DevnetResult::Ok(r)
        )
    }

    fn contract_class_at(&self, class_hash: &ClassHash) -> DevnetResult<ContractClass> {
        let get_class = || {
            if let Some(deprecated_contract_class) = self.class_hash_to_compiled_class.get(class_hash) {
                Ok(deprecated_contract_class.clone())
            } else {
                let compiled_class_hash = self
                    .class_hash_to_compiled_class_hash
                    .get(class_hash)
                    .ok_or(Error::StateError(StateError::NoneCompiledHash(*class_hash)))?;

                self.class_hash_to_compiled_class
                    .get(compiled_class_hash)
                    .ok_or(Error::StateError(StateError::NoneCasmClass(*compiled_class_hash)))
                    .cloned()
            }
        };
        match get_class() {
            Ok(class) => Ok(class),
            Err(Error::StateError(StateError::NoneCompiledHash(_))) => {
                self.fork_reader.as_ref().map_or(Err(Error::StateError(StateError::NoneClassHash(*class_hash))), |fork| {
                    fork.contract_class_at(class_hash)
                })
            },
            Err(err) => Err(err)
        }
    }
}

pub struct ForkStateReader {
    client: JsonRpcClient<HttpTransport>,
    block_id: BlockId,
    runtime: Runtime,
}

impl ForkStateReader {
    #[must_use]
    pub fn new(url: &str, block_id: BlockId) -> Self {
        ForkStateReader {
            client: JsonRpcClient::new(HttpTransport::new(Url::parse(url).unwrap())),
            block_id,
            runtime: Runtime::new().expect("Could not instantiate Runtime"),
        }
    }
}

fn node_connection_error<T>() -> DevnetResult<T> {
    Err(Error::StateReadError {
        msg: "Unable to reach the node. Check your internet connection and node url".to_string(),
    })
}

// TODO: not sure where these functions should be located
fn ca_to_fe(value: &ContractAddress) -> FieldElement {
    FieldElement::from_bytes_be(&Felt252::from(*value).to_be_bytes()).unwrap()
}

fn pk_to_fe(value: &PatriciaKey) -> FieldElement {
    value.to_felt().into()
}

fn ch_to_fe(value: &ClassHash) -> FieldElement {
    (*value).into()
}

fn fsc_to_scc(flattened_class: FlattenedSierraClass) -> SierraContractClass {
    let converted_sierra_program: Vec<BigUintAsHex> = flattened_class
        .sierra_program
        .iter()
        .map(|field_element| BigUintAsHex {
            value: BigUint::from_bytes_be(&field_element.to_bytes_be()),
        })
        .collect();

    let converted_entry_points: ContractEntryPoints =
        serde_json::from_str(
            &serde_json::to_string(&flattened_class.entry_points_by_type).unwrap(),
        )
        .unwrap();

    let converted_abi: Contract = serde_json::from_str(&flattened_class.abi).unwrap();

    SierraContractClass {
        sierra_program: converted_sierra_program,
        sierra_program_debug_info: None,
        contract_class_version: flattened_class.contract_class_version,
        entry_points_by_type: converted_entry_points,
        abi: Some(converted_abi),
    }
}

fn lc_to_c0cc(legacy_class: CompressedLegacyContractClass ) -> Cairo0ContractClass {

    let converted_entry_points: LegacyEntryPointsByType =
        serde_json::from_str(
            &serde_json::to_string(&legacy_class.entry_points_by_type).unwrap(),
        )
        .unwrap();

    let converted_abi: Vec<ContractClassAbiEntryWithType> =
        serde_json::from_str(
            &serde_json::to_string(&legacy_class.abi).unwrap()
        )
        .unwrap();

    let mut decoder = GzDecoder::new(&legacy_class.program[..]);

    let mut converted_program = String::new();

    decoder.read_to_string(&mut converted_program).unwrap();

    Cairo0ContractClass::Rpc(DeprecatedContractClass {
        abi: converted_abi,
        program: serde_json::from_str(&converted_program).unwrap(),
        entry_points_by_type: converted_entry_points,
    })
}

impl crate::traits::DevnetStateReader for ForkStateReader {

    fn compiled_class_hash_at(&self, class_hash: &ClassHash) -> DevnetResult<ClassHash> {
        Err(Error::StateReadError {
            msg: "Unable to get compiled class hash from the fork".to_string(),
        })
    }

    fn storage_at(&self, contract_storage_key: &ContractStorageKey) -> DevnetResult<Felt> {
        let contract_address = contract_storage_key.get_contract_address();
        let storage_key = contract_storage_key.get_storage_key();

        match self.runtime.block_on(self.client.get_storage_at(
            ca_to_fe(contract_address),
            pk_to_fe(storage_key),
            self.block_id,
        )) {
            Ok(value) => {
                Ok(Felt::from(value))
            }
            Err(ProviderError::Other(JsonRpcClientError::TransportError(_))) => {
                node_connection_error()
            }
            Err(_) => Err(Error::StateReadError {
                msg: format!("Unable to get storage at address: {contract_address:?} and key: {storage_key:?} form fork")
            }),
        }
    }

    fn nonce_at(&self, address: &ContractAddress) -> DevnetResult<Felt> {
        match self.runtime.block_on(
            self.client
                .get_nonce(self.block_id, ca_to_fe(address)),
        ) {
            Ok(nonce) => {
                Ok(Felt::from(nonce))
            }
            Err(ProviderError::Other(JsonRpcClientError::TransportError(_))) => {
                node_connection_error()
            }
            Err(_) => Err(Error::StateReadError {
                msg: format!("Unable to get nonce at {address:?} from fork")
            }),
        }
    }

    fn class_hash_at(&self, address: &ContractAddress) -> DevnetResult<ClassHash> {
        match self.runtime.block_on(
            self.client
                .get_class_hash_at(self.block_id, ca_to_fe(address)),
        ) {
            Ok(class_hash) => {
                Ok(Felt::from(class_hash))
            }
            Err(ProviderError::Other(JsonRpcClientError::TransportError(_))) => {
                node_connection_error()
            }
            Err(_) => Err(Error::StateReadError {
                msg: format!("Unable to get class hash at {address:?} from fork")
            }),
        }
    }

    fn contract_class_at(&self, class_hash: &ClassHash) -> DevnetResult<ContractClass> {
        let contract_class = match self.runtime.block_on(
            self.client.get_class(self.block_id, ch_to_fe(class_hash)),
        ) {
            Ok(contract_class) => Ok(contract_class),
            Err(ProviderError::Other(JsonRpcClientError::TransportError(_))) => {
                node_connection_error()
            }
            Err(_) => Err(Error::StateReadError {
                msg: format!("Unable to get class as class_hash {class_hash:?}")
            }),
        };

        Ok(match contract_class? {
            ContractClassStarknet::Sierra(flattened_class) =>
                ContractClass::Cairo1(fsc_to_scc(flattened_class)),
            ContractClassStarknet::Legacy(legacy_class) =>
                ContractClass::Cairo0(lc_to_c0cc(legacy_class))
        })
    }
}
