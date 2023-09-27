use std::collections::HashMap;
use std::sync::Arc;

use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::state::cached_state::CachedState;
use starknet_in_rust::state::in_memory_state_reader::InMemoryStateReader;
use starknet_in_rust::state::state_api::StateReader;
use starknet_in_rust::utils::Address;
use starknet_in_rust::CasmContractClass;
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, Felt};

use self::state_diff::StateDiff;
use crate::error::{DevnetResult, Error};
use crate::traits::{StateChanger, StateExtractor};

pub(crate) mod state_diff;
pub mod state_update;

#[derive(Debug, Default)]
pub(crate) struct StarknetState {
    pub state: CachedState<InMemoryStateReader>,
    pub(crate) contract_classes: HashMap<ClassHash, ContractClass>,
}

impl StarknetState {
    /// this method clears the state from data that was accumulated in the StateCache
    /// and restores it to the data in the state_reader, which is the "persistent" data
    pub(crate) fn clear_dirty_state(&mut self) {
        self.state = CachedState::new(
            self.state.state_reader.clone(),
            self.state.state_reader.class_hash_to_compiled_class.clone(),
        );
    }

    /// this method is making deep copy of the object
    /// because CachedState has a property of type Arc
    /// and the derived clone method is making a shallow copy
    pub(crate) fn make_deep_clone(&self) -> Self {
        Self {
            state: CachedState::new(
                Arc::new(self.state.state_reader.as_ref().clone()),
                self.state.state_reader.class_hash_to_compiled_class.clone(),
            ),
            contract_classes: self.contract_classes.clone(),
        }
    }
}

impl StateChanger for StarknetState {
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()> {
        self.contract_classes.insert(class_hash, contract_class.clone());
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        match contract_class {
            ContractClass::Cairo0(deprecated_contract_class) => {
                persistent_state.class_hash_to_compiled_class_mut().insert(
                    class_hash.bytes(),
                    starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass::Deprecated(Arc::new(StarknetInRustContractClass::try_from(deprecated_contract_class)?)),
                );
            }
            ContractClass::Cairo1(sierra_contract_class) => {
                persistent_state.class_hash_to_compiled_class_mut().insert(
                    class_hash.bytes(),
                    starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass::Casm(Arc::new(CasmContractClass::from_contract_class(sierra_contract_class, true)
                        .map_err(|_| Error::SierraCompilationError)?)),
                );
            }
        }

        Ok(())
    }

    fn deploy_contract(
        &mut self,
        address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        let addr: Address = address.into();
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_class_hash_mut().insert(addr.clone(), class_hash.bytes());
        persistent_state.address_to_nonce_mut().insert(addr, Felt252::new(0));

        Ok(())
    }

    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> DevnetResult<()> {
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_storage_mut().insert(storage_key.into(), data.into());

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()> {
        let addr: Address = address.into();
        let nonce = self.state.get_nonce_at(&addr)?;
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_nonce_mut().insert(addr, nonce + Felt252::new(1));

        Ok(())
    }

    fn apply_state_difference(&mut self, state_diff: StateDiff) -> DevnetResult<()> {
        let old_state = Arc::make_mut(&mut self.state.state_reader);

        // update contract storages
        state_diff.inner.storage_updates().iter().for_each(|(contract_address, storages)| {
            storages.iter().for_each(|(key, value)| {
                // old_state.storage_view.insert((contract_address, key), value);
                let key = (contract_address.clone(), key.to_be_bytes());
                old_state.address_to_storage_mut().insert(key, value.clone());
            })
        });

        // update cairo 0 differences
        for (class_hash, cairo_0_contract_class) in state_diff.cairo_0_declared_contracts {
            old_state.class_hash_to_compiled_class_mut().insert(
                class_hash.bytes(),
                CompiledClass::Deprecated(Arc::new(cairo_0_contract_class)),
            );
        }

        // update class_hash -> compiled_class_hash differences
        state_diff.class_hash_to_compiled_class_hash.into_iter().for_each(
            |(class_hash, compiled_class_hash)| {
                old_state
                    .class_hash_to_compiled_class_hash_mut()
                    .insert(class_hash.bytes(), compiled_class_hash.bytes());
            },
        );

        // update cairo 1 differences
        state_diff.declared_contracts.into_iter().for_each(|(class_hash, cairo_1_casm)| {
            old_state
                .class_hash_to_compiled_class_mut()
                .insert(class_hash.bytes(), CompiledClass::Casm(Arc::new(cairo_1_casm)));
        });

        // update deployed contracts
        state_diff.inner.address_to_class_hash().iter().for_each(
            |(contract_address, class_hash)| {
                old_state.address_to_class_hash_mut().insert(contract_address.clone(), *class_hash);
            },
        );

        // update accounts nonce
        state_diff.inner.address_to_nonce().iter().for_each(|(contract_address, nonce)| {
            old_state.address_to_nonce_mut().insert(contract_address.clone(), nonce.clone());
        });

        Ok(())
    }
}

impl StateExtractor for StarknetState {
    fn get_storage(&self, storage_key: ContractStorageKey) -> DevnetResult<Felt> {
        if !self.is_contract_deployed(storage_key.get_contract_address()) {
            return Err(Error::ContractNotFound);
        }

        let storage_entry = storage_key.into();
        let data = self.state.get_storage_at(&storage_entry).map(Felt::from)?;

        Ok(data)
    }

