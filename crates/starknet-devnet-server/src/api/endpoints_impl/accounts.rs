use serde::Deserialize;
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::block::{BlockId, BlockTag};
use starknet_types::rpc::transaction_receipt::FeeUnit;

use super::mint_token::{get_balance, get_erc20_address};
use crate::api::Api;
use crate::api::error::ApiError;
use crate::api::models::{AccountBalanceResponse, AccountBalancesResponse, SerializableAccount};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PredeployedAccountsQuery {
    pub with_balance: Option<bool>,
}

pub(crate) fn get_balance_unit(
    starknet: &mut Starknet,
    address: ContractAddress,
    unit: FeeUnit,
) -> Result<AccountBalanceResponse, ApiError> {
    let erc20_address =
        get_erc20_address(&unit).map_err(|e| ApiError::InvalidValueError { msg: e.to_string() })?;
    let amount =
        get_balance(starknet, address, erc20_address, BlockId::Tag(BlockTag::PreConfirmed))
            .map_err(|e| ApiError::GeneralError(e.to_string()))?;

    Ok(AccountBalanceResponse { amount: amount.to_string(), unit })
}

pub(crate) async fn get_predeployed_accounts_impl(
    api: &Api,
    params: PredeployedAccountsQuery,
) -> Result<Vec<SerializableAccount>, ApiError> {
    let mut starknet = api.starknet.lock().await;
    let mut predeployed_accounts: Vec<_> = starknet
        .get_predeployed_accounts()
        .into_iter()
        .map(|acc| SerializableAccount {
            initial_balance: acc.initial_balance.to_string(),
            address: acc.account_address,
            public_key: acc.keys.public_key,
            private_key: acc.keys.private_key,
            balance: None,
        })
        .collect();

    // handle with_balance query string
    if let Some(true) = params.with_balance {
        for account in predeployed_accounts.iter_mut() {
            let eth = get_balance_unit(&mut starknet, account.address, FeeUnit::WEI)?;
            let strk = get_balance_unit(&mut starknet, account.address, FeeUnit::FRI)?;

            account.balance = Some(AccountBalancesResponse { eth, strk });
        }
    }

    Ok(predeployed_accounts)
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BalanceQuery {
    address: Felt,
    unit: Option<FeeUnit>,
    block_id: Option<BlockId>,
}

pub(crate) async fn get_account_balance_impl(
    api: &Api,
    params: BalanceQuery,
) -> Result<AccountBalanceResponse, ApiError> {
    let account_address = ContractAddress::new(params.address)
        .map_err(|e| ApiError::InvalidValueError { msg: e.to_string() })?;
    let unit = params.unit.unwrap_or(FeeUnit::FRI);
    let erc20_address =
        get_erc20_address(&unit).map_err(|e| ApiError::InvalidValueError { msg: e.to_string() })?;

    let mut starknet = api.starknet.lock().await;

    let amount = get_balance(
        &mut starknet,
        account_address,
        erc20_address,
        params.block_id.unwrap_or(BlockId::Tag(BlockTag::Latest)),
    )
    .map_err(|e| ApiError::GeneralError(e.to_string()))?;

    Ok(AccountBalanceResponse { amount: amount.to_string(), unit })
}
