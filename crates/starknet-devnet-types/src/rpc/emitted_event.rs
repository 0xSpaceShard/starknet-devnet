use serde::Serialize;
use starknet_api::block::BlockNumber;
use starknet_types_core::felt::Felt;

use super::transactions::TransactionFinalityStatus;
use crate::contract_address::ContractAddress;
use crate::felt::{BlockHash, TransactionHash};

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct EmittedEvent {
    pub transaction_hash: TransactionHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<BlockNumber>,
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}

impl From<starknet_rs_core::types::EmittedEvent> for EmittedEvent {
    fn from(sn_rs_event: starknet_rs_core::types::EmittedEvent) -> Self {
        Self {
            transaction_hash: sn_rs_event.transaction_hash,
            transaction_index: None,
            event_index: None,
            block_hash: sn_rs_event.block_hash,
            block_number: sn_rs_event.block_number.map(BlockNumber),
            #[allow(clippy::expect_used)]
            from_address: ContractAddress::new(sn_rs_event.from_address)
                .expect("Converting address to address should not fail"),
            keys: sn_rs_event.keys,
            data: sn_rs_event.data,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct Event {
    pub from_address: ContractAddress,
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct OrderedEvent {
    pub keys: Vec<Felt>,
    pub data: Vec<Felt>,
    pub order: usize,
}

impl From<&blockifier::execution::call_info::OrderedEvent> for OrderedEvent {
    fn from(event: &blockifier::execution::call_info::OrderedEvent) -> Self {
        Self {
            order: event.order,
            keys: event.event.keys.iter().map(|k| k.0).collect(),
            data: event.event.data.0.clone(),
        }
    }
}

impl From<&EmittedEvent> for Event {
    fn from(emitted_event: &EmittedEvent) -> Self {
        Self {
            from_address: emitted_event.from_address,
            keys: emitted_event.keys.clone(),
            data: emitted_event.data.clone(),
        }
    }
}

impl From<EmittedEvent> for Event {
    fn from(emitted_event: EmittedEvent) -> Self {
        (&emitted_event).into()
    }
}

#[derive(Serialize, Clone, Debug)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct SubscriptionEmittedEvent {
    #[serde(flatten)]
    pub emitted_event: EmittedEvent,
    pub finality_status: TransactionFinalityStatus,
}
