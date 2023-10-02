use std::collections::HashMap;
use std::sync::Arc;

use starknet_api::hash::StarkFelt;
use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
use starknet_in_rust::state::cached_state::CachedState;
use starknet_in_rust::state::state_api::StateReader;
use starknet_in_rust::utils::Address;
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, CompiledClassHash, Felt};
use starknet_types::patricia_key::{PatriciaKey, StorageKey};

use self::state_diff::StateDiff;
use crate::error::{DevnetResult, Error};
use crate::traits::{StateChanger, StateExtractor};

pub(crate) mod state_diff;
pub mod state_update;

#[derive(Default)]
pub(crate) struct StarknetState {
    pub state: CachedState<DevnetState>,
    pub(crate) contract_classes: HashMap<ClassHash, ContractClass>,
}

#[derive(Default, Clone)]
pub(crate) struct DevnetState {
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub address_to_nonce: HashMap<ContractAddress, Felt>,
    pub address_to_storage: HashMap<ContractStorageKey, Felt>,
    pub class_hash_to_compiled_class: HashMap<ClassHash, ContractClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
}

impl blockifier::state::state_api::StateReader for DevnetState {
    fn get_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> blockifier::state::state_api::StateResult<starknet_api::hash::StarkFelt> {
        let storage = self
            .address_to_storage
            .get(&ContractStorageKey::new(contract_address.into(), key.0.into()))
            .map(StarkFelt::from)
            .unwrap_or_default();
        Ok(storage)
    }

    fn get_nonce_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::Nonce> {
        let nonce = self
            .address_to_nonce
            .get(&contract_address.into())
            .map(StarkFelt::from)
            .unwrap_or_default();
        Ok(starknet_api::core::Nonce(nonce))
    }

    fn get_class_hash_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::ClassHash> {
        let class_hash = self
            .address_to_class_hash
            .get(&contract_address.into())
            .map(StarkFelt::from)
            .unwrap_or_default();
        Ok(starknet_api::core::ClassHash(class_hash))
    }

    fn get_compiled_contract_class(
        &mut self,
        class_hash: &starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<
        blockifier::execution::contract_class::ContractClass,
    > {
        let contract_class = self.class_hash_to_compiled_class.get(&class_hash.0.into()).cloned();
        match contract_class {
            Some(contract_class) => {
                Ok(blockifier::execution::contract_class::ContractClass::try_from(contract_class)
                    .map_err(|err| {
                    blockifier::state::errors::StateError::StateReadError(err.to_string())
                })?)
            }
            _ => Err(blockifier::state::errors::StateError::UndeclaredClassHash(*class_hash)),
        }
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        let compiled_class_hash = self
            .class_hash_to_compiled_class_hash
            .get(&class_hash.0.into())
            .map(StarkFelt::from)
            .unwrap_or_default();
        Ok(starknet_api::core::CompiledClassHash(compiled_class_hash))
    }
}

impl starknet_in_rust::state::state_api::StateReader for DevnetState {
    fn get_contract_class(
        &self,
        class_hash: &starknet_in_rust::utils::ClassHash,
    ) -> Result<CompiledClass, starknet_in_rust::core::errors::state_errors::StateError> {
        let class_hash_as_felt = Felt::new(*class_hash).map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?;

        // Deprecated contract classes dont have a compiled_class_hash, we dont need to fetch it
        if let Some(compiled_class) = self.class_hash_to_compiled_class.get(&class_hash_as_felt) {
            return CompiledClass::try_from(compiled_class.clone()).map_err(|err| {
                starknet_in_rust::core::errors::state_errors::StateError::CustomError(
                    err.to_string(),
                )
            });
        }

        // we are sure that compiled_class_hash is in the range of Felt, because it is a hash,
        // so we can unwrap it
        let compiled_class_hash = self.get_compiled_class_hash(class_hash)?;
        if compiled_class_hash != *starknet_in_rust::state::cached_state::UNINITIALIZED_CLASS_HASH {
            let compiled_class = self
                .class_hash_to_compiled_class
                .get(&Felt::new(compiled_class_hash).map_err(|err| {
                    starknet_in_rust::core::errors::state_errors::StateError::CustomError(
                        err.to_string(),
                    )
                })?)
                .ok_or(
                    starknet_in_rust::core::errors::state_errors::StateError::NoneCompiledClass(
                        compiled_class_hash,
                    ),
                )?;

            CompiledClass::try_from(compiled_class.clone()).map_err(|err| {
                starknet_in_rust::core::errors::state_errors::StateError::CustomError(
                    err.to_string(),
                )
            })
        } else {
            Err(starknet_in_rust::core::errors::state_errors::StateError::MissingCasmClass(
                compiled_class_hash,
            ))
        }
    }

