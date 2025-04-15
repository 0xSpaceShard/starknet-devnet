use blockifier::execution::call_info::CallInfo;
use blockifier::execution::stack_trace::ErrorStack;
use blockifier::transaction::objects::TransactionExecutionInfo;
use indexmap::IndexMap;
use starknet_api::block::BlockNumber;
use starknet_rs_core::types::{ExecutionResult, TransactionFinalityStatus};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::{Event, OrderedEvent};
use starknet_types::felt::{BlockHash, TransactionHash};
use starknet_types::messaging::MessageToL2;
use starknet_types::rpc::messaging::{MessageToL1, OrderedMessageToL1};
use starknet_types::rpc::transaction_receipt::{
    DeployTransactionReceipt, FeeAmount, FeeInUnits, TransactionReceipt,
};
use starknet_types::rpc::transactions::{
    DeclareTransaction, DeployAccountTransaction, InvokeTransaction, Transaction,
    TransactionStatus, TransactionTrace, TransactionType, TransactionWithHash,
};

use crate::constants::UDC_CONTRACT_ADDRESS;
use crate::error::{DevnetResult, Error};
use crate::traits::{HashIdentified, HashIdentifiedMut};

#[derive(Default)]
pub struct StarknetTransactions(IndexMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
    }

    pub fn get(&self, transaction_hash: &TransactionHash) -> Option<&StarknetTransaction> {
        self.0.get(transaction_hash)
    }
}

impl HashIdentifiedMut for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash_mut(&mut self, hash: &Self::Hash) -> Option<&mut StarknetTransaction> {
        self.0.get_mut(hash)
    }
}

