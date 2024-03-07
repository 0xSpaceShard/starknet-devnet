use blockifier::state::state_api::StateReader;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0Json, ContractClass};
use starknet_types::felt::{Balance, ClassHash, Felt};

use crate::error::DevnetResult;
use crate::state::state_readers::DictState;
use crate::state::{CustomState, StarknetState};
use crate::traits::{Accounted, Deployed};

pub(crate) struct SystemContract {
    class_hash: ClassHash,
    address: ContractAddress,
    contract_class: ContractClass,
}

impl SystemContract {
    pub(crate) fn new_cairo0(
        class_hash: &str,
        address: &str,
        contract_class_json_str: &str,
    ) -> DevnetResult<Self> {
        Ok(Self {
            class_hash: Felt::from_prefixed_hex_str(class_hash)?,
            address: ContractAddress::new(Felt::from_prefixed_hex_str(address)?)?,
            contract_class: Cairo0Json::raw_json_from_json_str(contract_class_json_str)?.into(),
        })
    }

    pub(crate) fn new_cairo1(
        class_hash: &str,
        address: &str,
        contract_class_json_str: &str,
    ) -> DevnetResult<Self> {
        Ok(Self {
            class_hash: Felt::from_prefixed_hex_str(class_hash)?,
            address: ContractAddress::new(Felt::from_prefixed_hex_str(address)?)?,
            contract_class: ContractClass::cairo_1_from_sierra_json_str(contract_class_json_str)?
                .into(),
        })
    }
}

impl Deployed for SystemContract {
    fn deploy(&self, state: &mut StarknetState) -> DevnetResult<()> {
        self.declare_if_undeclared(state, self.class_hash, &self.contract_class)?;
        state.predeploy_contract(self.address, self.class_hash)?;
        Ok(())
    }

    fn get_address(&self) -> ContractAddress {
        self.address
    }
}

impl Accounted for SystemContract {
    fn set_initial_balance(&self, _state: &mut DictState) -> DevnetResult<()> {
        Ok(())
    }

    fn get_balance(
        &self,
        _state: &mut impl StateReader,
        _token: crate::account::FeeToken,
    ) -> DevnetResult<Balance> {
        Ok(Felt::default())
    }
}

#[cfg(test)]
mod tests {
    use starknet_types::contract_class::ContractClass;

    use super::SystemContract;
    use crate::constants::{
        CAIRO_1_ERC20_CONTRACT_CLASS_HASH, CAIRO_1_ERC20_CONTRACT_PATH, ETH_ERC20_CONTRACT_ADDRESS,
    };
    use crate::state::StarknetState;
    use crate::traits::Deployed;

    #[test]
    fn load_erc20_contract() {
        let json_str = std::fs::read_to_string(CAIRO_1_ERC20_CONTRACT_PATH).unwrap();
        assert!(ContractClass::cairo_1_from_sierra_json_str(&json_str).is_ok());
    }

    #[test]
    fn system_account_deployed_successfully() {
        let mut state = StarknetState::default();
        let sys_contract = SystemContract::new_cairo1(
            CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
            ETH_ERC20_CONTRACT_ADDRESS,
            std::fs::read_to_string(CAIRO_1_ERC20_CONTRACT_PATH).unwrap().as_str(),
        )
        .unwrap();

        assert!(sys_contract.deploy(&mut state).is_ok());
    }
}