    fn get_class_hash_at(
        &self,
        contract_address: &Address,
    ) -> Result<
        starknet_in_rust::utils::ClassHash,
        starknet_in_rust::core::errors::state_errors::StateError,
    > {
        let address = ContractAddress::try_from(contract_address).map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?;

        Ok(self.address_to_class_hash.get(&address).map(|f| f.bytes()).unwrap_or_default())
    }

    fn get_nonce_at(
        &self,
        contract_address: &Address,
    ) -> Result<Felt252, starknet_in_rust::core::errors::state_errors::StateError> {
        let address = ContractAddress::try_from(contract_address).map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?;
        Ok(self.address_to_nonce.get(&address).map(Felt252::from).unwrap_or_default())
    }

    fn get_storage_at(
        &self,
        storage_entry: &starknet_in_rust::state::state_cache::StorageEntry,
    ) -> Result<Felt252, starknet_in_rust::core::errors::state_errors::StateError> {
        let contract_address = ContractAddress::try_from(&storage_entry.0).map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?;
        let storage_key = StorageKey::new(Felt::new(storage_entry.1).map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?)
        .map_err(|err| {
            starknet_in_rust::core::errors::state_errors::StateError::CustomError(err.to_string())
        })?;
        Ok(self
            .address_to_storage
            .get(&ContractStorageKey::new(contract_address, storage_key))
            .map(Felt252::from)
            .unwrap_or_default())
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: &starknet_in_rust::utils::ClassHash,
    ) -> Result<
        starknet_in_rust::utils::CompiledClassHash,
        starknet_in_rust::core::errors::state_errors::StateError,
    > {
        let compiled_class_hash = self
            .class_hash_to_compiled_class_hash
            .get(&Felt::new(*class_hash).map_err(|err| {
                starknet_in_rust::core::errors::state_errors::StateError::CustomError(
                    err.to_string(),
                )
            })?)
            .map(|f| f.bytes())
            .ok_or(starknet_in_rust::core::errors::state_errors::StateError::NoneCompiledHash(
                *class_hash,
            ))?;

        Ok(compiled_class_hash)
    }
}

impl StarknetState {
    /// this method clears the state from data that was accumulated in the StateCache
    /// and restores it to the data in the state_reader, which is the "persistent" data
    pub(crate) fn clear_dirty_state(&mut self) {
        self.state = CachedState::new(
            self.state.state_reader.clone(),
            self.state.contract_classes().clone(),
        );
    }

