use starknet_in_rust::business_logic::{
    fact_state::in_memory_state_reader::InMemoryStateReader,
    state::{cached_state::CachedState, state_api::State},
};

use starknet_types::{contract_class::ContractClass, error::Error, felt::ClassHash, traits::StateChanger};

#[derive(Debug, Default)]
pub(crate) struct StarknetState {
    state: CachedState<InMemoryStateReader>,
}

impl StateChanger for StarknetState {
    fn declare_contract_class(&mut self, hash: ClassHash, contract_class: &ContractClass) -> Result<(), Error> {
        self.state
            .set_contract_class(&hash.bytes(), &(contract_class.clone().try_into()?))
            .map_err(Error::StarknetInRustStateError)
    }
}
