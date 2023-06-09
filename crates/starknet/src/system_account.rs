use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{Balance, ClassHash, Felt};
use starknet_types::DevnetResult;


use crate::traits::Accounted;

pub(crate) struct SystemAccount {
    class_hash: ClassHash,
    address: ContractAddress,
    contract_class: ContractClass,
}

impl SystemAccount {
    pub(crate) fn new(
        class_hash: &str,
        address: &str,
        contract_class_json_str: &str,
    ) -> DevnetResult<Self> {
        Ok(Self {
            class_hash: Felt::from_prefixed_hex_str(class_hash)?,
            address: ContractAddress::new(Felt::from_prefixed_hex_str(address)?)?,
            contract_class: ContractClass::from_json_str(contract_class_json_str)?,
        })
    }
}

impl Accounted for SystemAccount {
    fn deploy(
        &self,
        state: &mut impl crate::traits::StateChanger,
    ) -> Result<(), starknet_types::error::Error> {
        if !state.is_contract_declared(&self.class_hash)? {
            state.declare_contract_class(self.class_hash, self.contract_class.clone())?;
        }

        state.deploy_contract(self.address, self.class_hash)?;

        Ok(())
    }

    fn set_initial_balance(
        &self,
        _state: &mut impl crate::traits::StateChanger,
    ) -> DevnetResult<()> {
        Ok(())
    }

    fn get_address(&self) -> ContractAddress {
        self.address
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
    use starknet_types::contract_class::ContractClass;

    use super::SystemAccount;
    use crate::constants::{
        ERC20_OZ_ACCOUNT_ADDRESS, ERC20_OZ_ACCOUNT_HASH, ERC20_OZ_ACCOUNT_PATH,
    };
    use crate::state::StarknetState;
    
    use crate::traits::Accounted;
    #[test]
    fn load_erc20_contract() {
        let json_str = std::fs::read_to_string(ERC20_OZ_ACCOUNT_PATH).unwrap();
        assert!(ContractClass::from_json_str(&json_str).is_ok());
    }

    #[test]
    fn system_account_deployed_successfully() {
        let mut state = StarknetState::default();
        let sys_account = SystemAccount::new(
            ERC20_OZ_ACCOUNT_HASH,
            ERC20_OZ_ACCOUNT_ADDRESS,
            std::fs::read_to_string(ERC20_OZ_ACCOUNT_PATH).unwrap().as_str(),
        )
        .unwrap();

        assert!(sys_account.deploy(&mut state).is_ok());
    }
}
