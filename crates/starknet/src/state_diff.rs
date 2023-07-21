use std::collections::HashMap;

use starknet_in_rust::state::StateDiff as StarknetInRustStateDiff;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;

/// This struct is used to store the difference between state modifications
pub struct StateDiff {
    // data taken from starknet_in_rust
    pub(crate) inner: StarknetInRustStateDiff,
    // class hash to compiled_class_hash difference, used when declaring contracts
    // that are different from cairo 0
    pub(crate) class_hash_to_compiled_class_hash: HashMap<ClassHash, ClassHash>,
    // declare contracts that are not cairo 0
    pub(crate) declared_contracts: HashMap<ClassHash, ContractClass>,
    // cairo 0 declared contracts
    pub(crate) cairo_0_declared_contracts: HashMap<ClassHash, ContractClass>,
}
