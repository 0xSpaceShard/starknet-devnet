use std::collections::HashMap;

use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass as StarknetInRustCompiledClass;
use starknet_in_rust::state::cached_state::CachedState;
use starknet_in_rust::state::StateDiff as StarknetInRustStateDiff;
use starknet_in_rust::utils::subtract_mappings;
use starknet_types::felt::{ClassHash, Felt};

use super::DevnetState;
use crate::error::{DevnetResult, Error};

/// This struct is used to store the difference between state modifications
#[derive(PartialEq, Default, Debug, Clone)]
pub struct StateDiff {
    // data taken from starknet_in_rust
    pub(crate) inner: StarknetInRustStateDiff,
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
        mut new_state: CachedState<DevnetState>,
    ) -> DevnetResult<Self> {
        let mut class_hash_to_compiled_class_hash = HashMap::<ClassHash, ClassHash>::new();
        let mut declared_contracts = Vec::<ClassHash>::new();
        let mut cairo_0_declared_contracts = Vec::<ClassHash>::new();
        // extract differences of class_hash -> compile_class_hash mapping
        let class_hash_to_compiled_class_hash_subtracted_map = subtract_mappings(
            new_state.cache_mut().class_hash_to_compiled_class_hash_mut(),
            &old_state
                .class_hash_to_compiled_class_hash
                .iter()
                .map(|(k, v)| (k.bytes(), v.bytes()))
                .collect::<HashMap<[u8; 32], [u8; 32]>>(),
        );

        for (class_hash_bytes, compiled_class_hash_bytes) in
            class_hash_to_compiled_class_hash_subtracted_map
        {
            let key = Felt::new(class_hash_bytes).map_err(crate::error::Error::from)?;
            let value = Felt::new(compiled_class_hash_bytes).map_err(crate::error::Error::from)?;

            class_hash_to_compiled_class_hash.insert(key, value);
        }

        // extract difference of class hash -> CompiledClass. When CompiledClass is Cairo 1, then
        // the class hash is compiled class hash
        let new_compiled_contract_classes = subtract_mappings(
            new_state.contract_classes(),
            &old_state
                .class_hash_to_compiled_class
                .into_iter()
                .map(|(k,v)| {
                    //(k.bytes(), v.try_into()?)
                    let compiled_class: starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass = v.try_into().map_err(Error::TypesError)?;

                    Ok((k.bytes(), compiled_class))
                })
                .collect::<DevnetResult<HashMap<[u8;32], starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass>>>()?
            );

        for (class_hash, compiled_class) in new_compiled_contract_classes {
            let key = Felt::new(class_hash).map_err(crate::error::Error::from)?;

            match compiled_class {
                StarknetInRustCompiledClass::Deprecated(_) => {
                    cairo_0_declared_contracts.push(key);
                }
                StarknetInRustCompiledClass::Casm(_) => {
                    declared_contracts.push(key);
                }
            }
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

    use starknet_in_rust::services::api::contract_classes::compiled_class::CompiledClass;
    use starknet_in_rust::state::cached_state::{CachedState, ContractClassCache};
    use starknet_in_rust::CasmContractClass;
    use starknet_types::contract_class::Cairo0ContractClass;
    use starknet_types::felt::Felt;

    use super::StateDiff;
    use crate::state::DevnetState;
    use crate::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use crate::utils::test_utils::{
        dummy_cairo_1_contract_class, dummy_contract_address, dummy_felt,
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
        let old_state = DevnetState::default();
        let mut casm_cache = ContractClassCache::default();

        let compiled_class_hash = Felt::from(1);
        casm_cache.insert(
            compiled_class_hash.bytes(),
            CompiledClass::Casm(Arc::new(
                CasmContractClass::from_contract_class(dummy_cairo_1_contract_class(), true)
                    .unwrap(),
            )),
        );
        let new_state = CachedState::new(Arc::new(old_state.clone()), casm_cache);

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state).unwrap();

        let mut expected_diff = StateDiff::default();
        expected_diff.declared_contracts.push(compiled_class_hash);

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_cairo_0_declared_classes() {
        let old_state = DevnetState::default();

        let class_hash = Felt::from(1);
        let cairo_0_contract_class: Cairo0ContractClass = dummy_cairo_0_contract_class().into();
        let mut cairo_0_classes = ContractClassCache::new();
        cairo_0_classes.insert(
            class_hash.bytes(),
            CompiledClass::Deprecated(Arc::new(cairo_0_contract_class.try_into().unwrap())),
        );

        let new_state = CachedState::new(Arc::new(old_state.clone()), cairo_0_classes);

        let generated_diff =
            super::StateDiff::difference_between_old_and_new_state(old_state, new_state).unwrap();

        let expected_diff = StateDiff {
            cairo_0_declared_contracts: vec![class_hash].into_iter().collect(),
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

    fn setup() -> (DevnetState, CachedState<DevnetState>) {
        let state = DevnetState::default();
        let cached_state = CachedState::new(Arc::new(state.clone()), HashMap::new());

        (state, cached_state)
    }
}
