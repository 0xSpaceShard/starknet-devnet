pub mod rpc_sierra_contract_class_to_sierra_contract_class {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_to_sierra_contract_class<'de, D>(
        deserializer: D,
    ) -> Result<cairo_lang_starknet_classes::contract_class::ContractClass, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut json_obj = serde_json::Value::deserialize(deserializer)?;
        // Take the inner part of the string value which is expected to be a JSON array and replace
        // it with the deserialized value.
        // If for some reason the abi field is empty string, remove it from collection
        if let Some(serde_json::Value::String(abi_string)) = json_obj.get("abi") {
            if !abi_string.is_empty() {
                let arr: serde_json::Value =
                    serde_json::from_str(abi_string).map_err(serde::de::Error::custom)?;

                json_obj
                    .as_object_mut()
                    .ok_or(serde::de::Error::custom("Expected to be an object"))?
                    .insert("abi".to_string(), arr);
            } else {
                json_obj
                    .as_object_mut()
                    .ok_or(serde::de::Error::custom("Expected to be an object"))?
                    .remove("abi");
            }
        };

        serde_json::from_value(json_obj).map_err(serde::de::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        use serde::Deserialize;

        use crate::serde_helpers::rpc_sierra_contract_class_to_sierra_contract_class::deserialize_to_sierra_contract_class;

        #[test]
        fn correct_deserialization_from_sierra_contract_class_with_abi_field_as_string() {
            #[derive(Deserialize)]
            struct TestDeserialization(
                #[allow(unused)]
                #[serde(deserialize_with = "deserialize_to_sierra_contract_class")]
                cairo_lang_starknet_classes::contract_class::ContractClass,
            );

            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/sierra_contract_class_with_abi_as_string.json"
            );

            let json_str = std::fs::read_to_string(path).unwrap();

            serde_json::from_str::<TestDeserialization>(&json_str).unwrap();
        }
    }
}
pub mod hex_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use starknet_rs_core::types::Felt;

    use crate::contract_address::ContractAddress;
    use crate::felt::felt_from_prefixed_hex;
    use crate::patricia_key::PatriciaKey;

    pub fn deserialize_to_prefixed_patricia_key<'de, D>(
        deserializer: D,
    ) -> Result<PatriciaKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let felt = deserialize_prefixed_hex_string_to_felt(deserializer)?;
        PatriciaKey::new(felt).map_err(serde::de::Error::custom)
    }

    pub fn deserialize_to_prefixed_contract_address<'de, D>(
        deserializer: D,
    ) -> Result<ContractAddress, D::Error>
    where
        D: Deserializer<'de>,
    {
        let felt = deserialize_prefixed_hex_string_to_felt(deserializer)?;
        ContractAddress::new(felt).map_err(serde::de::Error::custom)
    }

    pub fn serialize_patricia_key_to_prefixed_hex<S>(
        patricia_key: &PatriciaKey,
        s: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&patricia_key.to_felt().to_hex_string())
    }

    pub fn serialize_contract_address_to_prefixed_hex<S>(
        contract_address: &ContractAddress,
        s: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_patricia_key_to_prefixed_hex(&contract_address.0, s)
    }

    pub fn deserialize_prefixed_hex_string_to_felt<'de, D>(
        deserializer: D,
    ) -> Result<Felt, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;

        felt_from_prefixed_hex(&buf).map_err(serde::de::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        use serde::{Deserialize, Serialize};
        use starknet_rs_core::types::Felt;

        use crate::contract_address::ContractAddress;
        use crate::felt::felt_from_prefixed_hex;
        use crate::patricia_key::PatriciaKey;
        use crate::serde_helpers::hex_string::{
            deserialize_to_prefixed_contract_address, deserialize_to_prefixed_patricia_key,
            serialize_contract_address_to_prefixed_hex,
        };

        #[test]
        fn deserialization_of_prefixed_hex_patricia_key_should_be_successful() {
            #[derive(Deserialize)]
            struct TestDeserialization {
                #[serde(deserialize_with = "deserialize_to_prefixed_patricia_key")]
                data: PatriciaKey,
            }

            let json_str =
                r#"{"data": "0x800000000000000000000000000000000000000000000000000000000000000"}"#;
            let data = serde_json::from_str::<TestDeserialization>(json_str).unwrap();
            assert!(
                data.data.to_felt()
                    == felt_from_prefixed_hex(
                        "0x800000000000000000000000000000000000000000000000000000000000000"
                    )
                    .unwrap()
            )
        }

        #[test]
        fn deserialization_of_prefixed_hex_patricia_key_should_return_error() {
            #[derive(Deserialize)]
            struct TestDeserialization {
                #[allow(unused)]
                #[serde(deserialize_with = "deserialize_to_prefixed_patricia_key")]
                data: PatriciaKey,
            }

            let json_str =
                r#"{"data": "0x800000000000000000000000000000000000000000000000000000000000001"}"#;
            assert!(serde_json::from_str::<TestDeserialization>(json_str).is_err())
        }

        #[test]
        fn deserialization_of_prefixed_hex_contract_address_should_return_error() {
            #[derive(Deserialize)]
            struct TestDeserialization {
                #[allow(unused)]
                #[serde(deserialize_with = "deserialize_to_prefixed_contract_address")]
                data: ContractAddress,
            }

            let json_str =
                r#"{"data": "0x800000000000000000000000000000000000000000000000000000000000001"}"#;
            assert!(serde_json::from_str::<TestDeserialization>(json_str).is_err())
        }

        #[test]
        fn serialization_of_prefixed_hex_contract_address_should_be_correct() {
            #[derive(Serialize)]
            struct TestSerialization(
                #[serde(serialize_with = "serialize_contract_address_to_prefixed_hex")]
                ContractAddress,
            );

            let data = TestSerialization(ContractAddress::new(Felt::ONE).unwrap());

            assert_eq!(serde_json::to_string(&data).unwrap(), r#""0x1""#);
        }
    }
}

