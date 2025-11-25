use core::fmt::Debug;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use blockifier::execution::contract_class::{
    CompiledClassV0, CompiledClassV0Inner, RunnableCompiledClass, deserialize_program,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
use cairo_vm::types::errors::program_errors::ProgramError;
use deprecated::json_contract_class::Cairo0Json;
use serde::de::IntoDeserializer;
use serde::{Serialize, Serializer};
use starknet_api::contract_class::{ClassInfo, EntryPointType, SierraVersion};
use starknet_api::deprecated_contract_class::{EntryPointOffset, EntryPointV0};
use starknet_rs_core::types::contract::{SierraClass, SierraClassDebugInfo};
use starknet_rs_core::types::{
    ContractClass as CodegenContractClass, FlattenedSierraClass as CodegenSierraContractClass,
    LegacyContractEntryPoint,
};
use starknet_types_core::felt::Felt;

use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
use crate::traits::HashProducer;
use crate::utils::compile_sierra_contract;

pub mod deprecated;
pub use deprecated::Cairo0ContractClass;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "testing", derive(Eq, PartialEq))]
#[allow(clippy::large_enum_variant)]
pub enum ContractClass {
    Cairo0(Cairo0ContractClass),
    Cairo1(SierraContractClass),
}

impl Serialize for ContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ContractClass::Cairo0(cairo0) => cairo0.serialize(serializer),
            ContractClass::Cairo1(contract) => contract.serialize(serializer),
        }
    }
}

impl ContractClass {
    pub fn cairo_1_from_sierra_json_str(json_str: &str) -> DevnetResult<SierraContractClass> {
        let sierra_contract_class: SierraContractClass =
            serde_json::from_str(json_str).map_err(JsonError::SerdeJsonError)?;

        Ok(sierra_contract_class)
    }
}

impl From<Cairo0ContractClass> for ContractClass {
    fn from(value: Cairo0ContractClass) -> Self {
        ContractClass::Cairo0(value)
    }
}

impl From<SierraContractClass> for ContractClass {
    fn from(value: SierraContractClass) -> Self {
        ContractClass::Cairo1(value)
    }
}

impl From<Cairo0Json> for ContractClass {
    fn from(value: Cairo0Json) -> Self {
        ContractClass::Cairo0(value.into())
    }
}

impl TryFrom<ContractClass> for SierraContractClass {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo1(sierra) => Ok(sierra),
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl TryFrom<ContractClass> for Cairo0ContractClass {
    type Error = Error;
    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo0(cairo_0) => Ok(cairo_0),
            _ => Err(Error::ConversionError(crate::error::ConversionError::InvalidFormat)),
        }
    }
}

impl TryFrom<ContractClass> for starknet_api::contract_class::ContractClass {
    type Error = Error;

    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo0(deprecated_contract_class) => {
                Ok(starknet_api::contract_class::ContractClass::V0(
                    deprecated_contract_class.try_into()?,
                ))
            }
            ContractClass::Cairo1(sierra_contract_class) => {
                let casm = compile_sierra_contract(&sierra_contract_class)?;

                Ok(starknet_api::contract_class::ContractClass::V1((
                    casm,
                    SierraVersion::from_str(&sierra_contract_class.contract_class_version)?,
                )))
            }
        }
    }
}

impl TryFrom<ContractClass> for ClassInfo {
    type Error = Error;

    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        match value {
            ContractClass::Cairo0(deprecated_contract_class) => {
                // Set abi_length to 0 as per this conversation
                // https://spaceshard.slack.com/archives/C03HL8DH52N/p1708512271256699?thread_ts=1707845482.455099&cid=C03HL8DH52N
                let abi_length = 0;
                ClassInfo::new(
                    &starknet_api::contract_class::ContractClass::V0(
                        deprecated_contract_class.try_into()?,
                    ),
                    0,
                    abi_length,
                    SierraVersion::DEPRECATED,
                )
                .map_err(|e| {
                    Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
                })
            }
            ContractClass::Cairo1(sierra_contract_class) => {
                let sierra_program_length = sierra_contract_class.sierra_program.len();
                // Calculated as the length of the stringified abi
                // https://spaceshard.slack.com/archives/C03HL8DH52N/p1708512271256699?thread_ts=1707845482.455099&cid=C03HL8DH52N
                let abi_length = if let Some(abi) = sierra_contract_class.abi.as_ref() {
                    serde_json::to_string(abi)
                        .map(|json_str| json_str.len())
                        .map_err(|err| Error::JsonError(JsonError::SerdeJsonError(err)))?
                } else {
                    0
                };

                let sierra_version =
                    SierraVersion::from_str(&sierra_contract_class.contract_class_version)?;
                ClassInfo::new(
                    &ContractClass::Cairo1(sierra_contract_class).try_into()?,
                    sierra_program_length,
                    abi_length,
                    sierra_version,
                )
                .map_err(|e| {
                    Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
                })
            }
        }
    }
}