impl HashIdentified for StarknetTransactions {
    type Hash = TransactionHash;
    type Element = StarknetTransaction;
    fn get_by_hash(&self, hash: Self::Hash) -> Option<&StarknetTransaction> {
        self.0.get(&hash)
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct StarknetTransaction {
    pub inner: TransactionWithHash,
    pub(crate) finality_status: TransactionFinalityStatus,
    pub(crate) execution_result: ExecutionResult,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    pub(crate) execution_info: TransactionExecutionInfo,
    pub(crate) trace: Option<TransactionTrace>,
}

impl StarknetTransaction {
    pub fn create_accepted(
        transaction: &TransactionWithHash,
        execution_info: TransactionExecutionInfo,
        trace: TransactionTrace,
    ) -> Self {
        Self {
            finality_status: TransactionFinalityStatus::AcceptedOnL2,
            execution_result: match execution_info.is_reverted() {
                true => ExecutionResult::Reverted {
                    reason: execution_info
                        .revert_error
                        .as_ref()
                        .unwrap_or(&ErrorStack::default().into())
                        .to_string(),
                },
                false => ExecutionResult::Succeeded,
            },
            inner: transaction.clone(),
            block_hash: None,
            block_number: None,
            execution_info,
            trace: Some(trace),
        }
    }

    pub fn get_events(&self) -> Vec<Event> {
        let mut events: Vec<Event> = vec![];

        fn get_blockifier_events_recursively(
            call_info: &blockifier::execution::call_info::CallInfo,
        ) -> Vec<(OrderedEvent, ContractAddress)> {
            let mut events: Vec<(OrderedEvent, ContractAddress)> = vec![];

            events.extend(
                call_info
                    .execution
                    .events
                    .iter()
                    .map(|e| (OrderedEvent::from(e), call_info.call.storage_address.into())),
            );

            call_info.inner_calls.iter().for_each(|call| {
                events.extend(get_blockifier_events_recursively(call));
            });

            events
        }

        let call_infos = vec![
            self.execution_info.validate_call_info.as_ref(),
            self.execution_info.execute_call_info.as_ref(),
            self.execution_info.fee_transfer_call_info.as_ref(),
        ];

        for inner_call_info in call_infos.into_iter().flatten() {
            let mut not_sorted_events = get_blockifier_events_recursively(inner_call_info);
            not_sorted_events.sort_by_key(|(ordered_event, _)| ordered_event.order);
            events.extend(not_sorted_events.into_iter().map(|(ordered_event, address)| Event {
                from_address: address,
                keys: ordered_event.keys,
                data: ordered_event.data,
            }));
        }

        events
    }

    /// Scans through events and gets information from Event generated from UDC with specific
    /// ContractDeployed. Returns the contract address
    ///
    /// # Arguments
    /// * `events` - The events that will be searched
    pub fn get_deployed_address_from_events(
        events: &[Event],
    ) -> DevnetResult<Option<ContractAddress>> {
        let contract_deployed_event_key =
            get_selector_from_name("ContractDeployed").map_err(|_| Error::FormatError)?;

        let udc_address = ContractAddress::new(UDC_CONTRACT_ADDRESS)?;

        let deployed_address = events
            .iter()
            .find(|e| {
                e.from_address == udc_address && e.keys.contains(&contract_deployed_event_key)
            })
            .map(|e| e.data.first().cloned().unwrap_or_default());

        Ok(if let Some(contract_address) = deployed_address {
            Some(ContractAddress::new(contract_address)?)
        } else {
            None
        })
    }

    pub fn get_receipt(&self) -> DevnetResult<TransactionReceipt> {
        let transaction_events = self.get_events();

        let transaction_messages = self.get_l2_to_l1_messages();

        // decide what units to set for actual fee.
        // L1 Handler transactions are in WEI
        // V3 transactions are in STRK(FRI)
        // Other transactions versions are in ETH(WEI)
        let fee_amount = FeeAmount { amount: self.execution_info.receipt.fee };
        let actual_fee_in_units = match self.inner.transaction {
            Transaction::L1Handler(_) => FeeInUnits::WEI(fee_amount),
            Transaction::Declare(DeclareTransaction::V3(_))
            | Transaction::DeployAccount(DeployAccountTransaction::V3(_))
            | Transaction::Invoke(InvokeTransaction::V3(_)) => FeeInUnits::FRI(fee_amount),
            _ => FeeInUnits::WEI(fee_amount),
        };

        let mut common_receipt = self.inner.create_common_receipt(
            &transaction_events,
            &transaction_messages,
            self.block_hash.as_ref(),
            self.block_number,
            &self.execution_result,
            self.finality_status,
            actual_fee_in_units,
            &self.execution_info,
        );

        match &self.inner.transaction {
            Transaction::DeployAccount(deploy_account_txn) => {
                Ok(TransactionReceipt::Deploy(DeployTransactionReceipt {
                    common: common_receipt,
                    contract_address: *deploy_account_txn.get_contract_address(),
                }))
            }
            Transaction::Invoke(_) => {
                let deployed_address =
                    StarknetTransaction::get_deployed_address_from_events(&transaction_events)?;

                let receipt = if let Some(contract_address) = deployed_address {
                    common_receipt.r#type = TransactionType::Deploy;
                    TransactionReceipt::Deploy(DeployTransactionReceipt {
                        common: common_receipt,
                        contract_address,
                    })
                } else {
                    TransactionReceipt::Common(common_receipt)
                };

                Ok(receipt)
            }
            Transaction::L1Handler(l1_transaction) => {
                let msg_hash = MessageToL2::try_from(l1_transaction)?.hash()?;
                Ok(TransactionReceipt::L1Handler(
                    starknet_types::rpc::transaction_receipt::L1HandlerTransactionReceipt {
                        common: common_receipt,
                        message_hash: msg_hash,
                    },
                ))
            }
            _ => Ok(TransactionReceipt::Common(common_receipt)),
        }
    }

    pub fn get_block_number(&self) -> Option<BlockNumber> {
        self.block_number
    }

    pub fn get_status(&self) -> TransactionStatus {
        TransactionStatus {
            finality_status: self.finality_status,
            failure_reason: self.execution_info.revert_error.as_ref().map(|err| err.to_string()),
            execution_status: self.execution_result.status(),
        }
    }

    pub fn get_trace(&self) -> Option<TransactionTrace> {
        self.trace.clone()
    }

    pub fn get_l2_to_l1_messages(&self) -> Vec<MessageToL1> {
        let mut messages = vec![];

        fn get_blockifier_messages_recursively(call_info: &CallInfo) -> Vec<OrderedMessageToL1> {
            let mut messages = vec![];

            // Ensure we always take the address of the contract that is sending the message.
            // In the case of a library syscall, storage address will automatically refer to the
            // caller address.
            let from_address = call_info.call.storage_address;

            messages.extend(call_info.execution.l2_to_l1_messages.iter().map(|m| {
                OrderedMessageToL1 {
                    order: m.order,
                    message: MessageToL1 {
                        to_address: m.message.to_address.into(),
                        from_address: from_address.into(),
                        payload: m.message.payload.0.clone(),
                    },
                }
            }));

            call_info.inner_calls.iter().for_each(|inner_call| {
                messages.extend(get_blockifier_messages_recursively(inner_call));
            });

            messages
        }

        let call_infos = self.execution_info.non_optional_call_infos();

        for inner_call_info in call_infos {
            let mut not_sorted_messages = get_blockifier_messages_recursively(inner_call_info);
            not_sorted_messages.sort_by_key(|message| message.order);
            messages.extend(not_sorted_messages.into_iter().map(|m| m.message));
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use blockifier::state::cached_state::CachedState;
    use blockifier::transaction::objects::TransactionExecutionInfo;
    use blockifier::versioned_constants;
    use starknet_api::transaction::fields::GasVectorComputationMode;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::rpc::transactions::{TransactionTrace, TransactionWithHash};

    use super::{StarknetTransaction, StarknetTransactions};
    use crate::starknet::transaction_trace::create_trace;
    use crate::state::state_readers::DictState;
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::dummy_declare_tx_v3_with_hash;

    fn dummy_trace(tx: &TransactionWithHash) -> TransactionTrace {
        create_trace(
            &mut CachedState::<DictState>::new(Default::default()),
            tx.get_type(),
            &Default::default(),
            Default::default(),
            versioned_constants::VersionedConstants::latest_constants(),
            &GasVectorComputationMode::All,
        )
        .unwrap()
    }

    #[test]
    fn get_transaction_by_hash() {
        let tx = dummy_declare_tx_v3_with_hash();

        let trace = dummy_trace(&tx);
        let sn_tx = StarknetTransaction::create_accepted(
            &tx,
            TransactionExecutionInfo::default(),
            trace.clone(),
        );
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            tx.get_transaction_hash(),
            StarknetTransaction::create_accepted(&tx, TransactionExecutionInfo::default(), trace),
        );

        let extracted_tran = sn_txs.get_by_hash_mut(tx.get_transaction_hash()).unwrap();

        assert_eq!(sn_tx.block_hash, extracted_tran.block_hash);
        assert_eq!(sn_tx.block_number, extracted_tran.block_number);
        assert!(sn_tx.inner == extracted_tran.inner);
        assert_eq!(sn_tx.finality_status, extracted_tran.finality_status);
        assert_eq!(sn_tx.execution_info, extracted_tran.execution_info);
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        let tx = dummy_declare_tx_v3_with_hash();
        let trace = dummy_trace(&tx);
        let sn_tran =
            StarknetTransaction::create_accepted(&tx, TransactionExecutionInfo::default(), trace);
        assert_eq!(sn_tran.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(sn_tran.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert!(sn_tran.block_hash.is_none());
        assert!(sn_tran.block_number.is_none());
        assert_eq!(sn_tran.inner, tx);
    }
}
