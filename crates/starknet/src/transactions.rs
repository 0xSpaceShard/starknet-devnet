use std::collections::HashMap;

use starknet_api::block::BlockNumber;
use starknet_in_rust::execution::{CallInfo, TransactionExecutionInfo};
use starknet_in_rust::transaction::error::TransactionError;
use starknet_rs_core::types::TransactionStatus;
use starknet_rs_core::utils::get_selector_from_name;
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::Event;
use starknet_types::felt::{BlockHash, Felt, TransactionHash};
use starknet_types::rpc::transactions::{
    DeployTransactionReceipt, Transaction, TransactionReceipt, TransactionReceiptWithStatus,
    TransactionType,
};

use crate::constants::UDC_CONTRACT_ADDRESS;
use crate::error::{DevnetResult, Error};
use crate::traits::{HashIdentified, HashIdentifiedMut};
use serde::{Serialize, Deserialize};

#[derive(Default)]
#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetTransactions(HashMap<TransactionHash, StarknetTransaction>);

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
#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetTransaction {
    pub(crate) status: TransactionStatus,
    pub inner: Transaction,
    pub(crate) block_hash: Option<BlockHash>,
    pub(crate) block_number: Option<BlockNumber>,
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) execution_info: Option<starknet_in_rust::execution::TransactionExecutionInfo>,
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) execution_error: Option<TransactionError>,
}

impl StarknetTransaction {
    pub fn create_rejected(transaction: &Transaction, execution_error: TransactionError) -> Self {
        Self {
            status: TransactionStatus::Rejected,
            inner: transaction.clone(),
            execution_info: None,
            execution_error: Some(execution_error),
            block_hash: None,
            block_number: None,
        }
    }

    pub fn create_successful(
        transaction: &Transaction,
        execution_info: &TransactionExecutionInfo,
    ) -> Self {
        Self {
            status: TransactionStatus::Pending,
            inner: transaction.clone(),
            execution_info: Some(execution_info.clone()),
            execution_error: None,
            block_hash: None,
            block_number: None,
        }
    }

    pub fn get_events(&self) -> DevnetResult<Vec<Event>> {
        let mut starknet_in_rust_events = Vec::<starknet_in_rust::execution::Event>::new();

        fn events_from_call_info(
            call_info: Option<&CallInfo>,
        ) -> DevnetResult<Vec<starknet_in_rust::execution::Event>> {
            if let Some(call_info) = call_info {
                call_info.get_sorted_events().map_err(crate::error::Error::from)
            } else {
                Ok(Vec::<starknet_in_rust::execution::Event>::new())
            }
        }

        if let Some(execution_info) = &self.execution_info {
            starknet_in_rust_events
                .extend(events_from_call_info(execution_info.validate_info.as_ref())?);
            starknet_in_rust_events
                .extend(events_from_call_info(execution_info.call_info.as_ref())?);
            starknet_in_rust_events
                .extend(events_from_call_info(execution_info.fee_transfer_info.as_ref())?);
        }
        let mut result: Vec<Event> = Vec::new();
        for event in starknet_in_rust_events.into_iter() {
            result.push(Event {
                from_address: event.from_address.try_into()?,
                keys: event.keys.into_iter().map(Felt::from).collect(),
                data: event.data.into_iter().map(Felt::from).collect(),
            });
        }

        Ok(result)
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

    pub fn get_receipt(&self) -> DevnetResult<TransactionReceiptWithStatus> {
        let transaction_events = self.get_events()?;

        let mut common_receipt = self.inner.create_common_receipt(
            &transaction_events,
            &self.block_hash.unwrap_or_default(),
            self.block_number.unwrap_or_default(),
        );

        match &self.inner {
            Transaction::Invoke(_) => {
                let deployed_address =
                    StarknetTransaction::get_deployed_address_from_events(&transaction_events)?;

                let receipt = if let Some(contract_address) = deployed_address {
                    common_receipt.r#type = TransactionType::Deploy;
                    TransactionReceiptWithStatus {
                        status: self.status,
                        receipt: TransactionReceipt::Deploy(DeployTransactionReceipt {
                            common: common_receipt,
                            contract_address,
                        }),
                    }
                } else {
                    TransactionReceiptWithStatus {
                        status: self.status,
                        receipt: TransactionReceipt::Common(common_receipt),
                    }
                };

                Ok(receipt)
            }
            _ => Ok(TransactionReceiptWithStatus {
                status: self.status,
                receipt: TransactionReceipt::Common(common_receipt),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use starknet_in_rust::execution::TransactionExecutionInfo;
    use starknet_rs_core::types::TransactionStatus;
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

        let sn_tx =
            StarknetTransaction::create_successful(&tx, &TransactionExecutionInfo::default());
        let mut sn_txs = StarknetTransactions::default();
        sn_txs.insert(
            &hash,
            StarknetTransaction::create_successful(&tx, &TransactionExecutionInfo::default()),
        );

        let extracted_tran = sn_txs.get_by_hash_mut(&hash).unwrap();

        assert_eq!(sn_tx.block_hash, extracted_tran.block_hash);
        assert_eq!(sn_tx.block_number, extracted_tran.block_number);
        assert!(sn_tx.inner == extracted_tran.inner);
        assert_eq!(sn_tx.status, extracted_tran.status);
        assert_eq!(sn_tx.execution_error.is_some(), extracted_tran.execution_error.is_some());
        assert_eq!(sn_tx.execution_info.is_some(), extracted_tran.execution_info.is_some());
    }

    #[test]
    fn check_correct_rejected_transaction_creation() {
        let tx = Transaction::Declare(DeclareTransaction::Version1(dummy_declare_transaction_v1()));
        check_correct_transaction_properties(tx, false);
    }

    #[test]
    fn check_correct_successful_transaction_creation() {
        let tx = Transaction::Declare(DeclareTransaction::Version1(dummy_declare_transaction_v1()));
        check_correct_transaction_properties(tx, true);
    }

    fn check_correct_transaction_properties(tran: Transaction, is_success: bool) {
        let sn_tran = if is_success {
            StarknetTransaction::create_successful(&tran, &TransactionExecutionInfo::default())
        } else {
            StarknetTransaction::create_rejected(
                &tran,
                starknet_in_rust::transaction::error::TransactionError::AttempToUseNoneCodeAddress,
            )
        };

        if is_success {
            assert!(sn_tran.status == TransactionStatus::Pending);
        } else {
            assert!(sn_tran.status == TransactionStatus::Rejected);
        }

        assert_eq!(sn_tran.execution_info.is_some(), is_success);
        assert_eq!(sn_tran.execution_error.is_none(), is_success);
        assert!(sn_tran.block_hash.is_none());
        assert!(sn_tran.block_number.is_none());
        assert!(sn_tran.inner == tran);
    }
}
