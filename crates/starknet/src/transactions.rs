use blockifier::execution::call_info::CallInfo;
use blockifier::transaction::objects::TransactionExecutionInfo;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use starknet_api::block::BlockNumber;
use starknet_rs_core::types::{ExecutionResult, FieldElement, MsgToL1, TransactionFinalityStatus};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::Event;
use starknet_types::felt::{BlockHash, Felt, TransactionHash};
use starknet_types::rpc::transaction_receipt::{
    DeployTransactionReceipt, MessageToL1, TransactionOutput, TransactionReceipt,
};
use starknet_types::rpc::transactions::{Transaction, TransactionType};

use crate::constants::UDC_CONTRACT_ADDRESS;
use crate::error::{DevnetResult, Error};
use crate::traits::{HashIdentified, HashIdentifiedMut};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StarknetTransactions(IndexMap<TransactionHash, StarknetTransaction>);

impl StarknetTransactions {
    pub fn insert(&mut self, transaction_hash: &TransactionHash, transaction: StarknetTransaction) {
        self.0.insert(*transaction_hash, transaction);
    }

    pub fn get(&self, transaction_hash: &TransactionHash) -> Option<&StarknetTransaction> {
        self.0.get(transaction_hash)
    }

    pub fn iter(&self) -> indexmap::map::Iter<'_, Felt, StarknetTransaction> {
        self.0.iter()
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
#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetTransaction {
    pub inner: Transaction,
    pub(crate) finality_status: TransactionFinalityStatus,
    pub(crate) execution_result: ExecutionResult,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    #[serde(skip)]
    pub(crate) execution_info: TransactionExecutionInfo,
}

impl StarknetTransaction {
    pub fn create_accepted(
        transaction: &Transaction,
        execution_info: TransactionExecutionInfo,
    ) -> Self {
        Self {
            finality_status: TransactionFinalityStatus::AcceptedOnL2,
            execution_result: match execution_info.is_reverted() {
                true => ExecutionResult::Reverted {
                    reason: execution_info
                        .revert_error
                        .clone()
                        .unwrap_or("No revert error".to_string()),
                },
                false => ExecutionResult::Succeeded,
            },
            inner: transaction.clone(),
            block_hash: None,
            block_number: None,
            execution_info,
        }
    }