impl HashProducer for ContractClass {
    type Error = Error;
    fn generate_hash(&self) -> DevnetResult<Felt> {
        match self {
            ContractClass::Cairo0(contract) => Ok(contract.generate_hash()?),
            ContractClass::Cairo1(sierra) => {
                let sierra_felt252_hash = compute_sierra_class_hash(sierra)?;
                Ok(sierra_felt252_hash)
            }
        }
    }
}

impl TryInto<CodegenContractClass> for ContractClass {
    type Error = Error;
    fn try_into(self) -> Result<CodegenContractClass, Self::Error> {
        match self {
            ContractClass::Cairo0(contract_class) => {
                Ok(CodegenContractClass::Legacy(contract_class.try_into()?))
            }
            ContractClass::Cairo1(contract_class) => {
                Ok(CodegenContractClass::Sierra(convert_sierra_to_codegen(&contract_class)?))
            }
        }
    }
}

impl TryInto<ContractClass> for CodegenContractClass {
    type Error = Error;
    fn try_into(self) -> Result<ContractClass, Self::Error> {
        let jsonified = serde_json::to_value(self.clone()).map_err(JsonError::SerdeJsonError)?;
        Ok(match self {
            CodegenContractClass::Sierra(_) => {
                let devnet_class =
                    deserialize_to_sierra_contract_class(jsonified.into_deserializer())
                        .map_err(JsonError::SerdeJsonError)?;
                ContractClass::Cairo1(devnet_class)
            }
            CodegenContractClass::Legacy(_) => ContractClass::Cairo0(
                serde_json::from_value(jsonified).map_err(JsonError::SerdeJsonError)?,
            ),
        })
    }
}

impl TryFrom<ContractClass> for RunnableCompiledClass {
    type Error = Error;

    fn try_from(value: ContractClass) -> Result<Self, Self::Error> {
        Ok(match value {
            ContractClass::Cairo0(class) => class.try_into()?,
            ContractClass::Cairo1(class) => {
                let json_value = serde_json::to_value(&class).map_err(JsonError::SerdeJsonError)?;
                jsonified_sierra_to_runnable_casm(json_value, &class.contract_class_version)?
            }
        })
    }
}

impl TryFrom<Cairo0ContractClass> for RunnableCompiledClass {
    type Error = Error;

    fn try_from(class: Cairo0ContractClass) -> Result<Self, Self::Error> {
        Ok(RunnableCompiledClass::V0(match class {
            Cairo0ContractClass::RawJson(cairo0_json) => serde_json::from_value(cairo0_json.inner)
                .map_err(|e| {
                    Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
                })?,
            Cairo0ContractClass::Rpc(deprecated_contract_class) => {
                let deserializer = deprecated_contract_class.program.into_deserializer();
                let program = deserialize_program(deserializer).map_err(|e| {
                    Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
                })?;

                fn convert_to_entrypoints_v0<'a, I>(entry_points: I) -> Vec<EntryPointV0>
                where
                    I: Iterator<Item = &'a LegacyContractEntryPoint>,
                {
                    entry_points
                        .map(|entry_point| EntryPointV0 {
                            selector: starknet_api::core::EntryPointSelector(entry_point.selector),
                            offset: EntryPointOffset(entry_point.offset as usize),
                        })
                        .collect()
                }

                let mut entry_points_by_type = HashMap::new();
                entry_points_by_type.insert(
                    EntryPointType::Constructor,
                    convert_to_entrypoints_v0(
                        deprecated_contract_class.entry_points_by_type.constructor.iter(),
                    ),
                );

                entry_points_by_type.insert(
                    EntryPointType::External,
                    convert_to_entrypoints_v0(
                        deprecated_contract_class.entry_points_by_type.external.iter(),
                    ),
                );

                entry_points_by_type.insert(
                    EntryPointType::L1Handler,
                    convert_to_entrypoints_v0(
                        deprecated_contract_class.entry_points_by_type.l1_handler.iter(),
                    ),
                );

                CompiledClassV0(Arc::new(CompiledClassV0Inner { program, entry_points_by_type }))
            }
        }))
    }
}

