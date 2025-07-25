use starknet_core::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
use starknet_core::error::DevnetResult;
use starknet_core::starknet::Starknet;
use starknet_rs_core::types::{Felt, TransactionExecutionStatus};
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::join_felts;
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::block::{BlockId, BlockTag};
use starknet_types::rpc::transaction_receipt::FeeUnit;

use crate::api::Api;
use crate::api::http::models::{MintTokensRequest, MintTokensResponse};
use crate::api::json_rpc::DevnetResponse;
use crate::api::json_rpc::error::{ApiError, StrictRpcResult};

/// get the balance of the `address`
pub fn get_balance(
    starknet: &mut Starknet,
    address: ContractAddress,
    erc20_address: ContractAddress,
    block_id: BlockId,
) -> Result<BigUint, ApiError> {
    let balance_selector =
        starknet_rs_core::utils::get_selector_from_name("balanceOf").map_err(|err| {
            starknet_core::error::Error::UnexpectedInternalError { msg: err.to_string() }
        })?;
    let new_balance_raw = starknet.call(
        &block_id,
        erc20_address.into(),
        balance_selector,
        vec![Felt::from(address)], // calldata = the address being queried
    )?;

    match new_balance_raw.as_slice() {
        // format balance for output - initially it is a 2-member vector (low, high)
        [new_balance_low, new_balance_high] => Ok(join_felts(new_balance_high, new_balance_low)),
        _ => {
            let msg = format!(
                "Fee token contract expected to return 2 values; got: {new_balance_raw:?}",
            );

            Err(ApiError::StarknetDevnetError(
                starknet_core::error::Error::UnexpectedInternalError { msg },
            ))
        }
    }
}

/// Returns the address of the ERC20 (fee token) contract associated with the unit.
pub fn get_erc20_address(unit: &FeeUnit) -> DevnetResult<ContractAddress> {
    let erc20_contract_address = match unit {
        FeeUnit::WEI => ETH_ERC20_CONTRACT_ADDRESS,
        FeeUnit::FRI => STRK_ERC20_CONTRACT_ADDRESS,
    };

    Ok(ContractAddress::new(erc20_contract_address)?)
}

pub(crate) async fn mint_impl(api: &Api, request: MintTokensRequest) -> StrictRpcResult {
    let mut starknet = api.starknet.lock().await;
    let unit = request.unit.unwrap_or(FeeUnit::FRI);
    let erc20_address = get_erc20_address(&unit)?;

    // increase balance
    let tx_hash = starknet.mint(request.address, request.amount, erc20_address).await?;

    let tx = starknet.get_transaction_execution_and_finality_status(tx_hash)?;
    match tx.execution_status {
        TransactionExecutionStatus::Succeeded => {
            let new_balance = get_balance(
                &mut starknet,
                request.address,
                erc20_address,
                BlockId::Tag(BlockTag::PreConfirmed),
            )?;
            let new_balance = new_balance.to_str_radix(10);

            Ok(DevnetResponse::MintTokens(MintTokensResponse { new_balance, unit, tx_hash }).into())
        }
        TransactionExecutionStatus::Reverted => Err(ApiError::MintingReverted {
            tx_hash,
            revert_reason: tx.failure_reason.map(|reason| {
                if reason.contains("u256_add Overflow") {
                    "The requested minting amount overflows the token contract's total_supply."
                        .into()
                } else {
                    reason
                }
            }),
        }),
    }
}
