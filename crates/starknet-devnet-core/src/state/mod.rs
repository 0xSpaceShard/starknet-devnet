use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, CompiledClassHash, Felt};

use self::state_diff::StateDiff;
use crate::error::{DevnetResult, Error, StateError};
use crate::traits::{DevnetStateReader, StateChanger, StateExtractor};

pub(crate) mod state_diff;
pub mod state_update;

pub(crate) struct StarknetState {
    pub state: CachedState<DevnetState>,
    pub(crate) contract_classes: HashMap<ClassHash, ContractClass>,
}

impl Default for StarknetState {
    fn default() -> Self {
        Self {
            state: CachedState::new(Default::default(), Default::default()),
            contract_classes: Default::default(),
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct DevnetState {
    pub address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    pub address_to_nonce: HashMap<ContractAddress, Felt>,
    pub address_to_storage: HashMap<ContractStorageKey, Felt>,
    pub class_hash_to_compiled_class: HashMap<ClassHash, ContractClass>,
    pub class_hash_to_compiled_class_hash: HashMap<ClassHash, CompiledClassHash>,
}

impl crate::traits::DevnetStateReader for DevnetState {
    fn compiled_class_hash_at(&self, class_hash: &ClassHash) -> ClassHash {
        self.class_hash_to_compiled_class_hash.get(class_hash).cloned().unwrap_or_default()
    }

    fn storage_at(&self, storage_key: &ContractStorageKey) -> Felt {
        self.address_to_storage.get(storage_key).cloned().unwrap_or_default()
    }

    fn nonce_at(&self, address: &ContractAddress) -> Felt {
        self.address_to_nonce.get(address).cloned().unwrap_or_default()
    }

    fn class_hash_at(&self, address: &ContractAddress) -> ClassHash {
        self.address_to_class_hash.get(address).cloned().unwrap_or_default()
    }

    fn contract_class_at(&self, class_hash: &ClassHash) -> DevnetResult<ContractClass> {
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
    }
}

impl blockifier::state::state_api::StateReader for DevnetState {
    fn get_storage_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> blockifier::state::state_api::StateResult<starknet_api::hash::StarkFelt> {
        let storage = crate::traits::DevnetStateReader::storage_at(
            self,
            &ContractStorageKey::new(contract_address.into(), key.0.into()),
        );
        Ok(storage.into())
    }

    fn get_nonce_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::Nonce> {
        let nonce = crate::traits::DevnetStateReader::nonce_at(self, &contract_address.into());
        Ok(starknet_api::core::Nonce(nonce.into()))
    }

    fn get_class_hash_at(
        &mut self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::ClassHash> {
        let class_hash =
            crate::traits::DevnetStateReader::class_hash_at(self, &contract_address.into());
        Ok(starknet_api::core::ClassHash(class_hash.into()))
    }

    fn get_compiled_contract_class(
        &mut self,
        class_hash: &starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<
        blockifier::execution::contract_class::ContractClass,
    > {
        let contract_class =
            crate::traits::DevnetStateReader::contract_class_at(self, &class_hash.0.into())
                .map_err(|_| {
                    blockifier::state::errors::StateError::UndeclaredClassHash(*class_hash)
                })?;

        blockifier::execution::contract_class::ContractClass::try_from(contract_class)
            .map_err(|err| blockifier::state::errors::StateError::StateReadError(err.to_string()))
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        let compiled_class_hash =
            crate::traits::DevnetStateReader::compiled_class_hash_at(self, &(class_hash.0.into()));
        Ok(starknet_api::core::CompiledClassHash(compiled_class_hash.into()))
    }
}

impl Clone for StarknetState {
    fn clone(&self) -> Self {
        Self {
            state: CachedState::new(self.state.state.clone(), Default::default()),
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
        let persistent_state = &mut self.state.state;

        persistent_state.class_hash_to_compiled_class.insert(class_hash, contract_class);

        Ok(())
    }

    fn deploy_contract(
        &mut self,
        address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        let persistent_state = &mut self.state.state;

        persistent_state.address_to_class_hash.insert(address, class_hash);
        persistent_state.address_to_nonce.insert(address, Felt::from(0));

        Ok(())
    }

    fn change_storage(&mut self, storage_key: ContractStorageKey, data: Felt) -> DevnetResult<()> {
        let persistent_state = &mut self.state.state;

        persistent_state.address_to_storage.insert(storage_key, data);

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()> {
        let nonce = self.state.state.nonce_at(&address);
        let persistent_state = &mut self.state.state;

        persistent_state
            .address_to_nonce
            .insert(address, (Felt252::from(nonce) + Felt252::new(1)).into());

        Ok(())
    }
}

impl StateExtractor for StarknetState {
    fn get_storage(&self, storage_key: ContractStorageKey) -> DevnetResult<Felt> {
        if !self.is_contract_deployed(storage_key.get_contract_address()) {
            return Err(Error::ContractNotFound);
        }

        let data = self.state.state.storage_at(&storage_key);

        Ok(data)
    }

    fn is_contract_declared(&mut self, class_hash: &ClassHash) -> bool {
        self.state.state.class_hash_to_compiled_class_hash.contains_key(class_hash)
            || self.state.state.class_hash_to_compiled_class.contains_key(class_hash)
    }

    fn is_contract_deployed(&self, address: &ContractAddress) -> bool {
        self.state.state.address_to_class_hash.contains_key(address)
    }

    fn get_class_hash_at_contract_address(
        &mut self,
        contract_address: &ContractAddress,
    ) -> DevnetResult<ClassHash> {
        Ok(self
            .state
            .get_class_hash_at((*contract_address).try_into()?)
            .map(|f| Felt::from(f.0))?)
    }

    fn extract_state_diff_from_pending_state(&mut self) -> DevnetResult<StateDiff> {
        StateDiff::difference_between_old_and_new_state(self.state.state.clone(), &mut self.state)
    }

    fn get_nonce(&self, address: &ContractAddress) -> DevnetResult<Felt> {
        if !self.is_contract_deployed(address) {
            return Err(Error::ContractNotFound);
        }

        Ok(self.state.state.nonce_at(address))
    }
}

#[cfg(test)]
mod tests {
    use blockifier::state::state_api::{State, StateReader};
    use blockifier::test_utils::DictStateReader;
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
    fn synchronize_states_after_changing_pending_state_it_should_be_empty() {
        let mut state = StarknetState::default();
        let contract_address = starknet_api::core::ContractAddress::try_from(
            *dummy_contract_storage_key().get_contract_address(),
        )
        .unwrap();
        let storage_key = starknet_api::state::StorageKey(
            (*dummy_contract_storage_key().get_storage_key()).try_into().unwrap(),
        );

        state.state.set_storage_at(contract_address, storage_key, dummy_felt().into());

        state.state.get_storage_at(contract_address, storage_key).unwrap();

        assert_eq!(
            state.state.get_storage_at(contract_address, storage_key).unwrap(),
            Felt::default().into()
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
        assert!(state.state.state.class_hash_to_compiled_class.len() == 1);
        let declared_contract_class =
            state.state.state.class_hash_to_compiled_class.get(&class_hash).unwrap().to_owned();

        match declared_contract_class {
            starknet_types::contract_class::ContractClass::Cairo0(deprecated_contract_class) => {
                assert_eq!(
                    blockifier::execution::contract_class::ContractClassV0::try_from(
                        deprecated_contract_class
                    )
                    .unwrap(),
                    blockifier::execution::contract_class::ContractClassV0::try_from(
                        contract_class
                    )
                    .unwrap()
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
        assert!(state.state.state.address_to_class_hash.len() == 1);
        assert!(state.state.state.address_to_class_hash.contains_key(&address));
        assert!(state.state.state.address_to_nonce.get(&address).unwrap().eq(&Felt::from(0)));
    }

    #[test]
    fn change_storage_successfully() {
        let mut state = StarknetState::default();
        let storage_key = dummy_contract_storage_key();

        assert!(state.change_storage(storage_key, dummy_felt()).is_ok());
        assert!(state.state.state.address_to_storage.len() == 1);
        assert!(state.state.state.address_to_storage.contains_key(&(storage_key)));
    }

    #[test]
    fn increment_nonce_successful() {
        let (mut state, address) = setup();

        state.increment_nonce(address).unwrap();
        let nonce = *state.state.state.address_to_nonce.get(&address).unwrap();
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
