use serde::{Deserialize, Serialize};
use starknet_rs_core::types::{EthAddress, Hash256, MsgToL1, MsgToL2};
use starknet_rs_core::types::Felt;

use crate::contract_address::ContractAddress;
use crate::error::{DevnetResult, Error};
use crate::felt::{Calldata, EntryPointSelector, Nonce};
use crate::rpc::eth_address::EthAddressWrapper;

/// An L1 to L2 message.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageToL2 {
    pub l2_contract_address: ContractAddress,
    pub entry_point_selector: EntryPointSelector,
    pub l1_contract_address: ContractAddress,
    pub payload: Calldata,
    pub paid_fee_on_l1: Felt,
    pub nonce: Nonce,
}

impl MessageToL2 {
    pub fn hash(&self) -> DevnetResult<Hash256> {
        let msg_to_l2 = MsgToL2 {
            from_address: EthAddress::from_felt(&self.l1_contract_address.into()).map_err(
                |err| {
                    Error::ConversionError(crate::error::ConversionError::OutOfRangeError(
                        err.to_string(),
                    ))
                },
            )?,
            to_address: self.l2_contract_address.into(),
            selector: self.entry_point_selector.into(),
            payload: self.payload,
            nonce: u64::try_from(self.nonce.to_bigint()).map_err(|err| {
                Error::ConversionError(crate::error::ConversionError::OutOfRangeError(
                    err.to_string(),
                ))
            })?,
        };

        Ok(msg_to_l2.hash())
    }
}

pub type L2ToL1Payload = Vec<Felt>;

/// An L2 to L1 message.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageToL1 {
    pub from_address: ContractAddress,
    pub to_address: EthAddressWrapper,
    pub payload: L2ToL1Payload,
}

impl MessageToL1 {
    /// Computes the hash of a `MessageToL1`.
    /// Re-uses the already tested hash computation
    /// from starknet-rs.
    pub fn hash(&self) -> Hash256 {
        let msg_to_l1 = MsgToL1 {
            from_address: self.from_address.into(),
            to_address: self.to_address.inner.clone().into(),
            payload: self.payload.clone().into_iter().map(|f| f.into()).collect(),
        };

        msg_to_l1.hash()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedMessageToL1 {
    pub order: usize,
    #[serde(flatten)]
    pub message: MessageToL1,
}

impl OrderedMessageToL1 {
    pub fn new(
        msg: &blockifier::execution::call_info::OrderedL2ToL1Message,
        from_address: ContractAddress,
    ) -> Self {
        Self {
            order: msg.order,
            message: MessageToL1 {
                from_address,
                to_address: msg.message.to_address.into(),
                payload: msg.message.payload.0,
            },
        }
    }
}
