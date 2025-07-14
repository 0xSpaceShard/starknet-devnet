use std::sync::Arc;

use serde::{Deserialize, Serialize};
use starknet_api::core::EntryPointSelector;
use starknet_api::executable_transaction::L1HandlerTransaction;
use starknet_api::transaction::fields::Calldata;
use starknet_rs_core::types::{Felt, MsgFromL1 as SrMsgFromL1, MsgFromL1, PriceUnit};

use super::block::BlockId;
use crate::error::DevnetResult;
use crate::rpc::eth_address::EthAddressWrapper;

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "testing", derive(serde::Deserialize), serde(deny_unknown_fields))]
pub struct FeeEstimateWrapper {
    pub l1_gas_consumed: Felt,
    pub l1_data_gas_consumed: Felt,
    pub l1_gas_price: Felt,
    pub l1_data_gas_price: Felt,
    pub l2_gas_consumed: Felt,
    pub l2_gas_price: Felt,
    pub overall_fee: Felt,
    pub unit: PriceUnit,
}

/// Request for method starknet_estimateMessageFee
#[derive(Debug, Clone, Deserialize)]
pub struct EstimateMessageFeeRequest {
    /// the message's parameters
    pub message: MsgFromL1,
    /// The hash of the requested block, or number (height) of the requested block, or a block tag,
    /// for the block referencing the state or call the transaction on.
    pub block_id: BlockId,
}

impl EstimateMessageFeeRequest {
    pub fn new(block_id: BlockId, msg_from_l1: MsgFromL1) -> Self {
        Self { message: msg_from_l1, block_id }
    }

    // TODO: add ref wrapper
    pub fn get_from_address(&self) -> EthAddressWrapper {
        EthAddressWrapper { inner: self.message.from_address.clone() }
    }

    pub fn get_to_address(&self) -> Felt {
        self.message.to_address
    }

    pub fn get_entry_point_selector(&self) -> Felt {
        self.message.entry_point_selector
    }

    pub fn get_payload(&self) -> &[Felt] {
        &self.message.payload
    }

    pub fn get_block_id(&self) -> &BlockId {
        &self.block_id
    }

    pub fn get_raw_message(&self) -> &SrMsgFromL1 {
        &self.message
    }

    pub fn create_blockifier_l1_transaction(&self) -> DevnetResult<L1HandlerTransaction> {
        let calldata = [&[self.get_from_address().into()], self.get_payload()].concat();

        let l1_transaction = L1HandlerTransaction {
            tx: starknet_api::transaction::L1HandlerTransaction {
                contract_address: starknet_api::core::ContractAddress::try_from(
                    self.get_to_address(),
                )?,
                entry_point_selector: EntryPointSelector(self.get_entry_point_selector()),
                calldata: Calldata(Arc::new(calldata)),
                ..Default::default()
            },
            paid_fee_on_l1: starknet_api::transaction::fields::Fee(1),
            tx_hash: Default::default(),
        };

        Ok(l1_transaction)
    }
}
