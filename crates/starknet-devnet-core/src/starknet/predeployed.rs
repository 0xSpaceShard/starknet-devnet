use blockifier::context::BlockContext;
use blockifier::state::state_api::State;
use starknet_rs_core::types::Felt;
use starknet_rs_core::utils::cairo_short_string_to_felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::felt_from_prefixed_hex;

use super::starknet_config::StarknetConfig;
use crate::account::Account;
use crate::constants::{
    ARGENT_CONTRACT_CLASS_HASH, ARGENT_CONTRACT_SIERRA, ARGENT_MULTISIG_CONTRACT_CLASS_HASH,
    ARGENT_MULTISIG_CONTRACT_SIERRA, CHARGEABLE_ACCOUNT_ADDRESS, ETH_ERC20_CONTRACT_ADDRESS,
    ETH_ERC20_NAME, ETH_ERC20_SYMBOL, STRK_ERC20_CONTRACT_ADDRESS, STRK_ERC20_NAME,
    STRK_ERC20_SYMBOL, UDC_CONTRACT, UDC_CONTRACT_ADDRESS, UDC_CONTRACT_CLASS_HASH,
};
use crate::contract_class_choice::AccountContractClassChoice;
use crate::error::{DevnetResult, Error};
use crate::predeployed_accounts::PredeployedAccounts;
use crate::state::{CustomState, StarknetState};
use crate::system_contract::SystemContract;
use crate::traits::{AccountGenerator, Deployed};
use crate::utils::get_storage_var_address;

pub(crate) struct Predeployer<'a> {
    pub(crate) state: StarknetState,
    pub(crate) predeployed_accounts: PredeployedAccounts,
    block_context: BlockContext,
    config: &'a StarknetConfig,
    eth_fee_token_address: Felt,
    strk_fee_token_address: Felt,
    chain_id: Felt,
}
impl<'a> Predeployer<'a> {
    pub(crate) fn new(
        block_context: BlockContext,
        config: &'a StarknetConfig,
        state: StarknetState,
    ) -> DevnetResult<Self> {
        let chain_id = config.chain_id.to_felt();
        let predeployed_accounts = PredeployedAccounts::new(
            config.seed,
            config.predeployed_accounts_initial_balance.clone(),
            ContractAddress::new(ETH_ERC20_CONTRACT_ADDRESS)?,
            ContractAddress::new(STRK_ERC20_CONTRACT_ADDRESS)?,
            config.account_type,
            chain_id,
        );

        Ok(Self {
            block_context,
            config,
            eth_fee_token_address: ETH_ERC20_CONTRACT_ADDRESS,
            strk_fee_token_address: STRK_ERC20_CONTRACT_ADDRESS,
            state,
            predeployed_accounts,
            chain_id,
        })
    }

    pub(crate) fn deploy_eth_fee_token(&mut self) -> DevnetResult<&mut Self> {
        let eth_erc20_fee_contract = create_erc20_at_address_extended(
            self.eth_fee_token_address,
            self.config.eth_erc20_class_hash,
            &self.config.eth_erc20_contract_class,
        )?;

        eth_erc20_fee_contract.deploy(&mut self.state)?;

        initialize_erc20_at_address(
            &mut self.state,
            ETH_ERC20_CONTRACT_ADDRESS,
            ETH_ERC20_NAME,
            ETH_ERC20_SYMBOL,
        )?;

        Ok(self)
    }

    pub(crate) fn deploy_strk_fee_token(&mut self) -> DevnetResult<&mut Self> {
        let strk_erc20_fee_contract = create_erc20_at_address_extended(
            self.strk_fee_token_address,
            self.config.strk_erc20_class_hash,
            &self.config.strk_erc20_contract_class,
        )?;

        strk_erc20_fee_contract.deploy(&mut self.state)?;

        initialize_erc20_at_address(
            &mut self.state,
            STRK_ERC20_CONTRACT_ADDRESS,
            STRK_ERC20_NAME,
            STRK_ERC20_SYMBOL,
        )?;

        Ok(self)
    }

    pub(crate) fn deploy_udc(&mut self) -> DevnetResult<&mut Self> {
        let udc_contract = create_udc()?;
        udc_contract.deploy(&mut self.state)?;

        Ok(self)
    }

