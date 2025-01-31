use std::collections::HashMap;
use std::sync::Arc;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::{State, StateReader};
use parking_lot::RwLock;
use starknet_api::core::CompiledClassHash;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;

use self::state_diff::StateDiff;
use self::state_readers::DictState;
use crate::error::{DevnetResult, Error};
use crate::starknet::defaulter::StarknetDefaulter;
use crate::utils::calculate_casm_hash;

pub(crate) mod state_diff;
pub(crate) mod state_readers;

pub enum BlockNumberOrPending {
    Pending,
    Number(u64),
}

pub trait CustomStateReader {
    fn is_contract_deployed(&self, contract_address: ContractAddress) -> DevnetResult<bool>;
    /// using is_contract_deployed with forked state returns that the contract is deployed on the
    /// forked side and a validation cannot be skipped when creating a transaction with
    /// impersonated account
    fn is_contract_deployed_locally(&self, contract_address: ContractAddress)
    -> DevnetResult<bool>;
    fn is_contract_declared(&self, class_hash: ClassHash) -> bool;
}

pub trait CustomState {
    /// Link class with its hash; if cairo1 class: calculate casm hash, link class hash with it
    fn predeclare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()>;

    /// Link class with its hash; if cairo1 class: link class hash with casm hash
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        casm_hash: Option<starknet_types::felt::CompiledClassHash>,
        contract_class: ContractClass,
    ) -> DevnetResult<()>;

    /// Link contract address to class hash
    fn predeploy_contract(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()>;
}

#[derive(Default, Clone)]
/// Utility structure that makes it easier to calculate state diff later on. Classes are first
/// inserted into the staging area (pending state), later to be committed (assigned a block number
/// to mark when they were added). Committed doesn't necessarily mean the class is a part of the
/// latest state, just that it is bound to be. Since there is no way of telling if a class is in the
/// latest state or no, retrieving from the latest state has to be done via block number.
pub struct CommittedClassStorage {
    staging: HashMap<ClassHash, ContractClass>,
    committed: HashMap<ClassHash, (ContractClass, u64)>,
    /// Remembers all classes committed at a block
    block_number_to_classes: HashMap<u64, Vec<ClassHash>>,
}

impl CommittedClassStorage {
    /// Insert a new class into the staging area. Once it can be committed, call `commit`.
    pub fn insert(&mut self, class_hash: ClassHash, contract_class: ContractClass) {
        self.staging.insert(class_hash, contract_class);
    }

    /// Commits all of the staged classes and returns them, together with their hashes.
    pub fn commit(&mut self, block_number: u64) -> HashMap<ClassHash, ContractClass> {
        let mut newly_committed = HashMap::new();

        let hashes_at_this_block = self.block_number_to_classes.entry(block_number).or_default();
        for (class_hash, class) in &self.staging {
            newly_committed.insert(*class_hash, class.clone());
            self.committed.insert(*class_hash, (class.clone(), block_number));

            hashes_at_this_block.push(*class_hash);
        }

        self.empty_staging();
        newly_committed
    }

