use starknet_rs_core::types::Felt;

pub const OUTPUT_BUILTIN_NAME: &str = "output_builtin";
pub const HASH_BUILTIN_NAME: &str = "pedersen_builtin";
pub const RANGE_CHECK_BUILTIN_NAME: &str = "range_check_builtin";
pub const SIGNATURE_BUILTIN_NAME: &str = "ecdsa_builtin";
pub const BITWISE_BUILTIN_NAME: &str = "bitwise_builtin";
pub const EC_OP_BUILTIN_NAME: &str = "ec_op_builtin";
pub const KECCAK_BUILTIN_NAME: &str = "keccak_builtin";
pub const POSEIDON_BUILTIN_NAME: &str = "poseidon_builtin";
pub const SEGMENT_ARENA_BUILTIN_NAME: &str = "segment_arena_builtin";
pub const N_STEPS: &str = "n_steps";

// copied from starknet-rs, because it is not exposed as public type
pub const QUERY_VERSION_OFFSET: Felt =
Felt::from_raw([
    576460752142434320,
    18446744073709551584,
    17407,
    18446744073700081665,
]);

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
