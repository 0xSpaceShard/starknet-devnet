use account::Account;
use constants::{
    ERC20_CONTRACT_ADDRESS, ERC20_CONTRACT_CLASS_HASH, ERC20_CONTRACT_PATH, UDC_CONTRACT_ADDRESS,
    UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_PATH,
};
use predeployed_accounts::PredeployedAccounts;
use starknet_types::DevnetResult;
use starknet_types::{error::Error, traits::HashProducer};
use state::StarknetState;
use system_contract::SystemContract;
use traits::{AccountGenerator, Accounted};

mod account;
mod constants;
mod predeployed_accounts;
mod state;
mod system_contract;
mod traits;
mod utils;

#[derive(Debug)]
pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    pub predeployed_accounts_initial_balance: u128,
}

impl Default for StarknetConfig {
    fn default() -> Self {
        Self { seed: 123, total_accounts: 3, predeployed_accounts_initial_balance: 100 }
    }
}
#[derive(Default)]
pub struct Starknet {
    state: StarknetState,
    predeployed_accounts: PredeployedAccounts,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_contract_class_json_str = std::fs::read_to_string(ERC20_CONTRACT_PATH)
            .map_err(|err| Error::ReadFileError {
                source: err,
                path: ERC20_CONTRACT_PATH.to_string(),
            })?;
        let erc20_fee_account = SystemContract::new(
            ERC20_CONTRACT_CLASS_HASH,
            ERC20_CONTRACT_ADDRESS,
            &erc20_contract_class_json_str,
        )?;

        let udc_contract_class_json_str =
            std::fs::read_to_string(UDC_CONTRACT_PATH).map_err(|err| Error::ReadFileError {
                source: err,
                path: UDC_CONTRACT_PATH.to_string(),
            })?;
        let udc_account = SystemContract::new(
            UDC_CONTRACT_CLASS_HASH,
            UDC_CONTRACT_ADDRESS,
            &udc_contract_class_json_str,
        )?;
        erc20_fee_account.deploy(&mut state)?;
        udc_account.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance,
            erc20_fee_account.get_address(),
        );
        let contract_class =
            utils::load_cairo_0_contract_class(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
        let class_hash = contract_class.generate_hash()?;

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            class_hash,
            contract_class,
        )?;
        for account in accounts {
            account.deploy(&mut state)?;
            account.set_initial_balance(&mut state)?;
        }

        Ok(Self { state, predeployed_accounts })
    }

    pub fn get_predeployed_accounts(&self) -> Vec<Account> {
        self.predeployed_accounts.get_accounts().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use starknet_types::{felt::Felt, DevnetResult};

    use crate::{traits::Accounted, Starknet, StarknetConfig};

    #[test]
    fn correct_initial_state_with_default_config() -> DevnetResult<()> {
        let config = StarknetConfig::default();
        let mut starknet = Starknet::new(&config)?;
        let predeployed_accounts = starknet.predeployed_accounts.get_accounts();
        let expected_balance = Felt::from(config.predeployed_accounts_initial_balance);

        for account in predeployed_accounts {
            let account_balance = account.get_balance(&mut starknet.state)?;
            assert_eq!(expected_balance, account_balance);
        }

        Ok(())
    }
}
