use std::sync::Arc;

use blockifier::fee::fee_utils::{calculate_l1_gas_by_vm_usage, extract_l1_gas_and_vm_usage};
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::transactions::{ExecutableTransaction, L1HandlerTransaction};
use starknet_api::core::EntryPointSelector;
use starknet_api::transaction::Calldata;
use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_rs_core::types::{BlockId, MsgFromL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::{BroadcastedDeclareTransaction, BroadcastedTransaction};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;
use crate::state::StarknetState;

pub fn estimate_fee(
    starknet: &Starknet,
    block_id: BlockId,
    transactions: &[BroadcastedTransaction],
) -> DevnetResult<Vec<FeeEstimateWrapper>> {
    let mut state = starknet.get_state_at(&block_id)?.make_deep_clone();
    let block_context = starknet.block_context.to_blockifier()?;
    let chain_id = starknet.chain_id().to_felt();

    let transactions = transactions
        .iter()
        .map(|txn| match txn {
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(broadcasted_tx)) => {
                let class_hash = broadcasted_tx.generate_class_hash()?;
                let transaction_hash =
                    broadcasted_tx.calculate_transaction_hash(&chain_id, &class_hash)?;

                let declare_tx =
                    broadcasted_tx.create_blockifier_declare(class_hash, transaction_hash)?;

                Ok(blockifier::transaction::account_transaction::AccountTransaction::Declare(
                    declare_tx,
                ))
            }
            BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(broadcasted_tx)) => {
                let declare_tx = broadcasted_tx.create_blockifier_declare(chain_id)?;

                Ok(blockifier::transaction::account_transaction::AccountTransaction::Declare(
                    declare_tx,
                ))
            }
            BroadcastedTransaction::DeployAccount(broadcasted_tx) => {
                let deploy_tx = broadcasted_tx.create_blockifier_deploy_account(chain_id)?;

                Ok(blockifier::transaction::account_transaction::AccountTransaction::DeployAccount(
                    deploy_tx,
                ))
            }
            BroadcastedTransaction::Invoke(broadcasted_tx) => {
                let invoke_tx = broadcasted_tx.create_blockifier_invoke_transaction(chain_id)?;

                Ok(blockifier::transaction::account_transaction::AccountTransaction::Invoke(
                    invoke_tx,
                ))
            }
        })
        .collect::<DevnetResult<Vec<AccountTransaction>>>()?;

    transactions
        .into_iter()
        .map(|transaction| {
            estimate_transaction_fee(&mut state, &block_context, blockifier::transaction::transaction_execution::Transaction::AccountTransaction(transaction))
        })
        .collect()
}

pub fn estimate_message_fee(
    starknet: &Starknet,
    block_id: BlockId,
    message: MsgFromL1,
) -> DevnetResult<FeeEstimateWrapper> {
    let estimate_message_fee = EstimateMessageFeeRequestWrapper::new(block_id, message);
    let mut state = starknet.get_state_at(estimate_message_fee.get_raw_block_id())?.make_deep_clone();

    match starknet
        .get_class_hash_at(block_id, ContractAddress::new(estimate_message_fee.get_to_address())?)
    {
        Ok(_) => Ok(()),
        Err(Error::StateError(StateError::NoneContractState(_))) => Err(Error::ContractNotFound),
        Err(err) => Err(err),
    }?;

    let l1_transaction = estimate_message_fee.create_blockifier_l1_transaction()?;

    estimate_transaction_fee(&mut state, &starknet.block_context.to_blockifier()?, blockifier::transaction::transaction_execution::Transaction::L1HandlerTransaction(l1_transaction))
}

fn estimate_transaction_fee(
    state: &mut StarknetState,
    block_context: &blockifier::block_context::BlockContext,
    transaction: blockifier::transaction::transaction_execution::Transaction,
) -> DevnetResult<FeeEstimateWrapper> {
    let transaction_execution_info =
        transaction.execute(&mut state.state, block_context, false, true)?;

    if transaction_execution_info.revert_error.is_some() {
        return Err(Error::BlockifierTransactionError(
            blockifier::transaction::errors::TransactionExecutionError::ExecutionError(
                blockifier::execution::errors::EntryPointExecutionError::ExecutionFailed {
                    error_data: vec![],
                },
            ),
        ));
    }

    let (l1_gas_usage, vm_resources) =
        extract_l1_gas_and_vm_usage(&transaction_execution_info.actual_resources);
    let l1_gas_by_vm_usage = calculate_l1_gas_by_vm_usage(&block_context, &vm_resources)?;
    let total_l1_gas_usage = l1_gas_usage as f64 + l1_gas_by_vm_usage;
    let total_l1_gas_usage = total_l1_gas_usage.ceil() as u64;

    let gas_price = block_context.gas_price as u64;

    Ok(FeeEstimateWrapper::new(total_l1_gas_usage, gas_price, total_l1_gas_usage * gas_price))
}