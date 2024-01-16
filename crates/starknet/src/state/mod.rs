use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::test_utils::DictStateReader;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;

pub(crate) mod state_diff;
pub mod state_update;

pub(crate) struct StarknetState {
    pub state: CachedState<DictStateReader>,
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

impl StarknetState {
    fn get_contract_class(&self, class_hash: &starknet_api::core::ClassHash) {
        todo!("sierra for cairo1, regular artifact for cairo0")
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
        self.get_compiled_contract_class(class_hash)
    }

    fn get_compiled_class_hash(
        &mut self,
        class_hash: starknet_api::core::ClassHash,
    ) -> blockifier::state::state_api::StateResult<starknet_api::core::CompiledClassHash> {
        self.state.get_compiled_class_hash(class_hash)
    }
}

impl Clone for StarknetState {
    fn clone(&self) -> Self {
        Self {
            state: CachedState::new(
                DictStateReader {
                    storage_view: self.state.state.storage_view.clone(),
                    address_to_nonce: self.state.state.address_to_nonce.clone(),
                    address_to_class_hash: self.state.state.address_to_class_hash.clone(),
                    class_hash_to_class: self.state.state.class_hash_to_class.clone(),
                    class_hash_to_compiled_class_hash: self
                        .state
                        .state
                        .class_hash_to_compiled_class_hash
                        .clone(),
                },
                Default::default(),
            ),
            contract_classes: self.contract_classes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use blockifier::state::state_api::{State, StateReader};
    use blockifier::test_utils::DictStateReader;
    use starknet_api::state::StorageKey;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::contract_storage_key::ContractStorageKey;
    use starknet_types::felt::{ClassHash, Felt};

    use super::StarknetState;
    use crate::error::Error;
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_contract_address, dummy_contract_storage_key, dummy_felt,
    };

    #[test]
    fn apply_state_update_for_contract_class_successfully() {
        let mut state = StarknetState::default();

        let class_hash = dummy_felt().into();
        let contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();

        state
            .state
            .set_contract_class(
                &class_hash,
                ContractClass::Cairo0(contract_class.clone()).try_into().unwrap(),
            )
            .unwrap();

        state.contract_classes.insert(class_hash.into(), contract_class.into());

        assert!(!state.is_contract_declared(&dummy_felt()));
        state.state.get_compiled_contract_class(&class_hash).unwrap();
        let state_diff = state.extract_state_diff_from_pending_state().unwrap();
        state.apply_state_difference(state_diff).unwrap();

        assert!(state.is_contract_declared(&dummy_felt()));
    }

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

        state.clear_dirty_state();

        assert_eq!(
            state.state.get_storage_at(contract_address, storage_key).unwrap(),
            Felt::default().into()
        );
    }

    #[test]
    fn apply_state_updates_for_storage_successfully() {
        let mut state = StarknetState::default();

        let contract_address = *dummy_contract_storage_key().get_contract_address();
        state
            .state
            .set_class_hash_at(
                contract_address.try_into().unwrap(),
                starknet_api::core::ClassHash(dummy_felt().into()),
            )
            .unwrap();

        state.state.set_storage_at(
            contract_address.try_into().unwrap(),
            StorageKey((*dummy_contract_storage_key().get_storage_key()).try_into().unwrap()),
            dummy_felt().into(),
        );

        let get_storage_result = state.get_storage(dummy_contract_storage_key());

        assert!(matches!(get_storage_result.unwrap_err(), Error::ContractNotFound));

        // apply changes to persistent state
        let state_diff = state.extract_state_diff_from_pending_state().unwrap();
        state.apply_state_difference(state_diff).unwrap();
        assert_eq!(state.get_storage(dummy_contract_storage_key()).unwrap(), dummy_felt());
    }

    #[test]
    fn apply_state_updates_for_address_nonce_successfully() {
        let mut state = StarknetState::default();

        state.deploy_contract(dummy_contract_address(), dummy_felt()).unwrap();
        let contract_address = dummy_contract_address();

        // check if current nonce is 0
        assert!(
            state.state.state.address_to_nonce.get(&contract_address).unwrap().eq(&Felt::from(0))
        );

        state.clear_dirty_state();
        state.state.increment_nonce(contract_address.try_into().unwrap()).unwrap();
        let state_diff = state.extract_state_diff_from_pending_state().unwrap();
        state.apply_state_difference(state_diff).unwrap();

        // check if nonce update was correct
        assert!(
            state.state.state.address_to_nonce.get(&contract_address).unwrap().eq(&Felt::from(1))
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

    fn setup_devnet_state() -> (StarknetState, ClassHash, ContractAddress, ContractStorageKey) {
        let mut state = StarknetState::default();
        let class_hash = dummy_felt();
        let compiled_class_hash = Felt::from(1);
        let address = dummy_contract_address();
        let storage_key = dummy_contract_storage_key();

        unimplemented!()
        //     state.class_hash_to_compiled_class_hash.insert(class_hash, compiled_class_hash);
        //     state
        //         .class_hash_to_compiled_class
        //         .insert(dummy_felt(), dummy_cairo_1_contract_class().into());
        //     state.address_to_class_hash.insert(address, class_hash);
        //     state.address_to_storage.insert(storage_key, class_hash);
        //     state.address_to_nonce.insert(address, Felt::from(1));

        //     (state, class_hash, address, storage_key)
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
