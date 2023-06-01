use starknet_in_rust::{
    business_logic::{
        fact_state::in_memory_state_reader::InMemoryStateReader,
        state::{cached_state::CachedState, state_api::State},
    },
};

use crate::{error::Error, traits::StateChanger, types::{felt::ClassHash, contract_class::ContractClass}};

#[derive(Debug, Default)]
pub(crate) struct StarknetState {
    state: CachedState<InMemoryStateReader>,
}

impl StateChanger for StarknetState {
    fn declare_contract_class(&mut self, hash: ClassHash, contract_class: &ContractClass) -> Result<(), Error> {
        self.state.set_contract_class(&hash.0, &(contract_class.into())).map_err(Error::StarknetInRustStateError)
    }
}
