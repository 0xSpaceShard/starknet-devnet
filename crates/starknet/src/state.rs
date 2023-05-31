use starknet_in_rust::{
    business_logic::{
        fact_state::in_memory_state_reader::InMemoryStateReader,
        state::{cached_state::CachedState, state_api::State},
    },
    services::api::contract_classes::deprecated_contract_class::ContractClass,
};

use crate::{error::Error, traits::StateChanger};

#[derive(Debug, Default)]
pub(crate) struct StarknetState {
    state: CachedState<InMemoryStateReader>,
}

impl StateChanger for StarknetState {
    fn declare_contract_class(
        &mut self,
        hash: crate::types::ClassHash,
        contract_class: &ContractClass,
    ) -> Result<(), Error> {
        self.state.set_contract_class(&hash.0, contract_class).map_err(Error::StarknetInRustStateError)
    }
}
