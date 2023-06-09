use starknet_in_rust::business_logic::fact_state::in_memory_state_reader::InMemoryStateReader;
use starknet_in_rust::business_logic::state::state_api::StateReader;
use starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass as StarknetInRustContractClass;
use starknet_in_rust::utils::Address;
use starknet_types::cairo_felt::Felt252;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::contract_storage_key::ContractStorageKey;
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::DevnetResult;

use crate::traits::{StateChanger, StateExtractor};

#[derive(Debug, Default)]
pub(crate) struct StarknetState {
    state: InMemoryStateReader,
}

impl StateChanger for StarknetState {
    fn declare_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> DevnetResult<()> {
        self.state
            .class_hash_to_contract_class_mut()
            .insert(class_hash.bytes(), StarknetInRustContractClass::try_from(contract_class)?);

        Ok(())
    }

    fn deploy_contract(
        &mut self,
        address: ContractAddress,
        class_hash: ClassHash,
    ) -> DevnetResult<()> {
        let addr: Address = address.try_into()?;
        self.state.address_to_class_hash_mut().insert(addr.clone(), class_hash.bytes());
        self.state.address_to_nonce_mut().insert(addr, Felt252::new(0));

        Ok(())
    }

    fn change_storage(
        &mut self,
        storage_key: ContractStorageKey,
        data: starknet_types::felt::Felt,
    ) -> DevnetResult<()> {
        self.state.address_to_storage_mut().insert(storage_key.try_into()?, data.into());

        Ok(())
    }

    fn increment_nonce(&mut self, address: ContractAddress) -> DevnetResult<()> {
        let addr: Address = address.try_into()?;
        let nonce = self.state.get_nonce_at(&addr)?;
        self.state.address_to_nonce_mut().insert(addr, nonce + Felt252::new(1));

        Ok(())
    }

    fn is_contract_declared(&mut self, class_hash: &ClassHash) -> DevnetResult<bool> {
        Ok(self.state.class_hash_to_contract_class.contains_key(&(class_hash.bytes())))
    }
}

impl StateExtractor for StarknetState {
    fn get_storage(&mut self, storage_key: ContractStorageKey) -> DevnetResult<Felt> {
        Ok(self.state.get_storage_at(&storage_key.try_into()?).map(Felt::from)?)
    }
}

#[cfg(test)]
mod tests {
    use starknet_types::cairo_felt::Felt252;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::felt::Felt;

    use super::StarknetState;
    use crate::utils::test_utils::{
        dummy_contract_address, dummy_contract_class, dummy_contract_storage_key, dummy_felt,
    };
    use crate::traits::{StateChanger, StateExtractor};
    #[test]
    fn declare_contract_class_successfully() {
        let mut state = StarknetState::default();
        let class_hash = Felt::from_prefixed_hex_str("0xFE").unwrap();

        assert!(state.declare_contract_class(class_hash, dummy_contract_class()).is_ok());
        assert!(state.state.class_hash_to_contract_class.len() == 1);
        let contract_class = state.state.class_hash_to_contract_class.get(&class_hash.bytes());
        assert!(contract_class.is_some());
        assert_eq!(*contract_class.unwrap(), dummy_contract_class().try_into().unwrap());
    }

    #[test]
    fn deploy_contract_class_successfully() {
        let (mut state, address) = setup();
        let felt = dummy_felt();

        assert!(state.deploy_contract(address, felt).is_ok());
        assert!(state.state.address_to_class_hash.len() == 1);
        assert!(state.state.address_to_class_hash.contains_key(&(address.try_into().unwrap())));
        assert!(state
            .state
            .address_to_nonce
            .get(&(address.try_into().unwrap()))
            .unwrap()
            .eq(&Felt252::from(0)));
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
        let contract_class = dummy_contract_class();
        let class_hash = dummy_felt();

        state.declare_contract_class(class_hash, contract_class).unwrap();
        state.deploy_contract(address, class_hash).unwrap();

        (state, address)
    }
}
