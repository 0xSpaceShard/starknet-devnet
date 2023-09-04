use cairo_felt::Felt252;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_in_rust::transaction::L1Handler as SirL1Handler;
use starknet_in_rust::utils::Address as SirAddress;
use starknet_rs_core::types::requests::EstimateMessageFeeRequest;
use starknet_rs_core::types::{
    BlockId as SrBlockId, FeeEstimate, MsgFromL1 as SrMsgFromL1, MsgFromL1,
};

use crate::error::DevnetResult;
use crate::felt::Felt;
use crate::rpc::block::BlockId;
use crate::rpc::eth_address::EthAddressWrapper;
use crate::{impl_wrapper_deserialize, impl_wrapper_serialize};

#[derive(Debug, Clone)]
pub struct FeeEstimateWrapper {
    inner: FeeEstimate,
}

impl_wrapper_serialize!(FeeEstimateWrapper);
impl_wrapper_deserialize!(FeeEstimateWrapper, FeeEstimate);

impl FeeEstimateWrapper {
    pub fn new(gas_consumed: u64, gas_price: u64, overall_fee: u64) -> Self {
        FeeEstimateWrapper { inner: FeeEstimate { gas_consumed, gas_price, overall_fee } }
    }
}

#[derive(Debug, Clone)]
pub struct EstimateMessageFeeRequestWrapper {
    inner: EstimateMessageFeeRequest,
}

impl EstimateMessageFeeRequestWrapper {
    pub fn new(block_id: SrBlockId, msg_from_l1: MsgFromL1) -> Self {
        Self { inner: EstimateMessageFeeRequest { message: msg_from_l1, block_id } }
    }

    // TODO: add ref wrapper
    pub fn get_from_address(&self) -> EthAddressWrapper {
        EthAddressWrapper { inner: self.inner.message.from_address.clone() }
    }

    pub fn get_to_address(&self) -> Felt {
        self.inner.message.to_address.into()
    }

    pub fn get_entry_point_selector(&self) -> Felt {
        self.inner.message.entry_point_selector.into()
    }

    pub fn get_payload(&self) -> Vec<Felt> {
        self.inner.message.payload.iter().map(|el| (*el).into()).collect()
    }

    pub fn get_block_id(&self) -> BlockId {
        self.inner.block_id.into()
    }

    pub fn get_raw_block_id(&self) -> &SrBlockId {
        &self.inner.block_id
    }

    pub fn get_raw_message(&self) -> &SrMsgFromL1 {
        &self.inner.message
    }

    pub fn create_sir_l1_handler(&self, chain_id: Felt) -> DevnetResult<SirL1Handler> {
        let from_address = self.get_from_address();
        let entry_point_selector: Felt = self.get_entry_point_selector();
        let payload: Vec<Felt> = self.get_payload();

        let sir_nonce: Felt252 = 0.into();
        let sir_payload: Vec<Felt252> = payload.iter().map(Felt252::from).collect::<Vec<Felt252>>();
        let sir_calldata = [&[from_address.into()], sir_payload.as_slice()].concat();
        let sir_paid_fee_on_l1 = None;

        Ok(SirL1Handler::new(
            SirAddress(self.get_to_address().into()),
            entry_point_selector.into(),
            sir_calldata,
            sir_nonce,
            chain_id.into(),
            sir_paid_fee_on_l1,
        )?)
    }
}

impl Serialize for EstimateMessageFeeRequestWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        EstimateMessageFeeRequest::serialize(&self.inner, serializer)
    }
}

impl<'de> Deserialize<'de> for EstimateMessageFeeRequestWrapper {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(EstimateMessageFeeRequestWrapper {
            inner: EstimateMessageFeeRequest::deserialize(deserializer)?,
        })
    }
}
