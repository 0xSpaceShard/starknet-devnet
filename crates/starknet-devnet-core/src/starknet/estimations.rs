use blockifier::fee::fee_utils::{self};
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::account_transaction::ExecutionFlags;
use blockifier::transaction::objects::HasRelatedFeeType;
use blockifier::transaction::transaction_execution::Transaction;
use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_api::transaction::fields::GasVectorComputationMode;
use starknet_rs_core::types::{BlockId, Felt, MsgFromL1, PriceUnit};
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::BroadcastedTransaction;

use crate::error::{DevnetResult, Error};
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
                let skip_validate_due_to_impersonation =
                    Starknet::should_transaction_skip_validation_if_sender_is_impersonated(
                        state, &cheats, txn,
                    )?;
                let validate = skip_validate_due_to_impersonation
                    .then_some(false)
                    .or(validate)
                    .unwrap_or(true);
                Ok((txn.to_sn_api_account_transaction(&chain_id)?, validate))
            })
            .collect::<DevnetResult<Vec<_>>>()?
    };

    let mut transactional_state = CachedState::create_transactional(&mut state.state);

    transactions
        .into_iter()
        .enumerate()
        .map(|(idx, (transaction, validate))| {
            let estimate_fee_result = estimate_transaction_fee(
                &mut transactional_state,
                &block_context,
                Transaction::Account(
                    blockifier::transaction::account_transaction::AccountTransaction {
                        tx: transaction,
                        execution_flags: ExecutionFlags {
                            only_query: true,
                            charge_fee: charge_fee.unwrap_or(false),
                            validate,
                        },
                    },
                ),
                return_error_on_reverted_execution,
            );

            match estimate_fee_result {
                Ok(estimated_fee) => Ok(estimated_fee),
                // reverted transactions are failing with ExecutionError, but index is set to 0, so we override the index property
                Err(Error::ExecutionError { execution_error, .. }) => {
                    Err(Error::ExecutionError { execution_error, index: idx })
                }
                Err(err) => {
                    Err(Error::ExecutionError { execution_error: err.to_string(), index: idx })
                }
            }
        })
        .collect()
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
        Transaction::L1Handler(l1_transaction),
        true,
    )
}

fn estimate_transaction_fee<S: StateReader>(
    transactional_state: &mut CachedState<S>,
    block_context: &blockifier::context::BlockContext,
    transaction: Transaction,
    return_error_on_reverted_execution: bool,
) -> DevnetResult<FeeEstimateWrapper> {
    let fee_type = match transaction {
        Transaction::Account(tx) => tx.fee_type(),
        Transaction::L1Handler(tx) => tx.fee_type(),
    };

    let transaction_execution_info = transaction.execute(transactional_state, block_context)?;

    // reverted transactions can only be Invoke transactions
    if let (true, Some(revert_error)) =
        (return_error_on_reverted_execution, transaction_execution_info.revert_error)
    {
        return Err(Error::ExecutionError { execution_error: revert_error.to_string(), index: 0 });
    }

    let gas_vector = transaction_execution_info.receipt.resources.to_gas_vector(
        &get_versioned_constants(),
        block_context.block_info().use_kzg_da,
        &GasVectorComputationMode::NoL2Gas, // TODO All?
    );
    let total_fee =
        fee_utils::get_fee_by_gas_vector(block_context.block_info(), gas_vector, &fee_type);

    let (gas_price, data_gas_price, unit) = match fee_type {
        starknet_api::block::FeeType::Strk => (
            block_context.block_info().gas_prices.strk_l1_gas_price.get(),
            block_context.block_info().gas_prices.strk_l1_data_gas_price.get(),
            PriceUnit::Fri,
        ),
        starknet_api::block::FeeType::Eth => (
            block_context.block_info().gas_prices.eth_l1_gas_price.get(),
            block_context.block_info().gas_prices.eth_l1_data_gas_price.get(),
            PriceUnit::Wei,
        ),
    };

    Ok(FeeEstimateWrapper {
        gas_consumed: Felt::from(gas_vector.l1_gas),
        data_gas_consumed: Felt::from(gas_vector.l1_data_gas),
        gas_price: Felt::from(gas_price),
        data_gas_price: Felt::from(data_gas_price),
        overall_fee: Felt::from(total_fee.0),
        unit,
    })
}
