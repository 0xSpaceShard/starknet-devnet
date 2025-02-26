/// Copied from https://github.com/xJonathanLEI/starknet-rs/
use starknet_rs_core::types::{Call, Felt};
use starknet_rs_crypto::PoseidonHasher;
use starknet_types::constants::PREFIX_INVOKE;

/// 2 ^ 128 + 3
const QUERY_VERSION_THREE: Felt =
    Felt::from_raw([576460752142432688, 18446744073709551584, 17407, 18446744073700081569]);

pub(crate) fn encode_calls(calls: &[Call]) -> Vec<Felt> {
    let mut execute_calldata: Vec<Felt> = vec![calls.len().into()];
    for call in calls {
        execute_calldata.push(call.to); // to
        execute_calldata.push(call.selector); // selector

        execute_calldata.push(call.calldata.len().into()); // calldata.len()
        execute_calldata.extend_from_slice(&call.calldata);
    }

    execute_calldata
}

/// Calculates transaction hash given `chain_id`, `address`, `query_only`, and `encoder`.
pub(crate) fn invoke_v3_hash(
    encoded_calls: &[Felt],
    gas: u64,
    gas_price: u128,
    nonce: Felt,
    chain_id: Felt,
    address: Felt,
    query_only: bool,
) -> Felt {
    let mut hasher = PoseidonHasher::new();

    hasher.update(PREFIX_INVOKE);
    hasher.update(if query_only { QUERY_VERSION_THREE } else { Felt::THREE });
    hasher.update(address);

    hasher.update({
        let mut fee_hasher = PoseidonHasher::new();

        // Tip: fee market has not been been activated yet so it's hard-coded to be 0
        fee_hasher.update(Felt::ZERO);

        let mut resource_buffer = [
            0, 0, b'L', b'1', b'_', b'G', b'A', b'S', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        resource_buffer[8..(8 + 8)].copy_from_slice(&gas.to_be_bytes());
        resource_buffer[(8 + 8)..].copy_from_slice(&gas_price.to_be_bytes());
        fee_hasher.update(Felt::from_bytes_be(&resource_buffer));

        // L2 resources are hard-coded to 0
        let resource_buffer = [
            0, 0, b'L', b'2', b'_', b'G', b'A', b'S', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        fee_hasher.update(Felt::from_bytes_be(&resource_buffer));

        fee_hasher.finalize()
    });

    // Hard-coded empty `paymaster_data`
    hasher.update(PoseidonHasher::new().finalize());

    hasher.update(chain_id);
    hasher.update(nonce);

    // Hard-coded L1 DA mode for nonce and fee
    hasher.update(Felt::ZERO);

    // Hard-coded empty `account_deployment_data`
    hasher.update(PoseidonHasher::new().finalize());

    hasher.update({
        let mut calldata_hasher = PoseidonHasher::new();

        encoded_calls.iter().for_each(|element| calldata_hasher.update(*element));

        calldata_hasher.finalize()
    });

    hasher.finalize()
}
