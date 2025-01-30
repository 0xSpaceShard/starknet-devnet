use core::fmt::Debug;
use std::str::FromStr;

use blockifier::execution::contract_class::RunnableCompiledClass;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass as SierraContractClass;
use cairo_vm::types::errors::program_errors::ProgramError;
use serde::de::IntoDeserializer;
use serde::{Serialize, Serializer};
use starknet_api::contract_class::{ClassInfo, SierraVersion};
use starknet_rs_core::types::contract::{SierraClass, SierraClassDebugInfo};
use starknet_rs_core::types::{
    ContractClass as CodegenContractClass, FlattenedSierraClass as CodegenSierraContractClass,
};
use starknet_types_core::felt::Felt;

use crate::error::{ConversionError, DevnetResult, Error, JsonError};
use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
use crate::traits::HashProducer;

pub mod deprecated;
pub use deprecated::Cairo0ContractClass;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "testing", derive(Eq, PartialEq))]
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
            ContractClass::Cairo0(deprecated_contract_class) => Ok(
                starknet_api::contract_class::ContractClass::V0(deprecated_contract_class.into()),
            ),
            ContractClass::Cairo1(ref sierra_contract_class) => {
                // TODO any difference: USC vs CasmContractClass::from_contract_class ?
                let casm_json =
                    usc::compile_contract(serde_json::to_value(sierra_contract_class).map_err(
                        |err| Error::JsonError(JsonError::Custom { msg: err.to_string() }),
                    )?)
                    .map_err(|err| Error::SierraCompilationError { reason: err.to_string() })?;

                let casm = serde_json::from_value::<CasmContractClass>(casm_json)
                    .map_err(|err| Error::JsonError(JsonError::Custom { msg: err.to_string() }))?;

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
                        deprecated_contract_class.into(),
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
            ContractClass::Cairo0(Cairo0ContractClass::Rpc(class)) => RunnableCompiledClass::V0(
                class
                    .try_into()
                    .map_err(|e: ProgramError| ConversionError::InvalidInternalStructure(e.to_string()))?,
            ),
            ContractClass::Cairo1(class) => {
                // TODO extract this as common logic
                let json_value = serde_json::to_value(&class).map_err(JsonError::SerdeJsonError)?;
                let casm_json = usc::compile_contract(json_value)
                    .map_err(|err| Error::SierraCompilationError { reason: err.to_string() })?;
    
                let casm = serde_json::from_value::<CasmContractClass>(casm_json)
                    .map_err(|err| Error::JsonError(JsonError::Custom { msg: err.to_string() }))?;
    
                let versioned_casm = (casm, SierraVersion::from_str(&class.contract_class_version)?);
                let compiled = versioned_casm.try_into().map_err(|e: ProgramError| {
                    Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
                })?;
                RunnableCompiledClass::V1(compiled)
            }
        })
    }
}

impl TryFrom<Cairo0ContractClass> for RunnableCompiledClass {
    type Error = Error;

    fn try_from(value: Cairo0ContractClass) -> Result<Self, Self::Error> {
        let Cairo0ContractClass::Rpc(class) = value;
        let compiled_class = class.try_into().map_err(|e: ProgramError| {
            Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
        })?;
        Ok(RunnableCompiledClass::V0(compiled_class))
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

pub fn convert_codegen_to_blockifier_compiled_class(
    class: CodegenContractClass,
) -> Result<RunnableCompiledClass, Error> {
    Ok(match &class {
        CodegenContractClass::Sierra(sierra) => {
            let json_value = serde_json::to_value(&class).map_err(JsonError::SerdeJsonError)?;
            let casm_json = usc::compile_contract(json_value)
                .map_err(|err| Error::SierraCompilationError { reason: err.to_string() })?;

            let casm = serde_json::from_value::<CasmContractClass>(casm_json)
                .map_err(|err| Error::JsonError(JsonError::Custom { msg: err.to_string() }))?;

            let versioned_casm = (casm, SierraVersion::from_str(&sierra.contract_class_version)?);
            let compiled = versioned_casm.try_into().map_err(|e: ProgramError| {
                Error::ConversionError(ConversionError::InvalidInternalStructure(e.to_string()))
            })?;
            RunnableCompiledClass::V1(compiled)
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
    use serde_json::Deserializer;

    use crate::contract_class::{ContractClass, convert_sierra_to_codegen};
    use crate::felt::felt_from_prefixed_hex;
    use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;
    use crate::traits::HashProducer;
    use crate::utils::test_utils::{CAIRO_1_CONTRACT_SIERRA_HASH, CAIRO_1_EVENTS_CONTRACT_PATH};

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
}
