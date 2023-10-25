use std::io;

use serde_json::ser::Formatter;
use serde_json::{Map, Value};

/// The preserve_order feature enabled in the serde_json crate
/// removing a key from the object changes the order of the keys
/// When serde_json is not being used with the preserver order feature
/// deserializing to a serde_json::Value changes the order of the keys
///
/// go through the object by visiting every key and value recursively,
/// and not including them into a new json obj if the condition is met
/// Empty objects are not included
/// Exclude_condition is a function that takes a key and a value and returns a bool
/// If the exclude_condition evaluates to true, the key and value are not included in the new object
pub fn traverse_and_exclude_recursively<F>(
    value: &Value,
    exclude_condition: &F,
) -> serde_json::Value
where
    F: Fn(&String, &Value) -> bool,
{
    match value {
        Value::Object(object) => {
            let mut new_object = Map::new();

            for (key, value) in object {
                if exclude_condition(key, value) {
                    continue;
                }
                let inner_val = traverse_and_exclude_recursively(value, exclude_condition);
                new_object.insert(key.to_string(), inner_val);
            }

            Value::Object(new_object.clone())
        }
        // arrays are visited like the objects - recursively
        Value::Array(array) => {
            let mut inner_arr = Vec::<Value>::new();

            for value in array {
                let inner_val = traverse_and_exclude_recursively(value, exclude_condition);

                if !(inner_val.is_object()
                    && inner_val.as_object().expect("Not a valid JSON object").is_empty())
                {
                    inner_arr.push(inner_val)
                }
            }

            Value::Array(inner_arr)
        }
        // handle non-object, non-array values
        _ => value.clone(),
    }
}

/// JSON Formatter that serializes an object with the desired spaces
/// So the serialized object can match the object structure when compiling cairo program.
/// When serializing with the default formatter, the JSON string is without any spaces between
/// elements. Example here <https://www.cairo-lang.org/docs/hello_starknet/intro.html#>.
pub struct StarknetFormatter;

impl Formatter for StarknetFormatter {
    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first { Ok(()) } else { writer.write_all(b", ") }
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first { Ok(()) } else { writer.write_all(b", ") }
    }
}

#[cfg(test)]
pub(crate) mod test_utils {

    pub(crate) const CAIRO_0_ACCOUNT_CONTRACT_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/Cairo0_contract.json");

    pub(crate) const CAIRO_0_ACCOUNT_CONTRACT_HASH: &str =
        "0x4d07e40e93398ed3c76981e72dd1fd22557a78ce36c0515f679e27f0bb5bc5f";

    pub(crate) const CAIRO_0_RPC_CONTRACT_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/cairo_0_rpc.json");

    pub(crate) const CAIRO_0_ZIPPED_PROGRAM_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/rpc/cairo_0_base64_gzipped_program.json");

    /// contract declared in transaction https://alpha4.starknet.io/feeder_gateway/get_transaction?transactionHash=0x01b852f1fe2b13db21a44f8884bc4b7760dc277bb3820b970dba929860275617
    pub(crate) const CAIRO_1_CONTRACT_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_data/events_cairo1.sierra");

    pub(crate) const CAIRO_1_CONTRACT_SIERRA_HASH: &str =
        "0x113bf26d112a164297e04381212c9bd7409f07591f0a04f539bdf56693eaaf3";
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    #[test]
    fn serde_remove_elements_from_json() {
        let input = r#"
            {
                "name": "John Doe",
                "isStudent": true,
                "age":30,
                "address": {
                    "street": "Vlvo",
                    "city": "Anytown",
                    "state": "Any"
                },
                "should_be_removed": [],
                "scores": 
                [
                    {
                        "street": "AAA",
                        "age": 5,
                        "should_be_removed": []
                    },
                    {
                        "age": 5
                    }
                ],
                "arr": [90, 85, 95]
            }
        "#;
        let expected_output = r#"
            {
                "name": "John Doe",
                "isStudent": true,
                "age":30,
                "address": {
                    "street": "Vlvo",
                    "city": "Anytown",
                    "state": "Any"
                },
                "scores": 
                [
                    {
                        "street": "AAA",
                        "age": 5
                    },
                    {
                        "age": 5
                    }
                ],
                "arr": [90, 85, 95]
            }
        "#;
        let value: Value = serde_json::from_str(input).unwrap();

        let res = crate::utils::traverse_and_exclude_recursively(&value, &|key, val| {
            return key == "should_be_removed"
                && val.is_array()
                && val.as_array().unwrap().is_empty();
        });

        assert_eq!(res, serde_json::from_str::<serde_json::Value>(expected_output).unwrap());
    }
}
