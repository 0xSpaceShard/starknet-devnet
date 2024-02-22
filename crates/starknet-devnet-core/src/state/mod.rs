use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::{State, StateReader};
use starknet_api::core::CompiledClassHash;
use starknet_api::hash::StarkFelt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt};

use self::state_diff::StateDiff;
use self::state_readers::DictState;
use crate::error::{DevnetResult, Error};

pub(crate) mod state_diff;
mod state_readers;
pub mod state_update;

pub trait CustomStateReader {
    fn is_contract_deployed(&mut self, contract_address: ContractAddress) -> bool;
    fn is_contract_declared(&mut self, class_hash: ClassHash) -> bool;
    /// sierra for cairo1, only artifact for cairo0
    fn get_rpc_contract_class(&self, class_hash: &ClassHash) -> Option<&ContractClass>;
}

pub trait CustomState {
    fn predeclare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()>;
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()>;
    fn predeploy_contract(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()>;
}

#[derive(Default, Clone)]
/// Utility structure that makes it easier to calculate state diff later on
pub struct CommittedClassStorage {
    staging: HashMap<ClassHash, ContractClass>,
    committed: HashMap<ClassHash, ContractClass>,
}

impl CommittedClassStorage {
    pub fn insert(&mut self, class_hash: ClassHash, contract_class: ContractClass) {
        self.staging.insert(class_hash, contract_class);
    }

    pub fn commit(&mut self) -> HashMap<ClassHash, ContractClass> {
        let diff = self.staging.clone();
        self.committed.extend(self.staging.drain());
        diff
    }

    /// Skips the staging phase
    fn insert_and_commit(&mut self, class_hash: ClassHash, contract_class: ContractClass) {
        assert!(self.staging.is_empty());
        self.insert(class_hash, contract_class);
        self.commit();
    }
}

pub(crate) struct StarknetState {
    pub(crate) state: CachedState<DictState>,
    rpc_contract_classes: CommittedClassStorage,
    /// `None` indicates the state hasn't yet been cloned for old-state preservation purpose
    historic_state: Option<DictState>,
}

impl Default for StarknetState {
    fn default() -> Self {
        Self {
            state: CachedState::new(Default::default(), Default::default()),
            rpc_contract_classes: Default::default(),
            historic_state: Default::default(),
        }
    }
}

impl StarknetState {
    pub fn clone_rpc_contract_classes(&self) -> CommittedClassStorage {
        self.rpc_contract_classes.clone()
    }

    pub fn commit_full_state_and_get_diff(&mut self) -> DevnetResult<StateDiff> {
        StateDiff::generate_commit(&mut self.state, &mut self.rpc_contract_classes)
    }

    pub fn assert_contract_deployed(
        &mut self,
        contract_address: ContractAddress,
    ) -> DevnetResult<()> {
        if !self.is_contract_deployed(contract_address) {
            return Err(Error::ContractNotFound);
        }
        Ok(())
    }

    /// Expands the internal historic state copy and returns a StarknetState wrapper of it,
    /// together with current contract_classes
    pub fn expand_historic(&mut self, state_diff: StateDiff) -> DevnetResult<Self> {
        if self.historic_state.is_none() {
            // to have predeployed content in the historic state; done only once
            self.historic_state = Some(self.state.state.clone());
        }
        let mut historic_state = self.historic_state.as_ref().unwrap().clone();

        for (address, class_hash) in state_diff.address_to_class_hash {
            historic_state.set_class_hash_at(address.try_into()?, class_hash.into())?;
        }
        for (class_hash, casm_hash) in state_diff.class_hash_to_compiled_class_hash {
            historic_state.set_compiled_class_hash(class_hash.into(), casm_hash.into())?;
        }
        for (address, _nonce) in state_diff.address_to_nonce {
            // assuming that historic_state.get_nonce(address) == _nonce - 1
            historic_state.increment_nonce(address.try_into()?)?;
        }
        for (address, storage_updates) in state_diff.storage_updates {
            let core_address = address.try_into()?;
            for (key, value) in storage_updates {
                historic_state.set_storage_at(core_address, key.try_into()?, value.into());
            }
        }
        for class_hash in state_diff.cairo_0_declared_contracts {
            let compiled_class = self.get_compiled_contract_class(&class_hash.into())?;
            historic_state.set_contract_class(&class_hash.into(), compiled_class)?;
        }
        for class_hash in state_diff.declared_contracts {
            let compiled_class = self.get_compiled_contract_class(&class_hash.into())?;
            historic_state.set_contract_class(&class_hash.into(), compiled_class)?;
        }

        self.historic_state = Some(historic_state.clone());

        Ok(Self {
            state: CachedState::new(historic_state, Default::default()),
            rpc_contract_classes: self.rpc_contract_classes.clone(),
            historic_state: None,
        })
    }
}

impl State for StarknetState {
    fn set_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
        value: starknet_api::hash::StarkFelt,
    ) {
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
        class_hash: &starknet_api::core::ClassHash,
        contract_class: blockifier::execution::contract_class::ContractClass,
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

    fn to_state_diff(&mut self) -> blockifier::state::cached_state::CommitmentStateDiff {
        self.state.to_state_diff()
    }
}

impl blockifier::state::state_api::StateReader for StarknetState {
    fn get_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> blockifier::state::state_api::StateResult<starknet_api::hash::StarkFelt> {
        self.state.get_storage_at(contract_address, key)
    }

