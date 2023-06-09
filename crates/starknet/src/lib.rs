use account::Account;
use constants::{
    ERC20_OZ_ACCOUNT_ADDRESS, ERC20_OZ_ACCOUNT_HASH, ERC20_OZ_ACCOUNT_PATH, UDC_OZ_ACCOUNT_ADDRESS,
    UDC_OZ_ACCOUNT_HASH, UDC_OZ_ACCOUNT_PATH,
};
use predeployed_account::PredeployedAccount;
use starknet_types::traits::HashProducer;
use starknet_types::DevnetResult;
use state::StarknetState;
use system_account::SystemAccount;
use traits::{AccountGenerator, Accounted};

mod account;
mod constants;
mod predeployed_account;
mod state;
mod system_account;
mod test_utils;
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
    predeployed_accounts: PredeployedAccount,
}

impl Starknet {
    pub fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut state = StarknetState::default();
        // deploy udc and erc20 contracts
        let erc20_contract_class_json_str = std::fs::read_to_string(ERC20_OZ_ACCOUNT_PATH)?;
        let erc20_fee_account = SystemAccount::new(
            ERC20_OZ_ACCOUNT_HASH,
            ERC20_OZ_ACCOUNT_ADDRESS,
            &erc20_contract_class_json_str,
        )?;

        let udc_contract_class_json_str = std::fs::read_to_string(UDC_OZ_ACCOUNT_PATH)?;
        let udc_account = SystemAccount::new(
            UDC_OZ_ACCOUNT_HASH,
            UDC_OZ_ACCOUNT_ADDRESS,
            &udc_contract_class_json_str,
        )?;
        erc20_fee_account.deploy(&mut state)?;
        udc_account.deploy(&mut state)?;

        let mut predeployed_accounts = PredeployedAccount::new(
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

        Ok(Self {
            state,
            predeployed_accounts,
        })
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
