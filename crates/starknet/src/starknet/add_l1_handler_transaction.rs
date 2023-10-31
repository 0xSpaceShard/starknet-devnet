use blockifier::transaction::transactions::ExecutableTransaction;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::transactions::{L1HandlerTransaction, Transaction};
use tracing::trace;

use super::Starknet;
use crate::error::DevnetResult;

pub fn add_l1_handler_transaction(
    starknet: &mut Starknet,
    transaction: L1HandlerTransaction,
) -> DevnetResult<TransactionHash> {
    let transaction_hash = transaction.transaction_hash;
    trace!("Executing L1 handler transaction [{:#064x}]", transaction.transaction_hash);

    let blockifier_transaction = transaction.create_blockifier_transaction()?;

    let charge_fee = false;
    let validate = true;

    let blockifier_execution_result = blockifier_transaction.execute(
        &mut starknet.state.state,
        &starknet.block_context,
        charge_fee,
        validate,
    );

    starknet.handle_transaction_result(
        Transaction::L1Handler(transaction),
        blockifier_execution_result,
    )?;

    Ok(transaction_hash)
}
