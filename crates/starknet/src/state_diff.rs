use std::collections::HashMap;

use starknet_types::felt::ClassHash;
use starknet_types::contract_class::ContractClass;
use starknet_in_rust::state::StateDiff as StarknetInRustStateDiff;

pub struct StateDiff {
    pub(crate) inner: StarknetInRustStateDiff,
    pub(crate) class_hash_to_compiled_class_hash: HashMap<ClassHash, ClassHash>,
    pub(crate) cairo_0_declared_contracts: HashMap<ClassHash, ContractClass>,
    pub(crate) cairo_1_declared_contracts: HashMap<ClassHash, ContractClass>,
}