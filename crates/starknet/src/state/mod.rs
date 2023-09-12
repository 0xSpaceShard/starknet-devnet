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

#[derive(Debug, Clone)]
pub(crate) struct StarknetState {
    pub state: InMemoryStateReader,
    pub pending_state: CachedState<InMemoryStateReader>,
    pub(crate) contract_classes: HashMap<ClassHash, ContractClass>,
}

impl StarknetState {
    // this is used to copy "persistent" data that is present in "state" variable into
    // "pending_state" this is done, because "pending_state" doesnt hold a reference to state,
    // but rather a copy.
    pub(crate) fn synchronize_states(&mut self) {
        self.pending_state = CachedState::new(
            Arc::new(self.state.clone()),
            self.state.class_hash_to_compiled_class_mut().clone(),
        );
    }
}

impl Default for StarknetState {
    fn default() -> Self {
        let in_memory_state = InMemoryStateReader::default();
        Self {
            state: in_memory_state.clone(),
            pending_state: CachedState::new(Arc::new(in_memory_state), Default::default()),
            contract_classes: HashMap::new(),
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

        match contract_class {
            ContractClass::Cairo0(deprecated_contract_class) => {
                self.state.class_hash_to_compiled_class_mut().insert(
                    class_hash.bytes(),
                    starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass::Deprecated(Arc::new(StarknetInRustContractClass::try_from(deprecated_contract_class)?)),
                );
            }
            ContractClass::Cairo1(sierra_contract_class) => {
                self.state.class_hash_to_compiled_class_mut().insert(
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
        self.state.address_to_class_hash_mut().insert(addr.clone(), class_hash.bytes());
        self.state.address_to_nonce_mut().insert(addr, Felt252::new(0));

        Ok(())
    }

    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> DevnetResult<()> {
        self.state.address_to_storage_mut().insert(storage_key.into(), data.into());

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()> {
        let addr: Address = address.into();
        let nonce = self.state.get_nonce_at(&addr)?;
        self.state.address_to_nonce_mut().insert(addr, nonce + Felt252::new(1));

        Ok(())
    }

    fn apply_state_difference(&mut self, state_diff: StateDiff) -> DevnetResult<()> {
        let old_state = &mut self.state;
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
        self.state.class_hash_to_compiled_class_hash_mut().contains_key(&class_hash.bytes())
            || self.state.class_hash_to_compiled_class.contains_key(&(class_hash.bytes()))
    }

    fn is_contract_deployed(&self, address: &ContractAddress) -> bool {
        let address_felt: Felt252 = (*address).into();
        self.state.address_to_class_hash.contains_key(&Address(address_felt))
    }

    fn get_class_hash_at_contract_address(
        &mut self,
        contract_address: &ContractAddress,
    ) -> DevnetResult<ClassHash> {
        Ok(self.state.get_class_hash_at(&contract_address.into()).map(Felt::new)??)
    }

    fn extract_state_diff_from_pending_state(&self) -> DevnetResult<StateDiff> {
        StateDiff::difference_between_old_and_new_state(
            self.state.clone(),
            self.pending_state.clone(),
        )
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
            .pending_state
            .set_contract_class(
                &class_hash,
                &CompiledClass::Deprecated(Arc::new(contract_class.try_into().unwrap())),
            )
            .unwrap();

        assert!(!state.is_contract_declared(&dummy_felt()));
        state.pending_state.get_contract_class(&class_hash).unwrap();
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();

        assert!(state.is_contract_declared(&dummy_felt()));
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
            .deploy_contract(
                dummy_contract_storage_key().get_contract_address().into(),
                Felt::from(1).into(),
            )
            .unwrap();

        state
            .pending_state
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
                .address_to_nonce
                .get(&starknet_in_rust_address)
                .unwrap()
                .eq(&Felt252::from(0))
        );

        state.synchronize_states();
        state.pending_state.increment_nonce(&starknet_in_rust_address).unwrap();
        state
            .apply_state_difference(state.extract_state_diff_from_pending_state().unwrap())
            .unwrap();

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

        let contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();
        assert!(
            state
                .declare_contract_class(class_hash, contract_class.clone().try_into().unwrap())
                .is_ok()
        );
        assert!(state.state.class_hash_to_compiled_class.len() == 1);
        let declared_contract_class =
            state.state.class_hash_to_compiled_class.get(&class_hash.bytes()).unwrap().to_owned();

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

        state
            .deploy_contract(*dummy_contract_storage_key().get_contract_address(), dummy_felt())
            .unwrap();
        state.change_storage(dummy_contract_storage_key(), expected_result).unwrap();
        let generated_result = state.get_storage(dummy_contract_storage_key()).unwrap();
        assert_eq!(expected_result, generated_result);
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