    fn is_contract_declared(&mut self, class_hash: &ClassHash) -> bool {
        self.state.state_reader.class_hash_to_compiled_class_hash.contains_key(&class_hash.bytes())
            || self
                .state
                .state_reader
                .class_hash_to_compiled_class
                .contains_key(&class_hash.bytes())
    }

    fn is_contract_deployed(&self, address: &ContractAddress) -> bool {
        let address_felt: Felt252 = (*address).into();
        self.state.state_reader.address_to_class_hash.contains_key(&Address(address_felt))
    }

    fn get_class_hash_at_contract_address(
        &mut self,
        contract_address: &ContractAddress,
    ) -> DevnetResult<ClassHash> {
        Ok(self.state.get_class_hash_at(&contract_address.into()).map(Felt::new)??)
    }

    fn extract_state_diff_from_pending_state(&self) -> DevnetResult<StateDiff> {
        StateDiff::difference_between_old_and_new_state(
            self.state.state_reader.as_ref().clone(),
            self.state.clone(),
        )
    }

    fn get_nonce(&self, address: &ContractAddress) -> DevnetResult<Felt> {
        if !self.is_contract_deployed(address) {
            return Err(Error::ContractNotFound);
        }

        self.state.get_nonce_at(&address.into()).map(Felt::from).map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
    use starknet_in_rust::state::state_api::{State, StateReader};
    use starknet_types::cairo_felt::Felt252;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::Cairo0ContractClass;
    use starknet_types::felt::Felt;

    use super::StarknetState;
    use crate::error::Error;
    use crate::traits::{StateChanger, StateExtractor};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_contract_address, dummy_contract_storage_key, dummy_felt,
    };

    #[test]
    fn apply_state_update_for_contract_class_successfully() {
        let mut state = StarknetState::default();

        let class_hash = dummy_felt().bytes();
        let contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();

        state
            .state
            .set_contract_class(
                &class_hash,
                &CompiledClass::Deprecated(Arc::new(contract_class.try_into().unwrap())),
            )
            .unwrap();

        assert!(!state.is_contract_declared(&dummy_felt()));
        state.state.get_contract_class(&class_hash).unwrap();
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();

        assert!(state.is_contract_declared(&dummy_felt()));
    }

    #[test]
    fn synchronize_states_after_changing_pending_state_it_should_be_empty() {
        let mut state = StarknetState::default();
        state
            .state
            .set_storage_at(&dummy_contract_storage_key().try_into().unwrap(), dummy_felt().into());

        state.state.get_storage_at(&dummy_contract_storage_key().try_into().unwrap()).unwrap();

        state.clear_dirty_state();

        assert_eq!(
            state.state.get_storage_at(&dummy_contract_storage_key().try_into().unwrap()).unwrap(),
            Felt::default().into()
        );
    }

    #[test]
    fn apply_state_updates_for_storage_successfully() {
        let mut state = StarknetState::default();
        state
            .state
            .deploy_contract(
                dummy_contract_storage_key().get_contract_address().into(),
                dummy_felt().into(),
            )
            .unwrap();

        state
            .state
            .set_storage_at(&dummy_contract_storage_key().try_into().unwrap(), dummy_felt().into());

        let get_storage_result = state.get_storage(dummy_contract_storage_key());

        assert!(matches!(get_storage_result.unwrap_err(), Error::ContractNotFound));

        // apply changes to persistent state
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();
        assert_eq!(state.get_storage(dummy_contract_storage_key()).unwrap(), dummy_felt());
    }

