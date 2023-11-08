pub mod rpc_sierra_contract_class_to_sierra_contract_class {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_to_sierra_contract_class<'de, D>(
        deserializer: D,
    ) -> Result<cairo_lang_starknet::contract_class::ContractClass, D::Error>
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
        fn correct_deserialzation_from_sierra_contract_class_with_abi_field_as_string() {
            #[derive(Deserialize)]
            struct TestDeserialization(
                #[allow(unused)]
                #[serde(deserialize_with = "deserialize_to_sierra_contract_class")]
                cairo_lang_starknet::contract_class::ContractClass,
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

    use crate::contract_address::ContractAddress;
    use crate::felt::Felt;
    use crate::patricia_key::PatriciaKey;
    use crate::traits::ToHexString;

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
        s.serialize_str(patricia_key.to_felt().to_prefixed_hex_str().as_str())
    }

    pub fn serialize_contract_address_to_prefixed_hex<S>(
        contract_address: &ContractAddress,
        s: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(contract_address.to_prefixed_hex_str().as_str())
    }

    #[allow(unused)]
    pub fn deserialize_non_prefixed_hex_string_to_felt<'de, D>(
        deserializer: D,
    ) -> Result<Felt, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;

        Felt::from_prefixed_hex_str(&format!("0x{buf}")).map_err(serde::de::Error::custom)
    }

    pub fn serialize_to_prefixed_hex<S>(felt: &Felt, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(felt.to_prefixed_hex_str().as_str())
    }

    pub fn deserialize_prefixed_hex_string_to_felt<'de, D>(
        deserializer: D,
    ) -> Result<Felt, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;

        Felt::from_prefixed_hex_str(&buf).map_err(serde::de::Error::custom)
    }

    #[cfg(test)]
    mod tests {
        use serde::{Deserialize, Serialize};
        use serde_json::json;
        use starknet_api::serde_utils::bytes_from_hex_str;

        use crate::contract_address::ContractAddress;
        use crate::felt::Felt;
        use crate::patricia_key::PatriciaKey;
        use crate::serde_helpers::hex_string::{
            deserialize_non_prefixed_hex_string_to_felt, deserialize_prefixed_hex_string_to_felt,
            deserialize_to_prefixed_contract_address, deserialize_to_prefixed_patricia_key,
            serialize_contract_address_to_prefixed_hex, serialize_to_prefixed_hex,
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
                    == Felt::from_prefixed_hex_str(
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

            let data = TestSerialization(
                ContractAddress::new(Felt::from_prefixed_hex_str("0x1").unwrap()).unwrap(),
            );

            assert_eq!(serde_json::to_string(&data).unwrap(), r#""0x1""#);
        }

        #[test]
        fn deserialization_of_prefixed_hex_str() {
            check_prefixed_hex_string_and_expected_result("0x0001", true);
            check_prefixed_hex_string_and_expected_result(
                "0x1000000000000000000000000000000000000000000000000000000000000001",
                false,
            );
        }

        #[test]
        fn deserialization_of_non_prefixed_hex_str() {
            check_non_prefixed_hex_string_and_expected_result("0001", true);
            check_non_prefixed_hex_string_and_expected_result(
                "1000000000000000000000000000000000000000000000000000000000000001",
                false,
            );
        }

        #[test]
        fn correct_felt_serializiation() {
            #[derive(Serialize)]
            struct TestSerialzation {
                #[serde(serialize_with = "serialize_to_prefixed_hex")]
                felt: Felt,
            }

            let felt = TestSerialzation { felt: Felt::from(256) };

            assert_eq!(serde_json::to_string(&felt).unwrap(), r#"{"felt":"0x100"}"#);
        }

        fn check_prefixed_hex_string_and_expected_result(hex_str: &str, is_correct: bool) {
            #[derive(Deserialize)]
            struct TestDeserialization {
                #[serde(deserialize_with = "deserialize_prefixed_hex_string_to_felt")]
                felt: Felt,
            }

            let json_str = json!({ "felt": hex_str });

            let result = serde_json::from_value::<TestDeserialization>(json_str);
            if is_correct {
                assert!(result.unwrap().felt == Felt::from_prefixed_hex_str(hex_str).unwrap());
            } else {
                assert!(result.is_err());
            }
        }

        fn check_non_prefixed_hex_string_and_expected_result(hex_str: &str, is_correct: bool) {
            #[derive(Deserialize)]
            struct TestDeserialization {
                #[serde(deserialize_with = "deserialize_non_prefixed_hex_string_to_felt")]
                felt: Felt,
            }

            let json_str = json!({ "felt": hex_str });

            let result = serde_json::from_value::<TestDeserialization>(json_str);
            if is_correct {
                let bytes = bytes_from_hex_str::<32_usize, false>(hex_str).unwrap();
                assert!(result.unwrap().felt == Felt::new(bytes).unwrap());
            } else {
                assert!(result.is_err());
            }
        }
    }
}

pub mod base_64_gzipped_json_string {
    use base64::Engine;
    use flate2::write::GzEncoder;
    use flate2::Compression;
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
        let encoder = GzEncoder::new(&mut buffer, Compression::default());
        serde_json::to_writer(encoder, program)
            .map_err(|_| serde::ser::Error::custom("program: Unable to encode program"))?;

        base64Sir::serialize(&buffer, serializer)
    }

    #[cfg(test)]
    mod tests {
        use serde::Deserialize;

        use crate::serde_helpers::base_64_gzipped_json_string::deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order;
        use crate::utils::test_utils::CAIRO_0_ZIPPED_PROGRAM_PATH;

        #[test]
        fn deserialize_successfully_starknet_api_program() {
            let json_str = std::fs::read_to_string(CAIRO_0_ZIPPED_PROGRAM_PATH).unwrap();

            #[derive(Deserialize)]
            struct TestDeserialization {
                #[allow(unused)]
                #[serde(
                    deserialize_with = "deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order"
                )]
                program: serde_json::Value,
            }

            serde_json::from_str::<TestDeserialization>(&json_str).unwrap();
        }
    }
}
