/// Copied from https://github.com/xJonathanLEI/starknet-rs/
use starknet_rs_core::{crypto::pedersen_hash, types::FieldElement};

use crate::constants::SUPPORTED_TX_VERSION;

/// Cairo string for "invoke"
const PREFIX_INVOKE: FieldElement = FieldElement::from_mont([
    18443034532770911073,
    18446744073709551615,
    18446744073709551615,
    513398556346534256,
]);
// TODO try using TransactionHashPrefix instead

#[derive(Debug, Clone)]
pub struct Call {
    pub to: FieldElement,
    pub selector: FieldElement,
    pub calldata: Vec<FieldElement>,
}

#[derive(Debug)]
pub struct RawExecution {
    pub calls: Vec<Call>,
    pub nonce: FieldElement,
    pub max_fee: FieldElement,
}

pub fn compute_hash_on_elements(data: &[FieldElement]) -> FieldElement {
    let mut current_hash = FieldElement::ZERO;

    for item in data.iter() {
        current_hash = pedersen_hash(&current_hash, item);
    }

    let data_len = FieldElement::from(data.len());
    pedersen_hash(&current_hash, &data_len)
}

impl RawExecution {
    pub fn raw_calldata(&self) -> Vec<FieldElement> {
        let mut concated_calldata: Vec<FieldElement> = vec![];
        let mut execute_calldata: Vec<FieldElement> = vec![self.calls.len().into()];
        for call in self.calls.iter() {
            execute_calldata.push(call.to); // to
            execute_calldata.push(call.selector); // selector
            execute_calldata.push(concated_calldata.len().into()); // data_offset
            execute_calldata.push(call.calldata.len().into()); // data_len

            for item in call.calldata.iter() {
                concated_calldata.push(*item);
            }
        }
        execute_calldata.push(concated_calldata.len().into()); // calldata_len
        for item in concated_calldata.into_iter() {
            execute_calldata.push(item); // calldata
        }

        execute_calldata
    }

    pub fn transaction_hash(&self, chain_id: FieldElement, address: FieldElement) -> FieldElement {
        compute_hash_on_elements(&[
            PREFIX_INVOKE,
            FieldElement::from(SUPPORTED_TX_VERSION), // version
            address,
            FieldElement::ZERO, // entry_point_selector
            compute_hash_on_elements(&self.raw_calldata()),
            self.max_fee,
            chain_id,
            self.nonce,
        ])
    }
}
