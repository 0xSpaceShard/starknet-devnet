use blockifier::fee::fee_utils::{self};
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::objects::HasRelatedFeeType;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_rs_core::types::{BlockId, Felt, MsgFromL1, PriceUnit};
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::BroadcastedTransaction;

use crate::error::{DevnetResult, Error};
use crate::stack_trace::ErrorStack;
use crate::starknet::Starknet;
use crate::utils::get_versioned_constants;

pub fn estimate_fee(
    starknet: &mut Starknet,
    block_id: &BlockId,
    transactions: &[BroadcastedTransaction],
    charge_fee: Option<bool>,
    validate: Option<bool>,
    return_error_on_reverted_execution: bool,
) -> DevnetResult<Vec<FeeEstimateWrapper>> {
    let chain_id = starknet.chain_id().to_felt();
    let block_context = starknet.block_context.clone();
    let cheats = starknet.cheats.clone();
    let state = starknet.get_mut_state_at(block_id)?;

    let transactions = {
        transactions
            .iter()
            .map(|txn| {
                Ok((
                    txn.to_blockifier_account_transaction(&chain_id, true)?,
                    Starknet::should_transaction_skip_validation_if_sender_is_impersonated(
                        state, &cheats, txn,
                    )?,
                ))
            })
            .collect::<DevnetResult<Vec<(AccountTransaction, bool)>>>()?
    };

    let mut transactional_state = CachedState::create_transactional(&mut state.state);

    let mut estimations = vec![];
    for (tx_idx, (tx, skip_validate_due_to_impersonation)) in transactions.into_iter().enumerate() {
        // If skip validate is true, this tx has to skip validation, because the sender is
        // impersonated. Otherwise use the validate parameter passed to the estimateFee request.
        let validate = skip_validate_due_to_impersonation.then_some(false).or(validate);
        let estimation = estimate_transaction_fee(
            &mut transactional_state,
            &block_context,
            blockifier::transaction::transaction_execution::Transaction::AccountTransaction(tx),
            charge_fee,
            validate,
            return_error_on_reverted_execution,
        )
        .map_err(|e| match e {
            Error::ContractExecutionError(error_stack) => {
                Error::ContractExecutionErrorInSimulation { failure_index: tx_idx, error_stack }
            }
            other => Error::ContractExecutionErrorInSimulation {
                failure_index: tx_idx,
                error_stack: ErrorStack::from_str_err(&other.to_string()),
            },
        })?;
        estimations.push(estimation);
    }

    Ok(estimations)
}

pub fn estimate_message_fee(
    starknet: &mut Starknet,
    block_id: &BlockId,
    message: MsgFromL1,
) -> DevnetResult<FeeEstimateWrapper> {
    let estimate_message_fee = EstimateMessageFeeRequestWrapper::new(*block_id, message);

    let block_context = starknet.block_context.clone();
    let state = starknet.get_mut_state_at(estimate_message_fee.get_block_id())?;

    let address = ContractAddress::new(estimate_message_fee.get_to_address())?;
    state.assert_contract_deployed(address)?;

    let mut transactional_state = CachedState::create_transactional(&mut state.state);

    let l1_transaction = estimate_message_fee.create_blockifier_l1_transaction()?;

    estimate_transaction_fee(
        &mut transactional_state,
        &block_context,
        blockifier::transaction::transaction_execution::Transaction::L1HandlerTransaction(
            l1_transaction,
        ),
        None,
        None,
        true,
    )
}

fn estimate_transaction_fee<S: StateReader>(
    transactional_state: &mut CachedState<S>,
    block_context: &blockifier::context::BlockContext,
    transaction: blockifier::transaction::transaction_execution::Transaction,
    charge_fee: Option<bool>,
    validate: Option<bool>,
    return_error_on_reverted_execution: bool,
) -> DevnetResult<FeeEstimateWrapper> {
    let fee_type = match transaction {
        blockifier::transaction::transaction_execution::Transaction::AccountTransaction(ref tx) => {
            tx.fee_type()
        }
        blockifier::transaction::transaction_execution::Transaction::L1HandlerTransaction(
            ref tx,
        ) => tx.fee_type(),
    };

    let transaction_execution_info = transaction.execute(
        transactional_state,
        block_context,
        charge_fee.unwrap_or(false),
        validate.unwrap_or(true),
    )?;

    // reverted transactions can only be Invoke transactions
    if let (true, Some(revert_error)) =
        (return_error_on_reverted_execution, transaction_execution_info.revert_error)
    {
        // TODO until blockifier makes the actual stack trace available, we return the stringified
        // error. The RPC spec would prefer a structured one, but a string is allowed.
        return Err(Error::ContractExecutionError(ErrorStack::from_str_err(&revert_error)));
    }

    let gas_vector = transaction_execution_info
        .transaction_receipt
        .resources
        .to_gas_vector(&get_versioned_constants(), block_context.block_info().use_kzg_da)?;
    let total_fee =
        fee_utils::get_fee_by_gas_vector(block_context.block_info(), gas_vector, &fee_type);

    let (gas_price, data_gas_price, unit) = match fee_type {
        blockifier::transaction::objects::FeeType::Strk => (
            block_context.block_info().gas_prices.strk_l1_gas_price.get(),
            block_context.block_info().gas_prices.strk_l1_data_gas_price.get(),
            PriceUnit::Fri,
        ),
        blockifier::transaction::objects::FeeType::Eth => (
            block_context.block_info().gas_prices.eth_l1_gas_price.get(),
            block_context.block_info().gas_prices.eth_l1_data_gas_price.get(),
            PriceUnit::Wei,
        ),
    };

    // TODO: change l2 fields logic
    Ok(FeeEstimateWrapper {
        l1_gas_consumed: Felt::from(gas_vector.l1_gas),
        l1_data_gas_consumed: Felt::from(gas_vector.l1_data_gas),
        l1_gas_price: Felt::from(gas_price),
        l1_data_gas_price: Felt::from(data_gas_price),
        l2_gas_consumed: Felt::ZERO,
        l2_gas_price: Felt::ZERO,
        overall_fee: Felt::from(total_fee.0),
        unit,
    })
}