    /// this method is making deep copy of the object
    /// because CachedState has a property of type Arc
    /// and the derived clone method is making a shallow copy
    pub(crate) fn make_deep_clone(&self) -> Self {
        Self {
            state: CachedState::new(
                Arc::new(self.state.state_reader.as_ref().clone()),
                self.state.contract_classes().clone(),
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

        persistent_state.class_hash_to_compiled_class.insert(class_hash, contract_class);

        Ok(())
    }

    fn deploy_contract(
        &mut self,
        address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_class_hash.insert(address, class_hash);
        persistent_state.address_to_nonce.insert(address, Felt::from(0));

        Ok(())
    }

    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> DevnetResult<()> {
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_storage.insert(storage_key, data);

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()> {
        let nonce = self.state.state_reader.get_nonce_at(&address.into())?;
        let persistent_state = Arc::make_mut(&mut self.state.state_reader);

        persistent_state.address_to_nonce.insert(address, (nonce + Felt252::new(1)).into());

        Ok(())
    }

    fn apply_state_difference(&mut self, state_diff: StateDiff) -> DevnetResult<()> {
        let old_state = Arc::make_mut(&mut self.state.state_reader);
        let contract_classes_cache = &self.contract_classes;

        // update contract storages
        state_diff.inner.storage_updates().iter().try_for_each(
            |(contract_address, storages)| {
                let address = ContractAddress::try_from(contract_address)?;
                storages.iter().try_for_each(|(key, value)| -> DevnetResult<()> {
                    let storage_key = PatriciaKey::new(Felt::from(key))?;

                    old_state
                        .address_to_storage
                        .insert(ContractStorageKey::new(address, storage_key), Felt::from(value));

                    Ok(())
                })
            },
        )?;

        // update cairo 0 differences
        for class_hash in state_diff.cairo_0_declared_contracts {
            let cairo_0_contract_class =
            contract_classes_cache.get(&class_hash).ok_or(Error::StateError(
                    starknet_in_rust::core::errors::state_errors::StateError::MissingClassHash(),
                ))?;
            old_state.class_hash_to_compiled_class.insert(class_hash, cairo_0_contract_class.clone());
        }

        // update class_hash -> compiled_class_hash differences
        state_diff.class_hash_to_compiled_class_hash.into_iter().for_each(
            |(class_hash, compiled_class_hash)| {
                old_state.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
            },
        );

        // update cairo 1 differences
        // Note: due to the fact that starknet_in_rust holds Cairo 1 as CasmContractClass
        // We cant transform it to ContractClass, because there is no conversion from casm to sierra
        // so we use our mapping contract_classes of the StarknetState to get the sierra representation
        state_diff.declared_contracts.into_iter().try_for_each(
            |compiled_class_hash| -> DevnetResult<()> {
                let class_hash = old_state
                    .class_hash_to_compiled_class_hash
                    .iter()
                    .find(|(_, val)| compiled_class_hash == **val)
                    .map(|(key, _)| key)
                    .ok_or(Error::StateError(
                        starknet_in_rust::core::errors::state_errors::StateError::NoneCompiledClass(
                            compiled_class_hash.bytes(),
                        ),
                    ))?;

                let cairo_1_sierra =
                    contract_classes_cache.get(class_hash).ok_or(Error::StateError(
                        starknet_in_rust::core::errors::state_errors::StateError::MissingCasmClass(
                            class_hash.bytes(),
                        ),
                    ))?;
                old_state
                    .class_hash_to_compiled_class
                    .insert(compiled_class_hash, cairo_1_sierra.clone());

                Ok(())
            },
        )?;

        // update deployed contracts
        state_diff.inner.address_to_class_hash().iter().try_for_each(
            |(contract_address, class_hash)| -> DevnetResult<()> {
                old_state
                    .address_to_class_hash
                    .insert(contract_address.try_into()?, Felt::new(*class_hash)?);
                Ok(())
            },
        )?;

        // update accounts nonce
        state_diff.inner.address_to_nonce().iter().try_for_each(
            |(contract_address, nonce)| -> DevnetResult<()> {
                old_state.address_to_nonce.insert(contract_address.try_into()?, Felt::from(nonce));
                Ok(())
            },
        )?;

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
        self.state.state_reader.class_hash_to_compiled_class_hash.contains_key(class_hash)
            || self.state.state_reader.class_hash_to_compiled_class.contains_key(class_hash)
    }

    fn is_contract_deployed(&self, address: &ContractAddress) -> bool {
        self.state.state_reader.address_to_class_hash.contains_key(address)
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

    use blockifier::test_utils::DictStateReader;
    use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
    use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
    use starknet_in_rust::state::in_memory_state_reader::InMemoryStateReader;
    use starknet_in_rust::state::state_api::{State, StateReader};
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::Cairo0ContractClass;
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::{ClassHash, Felt};

    use super::{DevnetState, StarknetState};
    use crate::error::Error;
    use crate::traits::{StateChanger, StateExtractor};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_contract_storage_key,
        dummy_felt,
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
                &CompiledClass::Deprecated(Arc::new(contract_class.clone().try_into().unwrap())),
            )
            .unwrap();

        state.contract_classes.insert(class_hash.into(), contract_class.into());

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
        let contract_address = dummy_contract_address();

        // check if current nonce is 0
        assert!(
            state
                .state
                .state_reader
                .address_to_nonce
                .get(&contract_address)
                .unwrap()
                .eq(&Felt::from(0))
        );

        state.clear_dirty_state();
        state.state.increment_nonce(&contract_address.try_into().unwrap()).unwrap();
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();

        // check if nonce update was correct
        assert!(
            state
                .state
                .state_reader
                .address_to_nonce
                .get(&contract_address)
                .unwrap()
                .eq(&Felt::from(1))
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
            .get(&class_hash)
            .unwrap()
            .to_owned();

        match declared_contract_class {
            starknet_types::contract_class::ContractClass::Cairo0(deprecated_contract_class) => {
                assert_eq!(
                    StarknetInRustContractClass::try_from(deprecated_contract_class).unwrap(),
                    StarknetInRustContractClass::try_from(contract_class).unwrap()
                );
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
        assert!(state.state.state_reader.address_to_class_hash.contains_key(&address));
        assert!(
            state.state.state_reader.address_to_nonce.get(&address).unwrap().eq(&Felt::from(0))
        );
    }

    #[test]
    fn change_storage_successfully() {
        let mut state = StarknetState::default();
        let storage_key = dummy_contract_storage_key();

        assert!(state.change_storage(storage_key, dummy_felt()).is_ok());
        assert!(state.state.state_reader.address_to_storage.len() == 1);
        assert!(state.state.state_reader.address_to_storage.contains_key(&(storage_key)));
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        state.increment_nonce(address).unwrap();
        let nonce = *state.state.state_reader.address_to_nonce.get(&address).unwrap();
        let expected_nonce = Felt::from(1);

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

    #[test]
    fn check_devnet_state_with_starknet_in_rust_in_memory_state_reader() {
        let mut in_memory_state_reader = InMemoryStateReader::default();
        let (devnet_state, class_hash, address, storage_key) = setup_devnet_state();

        devnet_state.address_to_class_hash.iter().for_each(|(k, v)| {
            in_memory_state_reader
                .address_to_class_hash
                .insert(starknet_in_rust::utils::Address::from(*k), v.bytes());
        });

        devnet_state.address_to_nonce.iter().for_each(|(k, v)| {
            in_memory_state_reader
                .address_to_nonce
                .insert(starknet_in_rust::utils::Address::from(*k), v.into());
        });

        devnet_state.address_to_storage.iter().for_each(|(k, v)| {
            in_memory_state_reader.address_to_storage.insert(k.into(), v.into());
        });

        devnet_state.class_hash_to_compiled_class_hash.iter().for_each(|(k, v)| {
            in_memory_state_reader.class_hash_to_compiled_class_hash.insert(k.bytes(), v.bytes());
        });

        devnet_state.class_hash_to_compiled_class.iter().for_each(|(k, v)| {
            in_memory_state_reader
                .class_hash_to_compiled_class
                .insert(k.bytes(), CompiledClass::try_from(v.clone()).unwrap());
        });

        fn assert_equal_results(
            first: &impl starknet_in_rust::state::state_api::StateReader,
            second: &impl starknet_in_rust::state::state_api::StateReader,
            address: starknet_in_rust::utils::Address,
            class_hash: [u8; 32],
            storage_key: starknet_in_rust::state::state_cache::StorageEntry,
        ) {
            let second_result = second.get_nonce_at(&address);
            match first.get_nonce_at(&address) {
                Ok(nonce) => assert_eq!(nonce, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_class_hash_at(&address);
            match first.get_class_hash_at(&address) {
                Ok(class_hash) => assert_eq!(class_hash, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_storage_at(&storage_key);
            match first.get_storage_at(&storage_key) {
                Ok(storage) => assert_eq!(storage, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_compiled_class_hash(&class_hash);
            match first.get_compiled_class_hash(&class_hash) {
                Ok(compiled_class_hash) => assert_eq!(compiled_class_hash, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_contract_class(&class_hash);
            match first.get_contract_class(&class_hash) {
                Ok(contract_class) => assert_eq!(contract_class, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }
        }

        assert_equal_results(
            &devnet_state,
            &in_memory_state_reader,
            address.into(),
            class_hash.into(),
            storage_key.into(),
        );
        assert_equal_results(
            &devnet_state,
            &in_memory_state_reader,
            ContractAddress::default().into(),
            Felt::default().into(),
            ContractStorageKey::default().into(),
        );
    }

    #[test]
    fn check_devnet_state_with_blockifier_dict_state_reader() {
        let mut dict_state_reader = DictStateReader::default();
        let (mut devnet_state, class_hash, address, storage_key) = setup_devnet_state();

        devnet_state.address_to_class_hash.iter().for_each(|(k, v)| {
            dict_state_reader.address_to_class_hash.insert(
                starknet_api::core::ContractAddress::try_from(*k).unwrap(),
                starknet_api::core::ClassHash((*v).into()),
            );
        });

        devnet_state.address_to_nonce.iter().for_each(|(k, v)| {
            dict_state_reader.address_to_nonce.insert(
                starknet_api::core::ContractAddress::try_from(*k).unwrap(),
                starknet_api::core::Nonce((*v).into()),
            );
        });

        devnet_state.address_to_storage.iter().for_each(|(k, v)| {
            dict_state_reader.storage_view.insert(
                (
                    starknet_api::core::ContractAddress::try_from(*k.get_contract_address())
                        .unwrap(),
                    starknet_api::state::StorageKey(
                        starknet_api::core::PatriciaKey::try_from(*k.get_storage_key()).unwrap(),
                    ),
                ),
                v.into(),
            );
        });

        devnet_state.class_hash_to_compiled_class_hash.iter().for_each(|(k, v)| {
            dict_state_reader.class_hash_to_compiled_class_hash.insert(
                starknet_api::core::ClassHash((*k).into()),
                starknet_api::core::CompiledClassHash((*v).into()),
            );
        });

        devnet_state.class_hash_to_compiled_class.iter().for_each(|(k, v)| {
            dict_state_reader.class_hash_to_class.insert(
                starknet_api::core::ClassHash((*k).into()),
                blockifier::execution::contract_class::ContractClass::try_from(v.clone()).unwrap(),
            );
        });

        fn assert_equal_results(
            first: &mut impl blockifier::state::state_api::StateReader,
            second: &mut impl blockifier::state::state_api::StateReader,
            address: starknet_api::core::ContractAddress,
            class_hash: starknet_api::core::ClassHash,
            contract_storage_key: (
                starknet_api::core::ContractAddress,
                starknet_api::state::StorageKey,
            ),
        ) {
            let second_result = second.get_nonce_at(address);
            match first.get_nonce_at(address) {
                Ok(nonce) => assert_eq!(nonce, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_class_hash_at(address);
            match first.get_class_hash_at(address) {
                Ok(class_hash) => assert_eq!(class_hash, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result =
                second.get_storage_at(contract_storage_key.0, contract_storage_key.1);
            match first.get_storage_at(contract_storage_key.0, contract_storage_key.1) {
                Ok(storage) => assert_eq!(storage, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_compiled_class_hash(class_hash);
            match first.get_compiled_class_hash(class_hash) {
                Ok(compiled_class_hash) => assert_eq!(compiled_class_hash, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }

            let second_result = second.get_compiled_contract_class(&class_hash);
            match first.get_compiled_contract_class(&class_hash) {
                Ok(contract_class) => assert_eq!(contract_class, second_result.unwrap()),
                Err(err) => {
                    assert_eq!(err.to_string(), second_result.unwrap_err().to_string());
                }
            }
        }

        assert_equal_results(
            &mut devnet_state,
            &mut dict_state_reader,
            address.try_into().unwrap(),
            class_hash.into(),
            (
                starknet_api::core::ContractAddress::try_from(*storage_key.get_contract_address())
                    .unwrap(),
                starknet_api::state::StorageKey(
                    starknet_api::core::PatriciaKey::try_from(*storage_key.get_storage_key())
                        .unwrap(),
                ),
            ),
        )
    }

    fn setup_devnet_state() -> (DevnetState, ClassHash, ContractAddress, ContractStorageKey) {
        let mut state = DevnetState::default();
        let class_hash = dummy_felt();
        let compiled_class_hash = Felt::from(1);
        let address = dummy_contract_address();
        let storage_key = dummy_contract_storage_key();

        state.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
        state
            .class_hash_to_compiled_class
            .insert(dummy_felt(), dummy_cairo_1_contract_class().into());
        state.address_to_class_hash.insert(address, class_hash);
        state.address_to_storage.insert(storage_key, class_hash);
        state.address_to_nonce.insert(address, Felt::from(1));

        (state, class_hash, address, storage_key)
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
