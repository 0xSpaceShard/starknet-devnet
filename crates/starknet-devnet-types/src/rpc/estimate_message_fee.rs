use std::sync::Arc;

use blockifier::transaction::transactions::L1HandlerTransaction;
use serde::{Deserialize, Serialize};
use starknet_api::core::EntryPointSelector;
use starknet_api::transaction::Calldata;
use starknet_rs_core::types::requests::EstimateMessageFeeRequest;
use starknet_rs_core::types::{
    BlockId as SrBlockId, MsgFromL1 as SrMsgFromL1, MsgFromL1, PriceUnit,
};

use crate::error::DevnetResult;
use crate::felt::Felt;
use crate::rpc::eth_address::EthAddressWrapper;
use crate::{impl_wrapper_deserialize, impl_wrapper_serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeeEstimateWrapper {
    pub gas_consumed: Felt,
    pub data_gas_consumed: Felt,
    pub gas_price: Felt,
    pub data_gas_price: Felt,
    pub overall_fee: Felt,
    pub unit: PriceUnit,
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

    pub fn get_block_id(&self) -> &SrBlockId {
        &self.inner.block_id
    }

    pub fn get_raw_message(&self) -> &SrMsgFromL1 {
        &self.inner.message
    }

    pub fn create_blockifier_l1_transaction(&self) -> DevnetResult<L1HandlerTransaction> {
        let calldata = [&[self.get_from_address().into()], self.get_payload().as_slice()].concat();

        let l1_transaction = L1HandlerTransaction {
            tx: starknet_api::transaction::L1HandlerTransaction {
                contract_address: starknet_api::core::ContractAddress::try_from(
                    starknet_api::hash::StarkFelt::from(self.get_to_address()),
                )?,
                entry_point_selector: EntryPointSelector(self.get_entry_point_selector().into()),
                calldata: Calldata(Arc::new(calldata.into_iter().map(|f| f.into()).collect())),
                ..Default::default()
            },
            paid_fee_on_l1: starknet_api::transaction::Fee(1),
            tx_hash: Default::default(),
        };

        Ok(l1_transaction)
    }
}

impl_wrapper_serialize!(EstimateMessageFeeRequestWrapper);
impl_wrapper_deserialize!(EstimateMessageFeeRequestWrapper, EstimateMessageFeeRequest);