    #[test]
    fn apply_state_updates_for_address_nonce_successfully() {
        let mut state = StarknetState::default();

        state.deploy_contract(dummy_contract_address(), dummy_felt()).unwrap();
        let starknet_in_rust_address: starknet_in_rust::utils::Address =
            dummy_contract_address().try_into().unwrap();

        // check if current nonce is 0
        assert!(
            state
                .state
                .state_reader
                .address_to_nonce
                .get(&starknet_in_rust_address)
                .unwrap()
                .eq(&Felt252::from(0))
        );

        state.clear_dirty_state();
        state.state.increment_nonce(&starknet_in_rust_address).unwrap();
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();

        // check if nonce update was correct
        assert!(
            state
                .state
                .state_reader
                .address_to_nonce
                .get(&starknet_in_rust_address)
                .unwrap()
                .eq(&Felt252::from(1))
        );
    }

    #[test]
    fn declare_cairo_0_contract_class_successfully() {
        let mut state = StarknetState::default();
        let class_hash = Felt::from_prefixed_hex_str("0xFE").unwrap();

        let contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();
        assert!(
            state
                .declare_contract_class(class_hash, contract_class.clone().try_into().unwrap())
                .is_ok()
        );
        assert!(state.state.state_reader.class_hash_to_compiled_class.len() == 1);
        let declared_contract_class = state
            .state
            .state_reader
            .class_hash_to_compiled_class
            .get(&class_hash.bytes())
            .unwrap()
            .to_owned();

        match declared_contract_class {
            CompiledClass::Deprecated(deprecated_contract_class) => {
                assert_eq!(*deprecated_contract_class, contract_class.try_into().unwrap())
            }
            _ => panic!("Wrong version of contract class"),
        }
    }

    #[test]
    fn deploy_cairo_0_contract_class_successfully() {
        let (mut state, address) = setup();
        let felt = dummy_felt();

        assert!(state.deploy_contract(address, felt).is_ok());
        assert!(state.state.state_reader.address_to_class_hash.len() == 1);
        assert!(
            state
                .state
                .state_reader
                .address_to_class_hash
                .contains_key(&(address.try_into().unwrap()))
        );
        assert!(
            state
                .state
                .state_reader
                .address_to_nonce
                .get(&(address.try_into().unwrap()))
                .unwrap()
                .eq(&Felt252::from(0))
        );
    }

    #[test]
    fn change_storage_successfully() {
        let mut state = StarknetState::default();
        let storage_key = dummy_contract_storage_key();

        assert!(state.change_storage(storage_key, dummy_felt()).is_ok());
        assert!(state.state.state_reader.address_to_storage.len() == 1);
        assert!(
            state
                .state
                .state_reader
                .address_to_storage
                .contains_key(&(storage_key.try_into().unwrap()))
        );
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        state.increment_nonce(address).unwrap();
        let nonce = state
            .state
            .state_reader
            .address_to_nonce
            .get(&address.try_into().unwrap())
            .unwrap()
            .clone();
        let expected_nonce = Felt252::from(1);

        assert_eq!(expected_nonce, nonce);
    }

    #[test]
    fn read_from_storage_returns_correct_result() {
        let (mut state, _) = setup();
        let expected_result = Felt::from(33);

        state
            .deploy_contract(*dummy_contract_storage_key().get_contract_address(), dummy_felt())
            .unwrap();
        state.change_storage(dummy_contract_storage_key(), expected_result).unwrap();
        let generated_result = state.get_storage(dummy_contract_storage_key()).unwrap();
        assert_eq!(expected_result, generated_result);
    }

    #[test]
    fn get_nonce_should_return_error_when_contract_not_deployed() {
        let (state, _) = setup();
        match state.get_nonce(&ContractAddress::new(Felt::from(1)).unwrap()) {
            Err(Error::ContractNotFound) => {}
            _ => panic!("Expected to return an error - ContractNotFound"),
        }
    }

    #[test]
    fn get_nonce_should_return_zero_for_freshly_deployed_contract() {
        let (state, address) = setup();
        assert_eq!(state.get_nonce(&address).unwrap(), Felt::from(0));
    }

    #[test]
    fn check_clone_derived_vs_make_deep_clone_method() {
        let state = StarknetState::default();

        let deep_cloned_state = state.make_deep_clone();

        // get pointers to Arcs
        let p_state = Arc::as_ptr(&state.state.state_reader);
        let p_deep_cloned_state = Arc::as_ptr(&deep_cloned_state.state.state_reader);

        // deep cloned should not point to the same memory location as the original
        assert_ne!(p_deep_cloned_state, p_state);
    }

    fn setup() -> (StarknetState, ContractAddress) {
        let mut state = StarknetState::default();
        let address = dummy_contract_address();
        let contract_class = dummy_cairo_0_contract_class();
        let class_hash = dummy_felt();

        state.declare_contract_class(class_hash, contract_class.into()).unwrap();
        state.deploy_contract(address, class_hash).unwrap();

        (state, address)
    }
}
