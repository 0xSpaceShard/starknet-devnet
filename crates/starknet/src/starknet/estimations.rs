use blockifier::fee::fee_utils::{calculate_l1_gas_by_vm_usage, extract_l1_gas_and_vm_usage};
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_rs_core::types::{BlockId, MsgFromL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::BroadcastedTransaction;

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;
use crate::state::StarknetState;

pub fn estimate_fee(
    starknet: &Starknet,
    block_id: BlockId,
    transactions: &[BroadcastedTransaction],
    charge_fee: Option<bool>,
    validate: Option<bool>,
) -> DevnetResult<Vec<FeeEstimateWrapper>> {
    let mut state = starknet.get_state_at(&block_id)?.clone();
    let chain_id = starknet.chain_id().to_felt();

    let transactions = transactions
        .iter()
        .map(|txn| Ok(txn.to_blockifier_account_transaction(chain_id, true)?))
        .collect::<DevnetResult<Vec<AccountTransaction>>>()?;

    transactions
        .into_iter()
        .map(|transaction| {
            estimate_transaction_fee(
                &mut state,
                &starknet.block_context,
                blockifier::transaction::transaction_execution::Transaction::AccountTransaction(
                    transaction,
                ),
                charge_fee,
                validate,
            )
        })
        .collect()
}

pub fn estimate_message_fee(
    starknet: &Starknet,
    block_id: BlockId,
    message: MsgFromL1,
) -> DevnetResult<FeeEstimateWrapper> {
    let estimate_message_fee = EstimateMessageFeeRequestWrapper::new(block_id, message);
    let mut state = starknet.get_state_at(estimate_message_fee.get_raw_block_id())?.clone();

    match starknet
        .get_class_hash_at(block_id, ContractAddress::new(estimate_message_fee.get_to_address())?)
    {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }?;

    let l1_transaction = estimate_message_fee.create_blockifier_l1_transaction()?;

    estimate_transaction_fee(
        &mut state,
        &starknet.block_context,
        blockifier::transaction::transaction_execution::Transaction::L1HandlerTransaction(
            l1_transaction,
        ),
        None,
        None,
    )
}

fn estimate_transaction_fee(
    state: &mut StarknetState,
    block_context: &blockifier::block_context::BlockContext,
    transaction: blockifier::transaction::transaction_execution::Transaction,
    charge_fee: Option<bool>,
    validate: Option<bool>,
) -> DevnetResult<FeeEstimateWrapper> {
    let transaction_execution_info = transaction.execute(
        &mut state.state,
        block_context,
        charge_fee.unwrap_or(false),
        validate.unwrap_or(true),
    )?;

    if let Some(revert_error) = transaction_execution_info.revert_error {
        return Err(Error::ExecutionError { revert_error });
    }

    let (l1_gas_usage, vm_resources) =
        extract_l1_gas_and_vm_usage(&transaction_execution_info.actual_resources);
    let l1_gas_by_vm_usage = calculate_l1_gas_by_vm_usage(block_context, &vm_resources)?;
    let total_l1_gas_usage = l1_gas_usage as f64 + l1_gas_by_vm_usage;
    let total_l1_gas_usage = total_l1_gas_usage.ceil() as u64;

    let gas_price = block_context.gas_prices.eth_l1_gas_price as u64;

    Ok(FeeEstimateWrapper::new(total_l1_gas_usage, gas_price, total_l1_gas_usage * gas_price))
}