    pub fn get_events(&self) -> Vec<Event> {
        let mut events: Vec<Event> = vec![];

        fn get_blockifier_events_recursively(call_info: &CallInfo) -> Vec<(usize, Event)> {
            let mut events: Vec<(usize, Event)> = vec![];

            events.extend(call_info.execution.events.iter().map(|e| {
                (
                    e.order,
                    Event {
                        from_address: call_info.call.storage_address.into(),
                        data: e.event.data.0.iter().map(|d| (*d).into()).collect(),
                        keys: e.event.keys.iter().map(|k| k.0.into()).collect(),
                    },
                )
            }));

            call_info.inner_calls.iter().for_each(|call| {
                events.extend(get_blockifier_events_recursively(call));
            });

            events
        }

        if let Some(validate_call_info) = self.execution_info.validate_call_info.as_ref() {
            let mut not_sorted_events = get_blockifier_events_recursively(validate_call_info);
            not_sorted_events.sort_by_key(|(order, _)| *order);
            events.extend(not_sorted_events.into_iter().map(|(_, e)| e));
        }

        if let Some(execution_call_info) = self.execution_info.execute_call_info.as_ref() {
            let mut not_sorted_events = get_blockifier_events_recursively(execution_call_info);
            not_sorted_events.sort_by_key(|(order, _)| *order);
            events.extend(not_sorted_events.into_iter().map(|(_, e)| e));
        }

        if let Some(fee_transfer_call_info) = self.execution_info.fee_transfer_call_info.as_ref() {
            let mut not_sorted_events = get_blockifier_events_recursively(fee_transfer_call_info);
            not_sorted_events.sort_by_key(|(order, _)| *order);
            events.extend(not_sorted_events.into_iter().map(|(_, e)| e));
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
            Felt::from(get_selector_from_name("ContractDeployed").map_err(|_| Error::FormatError)?);

        let udc_address = ContractAddress::new(Felt::from_prefixed_hex_str(UDC_CONTRACT_ADDRESS)?)?;

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
        let events = self.get_events();

        let mut messages_sent: Vec<MessageToL1> = vec![];
        for message in self.get_l2_to_l1_messages().into_iter() {
            messages_sent.push(message.try_into()?);
        }

        let output = TransactionOutput {
            actual_fee: self.execution_info.actual_fee,
            messages_sent,
            // TODO: we may have performances issues here? Can have a lot's of events..
            events: events.clone(),
        };

        let mut common_receipt = self.inner.create_common_receipt(
            output,
            self.block_hash.as_ref(),
            self.block_number,
            &self.execution_result,
            self.finality_status,
        );

        match &self.inner {
            Transaction::DeployAccount(deploy_account_transaction) => {
                Ok(TransactionReceipt::Deploy(DeployTransactionReceipt {
                    common: common_receipt,
                    contract_address: deploy_account_transaction.contract_address,
                }))
            }
            Transaction::Invoke(_) => {
                let deployed_address =
                    StarknetTransaction::get_deployed_address_from_events(&events)?;

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
            _ => Ok(TransactionReceipt::Common(common_receipt)),
        }
    }

    pub fn get_l2_to_l1_messages(&self) -> Vec<MsgToL1> {
        let mut messages = vec![];

        fn get_blockifier_messages_recursively(call_info: &CallInfo) -> Vec<MsgToL1> {
            let mut messages = vec![];

            messages.extend(call_info.execution.l2_to_l1_messages.iter().map(|m| MsgToL1 {
                to_address:
                    FieldElement::from_byte_slice_be(m.message.to_address.0.as_bytes()).unwrap(),
                from_address: (*call_info.call.caller_address.0.key()).into(),
                payload: m.message.payload.0.iter().map(|p| (*p).into()).collect(),
            }));

            call_info.inner_calls.iter().for_each(|call| {
                messages.extend(get_blockifier_messages_recursively(call));
            });

            messages
        }

        if let Some(info) = self.execution_info.validate_call_info.as_ref() {
            messages.extend(get_blockifier_messages_recursively(info));
        }

        if let Some(info) = self.execution_info.execute_call_info.as_ref() {
            messages.extend(get_blockifier_messages_recursively(info));
        }

        if let Some(info) = self.execution_info.fee_transfer_call_info.as_ref() {
            messages.extend(get_blockifier_messages_recursively(info));
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use blockifier::transaction::objects::TransactionExecutionInfo;
    use starknet_rs_core::types::{TransactionExecutionStatus, TransactionFinalityStatus};
    use starknet_types::rpc::transactions::{DeclareTransaction, Transaction};
    use starknet_types::traits::HashProducer;

    use super::{StarknetTransaction, StarknetTransactions};
    use crate::traits::HashIdentifiedMut;
    use crate::utils::test_utils::dummy_declare_transaction_v1;

    #[test]
    fn get_transaction_by_hash() {
        let declare_transaction = dummy_declare_transaction_v1();
        let hash = declare_transaction.generate_hash().unwrap();
        let tx = Transaction::Declare(DeclareTransaction::Version1(declare_transaction));

        let sn_tx = StarknetTransaction::create_accepted(&tx, TransactionExecutionInfo::default());
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            &hash,
            StarknetTransaction::create_accepted(&tx, TransactionExecutionInfo::default()),
        );

        let extracted_tran = sn_txs.get_by_hash_mut(&hash).unwrap();

        assert_eq!(sn_tx.block_hash, extracted_tran.block_hash);
        assert_eq!(sn_tx.block_number, extracted_tran.block_number);
        assert!(sn_tx.inner == extracted_tran.inner);
        assert_eq!(sn_tx.finality_status, extracted_tran.finality_status);
        assert_eq!(sn_tx.execution_info, extracted_tran.execution_info);
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        let tx = Transaction::Declare(DeclareTransaction::Version1(dummy_declare_transaction_v1()));

        let sn_tran =
            StarknetTransaction::create_accepted(&tx, TransactionExecutionInfo::default());
        assert_eq!(sn_tran.finality_status, TransactionFinalityStatus::AcceptedOnL2);
        assert_eq!(sn_tran.execution_result.status(), TransactionExecutionStatus::Succeeded);

        assert!(sn_tran.block_hash.is_none());
        assert!(sn_tran.block_number.is_none());
        assert_eq!(sn_tran.inner, tx);
    }
}
