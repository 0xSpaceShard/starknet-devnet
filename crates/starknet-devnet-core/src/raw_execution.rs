/// Copied from https://github.com/xJonathanLEI/starknet-rs/
use starknet_rs_core::crypto::pedersen_hash;
use starknet_rs_core::types::{Call, Felt};
use starknet_types::constants::PREFIX_INVOKE;

#[derive(Debug)]
pub struct RawExecutionV1 {
    pub calls: Vec<Call>,
    pub nonce: Felt,
    pub max_fee: Felt,
}

pub fn compute_hash_on_elements(data: &[Felt]) -> Felt {
    let mut current_hash = Felt::ZERO;

    for item in data.iter() {
        current_hash = pedersen_hash(&current_hash, item);
    }

    let data_len = Felt::from(data.len());
    pedersen_hash(&current_hash, &data_len)
}

impl RawExecutionV1 {
    pub fn raw_calldata(&self) -> Vec<Felt> {
        let mut concated_calldata: Vec<Felt> = vec![];
        let mut execute_calldata: Vec<Felt> = vec![self.calls.len().into()];
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

    pub fn transaction_hash(&self, chain_id: Felt, address: Felt) -> Felt {
        compute_hash_on_elements(&[
            PREFIX_INVOKE,
            Felt::ONE, // version
            address,
            Felt::ZERO, // entry_point_selector
            compute_hash_on_elements(&self.raw_calldata()),
            self.max_fee,
            chain_id,
            self.nonce,
        ])
    }
}
