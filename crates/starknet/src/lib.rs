use account::Account;
use predeployed_accounts::PredeployedAccounts;
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;
use state::StarknetState;
use traits::{AccountGenerator, Accounted};

mod account;
mod constants;
mod predeployed_accounts;
mod services;
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

#[derive(Default)]
pub struct Starknet {
    state: StarknetState,
    predeployed_accounts: PredeployedAccounts,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_fee_contract = Starknet::create_erc20()?;
        let udc_contract = Starknet::create_udc20()?;

        erc20_fee_contract.deploy(&mut state)?;
        udc_contract.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance,
            erc20_fee_contract.get_address(),
        );
        let account_contract_class =
            utils::load_cairo_0_contract_class(constants::CAIRO_0_ACCOUNT_CONTRACT_PATH)?;
        let class_hash = account_contract_class.generate_hash()?;

        let accounts = predeployed_accounts.generate_accounts(
            config.total_accounts,
            class_hash,
            account_contract_class,
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

    fn starknet_config_for_test() -> StarknetConfig {
        StarknetConfig { seed: 123, total_accounts: 3, predeployed_accounts_initial_balance: 100 }
    }

    #[test]
    fn correct_initial_state_with_test_config() -> DevnetResult<()> {
        let config = starknet_config_for_test();
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
