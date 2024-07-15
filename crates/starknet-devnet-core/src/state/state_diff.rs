use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};
use starknet_types::rpc::state::{
    ClassHashes, ContractNonce, DeployedContract, StorageDiff, StorageEntry, ThinStateDiff,
};

use crate::error::DevnetResult;

/// This struct is used to store the difference between state modifications
#[derive(PartialEq, Default, Debug, Clone)]
pub struct StateDiff {
    pub(crate) storage_updates: HashMap<ContractAddress, HashMap<StorageKey, Felt>>,
    pub(crate) address_to_nonce: HashMap<ContractAddress, Felt>,
    pub(crate) address_to_class_hash: HashMap<ContractAddress, ClassHash>,
    // class hash to compiled_class_hash difference, used when declaring contracts
    // that are different from cairo 0
    pub(crate) class_hash_to_compiled_class_hash: HashMap<ClassHash, ClassHash>,
    // declare contracts that are not cairo 0
    pub(crate) declared_contracts: Vec<ClassHash>,
    // cairo 0 declared contracts
    pub(crate) cairo_0_declared_contracts: Vec<ClassHash>,
}

impl Eq for StateDiff {}

impl StateDiff {
    pub(crate) fn generate<S: StateReader>(
        state: &mut CachedState<S>,
        new_classes: HashMap<Felt, ContractClass>,
    ) -> DevnetResult<Self> {
        let mut declared_contracts = Vec::<ClassHash>::new();
        let mut cairo_0_declared_contracts = Vec::<ClassHash>::new();

        let diff = state.to_state_diff()?;

        for (class_hash, class) in new_classes {
            match class {
                starknet_types::contract_class::ContractClass::Cairo0(_) => {
                    cairo_0_declared_contracts.push(class_hash)
                }
                starknet_types::contract_class::ContractClass::Cairo1(_) => {
                    declared_contracts.push(class_hash)
                }
            }
        }

        // extract differences of class_hash -> compile_class_hash mapping
        let class_hash_to_compiled_class_hash = diff
            .compiled_class_hashes
            .into_iter()
            .map(|(class_hash, compiled_class_hash)| (class_hash.0, compiled_class_hash.0))
            .collect();

        let address_to_class_hash = diff
            .class_hashes
            .iter()
            .map(|(address, class_hash)| {
                let contract_address = ContractAddress::from(*address);

                (contract_address, class_hash.0)
            })
            .collect::<HashMap<ContractAddress, ClassHash>>();

        let address_to_nonce = diff
            .nonces
            .iter()
            .map(|(address, nonce)| {
                let contract_address = ContractAddress::from(*address);

                (contract_address, nonce.0)
            })
            .collect::<HashMap<ContractAddress, Felt>>();

        let mut storage_updates = HashMap::<ContractAddress, HashMap<StorageKey, Felt>>::new();
        diff.storage.iter().for_each(|((address, key), value)| {
            let address_updates = storage_updates.entry((*address).into()).or_default();
            address_updates.insert(key.0.into(), *value);
        });

        Ok(StateDiff {
            address_to_class_hash,
            address_to_nonce,
            storage_updates,
            class_hash_to_compiled_class_hash,
            cairo_0_declared_contracts,
            declared_contracts,
        })
    }

    /// Modify this object by extending all of its properties with the corresponding properties of
    /// the `other` object.
    pub(crate) fn extend(&mut self, other: &StateDiff) {
        self.address_to_class_hash.extend(&other.address_to_class_hash);
        self.address_to_nonce.extend(&other.address_to_nonce);
        self.storage_updates.extend(other.storage_updates.clone());
        self.class_hash_to_compiled_class_hash.extend(&other.class_hash_to_compiled_class_hash);
        self.cairo_0_declared_contracts.extend(&other.cairo_0_declared_contracts);
        self.declared_contracts.extend(&other.declared_contracts);
    }
}