    fn get_nonce_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::Nonce> {
        self.state.get_nonce_at(contract_address)
    }

    fn get_class_hash_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::ClassHash> {
        self.state.get_class_hash_at(contract_address)
    }

    fn get_compiled_contract_class(
        &mut self,
        class_hash: &starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<
        blockifier::execution::contract_class::ContractClass,
    > {
        self.state.get_compiled_contract_class(class_hash)
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        self.state.get_compiled_class_hash(class_hash)
    }
}

impl CustomStateReader for StarknetState {
    fn is_contract_deployed(&mut self, contract_address: ContractAddress) -> bool {
        let api_address = contract_address.try_into().unwrap();
        self.get_class_hash_at(api_address)
            .is_ok_and(|starknet_api::core::ClassHash(hash)| hash != StarkFelt::ZERO)
    }

    fn is_contract_declared(&mut self, class_hash: ClassHash) -> bool {
        self.get_compiled_class_hash(class_hash.into())
            .is_ok_and(|CompiledClassHash(class_hash)| class_hash != StarkFelt::ZERO)
            || self.get_compiled_contract_class(&class_hash.into()).is_ok()
    }

    fn get_rpc_contract_class(&self, class_hash: &ClassHash) -> Option<&ContractClass> {
        self.rpc_contract_classes.committed.get(class_hash)
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

        if let ContractClass::Cairo1(_) = contract_class {
            let cairo_lang_compiled_class: cairo_lang_starknet::casm_contract_class::CasmContractClass =
                contract_class.clone().try_into()?;
            let casm_hash =
                Felt::new(cairo_lang_compiled_class.compiled_class_hash().to_be_bytes())?;
            self.state.state.set_compiled_class_hash(class_hash.into(), casm_hash.into())?;
        };

        self.state.state.set_contract_class(&class_hash.into(), compiled_class)?;
        self.rpc_contract_classes.insert_and_commit(class_hash, contract_class);
        Ok(())
    }

    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()> {
        let compiled_class = contract_class.clone().try_into()?;

        if let ContractClass::Cairo1(_) = contract_class {
            let cairo_lang_compiled_class: cairo_lang_starknet::casm_contract_class::CasmContractClass =
                contract_class.clone().try_into()?;
            let casm_hash =
                Felt::new(cairo_lang_compiled_class.compiled_class_hash().to_be_bytes())?;
            self.set_compiled_class_hash(class_hash.into(), casm_hash.into())?;
        };

        self.set_contract_class(&class_hash.into(), compiled_class)?;
        self.rpc_contract_classes.insert(class_hash, contract_class);
        Ok(())
    }

    fn predeploy_contract(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        self.state
            .state
            .set_class_hash_at(contract_address.try_into()?, class_hash.into())
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use blockifier::state::errors::StateError;
    use blockifier::state::state_api::{State, StateReader};
    use starknet_api::core::Nonce;
    use starknet_api::hash::StarkFelt;
    use starknet_api::state::StorageKey;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::felt::Felt;

    use super::StarknetState;
    use crate::state::{CustomState, CustomStateReader};
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
        let contract_class = ContractClass::Cairo0(dummy_cairo_0_contract_class().into());

        state.declare_contract_class(class_hash, contract_class).unwrap();
        assert!(state.is_contract_declared(dummy_felt()));
    }

