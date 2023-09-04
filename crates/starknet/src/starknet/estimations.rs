use starknet_rs_core::types::BlockId;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedInvokeTransaction, BroadcastedTransaction,
};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;

// TODO: move to estimate_fee file
/// Returns just the gas usage, not the overall fee
pub fn estimate_fee(
    starknet: &Starknet,
    block_id: BlockId,
    transactions: &[BroadcastedTransaction],
) -> DevnetResult<Vec<FeeEstimateWrapper>> {
    let state = starknet.get_state_at(&block_id)?;

    // Vec<(Fee, GasUsage)>
    let estimation_pairs = starknet_in_rust::estimate_fee(
        &transactions
            .iter()
            .map(|txn| match &txn {
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V1(
                    broadcasted_tx,
                )) => {
                    let class_hash = broadcasted_tx.generate_class_hash()?;
                    let transaction_hash = broadcasted_tx.calculate_transaction_hash(
                        &starknet.config.chain_id.to_felt().into(),
                        &class_hash,
                    )?;

                    let declare_tx =
                        broadcasted_tx.create_sir_declare(class_hash, transaction_hash)?;

                    Ok(starknet_in_rust::transaction::Transaction::Declare(declare_tx))
                }
                BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V2(
                    broadcasted_tx,
                )) => {
                    let declare_tx = broadcasted_tx
                        .create_sir_declare(starknet.config.chain_id.to_felt().into())?;

                    Ok(starknet_in_rust::transaction::Transaction::DeclareV2(Box::new(declare_tx)))
                }
                BroadcastedTransaction::DeployAccount(broadcasted_tx) => {
                    let deploy_tx = broadcasted_tx
                        .create_sir_deploy_account(starknet.config.chain_id.to_felt().into())?;

                    Ok(starknet_in_rust::transaction::Transaction::DeployAccount(deploy_tx))
                }
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V1(
                    broadcasted_tx,
                )) => {
                    let invoke_tx = broadcasted_tx
                        .create_sir_invoke_function(starknet.config.chain_id.to_felt().into())?;

                    Ok(starknet_in_rust::transaction::Transaction::InvokeFunction(invoke_tx))
                }
                BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V0(_)) => {
                    Err(Error::UnsupportedAction { msg: "Invoke V0 is not supported".into() })
                }
            })
            .collect::<DevnetResult<Vec<starknet_in_rust::transaction::Transaction>>>()?,
        state.pending_state.clone(),
        &starknet.block_context,
    )?;

    // extract the gas usage because fee is always 0
    Ok(estimation_pairs
        .into_iter()
        .map(|(_, gas_consumed)| {
            let gas_consumed = gas_consumed as u64;
            FeeEstimateWrapper::new(
                gas_consumed,
                starknet.config.gas_price,
                starknet.config.gas_price * gas_consumed,
            )
        })
        .collect())
}

pub fn estimate_message_fee(
    starknet: &Starknet,
    request: EstimateMessageFeeRequestWrapper,
) -> DevnetResult<FeeEstimateWrapper> {
    let state = starknet.get_state_at(request.get_raw_block_id())?;
    let sir_l1_handler =
        request.create_sir_l1_handler(starknet.config.chain_id.to_felt().into())?;
    let (_, gas_consumed) = starknet_in_rust::estimate_message_fee(
        &sir_l1_handler,
        state.pending_state.clone(),
        &starknet.block_context,
    )?;

    let gas_consumed = gas_consumed as u64;
    Ok(FeeEstimateWrapper::new(
        gas_consumed,
        starknet.config.gas_price,
        starknet.config.gas_price * gas_consumed,
    ))
}
