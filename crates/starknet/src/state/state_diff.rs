use std::collections::HashMap;

use starknet_in_rust::state::cached_state::CachedState;
use starknet_in_rust::state::in_memory_state_reader::InMemoryStateReader;
use starknet_in_rust::state::StateDiff as StarknetInRustStateDiff;
use starknet_in_rust::utils::subtract_mappings;
use starknet_in_rust::CasmContractClass;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::{ClassHash, Felt};

use crate::error::Result;

/// This struct is used to store the difference between state modifications
#[derive(PartialEq, Default, Debug, Clone)]
pub struct StateDiff {
    // data taken from starknet_in_rust
    pub(crate) inner: StarknetInRustStateDiff,
    // class hash to compiled_class_hash difference, used when declaring contracts
    // that are different from cairo 0
    pub(crate) class_hash_to_compiled_class_hash: HashMap<ClassHash, ClassHash>,
    // declare contracts that are not cairo 0
    pub(crate) declared_contracts: HashMap<ClassHash, CasmContractClass>,
    // cairo 0 declared contracts
    pub(crate) cairo_0_declared_contracts: HashMap<ClassHash, ContractClass>,
}

impl Eq for StateDiff {
    fn assert_receiver_is_total_eq(&self) {}
}

impl StateDiff {
    pub(crate) fn difference_between_old_and_new_state(
        mut old_state: InMemoryStateReader,
        mut new_state: CachedState<InMemoryStateReader>,
    ) -> Result<Self> {
        let mut class_hash_to_compiled_class_hash = HashMap::<ClassHash, ClassHash>::new();
        let mut declared_contracts = HashMap::<ClassHash, CasmContractClass>::new();
        let mut cairo_0_declared_contracts = HashMap::<ClassHash, ContractClass>::new();

        // extract differences of class_hash -> compile_class_hash mapping
        let class_hash_to_compiled_class_hash_subtracted_map = subtract_mappings(
            new_state.cache_mut().class_hash_to_compiled_class_hash_mut().clone(),
            old_state.class_hash_to_compiled_class_hash_mut().clone(),
        );

        for (class_hash_bytes, compiled_class_hash_bytes) in
            class_hash_to_compiled_class_hash_subtracted_map
        {
            let key = Felt::new(class_hash_bytes).map_err(crate::error::Error::from)?;
            let value = Felt::new(compiled_class_hash_bytes).map_err(crate::error::Error::from)?;

            class_hash_to_compiled_class_hash.insert(key, value);
        }

        // extract difference of compiled_class_hash -> CasmContractClass mapping, which is Cairo 1
        // contract
        let new_casm_contract_classes =
            new_state.casm_contract_classes().clone().unwrap_or_default();

        let compiled_class_hash_to_cairo_casm = subtract_mappings(
            new_casm_contract_classes,
            old_state.casm_contract_classes_mut().clone(),
        );

        for (compiled_class_hash_bytes, casm_contract_class) in compiled_class_hash_to_cairo_casm {
            let key = Felt::new(compiled_class_hash_bytes).map_err(crate::error::Error::from)?;

            declared_contracts.insert(key, casm_contract_class);
        }

        // extract difference of class_hash -> Cairo 0 contract class
        let class_hash_to_cairo_0_contract_class = subtract_mappings(
            new_state.contract_classes().clone().unwrap_or_default(),
            old_state.class_hash_to_contract_class.clone(),
        );

        for (class_hash_bytes, cairo_0_contract_class) in class_hash_to_cairo_0_contract_class {
            let key = Felt::new(class_hash_bytes).map_err(crate::error::Error::from)?;

            cairo_0_declared_contracts.insert(key, ContractClass::from(cairo_0_contract_class));
        }

        let diff = StarknetInRustStateDiff::from_cached_state(new_state)?;

        Ok(StateDiff {
            inner: diff,
            class_hash_to_compiled_class_hash,
            cairo_0_declared_contracts,
            declared_contracts,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use starknet_in_rust::state::cached_state::{CachedState, CasmClassCache, ContractClassCache};
    use starknet_in_rust::state::in_memory_state_reader::InMemoryStateReader;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::felt::Felt;

    use super::StateDiff;
    use crate::utils::test_utils::{
        dummy_cairo_0_contract_class, dummy_cairo_1_contract_class, dummy_contract_address,
        dummy_felt,
    };

    #[test]
    fn correct_no_difference_between_non_modified_states() {
        let (old_state, new_state) = setup();

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state).unwrap();

        let expected_diff = StateDiff::default();

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_class_hash_to_compiled_class_hash() {
        let (old_state, mut new_state) = setup();

        let class_hash = [1; 32];
        let compiled_class_hash = [2; 32];

        new_state
            .cache_mut()
            .class_hash_to_compiled_class_hash_mut()
            .insert(class_hash, compiled_class_hash);

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state.clone())
                .unwrap();
        let mut expected_diff = StateDiff::default();
        expected_diff
            .class_hash_to_compiled_class_hash
            .insert(Felt::new(class_hash).unwrap(), Felt::new(compiled_class_hash).unwrap());

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_declared_classes() {
        let old_state = InMemoryStateReader::default();
        let mut casm_cache = CasmClassCache::default();

        let compiled_class_hash = Felt::from(1);
        casm_cache.insert(
            compiled_class_hash.bytes(),
            dummy_cairo_1_contract_class().try_into().unwrap(),
        );
        let new_state =
            CachedState::new(Arc::new(old_state.clone()), Some(HashMap::new()), Some(casm_cache));

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state).unwrap();

        let mut expected_diff = StateDiff::default();
        expected_diff
            .declared_contracts
            .insert(compiled_class_hash, dummy_cairo_1_contract_class().try_into().unwrap());

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_cairo_0_declared_classes() {
        let old_state = InMemoryStateReader::default();

        let class_hash = Felt::from(1);
        let cairo_0_contract_class = dummy_cairo_0_contract_class();
        let mut cairo_0_classes = ContractClassCache::new();
        cairo_0_classes.insert(class_hash.bytes(), cairo_0_contract_class.try_into().unwrap());

        let new_state = CachedState::new(
            Arc::new(old_state.clone()),
            Some(cairo_0_classes),
            Some(HashMap::new()),
        );

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state).unwrap();

        let cairo_0_contract_class = starknet_in_rust::services::api::contract_classes::deprecated_contract_class::ContractClass::try_from(dummy_cairo_0_contract_class()).unwrap();
        let expected_diff = StateDiff {
            cairo_0_declared_contracts: vec![(
                class_hash,
                ContractClass::Cairo0(starknet_types::contract_class::Cairo0ContractClass::Obj(
                    cairo_0_contract_class,
                )),
            )]
            .into_iter()
            .collect(),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_starknet_in_rust_state_diff_object() {
        let (old_state, mut new_state) = setup();
        let class_hash = dummy_felt();
        let contract_address = dummy_contract_address();

        new_state
            .cache_mut()
            .class_hash_writes_mut()
            .insert(contract_address.try_into().unwrap(), class_hash.bytes());

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state.clone())
                .unwrap();

        let expected_diff = StateDiff {
            inner: starknet_in_rust::state::StateDiff::new(
                vec![(contract_address.try_into().unwrap(), class_hash.bytes())]
                    .into_iter()
                    .collect(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    fn setup() -> (InMemoryStateReader, CachedState<InMemoryStateReader>) {
        let state = InMemoryStateReader::default();
        let cached_state =
            CachedState::new(Arc::new(state.clone()), Some(HashMap::new()), Some(HashMap::new()));

        (state, cached_state)
    }
}