    #[test]
    fn apply_state_updates_for_storage_successfully() {
        let mut state = StarknetState::default();
        let (contract_address, storage_key) = dummy_contract_storage_key();

        let storage_before = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(storage_before, StarkFelt::ZERO);

        state
            .state
            .set_class_hash_at(contract_address, starknet_api::core::ClassHash(dummy_felt().into()))
            .unwrap();

        state.state.set_storage_at(contract_address, storage_key, dummy_felt().into());
        state.commit_full_state_and_get_diff().unwrap();

        let storage_after = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(storage_after, dummy_felt().into());
    }

    #[test]
    fn apply_state_updates_for_address_nonce_successfully() {
        let mut state = StarknetState::default();

        state.predeploy_contract(dummy_contract_address(), dummy_felt()).unwrap();
        let contract_address = dummy_contract_address().try_into().unwrap();

        // should be zero before update
        assert_eq!(state.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::ZERO));

        state.state.increment_nonce(contract_address).unwrap();
        state.commit_full_state_and_get_diff().unwrap();

        // check if nonce update was correct
        assert_eq!(state.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::ONE));
    }

    #[test]
    fn declare_cairo_0_contract_class_successfully() {
        let mut state = StarknetState::default();
        let class_hash = Felt::from_prefixed_hex_str("0xFE").unwrap();

        match state.get_compiled_contract_class(&class_hash.into()) {
            Err(StateError::UndeclaredClassHash(reported_hash)) => {
                assert_eq!(reported_hash, class_hash.into())
            }
            other => panic!("Invalid result: {other:?}"),
        }

        let contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();
        state
            .declare_contract_class(class_hash, contract_class.clone().try_into().unwrap())
            .unwrap();

        state.commit_full_state_and_get_diff().unwrap();

        match state.get_compiled_contract_class(&class_hash.into()) {
            Ok(blockifier::execution::contract_class::ContractClass::V0(retrieved_class)) => {
                assert_eq!(retrieved_class, contract_class.clone().try_into().unwrap());
            }
            other => panic!("Invalid result: {other:?}"),
        }

        let retrieved_rpc_class = state.get_rpc_contract_class(&class_hash).unwrap();
        assert_eq!(retrieved_rpc_class, &contract_class.into());
    }

    #[test]
    fn deploy_cairo_0_contract_class_successfully() {
        let (mut state, address) = setup();
        let felt = dummy_felt();

        state.predeploy_contract(address, felt).unwrap();
        let core_address = address.try_into().unwrap();
        assert_eq!(state.get_nonce_at(core_address).unwrap(), Nonce(StarkFelt::ZERO));
    }

    #[test]
    fn change_storage_successfully() {
        let mut state = StarknetState::default();
        let (contract_address, storage_key) = dummy_contract_storage_key();
        let storage_value = dummy_felt();

        state.set_storage_at(contract_address, storage_key, storage_value.into());
        assert_eq!(
            state.get_storage_at(contract_address, storage_key).unwrap(),
            storage_value.into()
        );
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        let core_address = address.try_into().unwrap();
        state.increment_nonce(core_address).unwrap();

        let nonce = *state.get_nonce_at(core_address).unwrap();
        assert_eq!(nonce, StarkFelt::ONE)
    }

    #[test]
    fn read_from_storage_returns_correct_result() {
        let (mut state, _) = setup();
        let expected_result = StarkFelt::from(33_u32);
        let (contract_address, storage_key) = dummy_contract_storage_key();
        let class_hash = dummy_felt();

        state.predeploy_contract(contract_address.into(), class_hash).unwrap();

        state.set_storage_at(contract_address, storage_key, expected_result);
        let generated_result = state.get_storage_at(contract_address, storage_key).unwrap();
        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn get_nonce_should_return_zerp_when_contract_not_deployed() {
        let (mut state, _) = setup();

        let dummy_address = starknet_api::core::ContractAddress::from(1_u32);
        match state.get_nonce_at(dummy_address) {
            Ok(Nonce(StarkFelt::ZERO)) => {}
            other => panic!("Invalid nonce: {other:?}"),
        }
    }

    #[test]
    fn get_nonce_should_return_zero_for_freshly_deployed_contract() {
        let (mut state, address) = setup();
        let core_address = address.try_into().unwrap();
        assert_eq!(state.get_nonce_at(core_address).unwrap(), Nonce(StarkFelt::ZERO));
    }

    fn setup() -> (StarknetState, ContractAddress) {
        let mut state = StarknetState::default();
        let address = dummy_contract_address();
        let contract_class = dummy_cairo_0_contract_class();
        let class_hash = dummy_felt();

        state.declare_contract_class(class_hash, contract_class.into()).unwrap();
        state.predeploy_contract(address, class_hash).unwrap();

        (state, address)
    }
}
