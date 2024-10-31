use starknet_core::types::Felt;

// copied from starknet-rs, because it is not exposed as public type
pub const QUERY_VERSION_OFFSET: Felt =
    Felt::from_raw([576460752142434320, 18446744073709551584, 17407, 18446744073700081665]);

/// Cairo string for "invoke" from starknet-rs
pub const PREFIX_INVOKE: Felt = Felt::from_raw([
    513398556346534256,
    18446744073709551615,
    18446744073709551615,
    18443034532770911073,
]);

/// Cairo string for "deploy_account" from starknet-rs
pub(crate) const PREFIX_DEPLOY_ACCOUNT: Felt = Felt::from_raw([
    461298303000467581,
    18446744073709551615,
    18443211694809419988,
    3350261884043292318,
]);

/// Cairo string for "declare" from starknet-rs
pub(crate) const PREFIX_DECLARE: Felt = Felt::from_raw([
    191557713328401194,
    18446744073709551615,
    18446744073709551615,
    17542456862011667323,
]);

/// Cairo string for "l1_handler"
pub(crate) const PREFIX_L1_HANDLER: Felt = Felt::from_raw([
    157895833347907735,
    18446744073709551615,
    18446744073708665300,
    1365666230910873368,
]);
