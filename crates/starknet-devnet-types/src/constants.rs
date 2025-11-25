use starknet_rust::core::types::Felt;

// copied from starknet-rs, because it is not exposed as public type
pub const QUERY_VERSION_OFFSET: Felt =
    Felt::from_raw([576460752142434320, 18446744073709551584, 17407, 18446744073700081665]);

/// Cairo string for "l1_handler"
pub(crate) const PREFIX_L1_HANDLER: Felt = Felt::from_raw([
    157895833347907735,
    18446744073709551615,
    18446744073708665300,
    1365666230910873368,
]);
