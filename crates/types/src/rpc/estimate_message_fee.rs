use crate::error::DevnetResult;
use crate::felt::Felt;
use crate::rpc::block::BlockId;
use crate::rpc::eth_address::EthAddressWrapper;
use cairo_felt::Felt252;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_in_rust::transaction::L1Handler as SirL1Handler;
use starknet_in_rust::utils::Address as SirAddress;
use starknet_rs_core::types::requests::EstimateMessageFeeRequest;
use starknet_rs_core::types::{BlockId as SrBlockId, FeeEstimate};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeeEstimateWrapper {
    inner: FeeEstimate,
}

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
        self.inner.message.payload.iter().map(|el| el.clone().into()).collect()
    }

    pub fn get_raw_block_id(&self) -> &SrBlockId {
        &self.inner.block_id
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
