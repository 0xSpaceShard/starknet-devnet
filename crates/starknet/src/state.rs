use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::state::cached_state::CachedState;
use starknet_in_rust::state::in_memory_state_reader::InMemoryStateReader;
use starknet_in_rust::state::state_api::StateReader;
use starknet_in_rust::utils::{subtract_mappings, to_state_diff_storage_mapping, Address};
use starknet_in_rust::CasmContractClass;
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, Felt};

use crate::error::Result;
use crate::traits::{StateChanger, StateExtractor};

#[derive(Debug)]
pub(crate) struct StarknetState {
    pub state: InMemoryStateReader,
    pub pending_state: CachedState<InMemoryStateReader>,
}

impl StarknetState {
    // this is used to copy "persistent" data that is present in "state" variable into
    // "pending_state" this is done, because "pending_state" doesnt hold a reference to state,
    // but rather a copy.
    pub(crate) fn synchronize_states(&mut self) {
        self.pending_state = CachedState::new(
            self.state.clone(),
            Some(self.state.class_hash_to_contract_class.clone()),
            Some(self.state.casm_contract_classes_mut().clone()),
        );
    }
}

impl Default for StarknetState {
    fn default() -> Self {
        let in_memory_state = InMemoryStateReader::default();
        Self {
            state: in_memory_state.clone(),
            pending_state: CachedState::new(in_memory_state, None, None),
        }
    }
}

impl StateChanger for StarknetState {
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> Result<()> {
        match contract_class {
            ContractClass::Cairo0(_) => {
                self.state.class_hash_to_contract_class_mut().insert(
                    class_hash.bytes(),
                    StarknetInRustContractClass::try_from(contract_class)?,
                );
            }
            ContractClass::Cairo1(_) => {
                self.state
                    .casm_contract_classes_mut()
                    .insert(class_hash.bytes(), CasmContractClass::try_from(contract_class)?);
            }
        }

        Ok(())
    }

    fn deploy_contract(&mut self, address: ContractAddress, class_hash: ClassHash) -> Result<()> {
        let addr: Address = address.try_into()?;
        self.state.address_to_class_hash_mut().insert(addr.clone(), class_hash.bytes());
        self.state.address_to_nonce_mut().insert(addr, Felt252::new(0));

        Ok(())
    }

    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> Result<()> {
        self.state.address_to_storage_mut().insert(storage_key.try_into()?, data.into());

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> Result<()> {
        let addr: Address = address.try_into()?;
        let nonce = self.state.get_nonce_at(&addr)?;
        self.state.address_to_nonce_mut().insert(addr, nonce + Felt252::new(1));

        Ok(())
    }

    fn apply_cached_state(&mut self) -> Result<()> {
        let new_casm_classes =
            self.pending_state.casm_contract_classes().clone().unwrap_or_default();

        // get differences
        let state_cache = self.pending_state.cache_mut();

        let substracted_maps = subtract_mappings(
            state_cache.storage_writes().clone(),
            state_cache.storage_initial_values_mut().clone(),
        );

        let storage_updates = to_state_diff_storage_mapping(substracted_maps);

        let address_to_nonce = subtract_mappings(
            state_cache.nonce_writes_mut().clone(),
            state_cache.nonce_initial_values().clone(),
        );

        // Cairo 1 compiled class hash
        let class_hash_to_compiled_class = subtract_mappings(
            state_cache.compiled_class_hash_writes_mut().clone(),
            state_cache.compiled_class_hash_initial_values_mut().clone(),
        );

        // // Cairo 1 differences
        let class_hash_to_cairo_1_casm =
            subtract_mappings(new_casm_classes, self.state.casm_contract_classes_mut().clone());

        let address_to_class_hash = subtract_mappings(
            state_cache.class_hash_writes_mut().clone(),
            state_cache.class_hash_initial_values_mut().clone(),
        );

        // Cairo 0 differences
        let class_hash_to_cairo_0_contract_class = subtract_mappings(
            self.pending_state.contract_classes().clone().unwrap_or_default(),
            self.state.class_hash_to_contract_class.clone(),
        );

        let old_state = &mut self.state;

        // update contract storages
        storage_updates.into_iter().for_each(|(contract_address, storages)| {
            storages.into_iter().for_each(|(key, value)| {
                // old_state.storage_view.insert((contract_address, key), value);
                let key = (contract_address.clone(), key.to_be_bytes());
                old_state.address_to_storage_mut().insert(key, value);
            })
        });

        // update declared contracts
        // apply newly declared classses
        for (class_hash, contract_class) in class_hash_to_compiled_class {
            match contract_class {
                CompiledClass::Deprecated(artifact) => {
                    old_state.class_hash_to_contract_class_mut().insert(class_hash, *artifact);
                }
                CompiledClass::Casm(artifact) => {
                    old_state.casm_contract_classes_mut().insert(class_hash, *artifact);
                }
            }
        }

        // update cairo 0 differences
        class_hash_to_cairo_0_contract_class.into_iter().for_each(
            |(class_hash, cairo_0_contract_class)| {
                old_state.class_hash_to_contract_class.insert(class_hash, cairo_0_contract_class);
            },
        );

        // // update cairo 1 differences
        class_hash_to_cairo_1_casm.into_iter().for_each(|(class_hash, cairo_1_casm)| {
            old_state.casm_contract_classes_mut().insert(class_hash, cairo_1_casm);
        });

        // update deployed contracts
        address_to_class_hash.into_iter().for_each(|(contract_address, class_hash)| {
            old_state.address_to_class_hash_mut().insert(contract_address, class_hash);
        });

        // // update accounts nonce
        address_to_nonce.into_iter().for_each(|(contract_address, nonce)| {
            old_state.address_to_nonce_mut().insert(contract_address, nonce);
        });

        Ok(())
    }
}

impl StateExtractor for StarknetState {
    fn get_storage(&mut self, storage_key: ContractStorageKey) -> Result<Felt> {
        Ok(self.state.get_storage_at(&storage_key.try_into()?).map(Felt::from)?)
    }

