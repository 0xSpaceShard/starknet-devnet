use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::{Cairo0Json, ContractClass};
use starknet_types::felt::{Balance, ClassHash, Felt};

use crate::error::DevnetResult;
use crate::traits::{Accounted, Deployed, StateChanger, StateExtractor};

pub(crate) struct SystemContract {
    class_hash: ClassHash,
    address: ContractAddress,
    contract_class: ContractClass,
}

impl SystemContract {
    pub(crate) fn new(
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
}

impl Deployed for SystemContract {
    fn deploy(&self, state: &mut (impl StateChanger + StateExtractor)) -> DevnetResult<()> {
        if !state.is_contract_declared(&self.class_hash) {
            state.declare_contract_class(self.class_hash, self.contract_class.clone())?;
        }

        state.deploy_contract(self.address, self.class_hash)?;

        Ok(())
    }

    fn get_address(&self) -> ContractAddress {
        self.address
    }
}

impl Accounted for SystemContract {
    fn set_initial_balance(
        &self,
        _state: &mut impl crate::traits::StateChanger,
    ) -> DevnetResult<()> {
        Ok(())
    }

    fn get_balance(
        &self,
        _state: &mut impl crate::traits::StateExtractor,
    ) -> DevnetResult<Balance> {
        Ok(Felt::default())
    }
}

#[cfg(test)]
mod tests {
    use starknet_types::contract_class::Cairo0Json;

    use super::SystemContract;
    use crate::constants::{
        ERC20_CONTRACT_ADDRESS, ERC20_CONTRACT_CLASS_HASH, ERC20_CONTRACT_PATH,
    };
    use crate::state::StarknetState;
    use crate::traits::Deployed;
    #[test]
    fn load_erc20_contract() {
        let json_str = std::fs::read_to_string(ERC20_CONTRACT_PATH).unwrap();
        assert!(Cairo0Json::raw_json_from_json_str(&json_str).is_ok());
    }

    #[test]
    fn system_account_deployed_successfully() {
        let mut state = StarknetState::default();
        let sys_contract = SystemContract::new(
            ERC20_CONTRACT_CLASS_HASH,
            ERC20_CONTRACT_ADDRESS,
            std::fs::read_to_string(ERC20_CONTRACT_PATH).unwrap().as_str(),
        )
        .unwrap();

        assert!(sys_contract.deploy(&mut state).is_ok());
    }
}