fn convert_sierra_to_codegen(
    contract_class: &SierraContractClass,
) -> DevnetResult<CodegenSierraContractClass> {
    let abi = serde_json::to_string(&contract_class.abi).map_err(JsonError::SerdeJsonError)?;
    let sierra_program = contract_class
        .sierra_program
        .iter()
        .map(|bigint| Felt::from(bigint.value.clone()))
        .collect::<Vec<_>>();

    let entry_points_by_type_value =
        serde_json::to_value(contract_class.entry_points_by_type.clone())
            .map_err(JsonError::SerdeJsonError)?;
    let entry_points_by_type =
        serde_json::from_value(entry_points_by_type_value).map_err(JsonError::SerdeJsonError)?;

    Ok(CodegenSierraContractClass {
        sierra_program,
        contract_class_version: contract_class.contract_class_version.clone(),
        entry_points_by_type,
        abi,
    })
}

fn jsonified_sierra_to_runnable_casm(
    jsonified_sierra: serde_json::Value,
    sierra_version: &str,
) -> Result<RunnableCompiledClass, Error> {
    let casm_json = usc::compile_contract(jsonified_sierra)
        .map_err(|err| Error::SierraCompilationError { reason: err.to_string() })?;

    let casm = serde_json::from_value::<CasmContractClass>(casm_json)
        .map_err(|err| Error::JsonError(JsonError::Custom { msg: err.to_string() }))?;

    let versioned_casm = (casm, SierraVersion::from_str(sierra_version)?);
    let compiled = versioned_casm.try_into().map_err(|e: ProgramError| {
        Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
    })?;

    Ok(RunnableCompiledClass::V1(compiled))
}

pub fn convert_codegen_to_blockifier_compiled_class(
    class: CodegenContractClass,
) -> Result<RunnableCompiledClass, Error> {
    Ok(match &class {
        CodegenContractClass::Sierra(sierra) => {
            let json_value = serde_json::to_value(&class).map_err(JsonError::SerdeJsonError)?;
            jsonified_sierra_to_runnable_casm(json_value, &sierra.contract_class_version)?
        }
        CodegenContractClass::Legacy(_) => {
            let class_jsonified =
                serde_json::to_string(&class).map_err(JsonError::SerdeJsonError)?;
            let class: starknet_api::deprecated_contract_class::ContractClass =
                serde_json::from_str(&class_jsonified)
                    .map_err(|e| Error::JsonError(JsonError::SerdeJsonError(e)))?;
            let compiled = class.try_into().map_err(|e: ProgramError| {
                Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
            })?;
            RunnableCompiledClass::V0(compiled)
        }
    })
}

pub fn compute_sierra_class_hash(contract_class: &SierraContractClass) -> DevnetResult<Felt> {
    let mut contract_class_json_value =
        serde_json::to_value(contract_class).map_err(JsonError::SerdeJsonError)?;

    // to match SierraClass struct, the field sierra_program_debug_info dont have to be
    // Option::None, because during serialization it gets converted to null
    // and the next deserialization to SierraClass will fail, because it expects this key to have
    // some value
    if contract_class.sierra_program_debug_info.is_none() {
        contract_class_json_value["sierra_program_debug_info"] =
            serde_json::to_value(SierraClassDebugInfo {
                type_names: Default::default(),
                libfunc_names: Default::default(),
                user_func_names: Default::default(),
            })
            .map_err(JsonError::SerdeJsonError)?;
    }

    let sierra_class: SierraClass =
        serde_json::from_value(contract_class_json_value).map_err(JsonError::SerdeJsonError)?;

    sierra_class.class_hash().map_err(|_| Error::ConversionError(ConversionError::InvalidFormat))
}

#[cfg(test)]
mod tests {
    use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
    use serde::Deserialize;
    use serde_json::Deserializer;
    use starknet_rs_core::types::LegacyEntryPointsByType;

