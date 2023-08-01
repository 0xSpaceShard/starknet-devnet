pub mod base_64_gzipped_json_string {
    use base64::Engine;
    use serde::{Deserialize, Deserializer};
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

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(buf)
            .map_err(|_| serde::de::Error::custom("program: Unable to decode base64 string"))?;

        let decoder = flate2::read::GzDecoder::new(bytes.as_slice());

        let starknet_program: LegacyProgram = serde_json::from_reader(decoder)
            .map_err(|_| serde::de::Error::custom("program: Unable to decode gzipped bytes"))?;

        serde_json::to_value(starknet_program)
            .map_err(|_| serde::de::Error::custom("program: Unable to parse to JSON"))
    }

    #[cfg(test)]
    mod tests {
        use serde::Deserialize;

        use types::serde_helpers::base_64_gzipped_json_string::deserialize_to_serde_json_value_with_keys_ordered_in_alphabetical_order;

        #[test]
        fn deserialize_successfully_starknet_api_program() {
            let json_str = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_data/rpc/cairo_0_base64_gzipped_program.json"
            ))
            .unwrap();

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
