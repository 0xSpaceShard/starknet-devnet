use blockifier::execution::call_info::CallInfo;
use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use blockifier::transaction::objects::TransactionExecutionInfo;
use starknet_types::rpc::state::ThinStateDiff;
use starknet_types::rpc::transaction_receipt::ExecutionResources;
use starknet_types::rpc::transactions::{
    DeclareTransactionTrace, DeployAccountTransactionTrace, ExecutionInvocation,
    FunctionInvocation, InvokeTransactionTrace, L1HandlerTransactionTrace, TransactionTrace,
    TransactionType,
};

use crate::error::{DevnetResult, Error};

fn get_execute_call_info<S: StateReader>(
    state: &mut CachedState<S>,
    execution_info: &TransactionExecutionInfo,
) -> DevnetResult<ExecutionInvocation> {
    Ok(match &execution_info.execute_call_info {
        Some(call_info) => match call_info.execution.failed {
            false => ExecutionInvocation::Succeeded(FunctionInvocation::try_from_call_info(
                call_info, state,
            )?),
            true => ExecutionInvocation::Reverted(starknet_types::rpc::transactions::Reversion {
                revert_reason: execution_info
                    .revert_error
                    .clone()
                    .unwrap_or("Revert reason not found".into()),
            }),
        },
        None => match execution_info.revert_error.clone() {
            Some(revert_reason) => {
                ExecutionInvocation::Reverted(starknet_types::rpc::transactions::Reversion {
                    revert_reason,
                })
            }
            None => {
                return Err(Error::UnexpectedInternalError {
                    msg: "Simulation contains neither call_info nor revert_error".into(),
                });
            }
        },
    })
}

fn get_call_info_invocation<S: StateReader>(
    state: &mut CachedState<S>,
    call_info_invocation: &Option<CallInfo>,
) -> DevnetResult<Option<FunctionInvocation>> {
    Ok(if let Some(call_info) = call_info_invocation {
        Some(FunctionInvocation::try_from_call_info(call_info, state)?)
    } else {
        None
    })
}

pub(crate) fn create_trace<S: StateReader>(
    state: &mut CachedState<S>,
    tx_type: TransactionType,
    execution_info: &TransactionExecutionInfo,
    state_diff: ThinStateDiff,
) -> DevnetResult<TransactionTrace> {
    let state_diff = Some(state_diff);
    let validate_invocation = get_call_info_invocation(state, &execution_info.validate_call_info)?;
    let execution_resources = ExecutionResources::from(execution_info);

    let fee_transfer_invocation =
        get_call_info_invocation(state, &execution_info.fee_transfer_call_info)?;

    match tx_type {
        TransactionType::Declare => Ok(TransactionTrace::Declare(DeclareTransactionTrace {
            validate_invocation,
            fee_transfer_invocation,
            state_diff,
            execution_resources,
        })),
        TransactionType::DeployAccount => {
            Ok(TransactionTrace::DeployAccount(DeployAccountTransactionTrace {
                validate_invocation,
                constructor_invocation: get_call_info_invocation(
                    state,
                    &execution_info.execute_call_info,
                )?,
                fee_transfer_invocation,
                state_diff,
                execution_resources,
            }))
        }
        TransactionType::Invoke => Ok(TransactionTrace::Invoke(InvokeTransactionTrace {
            validate_invocation,
            execute_invocation: get_execute_call_info(state, execution_info)?,
            fee_transfer_invocation,
            state_diff,
            execution_resources,
        })),
        TransactionType::L1Handler => {
            match get_call_info_invocation(state, &execution_info.execute_call_info)? {
                Some(function_invocation) => {
                    Ok(TransactionTrace::L1Handler(L1HandlerTransactionTrace {
                        function_invocation,
                        state_diff,
                    }))
                }
                _ => Err(Error::NoTransactionTrace),
            }
        }
        _ => Err(Error::UnsupportedTransactionType),
    }
}