    use crate::contract_class::deprecated::json_contract_class::Cairo0Json;
    use crate::contract_class::deprecated::rpc_contract_class::{
        ContractClassAbiEntryWithType, DeprecatedContractClass,
    };
    use crate::contract_class::{ContractClass, convert_sierra_to_codegen};
    use crate::felt::felt_from_prefixed_hex;
    use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
    use crate::traits::HashProducer;
    use crate::utils::test_utils::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_0_ACCOUNT_CONTRACT_PATH, CAIRO_1_CONTRACT_SIERRA_HASH,
        CAIRO_1_EVENTS_CONTRACT_PATH,
    };

    #[test]
    fn cairo_1_contract_class_hash_generated_successfully() {
        let cairo_1_contract_sierra = ContractClass::Cairo1(
            ContractClass::cairo_1_from_sierra_json_str(
                &std::fs::read_to_string(CAIRO_1_EVENTS_CONTRACT_PATH).unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            felt_from_prefixed_hex(CAIRO_1_CONTRACT_SIERRA_HASH).unwrap(),
            cairo_1_contract_sierra.generate_hash().unwrap()
        );
    }

    #[test]
    fn cairo_1_to_codegen() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/sierra_contract_class_with_abi_as_string.json"
        );
        let contract_str = std::fs::read_to_string(path).unwrap();
        let mut deserializer = Deserializer::from_str(&contract_str);
        let contract_class: SierraContractClass =
            deserialize_to_sierra_contract_class(&mut deserializer).unwrap();

        convert_sierra_to_codegen(&contract_class).unwrap();
    }

    #[test]
    fn cairo_0_contract_class_hash_generated_successfully() {
        let json_str = std::fs::read_to_string(CAIRO_0_ACCOUNT_CONTRACT_PATH).unwrap();
        let contract_class = Cairo0Json::raw_json_from_json_str(&json_str).unwrap();
        let class_hash = contract_class.generate_hash().unwrap();
        let expected_class_hash = felt_from_prefixed_hex(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap();
        assert_eq!(class_hash, expected_class_hash);
    }

    #[test]
    fn contract_class_cairo_0_from_json_str_doesnt_accept_string_different_from_json() {
        assert!(Cairo0Json::raw_json_from_json_str(" not JSON string").is_err());
    }

    /// The test takes a .casm artifact as raw json and generates its class hash.
    /// Then it takes the same artifact as a `DeprecatedContractClass` and generates its class hash.
    /// The test checks if both hashes are the same.
    #[test]
    fn cairo_0_contract_class_hash_generated_successfully_and_its_the_same_as_raw_json_contract_class_hash()
     {
        let contract_class = Cairo0Json::raw_json_from_path(
            "../../contracts/test_artifacts/cairo0/ERC20_starknet_js.json",
        )
        .unwrap();
        let class_hash = contract_class.generate_hash().unwrap();

        // data taken from https://github.com/0xs34n/starknet.js/blob/ce57fdcaba61a8ef2382acc9233a9aac2ac8589a/__tests__/fixtures.ts#L126
        let expected_class_hash = felt_from_prefixed_hex(
            "0x54328a1075b8820eb43caf0caa233923148c983742402dcfc38541dd843d01a",
        )
        .unwrap();

        assert_eq!(class_hash, expected_class_hash);

        // this struct is for deserializing part of the raw json artifact
        // because DeprecatedContractClass expects the program property to be gzipped then base64
        // encoded we only take those params that dont have any special encoding
        // Then to construct the `DeprecatedContractClass` we will assign the program property,
        // instead of going through the process of gzipping and base64 encoding
        #[derive(Deserialize)]
        struct PartialDeprecatedContractClass {
            pub abi: Vec<ContractClassAbiEntryWithType>,
            /// The selector of each entry point is a unique identifier in the program.
            pub entry_points_by_type: LegacyEntryPointsByType,
        }

        // first check if generated class hash is the same when constructing
        // `DeprecatedContractClass` via assigning properties
        let PartialDeprecatedContractClass { abi, entry_points_by_type } =
            serde_json::from_value::<PartialDeprecatedContractClass>(contract_class.inner.clone())
                .unwrap();
        let program = contract_class.inner.get("program").unwrap();
        let deprecated_contract_class =
            DeprecatedContractClass { program: program.clone(), abi, entry_points_by_type };

        assert_eq!(deprecated_contract_class.generate_hash().unwrap(), expected_class_hash);

        // check if generated class hash is the same when deserializing to `DeprecatedContractClass`
        let serialized_deprecated_contract_class =
            serde_json::to_string(&deprecated_contract_class).unwrap();
        assert_eq!(
            DeprecatedContractClass::rpc_from_json_str(&serialized_deprecated_contract_class)
                .unwrap()
                .generate_hash()
                .unwrap(),
            expected_class_hash
        );
    }
}