impl From<StateDiff> for ThinStateDiff {
    fn from(value: StateDiff) -> Self {
        let declared_classes: Vec<(Felt, Felt)> =
            value.class_hash_to_compiled_class_hash.into_iter().collect();

        // cairo 0 declarations
        let cairo_0_declared_classes: Vec<Felt> = value.cairo_0_declared_contracts;

        // storage updates (contract address -> [(storage_entry, value)])
        let storage_updates: Vec<(ContractAddress, Vec<(PatriciaKey, Felt)>)> = value
            .storage_updates
            .into_iter()
            .map(|(address, entries)| (address, entries.into_iter().collect()))
            .collect();

        // contract nonces
        let nonces: Vec<(ContractAddress, Felt)> = value.address_to_nonce.into_iter().collect();

        // deployed contracts (address -> class hash)
        let deployed_contracts: Vec<(ContractAddress, Felt)> =
            value.address_to_class_hash.into_iter().collect();

        ThinStateDiff {
            deployed_contracts: deployed_contracts
                .into_iter()
                .map(|(address, class_hash)| DeployedContract { address, class_hash })
                .collect(),
            declared_classes: declared_classes
                .into_iter()
                .map(|(class_hash, compiled_class_hash)| ClassHashes {
                    class_hash,
                    compiled_class_hash,
                })
                .collect(),
            deprecated_declared_classes: cairo_0_declared_classes,
            nonces: nonces
                .into_iter()
                .map(|(address, nonce)| ContractNonce { contract_address: address, nonce })
                .collect(),
            storage_diffs: storage_updates
                .into_iter()
                .map(|(contract_address, updates)| StorageDiff {
                    address: contract_address,
                    storage_entries: updates
                        .into_iter()
                        .map(|(key, value)| StorageEntry { key, value })
                        .collect(),
                })
                .collect(),
            replaced_classes: vec![],
        }
    }
}
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use blockifier::state::state_api::State;
    use starknet_api::core::ClassHash;
    use starknet_rs_core::types::Felt;
    use starknet_types::contract_class::ContractClass;

    use super::StateDiff;
    use crate::state::{CustomState, StarknetState};
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
        DUMMY_CAIRO_1_COMPILED_CLASS_HASH,
    };

    #[test]
    fn correct_no_difference_between_non_modified_states() {
        let mut state = setup();
        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();
        let expected_diff = StateDiff::default();
        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_class_hash_to_compiled_class_hash() {
        let mut state = setup();

        let class_hash = Felt::ONE;
        let casm_hash = Felt::from_hex(DUMMY_CAIRO_1_COMPILED_CLASS_HASH).unwrap();

        let contract_class = ContractClass::Cairo1(dummy_cairo_1_contract_class());
        state.declare_contract_class(class_hash, Some(casm_hash), contract_class).unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff = StateDiff {
            declared_contracts: vec![class_hash],
            class_hash_to_compiled_class_hash: HashMap::from([(class_hash, casm_hash)]),
            ..Default::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_declared_classes() {
        let mut state = setup();

        let class_hash = Felt::ONE;
        let casm_hash = Felt::from_hex(DUMMY_CAIRO_1_COMPILED_CLASS_HASH).unwrap();
        let contract_class = ContractClass::Cairo1(dummy_cairo_1_contract_class());
        state.declare_contract_class(class_hash, Some(casm_hash), contract_class).unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff = StateDiff {
            declared_contracts: vec![class_hash],
            class_hash_to_compiled_class_hash: HashMap::from([(class_hash, casm_hash)]),
            ..Default::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_cairo_0_declared_classes() {
        let mut state = setup();
        let class_hash = Felt::ONE;
        let contract_class = ContractClass::Cairo0(dummy_cairo_0_contract_class().into());

        state.declare_contract_class(class_hash, None, contract_class).unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff =
            StateDiff { cairo_0_declared_contracts: vec![class_hash], ..StateDiff::default() };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_when_declaring_cairo0_and_cairo1() {
        let mut state = setup();

        // declare cairo0
        {
            let class_hash = Felt::ONE;
            let contract_class = ContractClass::Cairo0(dummy_cairo_0_contract_class().into());

            state.declare_contract_class(class_hash, None, contract_class).unwrap();

            let block_number = 1;
            let new_classes = state.rpc_contract_classes.write().commit(block_number);
            let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

            let expected_diff = StateDiff {
                cairo_0_declared_contracts: vec![class_hash].into_iter().collect(),
                ..StateDiff::default()
            };

            assert_eq!(generated_diff, expected_diff);
        }

        // declare cairo1
        {
            let class_hash = Felt::ONE;
            let casm_hash = Felt::from_hex(DUMMY_CAIRO_1_COMPILED_CLASS_HASH).unwrap();
            let contract_class = ContractClass::Cairo1(dummy_cairo_1_contract_class());
            state.declare_contract_class(class_hash, Some(casm_hash), contract_class).unwrap();

            let block_number = 1;
            let new_classes = state.rpc_contract_classes.write().commit(block_number);
            let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

            let expected_diff = StateDiff {
                declared_contracts: vec![class_hash],
                class_hash_to_compiled_class_hash: HashMap::from([(class_hash, casm_hash)]),
                ..Default::default()
            };

            assert_eq!(generated_diff, expected_diff);
        }
    }

    #[test]
    fn correct_difference_in_state_diff_object() {
        let mut state = setup();
        let class_hash = dummy_felt();
        let contract_address = dummy_contract_address();

        state
            .state
            .set_class_hash_at(contract_address.try_into().unwrap(), ClassHash(class_hash))
            .unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff = StateDiff {
            address_to_class_hash: vec![(contract_address, class_hash)].into_iter().collect(),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    fn setup() -> StarknetState {
        StarknetState::default()
    }
}