    pub(crate) fn deploy_accounts(&mut self) -> DevnetResult<&mut Self> {
        for account_class_choice in
            [AccountContractClassChoice::Cairo0, AccountContractClassChoice::Cairo1]
        {
            let class_wrapper = account_class_choice.get_class_wrapper()?;
            self.state.predeclare_contract_class(
                class_wrapper.class_hash,
                class_wrapper.contract_class,
            )?;
        }

        if self.config.predeclare_argent {
            for (class_hash, raw_sierra) in [
                (ARGENT_CONTRACT_CLASS_HASH, ARGENT_CONTRACT_SIERRA),
                (ARGENT_MULTISIG_CONTRACT_CLASS_HASH, ARGENT_MULTISIG_CONTRACT_SIERRA),
            ] {
                let contract_class =
                    ContractClass::Cairo1(ContractClass::cairo_1_from_sierra_json_str(raw_sierra)?);
                self.state.predeclare_contract_class(class_hash, contract_class)?;
            }
        }

        let eth_fee_token_address = ContractAddress::new(self.eth_fee_token_address)?;
        let strk_fee_token_address = ContractAddress::new(self.strk_fee_token_address)?;

        let accounts = self.predeployed_accounts.generate_accounts(
            self.config.total_accounts,
            self.config.account_contract_class_hash,
            &self.config.account_contract_class,
            self.block_context.clone(),
        )?;
        for account in accounts {
            account.deploy(&mut self.state)?;
        }

        let chargeable_account = Account::new_chargeable(
            eth_fee_token_address,
            strk_fee_token_address,
            self.block_context.clone(),
            self.chain_id,
        )?;
        chargeable_account.deploy(&mut self.state)?;

        Ok(self)
    }
}

pub(crate) fn create_erc20_at_address_extended(
    contract_address: Felt,
    class_hash: Felt,
    contract_class_json_str: &str,
) -> DevnetResult<SystemContract> {
    let erc20_fee_contract =
        SystemContract::new_cairo1(class_hash, contract_address, contract_class_json_str)?;
    Ok(erc20_fee_contract)
}

/// Set initial values of ERC20 contract storage
pub(crate) fn initialize_erc20_at_address(
    state: &mut StarknetState,
    contract_address: Felt,
    erc20_name: &str,
    erc20_symbol: &str,
) -> DevnetResult<()> {
    let contract_address = ContractAddress::new(contract_address)?;

    for (storage_var_name, storage_value) in [
        (
            "ERC20_name",
            cairo_short_string_to_felt(erc20_name)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?,
        ),
        (
            "ERC20_symbol",
            cairo_short_string_to_felt(erc20_symbol)
                .map_err(|err| Error::UnexpectedInternalError { msg: err.to_string() })?,
        ),
        ("ERC20_decimals", 18.into()),
        // necessary to set - otherwise minting txs cannot be executed
        ("Ownable_owner", felt_from_prefixed_hex(CHARGEABLE_ACCOUNT_ADDRESS)?),
    ] {
        let storage_var_address = get_storage_var_address(storage_var_name, &[])?.try_into()?;
        state.set_storage_at(contract_address.try_into()?, storage_var_address, storage_value)?;
    }

    Ok(())
}

pub(crate) fn create_udc() -> DevnetResult<SystemContract> {
    let udc_contract =
        SystemContract::new_cairo0(UDC_CONTRACT_CLASS_HASH, UDC_CONTRACT_ADDRESS, UDC_CONTRACT)?;

    Ok(udc_contract)
}

#[cfg(test)]
pub(crate) mod tests {
    use starknet_rs_core::types::Felt;

    use crate::constants::{CAIRO_1_ERC20_CONTRACT, CAIRO_1_ERC20_CONTRACT_CLASS_HASH};
    use crate::error::DevnetResult;
    use crate::system_contract::SystemContract;

    pub(crate) fn create_erc20_at_address(contract_address: Felt) -> DevnetResult<SystemContract> {
        let erc20_fee_contract = SystemContract::new_cairo1(
            CAIRO_1_ERC20_CONTRACT_CLASS_HASH,
            contract_address,
            CAIRO_1_ERC20_CONTRACT,
        )?;
        Ok(erc20_fee_contract)
    }
}
