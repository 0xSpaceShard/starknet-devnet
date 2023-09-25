use starknet_in_rust::core::errors::state_errors::StateError;
use starknet_rs_core::types::{BlockId, MsgFromL1};
use starknet_types::contract_address::ContractAddress;
use starknet_types::rpc::estimate_message_fee::{
    EstimateMessageFeeRequestWrapper, FeeEstimateWrapper,
};
use starknet_types::rpc::transactions::{BroadcastedDeclareTransaction, BroadcastedTransaction};

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;

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
                        &starknet.config.chain_id.to_felt(),
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
                        .create_sir_declare(starknet.config.chain_id.to_felt())?;

                    Ok(starknet_in_rust::transaction::Transaction::DeclareV2(Box::new(declare_tx)))
                }
                BroadcastedTransaction::DeployAccount(broadcasted_tx) => {
                    let deploy_tx = broadcasted_tx
                        .create_sir_deploy_account(starknet.config.chain_id.to_felt())?;

                    Ok(starknet_in_rust::transaction::Transaction::DeployAccount(deploy_tx))
                }
                BroadcastedTransaction::Invoke(broadcasted_tx) => {
                    let invoke_tx = broadcasted_tx
                        .create_sir_invoke_function(starknet.config.chain_id.to_felt())?;

                    Ok(starknet_in_rust::transaction::Transaction::InvokeFunction(invoke_tx))
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
    block_id: BlockId,
    message: MsgFromL1,
) -> DevnetResult<FeeEstimateWrapper> {
    let estimate_message_fee = EstimateMessageFeeRequestWrapper::new(block_id, message);
    let state = starknet.get_state_at(estimate_message_fee.get_raw_block_id())?;

    match starknet
        .get_class_hash_at(block_id, ContractAddress::new(estimate_message_fee.get_to_address())?)
    {
        Ok(_) => Ok(()),
        Err(Error::StateError(StateError::NoneContractState(_))) => Err(Error::ContractNotFound),
        Err(err) => Err(err),
    }?;

    let sir_l1_handler =
        estimate_message_fee.create_sir_l1_handler(starknet.config.chain_id.to_felt())?;
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
