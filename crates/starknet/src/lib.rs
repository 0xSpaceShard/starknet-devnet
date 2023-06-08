use account::Account;
use block::StarknetBlocks;
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
use transaction::StarknetTransactions;

mod account;
mod block;
mod constants;
mod predeployed_account;
mod state;
mod system_account;
mod test_utils;
mod traits;
mod transaction;
mod utils;

pub struct StarknetConfig {
    pub seed: u32,
    pub total_accounts: u8,
    pub predeployed_accounts_initial_balance: u32,
}
#[derive(Default)]
pub(crate) struct Starknet {
    pub blocks: StarknetBlocks,
    // pub block_context: BlockContext,
    pub state: StarknetState,
    pub transactions: StarknetTransactions,
}

impl Starknet {
    fn new(config: &StarknetConfig) -> DevnetResult<Self> {
        let mut this = Self::default();
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
        erc20_fee_account.deploy(&mut this.state)?;
        udc_account.deploy(&mut this.state)?;

        let predeployed_accounts = PredeployedAccount::new(
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
            account.deploy(&mut this.state)?;
            account.set_initial_balance(&mut this.state)?;
        }

        Ok(this)
    }
}
