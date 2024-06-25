use axum::extract::{Query, State};
use axum::Json;
use starknet_rs_core::types::BlockTag;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::Felt;
use starknet_types::rpc::transaction_receipt::FeeUnit;

use super::mint_token::{get_balance, get_erc20_address};
use crate::api::http::error::HttpApiError;
use crate::api::http::models::{AccountBalanceResponse, SerializableAccount};
use crate::api::http::{HttpApiHandler, HttpApiResult};
use crate::api::Api;

#[derive(serde::Deserialize, Debug)]
pub struct PredeployedAccountsQuery {
    with_balance: Option<bool>,
}

pub async fn get_predeployed_accounts(
    State(state): State<HttpApiHandler>,
    Query(params): Query<PredeployedAccountsQuery>,
) -> HttpApiResult<Json<Vec<SerializableAccount>>> {
    get_predeployed_accounts_impl(&state.api, params).await.map(Json::from)
}

pub(crate) async fn get_predeployed_accounts_impl(
    api: &Api,
    params: PredeployedAccountsQuery,
) -> HttpApiResult<Vec<SerializableAccount>> {
    let mut starknet = api.starknet.write().await;
    let mut predeployed_accounts: Vec<_> = starknet
        .get_predeployed_accounts()
        .into_iter()
        .map(|acc| SerializableAccount {
            initial_balance: acc.initial_balance.to_string(),
            address: acc.account_address,
            public_key: acc.public_key,
            private_key: acc.private_key,
            balances: None,
        })
        .collect();

    // handle with_balance query string
    if params.with_balance == Some(true) {
        for account in predeployed_accounts.iter_mut() {
            let mut balances = Vec::new();

            for unit in [FeeUnit::WEI, FeeUnit::FRI] {
                let erc20_address = get_erc20_address(&unit);
                let amount =
                    get_balance(&mut starknet, account.address, erc20_address, BlockTag::Pending)
                        .map_err(|e| HttpApiError::GeneralError(e.to_string()))?;
                let balance = AccountBalanceResponse { amount: amount.to_string(), unit };

                balances.push(balance)
            }

            account.balances = Some(balances);
        }
    }

    Ok(predeployed_accounts)
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BalanceQuery {
    address: Felt,
    unit: Option<FeeUnit>,
    block_tag: Option<BlockTag>,
}

pub async fn get_account_balance(
    State(state): State<HttpApiHandler>,
    Query(params): Query<BalanceQuery>,
) -> HttpApiResult<Json<AccountBalanceResponse>> {
    get_account_balance_impl(&state.api, params).await.map(Json::from)
}

pub(crate) async fn get_account_balance_impl(
    api: &Api,
    params: BalanceQuery,
) -> HttpApiResult<AccountBalanceResponse> {
    let account_address = ContractAddress::new(params.address)
        .map_err(|e| HttpApiError::InvalidValueError { msg: e.to_string() })?;
    let unit = params.unit.unwrap_or(FeeUnit::WEI);
    let erc20_address = get_erc20_address(&unit);

    let mut starknet = api.starknet.write().await;

    let amount = get_balance(
        &mut starknet,
        account_address,
        erc20_address,
        params.block_tag.unwrap_or(BlockTag::Latest),
    )
    .map_err(|e| HttpApiError::GeneralError(e.to_string()))?;

    Ok(AccountBalanceResponse { amount: amount.to_string(), unit })
}
