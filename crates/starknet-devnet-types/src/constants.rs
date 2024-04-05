use starknet_rs_ff::FieldElement;

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
pub const QUERY_VERSION_OFFSET: FieldElement = FieldElement::from_mont([
    18446744073700081665,
    17407,
    18446744073709551584,
    576460752142434320,
]);
