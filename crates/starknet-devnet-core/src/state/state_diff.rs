use std::collections::HashMap;

use blockifier::state::cached_state::CachedState;
use blockifier::state::state_api::StateReader;
use starknet_rs_core::types::Felt;
use starknet_types::contract_address::ContractAddress;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::ClassHash;
use starknet_types::patricia_key::{PatriciaKey, StorageKey};
use starknet_types::rpc::state::{
    ClassHashPair, ContractNonce, DeployedContract, ReplacedClasses, StorageDiff, StorageEntry,
    ThinStateDiff,
};

use crate::error::DevnetResult;

/// This struct is used to store the difference between state modifications
#[derive(Default, Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
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
    // collection of old class hash to new class hash
    pub(crate) replaced_classes: Vec<ReplacedClasses>,
}

impl StateDiff {
    pub(crate) fn generate<S: StateReader>(
        state: &mut CachedState<S>,
        new_classes: HashMap<Felt, ContractClass>,
    ) -> DevnetResult<Self> {
        let mut declared_contracts = Vec::<ClassHash>::new();
        let mut cairo_0_declared_contracts = Vec::<ClassHash>::new();
        let mut replaced_classes = vec![];

        let diff = state.to_state_diff()?;

        for (class_hash, class) in new_classes {
            match class {
                ContractClass::Cairo0(_) => cairo_0_declared_contracts.push(class_hash),
                ContractClass::Cairo1(_) => declared_contracts.push(class_hash),
            }
        }

        // extract differences of class_hash -> compile_class_hash mapping
        let class_hash_to_compiled_class_hash = diff
            .state_maps
            .compiled_class_hashes
            .into_iter()
            .map(|(class_hash, compiled_class_hash)| (class_hash.0, compiled_class_hash.0))
            .collect();

        let address_to_class_hash = diff
            .state_maps
            .class_hashes
            .iter()
            .map(|(address, class_hash)| {
                let contract_address = ContractAddress::from(*address);

                (contract_address, class_hash.0)
            })
            .collect::<HashMap<ContractAddress, ClassHash>>();

        for (contract_address, class_hash) in diff.state_maps.class_hashes {
            let old_class_hash = state.state.get_class_hash_at(contract_address)?;
            if old_class_hash != class_hash
                && old_class_hash != starknet_api::core::ClassHash::default()
            {
                replaced_classes.push(ReplacedClasses {
                    contract_address: contract_address.into(),
                    class_hash: class_hash.0,
                });
            }
        }

        let address_to_nonce = diff
            .state_maps
            .nonces
            .iter()
            .map(|(address, nonce)| {
                let contract_address = ContractAddress::from(*address);

                (contract_address, nonce.0)
            })
            .collect::<HashMap<ContractAddress, Felt>>();

        let mut storage_updates = HashMap::<ContractAddress, HashMap<StorageKey, Felt>>::new();
        diff.state_maps.storage.iter().for_each(|((address, key), value)| {
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
            replaced_classes,
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
        self.replaced_classes.extend(other.replaced_classes.clone());
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
                .map(|(class_hash, compiled_class_hash)| ClassHashPair {
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
            replaced_classes: value.replaced_classes,
        }
    }
}
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use blockifier::state::state_api::{State, StateReader};
    use nonzero_ext::nonzero;
    use starknet_api::core::ClassHash;
    use starknet_rs_core::types::{BlockId, BlockTag, Felt};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::compile_sierra_contract;
    use starknet_types::contract_address::ContractAddress;
    use starknet_types::contract_class::ContractClass;
    use starknet_types::rpc::state::{Balance, ReplacedClasses};
    use starknet_types::rpc::transactions::BroadcastedDeclareTransaction;
    use starknet_types::traits::HashProducer;

    use super::StateDiff;
    use crate::account::Account;
    use crate::constants::{ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS};
    use crate::starknet::Starknet;
    use crate::starknet::starknet_config::StarknetConfig;
    use crate::state::{CustomState, StarknetState};
    use crate::traits::Deployed;
    use crate::utils::test_utils::{
        DUMMY_CAIRO_1_COMPILED_CLASS_HASH, broadcasted_declare_tx_v3,
        cairo_0_account_without_validations, dummy_cairo_1_contract_class, dummy_contract_address,
        dummy_felt, dummy_key_pair, resource_bounds_with_price_1, test_invoke_transaction_v3,
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
    fn correct_difference_on_cairo1_class_declaration() {
        let mut state = setup();

        let class_hash = ClassHash(Felt::ONE);
        let casm_hash = DUMMY_CAIRO_1_COMPILED_CLASS_HASH;

        // necessary to prevent blockifier's state subtraction panic
        state.get_compiled_class(class_hash).expect_err("Shouldn't yet be declared");

        let contract_class = ContractClass::Cairo1(dummy_cairo_1_contract_class());
        state.declare_contract_class(class_hash.0, Some(casm_hash), contract_class).unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff = StateDiff {
            declared_contracts: vec![class_hash.0],
            class_hash_to_compiled_class_hash: HashMap::from([(class_hash.0, casm_hash)]),
            ..Default::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn correct_difference_in_state_diff_object() {
        let mut state = setup();
        let class_hash = dummy_felt();
        let contract_address = dummy_contract_address();
        let blockifier_address = contract_address.try_into().unwrap();

        // necessary to prevent blockifier's state subtraction panic
        assert_eq!(state.get_class_hash_at(blockifier_address).unwrap(), ClassHash(Felt::ZERO));

        state.state.set_class_hash_at(blockifier_address, ClassHash(class_hash)).unwrap();

        let block_number = 1;
        let new_classes = state.rpc_contract_classes.write().commit(block_number);
        let generated_diff = StateDiff::generate(&mut state.state, new_classes).unwrap();

        let expected_diff = StateDiff {
            address_to_class_hash: vec![(contract_address, class_hash)].into_iter().collect(),
            ..StateDiff::default()
        };

        assert_eq!(generated_diff, expected_diff);
    }

    #[test]
    fn test_class_replacement_produces_correct_state_diff() {
        let mut starknet = Starknet::new(&StarknetConfig {
            gas_price_wei: nonzero!(1u128),
            gas_price_fri: nonzero!(1u128),
            data_gas_price_wei: nonzero!(1u128),
            data_gas_price_fri: nonzero!(1u128),
            l2_gas_price_wei: nonzero!(1u128),
            l2_gas_price_fri: nonzero!(1u128),
            ..Default::default()
        })
        .unwrap();

        let account_without_validations_contract_class = cairo_0_account_without_validations();
        let account_without_validations_class_hash =
            account_without_validations_contract_class.generate_hash().unwrap();

        let account = Account::new(
            Balance::from(u128::MAX),
            dummy_key_pair(),
            account_without_validations_class_hash,
            "Custom",
            ContractClass::Cairo0(account_without_validations_contract_class),
            ContractAddress::new(ETH_ERC20_CONTRACT_ADDRESS).unwrap(),
            ContractAddress::new(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
        )
        .unwrap();

        account.deploy(&mut starknet.pending_state).unwrap();

        starknet.commit_diff().unwrap();
        starknet.generate_new_block_and_state().unwrap();
        starknet.restart_pending_block().unwrap();

        // dummy contract
        let replaceable_contract = dummy_cairo_1_contract_class();

        let replacing_contract = ContractClass::cairo_1_from_sierra_json_str(
            &std::fs::read_to_string(
                "../../contracts/test_artifacts/cairo1/events/events_2.0.1_compiler.sierra",
            )
            .unwrap(),
        )
        .unwrap();

        for (contract_class, nonce) in
            [(replaceable_contract.clone(), 0), (replacing_contract.clone(), 1)]
        {
            let compiled_class_hash =
                compile_sierra_contract(&contract_class).unwrap().compiled_class_hash();

            starknet
                .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(
                    broadcasted_declare_tx_v3(
                        account.account_address,
                        nonce.into(),
                        contract_class,
                        compiled_class_hash,
                        resource_bounds_with_price_1(0, 1000, 1e9 as u64),
                    ),
                )))
                .unwrap();
        }

        let replaceable_contract_address = ContractAddress::new(Felt::ONE).unwrap();
        let old_class_hash = ContractClass::Cairo1(replaceable_contract).generate_hash().unwrap();
        starknet
            .pending_state
            .predeploy_contract(replaceable_contract_address, old_class_hash)
            .unwrap();

        starknet.commit_diff().unwrap();
        starknet.generate_new_block_and_state().unwrap();
        starknet.restart_pending_block().unwrap();

        let new_class_hash = ContractClass::Cairo1(replacing_contract).generate_hash().unwrap();

        let invoke_txn = test_invoke_transaction_v3(
            account.account_address,
            replaceable_contract_address,
            get_selector_from_name("test_replace_class").unwrap(),
            &[new_class_hash],
            2, // nonce
            resource_bounds_with_price_1(0, 1000, 1e7 as u64),
        );

        starknet.add_invoke_transaction(invoke_txn).unwrap();

        let state_update = starknet.block_state_update(&BlockId::Tag(BlockTag::Latest)).unwrap();

        assert_eq!(
            state_update.get_state_diff().replaced_classes,
            vec![ReplacedClasses {
                contract_address: replaceable_contract_address,
                class_hash: new_class_hash
            }]
        );
    }

    fn setup() -> StarknetState {
        StarknetState::default()
    }
}