pub mod dec_string {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use num_bigint::BigUint;
    use serde::Deserialize;

    pub fn deserialize_biguint<'de, D>(deserializer: D) -> Result<BigUint, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let number = serde_json::Number::deserialize(deserializer)?;
        // biguint can't handle stringified scientific notation (that's what number can be)
        let big_decimal =
            BigDecimal::from_str(number.as_str()).map_err(serde::de::Error::custom)?;

        // scale to 0 to force stringifying without scientific notation for large values (e.g. 1e30)
        BigUint::from_str(&big_decimal.with_scale(0).to_string()).map_err(serde::de::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        use num_bigint::BigUint;
        use serde::Deserialize;

        use crate::serde_helpers::dec_string::deserialize_biguint;

        #[test]
        fn deserialization_biguint() {
            #[derive(Deserialize)]
            struct TestDeserializationStruct {
                #[serde(deserialize_with = "deserialize_biguint")]
                value: BigUint,
            }

            for (json_str, expected) in [
                (
                    r#"{"value": 3618502788666131106986593281521497120414687020801267626233049500247285301248}"#,
                    BigUint::from(1_u8) << 251,
                ),
                (r#"{"value": 1000000000000000000000000000000}"#, BigUint::from(10_u8).pow(30)),
                (
                    r#"{"value": 1000000000000000000000000000001}"#,
                    BigUint::from(10_u8).pow(30) + BigUint::from(1_u8),
                ),
                (r#"{"value": 1e30}"#, BigUint::from(10_u8).pow(30)),
                (r#"{"value": 1.23e1}"#, BigUint::from(12_u8)),
                (r#"{"value": 1.29e1}"#, BigUint::from(12_u8)),
                (r#"{"value": 100.0}"#, BigUint::from(100_u8)),
            ] {
                match serde_json::from_str::<TestDeserializationStruct>(json_str) {
                    Ok(data) => assert_eq!(data.value, expected),
                    Err(e) => panic!("Unexpected response {e} for parsing: {json_str}"),
                }
            }
        }
    }
}

pub mod base_64_gzipped_json_string {
    use base64::Engine;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;
    use starknet_rs_core::serde::byte_array::base64 as base64Sir;
    use starknet_rs_core::types::contract::legacy::LegacyProgram;

    pub fn deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order<'de, D>(
        deserializer: D,
    ) -> Result<serde_json::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;
        if buf.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        // TODO: change on starknet_rs_core::serde::byte_array::base64
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(buf)
            .map_err(|_| serde::de::Error::custom("program: Unable to decode base64 string"))?;

        let decoder = flate2::read::GzDecoder::new(bytes.as_slice());
        let starknet_program: LegacyProgram = serde_json::from_reader(decoder)
            .map_err(|_| serde::de::Error::custom("program: Unable to decode gzipped bytes"))?;

        serde_json::to_value(starknet_program)
            .map_err(|_| serde::de::Error::custom("program: Unable to parse to JSON"))
    }

    pub fn serialize_program_to_base64<S>(program: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = Vec::new();
        let encoder = GzEncoder::new(&mut buffer, Compression::best());
        serde_json::to_writer(encoder, program)
            .map_err(|_| serde::ser::Error::custom("program: Unable to encode program"))?;

        base64Sir::serialize(&buffer, serializer)
    }

    #[cfg(test)]
    mod tests {
        use std::fs::File;

        use serde::Deserialize;
        use serde_json::json;

        use crate::serde_helpers::base_64_gzipped_json_string::deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order;
        use crate::utils::test_utils::CAIRO_0_RPC_CONTRACT_PATH;

        #[test]
        fn deserialize_successfully_starknet_api_program() {
            let json_value: serde_json::Value =
                serde_json::from_reader(File::open(CAIRO_0_RPC_CONTRACT_PATH).unwrap()).unwrap();

            #[derive(Deserialize)]
            struct TestDeserialization {
                #[allow(unused)]
                #[serde(
                    deserialize_with = "deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order"
                )]
                program: serde_json::Value,
            }

            serde_json::from_str::<TestDeserialization>(
                &serde_json::to_string(&json!({ "program": json_value["program"]})).unwrap(),
            )
            .unwrap();
        }
    }
}
