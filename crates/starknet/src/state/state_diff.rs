use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::State;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{ClassHash, Felt};
use starknet_types::patricia_key::{PatriciaKey, StorageKey};
use starknet_types::rpc::state::{
    ClassHashes, ContractNonce, DeployedContract, StorageDiff, StorageEntry, ThinStateDiff,
};

use super::DevnetState;
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
    pub(crate) fn difference_between_old_and_new_state(
        old_state: DevnetState,
        new_state: &mut CachedState<DevnetState>,
    ) -> DevnetResult<Self> {
        let mut declared_contracts = Vec::<ClassHash>::new();
        let mut cairo_0_declared_contracts = Vec::<ClassHash>::new();

        // with this function we move all the classes from the accumulated data in the local class
        // cache of blockifier CachedState because we cant access it directly, we can only
        // access it through the global class cache
        new_state.move_classes_to_global_cache();
        let diff = new_state.to_state_diff();

        for (class_hash, class) in new_state.global_class_hash_to_class().get_order().iter() {
            let class_hash_as_felt: Felt = class_hash.0.into();

            if !(old_state.class_hash_to_compiled_class.contains_key(&class_hash_as_felt)
                || old_state.class_hash_to_compiled_class_hash.contains_key(&class_hash_as_felt))
            {
                match class {
                    blockifier::execution::contract_class::ContractClass::V0(_) => {
                        cairo_0_declared_contracts.push(class_hash_as_felt)
                    }
                    blockifier::execution::contract_class::ContractClass::V1(_) => {
                        declared_contracts.push(class_hash_as_felt)
                    }
                }
            }
        }

        // extract differences of class_hash -> compile_class_hash mapping
        let class_hash_to_compiled_class_hash = diff
            .class_hash_to_compiled_class_hash
            .into_iter()
            .map(|(class_hash, compiled_class_hash)| {
                (Felt::from(class_hash.0), Felt::from(compiled_class_hash.0))
            })
            .collect();

        let address_to_class_hash = diff
            .address_to_class_hash
            .iter()
            .map(|(address, class_hash)| {
                let contract_address = ContractAddress::from(*address);
                let class_hash = class_hash.0.into();

                (contract_address, class_hash)
            })
            .collect::<HashMap<ContractAddress, ClassHash>>();

        let address_to_nonce = diff
            .address_to_nonce
            .iter()
            .map(|(address, nonce)| {
                let contract_address = ContractAddress::from(*address);
                let nonce = nonce.0.into();

                (contract_address, nonce)
            })
            .collect::<HashMap<ContractAddress, Felt>>();

        let storage_updates = diff
            .storage_updates
            .iter()
            .map(|(address, storage)| {
                let contract_address = ContractAddress::from(*address);
                let storage = storage
                    .iter()
                    .map(|(key, value)| {
                        let key = PatriciaKey::from(key.0);
                        let value = (*value).into();

                        (key, value)
                    })
                    .collect::<HashMap<StorageKey, Felt>>();

                (contract_address, storage)
            })
            .collect::<HashMap<ContractAddress, HashMap<StorageKey, Felt>>>();

        Ok(StateDiff {
            address_to_class_hash,
            address_to_nonce,
            storage_updates,
            class_hash_to_compiled_class_hash,
            cairo_0_declared_contracts,
            declared_contracts,
        })
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

    use blockifier::state::cached_state::CachedState;
    use blockifier::state::state_api::State;
    use starknet_api::core::ClassHash;
    use starknet_api::hash::StarkFelt;
    use starknet_types::contract_class::{Cairo0ContractClass, ContractClass};
    use starknet_types::felt::Felt;

    use super::StateDiff;
    use crate::state::DevnetState;
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
    };

    #[test]
    fn correct_no_difference_between_non_modified_states() {
        let (old_state, mut new_state) = setup();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, &mut new_state)
                .unwrap();

        let expected_diff = StateDiff::default();

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_class_hash_to_compiled_class_hash() {
        let (old_state, mut new_state) = setup();

        let class_hash = StarkFelt::from(1u8);
        let compiled_class_hash = StarkFelt::from(2u8);

        new_state
            .set_compiled_class_hash(
                ClassHash(class_hash),
                starknet_api::core::CompiledClassHash(compiled_class_hash),
            )
            .unwrap();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, &mut new_state)
                .unwrap();
        let mut expected_diff = StateDiff::default();
        expected_diff
            .class_hash_to_compiled_class_hash
            .insert(Felt::from(class_hash), Felt::from(compiled_class_hash));

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_declared_classes() {
        let old_state = DevnetState::default();

        let compiled_class_hash = Felt::from(1);

        let mut new_state = CachedState::from(old_state.clone());
        new_state
            .set_contract_class(
                &ClassHash(compiled_class_hash.into()),
                ContractClass::Cairo1(dummy_cairo_1_contract_class()).try_into().unwrap(),
            )
            .unwrap();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, &mut new_state)
                .unwrap();

        let mut expected_diff = StateDiff::default();
        expected_diff.declared_contracts.push(compiled_class_hash);

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_cairo_0_declared_classes() {
        let old_state = DevnetState::default();

        let class_hash = Felt::from(1);
        let cairo_0_contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();

        let mut new_state = CachedState::from(old_state.clone());
        new_state
            .set_contract_class(
                &ClassHash(class_hash.into()),
                ContractClass::Cairo0(cairo_0_contract_class).try_into().unwrap(),
            )
            .unwrap();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, &mut new_state)
                .unwrap();

        let expected_diff = StateDiff {
            cairo_0_declared_contracts: vec![class_hash].into_iter().collect(),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_state_diff_object() {
        let (old_state, mut new_state) = setup();
        let class_hash = dummy_felt();
        let contract_address = dummy_contract_address();

        new_state
            .set_class_hash_at(contract_address.try_into().unwrap(), ClassHash(class_hash.into()))
            .unwrap();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, &mut new_state)
                .unwrap();

        let expected_diff = StateDiff {
            address_to_class_hash: vec![(contract_address, class_hash)].into_iter().collect(),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    fn setup() -> (DevnetState, CachedState<DevnetState>) {
        let state = DevnetState::default();
        let cached_state = CachedState::from(state.clone());

        (state, cached_state)
    }
}