    /// Returns sierra for cairo1; returns the only artifact for cairo0.
    pub fn get_class(
        &self,
        class_hash: &ClassHash,
        block_number_or_pending: &BlockNumberOrPending,
    ) -> Option<ContractClass> {
        if let Some((class, storage_block_number)) = self.committed.get(class_hash) {
            // If we're here, the requested class was committed at some point, need to see when.
            match block_number_or_pending {
                BlockNumberOrPending::Number(query_block_number) => {
                    // If the class was stored before the block at which we are querying (or at that
                    // block), we can return it.
                    if storage_block_number <= query_block_number {
                        Some(class.clone())
                    } else {
                        None
                    }
                }
                BlockNumberOrPending::Pending => {
                    // Class is requested at block_id=pending. Since it's present among the
                    // committed classes, it's in the latest block or older and can be returned.
                    Some(class.clone())
                }
            }
        } else if let Some(class) = self.staging.get(class_hash) {
            // If class present in storage.staging, it can only be retrieved if block_id=pending
            match block_number_or_pending {
                BlockNumberOrPending::Pending => Some(class.clone()),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Removes all classes committed at `block_number`. If no classes were committed at that block,
    /// does nothing.
    pub fn remove_classes_at(&mut self, block_number: u64) {
        if let Some(removable) = self.block_number_to_classes.remove(&block_number) {
            for class_hash in removable {
                self.committed.remove(&class_hash);
            }
        }
    }

    /// Removes all staged classes.
    pub fn empty_staging(&mut self) {
        self.staging = Default::default();
    }
}

pub struct StarknetState {
    pub(crate) state: CachedState<DictState>,
    /// The class storage is meant to be shared between states to prevent copying (due to memory
    /// concerns). Knowing which class was added when is made possible by storing the class
    /// together with the block number.
    rpc_contract_classes: Arc<RwLock<CommittedClassStorage>>,
    /// Used for old state preservation purposes.
    historic_state: DictState,
}

impl Default for StarknetState {
    fn default() -> Self {
        Self {
            state: CachedState::new(Default::default()),
            rpc_contract_classes: Default::default(),
            historic_state: Default::default(),
        }
    }
}

impl StarknetState {
    pub fn new(
        defaulter: StarknetDefaulter,
        rpc_contract_classes: Arc<RwLock<CommittedClassStorage>>,
    ) -> Self {
        Self {
            state: CachedState::new(DictState::new(defaulter)),
            rpc_contract_classes,
            historic_state: Default::default(),
        }
    }

    pub fn clone_rpc_contract_classes(&self) -> CommittedClassStorage {
        self.rpc_contract_classes.read().clone()
    }

    /// Commits and returns the state difference accumulated since the previous (historic) state.
    pub(crate) fn commit_diff(&mut self, block_number: u64) -> DevnetResult<StateDiff> {
        let new_classes = self.rpc_contract_classes.write().commit(block_number);

        let diff = StateDiff::generate(&mut self.state, new_classes)?;
        let new_historic = self.expand_historic(diff.clone())?;
        self.state = CachedState::new(new_historic.clone());

        Ok(diff)
    }

    pub fn assert_contract_deployed(
        &mut self,
        contract_address: ContractAddress,
    ) -> DevnetResult<()> {
        if !self.is_contract_deployed(contract_address)? {
            return Err(Error::ContractNotFound);
        }
        Ok(())
    }

    /// Expands the internal historic state copy and returns a reference to it
    fn expand_historic(&mut self, state_diff: StateDiff) -> DevnetResult<&DictState> {
        let mut historic_state = self.state.state.clone();

        for (address, class_hash) in state_diff.address_to_class_hash {
            historic_state.set_class_hash_at(
                address.try_into()?,
                starknet_api::core::ClassHash(class_hash),
            )?;
        }
        for (class_hash, casm_hash) in state_diff.class_hash_to_compiled_class_hash {
            historic_state.set_compiled_class_hash(
                starknet_api::core::ClassHash(class_hash),
                starknet_api::core::CompiledClassHash(casm_hash),
            )?;
        }
        for (address, _nonce) in state_diff.address_to_nonce {
            // assuming that historic_state.get_nonce(address) == _nonce - 1
            historic_state.increment_nonce(address.try_into()?)?;
        }
        for (address, storage_updates) in state_diff.storage_updates {
            let core_address = address.try_into()?;
            for (key, value) in storage_updates {
                historic_state.set_storage_at(core_address, key.try_into()?, value)?;
            }
        }
        for class_hash in state_diff.cairo_0_declared_contracts {
            let class_hash = starknet_api::core::ClassHash(class_hash);
            let compiled_class = self.get_compiled_class(class_hash)?;
            historic_state.set_contract_class(class_hash, compiled_class)?;
        }
        for class_hash in state_diff.declared_contracts {
            let class_hash = starknet_api::core::ClassHash(class_hash);
            let compiled_class = self.get_compiled_class(class_hash)?;
            historic_state.set_contract_class(class_hash, compiled_class)?;
        }
        self.historic_state = historic_state;
        Ok(&self.historic_state)
    }

    pub fn clone_historic(&self) -> Self {
        Self {
            state: CachedState::new(self.historic_state.clone()),
            rpc_contract_classes: self.rpc_contract_classes.clone(),
            historic_state: self.historic_state.clone(),
        }
    }
}

impl State for StarknetState {
    fn set_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
        value: Felt,
    ) -> std::result::Result<(), blockifier::state::errors::StateError> {
        self.state.set_storage_at(contract_address, key, value)
    }

    fn increment_nonce(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<()> {
        self.state.increment_nonce(contract_address)
    }

    fn set_class_hash_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<()> {
        self.state.set_class_hash_at(contract_address, class_hash)
    }

    fn set_contract_class(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
        contract_class: blockifier::execution::contract_class::RunnableCompiledClass,
    ) -> blockifier::state::state_api::StateResult<()> {
        self.state.set_contract_class(class_hash, contract_class)
    }

    fn set_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
        compiled_class_hash: starknet_api::core::CompiledClassHash,
    ) -> blockifier::state::state_api::StateResult<()> {
        self.state.set_compiled_class_hash(class_hash, compiled_class_hash)
    }
}

impl blockifier::state::state_api::StateReader for StarknetState {
    fn get_storage_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> blockifier::state::state_api::StateResult<Felt> {
        self.state.get_storage_at(contract_address, key)
    }

    fn get_nonce_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::Nonce> {
        self.state.get_nonce_at(contract_address)
    }

    fn get_class_hash_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::ClassHash> {
        self.state.get_class_hash_at(contract_address)
    }

    fn get_compiled_class(
        &self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<
        blockifier::execution::contract_class::RunnableCompiledClass,
    > {
        self.state.get_compiled_class(class_hash)
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        self.state.get_compiled_class_hash(class_hash)
    }
}

impl CustomStateReader for StarknetState {
    fn is_contract_deployed(&self, contract_address: ContractAddress) -> DevnetResult<bool> {
        let api_address = contract_address.try_into()?;
        let starknet_api::core::ClassHash(class_hash) = self.get_class_hash_at(api_address)?;
        Ok(class_hash != Felt::ZERO)
    }

    fn is_contract_declared(&self, class_hash: ClassHash) -> bool {
        // get_compiled_class is important if forking; checking hash is impossible via JSON-RPC
        let class_hash = starknet_api::core::ClassHash(class_hash);
        self.get_compiled_class_hash(class_hash)
            .is_ok_and(|CompiledClassHash(class_hash)| class_hash != Felt::ZERO)
            || self.get_compiled_class(class_hash).is_ok()
    }

    fn is_contract_deployed_locally(
        &self,
        contract_address: ContractAddress,
    ) -> DevnetResult<bool> {
        let api_address = contract_address.try_into()?;
        Ok(self.state.state.address_to_class_hash.contains_key(&api_address))
    }
}

impl CustomState for StarknetState {
    /// writes directly to the most underlying state, skipping cache
    fn predeclare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()> {
        let compiled_class = contract_class.clone().try_into()?;
        let class_hash = starknet_api::core::ClassHash(class_hash);

        if let ContractClass::Cairo1(cairo_lang_contract_class) = &contract_class {
            let casm_json = usc::compile_contract(
                serde_json::to_value(cairo_lang_contract_class)
                    .map_err(|err| Error::SerializationError { origin: err.to_string() })?,
            )
            .map_err(|err| {
                Error::TypesError(starknet_types::error::Error::SierraCompilationError {
                    reason: err.to_string(),
                })
            })?;

            let casm_hash = starknet_api::core::CompiledClassHash(calculate_casm_hash(casm_json)?);

            self.state.state.set_compiled_class_hash(class_hash, casm_hash)?;
        };

        self.state.state.set_contract_class(class_hash, compiled_class)?;
        let mut class_storage = self.rpc_contract_classes.write();
        class_storage.insert(*class_hash, contract_class);
        Ok(())
    }

    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        casm_hash: Option<starknet_types::felt::CompiledClassHash>,
        contract_class: ContractClass,
    ) -> DevnetResult<()> {
        let compiled_class = contract_class.clone().try_into()?;

        let class_hash = starknet_api::core::ClassHash(class_hash);
        if let Some(casm_hash) = casm_hash {
            self.set_compiled_class_hash(
                class_hash,
                starknet_api::core::CompiledClassHash(casm_hash),
            )?;
        };

        self.set_contract_class(class_hash, compiled_class)?;
        let mut class_storage = self.rpc_contract_classes.write();
        class_storage.insert(*class_hash, contract_class);
        Ok(())
    }

    fn predeploy_contract(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        self.state
            .state
            .set_class_hash_at(
                contract_address.try_into()?,
                starknet_api::core::ClassHash(class_hash),
            )
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use blockifier::state::errors::StateError;
    use blockifier::state::state_api::{State, StateReader};
    use starknet_api::core::Nonce;
    use starknet_api::state::StorageKey;
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;

    use super::StarknetState;
    use crate::state::{BlockNumberOrPending, CustomState, CustomStateReader};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{dummy_contract_address, dummy_felt};

    pub(crate) fn dummy_contract_storage_key() -> (starknet_api::core::ContractAddress, StorageKey)
    {
        (0xfe_u128.into(), 0xdd10_u128.into())
    }

    #[test]
    fn test_class_present_after_declaration() {
        let mut state = StarknetState::default();

        let class_hash = dummy_felt();
        let casm_hash = Some(dummy_felt());
        let contract_class = ContractClass::Cairo0(dummy_cairo_0_contract_class());

        state.declare_contract_class(class_hash, casm_hash, contract_class).unwrap();
        assert!(state.is_contract_declared(dummy_felt()));
    }

    #[test]
    fn apply_state_updates_for_storage_successfully() {
        let mut state = StarknetState::default();
        let (contract_address, storage_key) = dummy_contract_storage_key();

        let storage_before = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(storage_before, Felt::ZERO);

        state.set_storage_at(contract_address, storage_key, dummy_felt()).unwrap();

        let storage_after = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(storage_after, dummy_felt());
    }

    #[test]
    fn apply_state_updates_for_address_nonce_successfully() {
        let mut state = StarknetState::default();

        state.predeploy_contract(dummy_contract_address(), dummy_felt()).unwrap();
        let contract_address = dummy_contract_address().try_into().unwrap();

        // should be zero before update
        assert_eq!(state.get_nonce_at(contract_address).unwrap(), Nonce(Felt::ZERO));

        state.state.increment_nonce(contract_address).unwrap();
        state.commit_diff(1).unwrap();

        // check if nonce update was correct
        assert_eq!(state.get_nonce_at(contract_address).unwrap(), Nonce(Felt::ONE));
    }

    #[test]
    fn declare_cairo_0_contract_class_successfully() {
        let mut state = StarknetState::default();
        let class_hash = starknet_api::core::ClassHash(Felt::from_hex_unchecked("0xFE"));
        let casm_hash = Some(dummy_felt());

        match state.get_compiled_class(class_hash) {
            Err(StateError::UndeclaredClassHash(reported_hash)) => {
                assert_eq!(reported_hash, class_hash);
            }
            other => panic!("Invalid result: {other:?}"),
        }

        let contract_class = dummy_cairo_0_contract_class();
        state
            .declare_contract_class(class_hash.0, casm_hash, contract_class.clone().into())
            .unwrap();

        let block_number = 1;
        state.commit_diff(block_number).unwrap();

        match state.get_compiled_class(class_hash) {
            Ok(retrieved_class) => {
                assert_eq!(retrieved_class, contract_class.clone().try_into().unwrap());
            }
            other => panic!("Invalid result: {other:?}"),
        }

        let retrieved_rpc_class = state
            .rpc_contract_classes
            .read()
            .get_class(&class_hash, &BlockNumberOrPending::Number(block_number))
            .unwrap();
        assert_eq!(retrieved_rpc_class, contract_class.into());
    }

    #[test]
    fn deploy_cairo_0_contract_class_successfully() {
        let (mut state, address) = setup();
        let felt = dummy_felt();

        state.predeploy_contract(address, felt).unwrap();
        let core_address = address.try_into().unwrap();
        assert_eq!(state.get_nonce_at(core_address).unwrap(), Nonce(Felt::ZERO));
    }

    #[test]
    fn change_storage_successfully() {
        let mut state = StarknetState::default();
        let (contract_address, storage_key) = dummy_contract_storage_key();
        let storage_value = dummy_felt();

        state.set_storage_at(contract_address, storage_key, storage_value).unwrap();
        assert_eq!(state.get_storage_at(contract_address, storage_key).unwrap(), storage_value);
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        let core_address = address.try_into().unwrap();
        state.increment_nonce(core_address).unwrap();

        let nonce = *state.get_nonce_at(core_address).unwrap();
        assert_eq!(nonce, Felt::ONE)
    }

    #[test]
    fn read_from_storage_returns_correct_result() {
        let (mut state, _) = setup();
        let expected_result = Felt::from(33_u32);
        let (contract_address, storage_key) = dummy_contract_storage_key();
        let class_hash = dummy_felt();

        state.predeploy_contract(contract_address.into(), class_hash).unwrap();

        state.set_storage_at(contract_address, storage_key, expected_result).unwrap();
        let generated_result = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn get_nonce_should_return_zero_when_contract_not_deployed() {
        let (state, _) = setup();

        let dummy_address = starknet_api::core::ContractAddress::from(1_u32);
        match state.get_nonce_at(dummy_address) {
            Ok(Nonce(n)) => assert_eq!(n, Felt::ZERO),
            other => panic!("Invalid nonce: {other:?}"),
        }
    }

    #[test]
    fn get_nonce_should_return_zero_for_freshly_deployed_contract() {
        let (state, address) = setup();
        let core_address = address.try_into().unwrap();
        assert_eq!(state.get_nonce_at(core_address).unwrap(), Nonce(Felt::ZERO));
    }

    fn setup() -> (StarknetState, ContractAddress) {
        let mut state = StarknetState::default();
        let address = dummy_contract_address();
        let contract_class = dummy_cairo_0_contract_class();
        let class_hash = dummy_felt();
        let casm_hash = Some(dummy_felt());

        state.declare_contract_class(class_hash, casm_hash, contract_class.into()).unwrap();
        state.predeploy_contract(address, class_hash).unwrap();

        (state, address)
    }
}
