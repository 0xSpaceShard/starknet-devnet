use starknet_in_rust::core::contract_address::compute_casm_class_hash;
use starknet_in_rust::CasmContractClass;
use starknet_rs_core::types::contract::SierraClass;
use starknet_rs_core::types::{FieldElement, FlattenedSierraClass};
use starknet_types::felt::Felt;

pub fn get_events_contract_in_sierra_and_compiled_class_hash()
-> (FlattenedSierraClass, FieldElement) {
    let sierra_artifact = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/cairo1/events/events_2.0.1_compiler.sierra"
    ))
    .unwrap();
    let sierra_class: SierraClass = serde_json::from_str(&sierra_artifact).unwrap();

    let contract_class: starknet_in_rust::ContractClass =
        serde_json::from_str(&sierra_artifact).unwrap();

    let casm_contract_class =
        CasmContractClass::from_contract_class(contract_class, false).unwrap();
    let compiled_class_hash = compute_casm_class_hash(&casm_contract_class).unwrap();

    (sierra_class.flatten().unwrap(), Felt::from(compiled_class_hash).into())
}