    fn is_contract_declared(&self, class_hash: &ClassHash) -> Result<bool> {
        Ok(self.state.class_hash_to_contract_class.contains_key(&(class_hash.bytes())))
    }

    fn get_class_hash_at_contract_address(
        &mut self,
        contract_address: &ContractAddress,
    ) -> Result<ClassHash> {
        Ok(self.state.get_class_hash_at(&contract_address.try_into()?).map(Felt::new)??)
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::core::errors::state_errors::StateError;
    use starknet_in_rust::state::state_api::{State, StateReader};
    use starknet_types::cairo_felt::Felt252;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;

    use super::StarknetState;
    use crate::error::Error;
    use crate::traits::{StateChanger, StateExtractor};
    use crate::utils::test_utils::{
        dummy_cairo_0_contract_class, dummy_contract_address, dummy_contract_storage_key,
        dummy_felt,
    };

    #[test]
    fn apply_state_update_for_contract_class_successfully() {
        let mut state = StarknetState::default();

        let class_hash = dummy_felt().bytes();

        state
            .pending_state
            .set_contract_class(&class_hash, &dummy_cairo_0_contract_class().try_into().unwrap())
            .unwrap();

        assert!(!state.is_contract_declared(&dummy_felt()).unwrap());
        state.pending_state.get_contract_class(&class_hash).unwrap();
        state.apply_cached_state().unwrap();

        assert!(state.is_contract_declared(&dummy_felt()).unwrap());
    }

    #[test]
    fn synchronize_states_after_changing_pending_state_it_should_be_empty() {
        let mut state = StarknetState::default();
        state
            .pending_state
            .set_storage_at(&dummy_contract_storage_key().try_into().unwrap(), dummy_felt().into());

        state
            .pending_state
            .get_storage_at(&dummy_contract_storage_key().try_into().unwrap())
            .unwrap();

        state.synchronize_states();

        assert_eq!(
            state
                .pending_state
                .get_storage_at(&dummy_contract_storage_key().try_into().unwrap())
                .unwrap(),
            Felt::default().into()
        );
    }

    #[test]
    fn apply_state_updates_for_storage_successfully() {
        let mut state = StarknetState::default();
        state
            .pending_state
            .set_storage_at(&dummy_contract_storage_key().try_into().unwrap(), dummy_felt().into());

        let get_storage_result = state.get_storage(dummy_contract_storage_key());
        assert!(matches!(
            get_storage_result.unwrap_err(),
            Error::StateError(StateError::NoneStorage((_, _)))
        ));

        // apply changes to persistent state
        state.apply_cached_state().unwrap();
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
                .address_to_nonce
                .get(&starknet_in_rust_address)
                .unwrap()
                .eq(&Felt252::from(0))
        );

        state.synchronize_states();
        state.pending_state.increment_nonce(&starknet_in_rust_address).unwrap();
        state.apply_cached_state().unwrap();

        // check if nonce update was correct
        assert!(
            state
                .state
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

        assert!(state.declare_contract_class(class_hash, dummy_cairo_0_contract_class()).is_ok());
        assert!(state.state.class_hash_to_contract_class.len() == 1);
        let contract_class = state.state.class_hash_to_contract_class.get(&class_hash.bytes());
        assert!(contract_class.is_some());
        assert_eq!(*contract_class.unwrap(), dummy_cairo_0_contract_class().try_into().unwrap());
    }

    #[test]
    fn deploy_cairo_0_contract_class_successfully() {
        let (mut state, address) = setup();
        let felt = dummy_felt();

        assert!(state.deploy_contract(address, felt).is_ok());
        assert!(state.state.address_to_class_hash.len() == 1);
        assert!(state.state.address_to_class_hash.contains_key(&(address.try_into().unwrap())));
        assert!(
            state
                .state
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
        assert!(state.state.address_to_storage.len() == 1);
        assert!(state.state.address_to_storage.contains_key(&(storage_key.try_into().unwrap())));
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        state.increment_nonce(address).unwrap();
        let nonce = state.state.address_to_nonce.get(&address.try_into().unwrap()).unwrap().clone();
        let expected_nonce = Felt252::from(1);

        assert_eq!(expected_nonce, nonce);
    }

    #[test]
    fn read_from_storage_returns_correct_result() {
        let (mut state, _) = setup();
        let expected_result = Felt::from(33);

        state.change_storage(dummy_contract_storage_key(), expected_result).unwrap();
        let generated_result = state.get_storage(dummy_contract_storage_key()).unwrap();
        assert_eq!(expected_result, generated_result);
    }

    fn setup() -> (StarknetState, ContractAddress) {
        let mut state = StarknetState::default();
        let address = dummy_contract_address();
        let contract_class = dummy_cairo_0_contract_class();
        let class_hash = dummy_felt();

        state.declare_contract_class(class_hash, contract_class).unwrap();
        state.deploy_contract(address, class_hash).unwrap();

        (state, address)
    }
}
