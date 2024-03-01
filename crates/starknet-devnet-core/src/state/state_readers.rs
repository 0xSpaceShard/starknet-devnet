use std::collections::HashMap;
use std::sync::Arc;

use blockifier::execution::contract_class::ContractClass;
use blockifier::state::cached_state::StorageEntry;
use blockifier::state::errors::StateError;
use blockifier::state::state_api::{State, StateReader, StateResult};
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_rs_core::types::BlockId;
use starknet_rs_ff::FieldElement;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_types::felt::Felt;

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
    block_id: Option<BlockId>,
    origin_client: Option<Arc<JsonRpcClient<HttpTransport>>>,
}

impl DictState {
    pub fn new(fork_config: ForkConfig) -> Self {
        let (origin_client, block_id) = if let Some(fork_url) = fork_config.url {
            (
                Some(Arc::new(JsonRpcClient::new(HttpTransport::new(fork_url)))),
                Some(BlockId::Number(fork_config.block.unwrap())),
            )
        } else {
            (None, None)
        };
        Self { origin_client, block_id, ..Self::default() }
    }
}

impl StateReader for DictState {
    fn get_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt> {
        let contract_storage_key = (contract_address, key);
        Ok(match self.storage_view.get(&contract_storage_key) {
            Some(value) => *value,
            None => {
                if let Some(origin) = &self.origin_client {
                    let contract_address = FieldElement::from(Felt::from(contract_address.0));
                    let key = FieldElement::from(Felt::from(key.0));
                    let origin_result = tokio::runtime::Runtime::new().unwrap().block_on(
                        origin.get_storage_at(contract_address, key, self.block_id.unwrap()),
                    );
                    match origin_result {
                        Ok(value) => value.into(),
                        // TODO better error granularity
                        Err(_) => Default::default(),
                    }
                } else {
                    Default::default()
                }
            }
        })
    }

    fn get_nonce_at(&mut self, contract_address: ContractAddress) -> StateResult<Nonce> {
        let nonce = self.address_to_nonce.get(&contract_address).copied().unwrap_or_default();
        Ok(nonce)
    }

    fn get_compiled_contract_class(&mut self, class_hash: ClassHash) -> StateResult<ContractClass> {
        let contract_class = self.class_hash_to_class.get(&class_hash).cloned();
        match contract_class {
            Some(contract_class) => Ok(contract_class),
            _ => Err(StateError::UndeclaredClassHash(class_hash)),
        }
    }

    fn get_class_hash_at(&mut self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        let class_hash =
            self.address_to_class_hash.get(&contract_address).copied().unwrap_or_default();
        Ok(class_hash)
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
    ) -> StateResult<starknet_api::core::CompiledClassHash> {
        let compiled_class_hash =
            self.class_hash_to_compiled_class_hash.get(&class_hash).copied().unwrap_or_default();
        Ok(compiled_class_hash)
    }
}

impl State for DictState {
    fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
        value: StarkFelt,
    ) -> std::result::Result<(), blockifier::state::errors::StateError> {
        self.storage_view.insert((contract_address, key), value);
        Ok(())
    }

    fn increment_nonce(&mut self, contract_address: ContractAddress) -> StateResult<()> {
        let current_nonce = self.get_nonce_at(contract_address)?;
        let current_nonce_as_u64 = usize::try_from(current_nonce.0)? as u64;
        let next_nonce_val = 1_u64 + current_nonce_as_u64;
        let next_nonce = Nonce(StarkFelt::from(next_nonce_val));
        self.address_to_nonce.insert(contract_address, next_nonce);

        Ok(())
    }

    fn set_class_hash_at(
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

    fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> StateResult<()> {
        self.class_hash_to_class.insert(class_hash, contract_class);
        Ok(())
    }

    fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        self.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
        Ok(())
    }

    fn to_state_diff(&mut self) -> blockifier::state::cached_state::CommitmentStateDiff {
        panic!("Should never be called")
    }

    fn add_visited_pcs(&mut self, _class_hash: ClassHash, _pcs: &std::collections::HashSet<usize>) {
        todo!("What with this")
    }
}
