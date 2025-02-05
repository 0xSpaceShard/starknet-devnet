use std::fs;

use serde::Deserialize;
use serde_json::Value;

use super::Spec;

/// Note: this structs is not all round solution for modifying the specification.
/// It has some limitations, if you need to modify the specification in a way that is not supported
/// Please use the remove_manually entry of the .yaml file
#[derive(Deserialize)]
pub struct SpecModifier {
    clean: Vec<String>,
    replace_property: Vec<ReplacePropertyData>,
    add: Vec<AddPropertyData>,
    remove_from_array: Vec<RemoveArrayElement>,
}

#[derive(Deserialize)]
struct ReplacePropertyData {
    path: String,
    new_name: String,
}

#[derive(Deserialize)]
struct AddPropertyData {
    path: String,
    new_entry: String,
    // default value is false
    #[serde(default)]
    replace_mode: bool,
}

#[derive(Deserialize)]
struct RemoveArrayElement {
    path: String,
    index: usize,
}

fn rename_property(json_obj: &mut Value, path_parts: &[&str], new_name: &str) {
    if path_parts.len() == 1 {
        if let Some(obj) = json_obj.as_object_mut() {
            if let Some(value) = obj.remove(path_parts[0]) {
                obj.insert(new_name.to_string(), value);
            }
        }
    } else if let Some(next_obj) = json_obj.get_mut(path_parts[0]) {
        rename_property(next_obj, &path_parts[1..], new_name);
    }
}

/// Deletes a property from a JSON object
fn delete_property(json_obj: &mut Value, path_parts: &[&str]) {
    if path_parts.len() == 1 {
        if let Some(obj) = json_obj.as_object_mut() {
            obj.remove(path_parts[0]);
        }
    } else if let Some(next_obj) = json_obj.get_mut(path_parts[0]) {
        delete_property(next_obj, &path_parts[1..]);
    }
}

/// Adds a property to a JSON object.
///
/// The new property is specified in the form `"key/value"`.
/// The value is added as a string or number, depending on whether it can be parsed into a number.
///
/// - If `do_replace` is `true`, the value will be added only if the key already exists.
/// - If `do_replace` is `false`, the value will be added anyways.
///
/// # Arguments
///
/// - `json_obj`: A mutable reference to the JSON `Value` to modify.
/// - `path_parts`: A slice of strings representing the path within the JSON object where the
///   property should be added.
/// - `new_entry`: A string in the form `"key/value"` specifying the new property to add.
/// - `do_replace`: A boolean flag indicating whether to replace the value if the key already
///   exists.
fn add_property(json_obj: &mut Value, path_parts: &[&str], new_entry: &str, do_replace: bool) {
    if path_parts.is_empty() {
        if let Some(obj) = json_obj.as_object_mut() {
            let new_entry_parts = new_entry.split('/').collect::<Vec<&str>>();
            let value = new_entry_parts[1..].join("/");
            let value = if let Ok(parsed_number) = value.parse::<u64>() {
                serde_json::Value::Number(parsed_number.into())
            } else {
                serde_json::Value::String(value)
            };

            let key = new_entry_parts[0];
            if do_replace {
                if obj.contains_key(key) {
                    obj.insert(key.to_string(), value);
                }
            } else {
                obj.insert(key.to_string(), value);
            }
        }
    } else if let Some(next_obj) = json_obj.get_mut(path_parts[0]) {
        add_property(next_obj, &path_parts[1..], new_entry, do_replace);
    }
}

fn remove_array_element(json_obj: &mut Value, path_parts: &[&str], index: usize) {
    if path_parts.is_empty() {
        if let Some(arr) = json_obj.as_array_mut() {
            arr.remove(index);
        }
    } else if let Some(next_obj) = json_obj.get_mut(path_parts[0]) {
        remove_array_element(next_obj, &path_parts[1..], index);
    }
}

impl SpecModifier {
    pub fn load_from_path(path: &str) -> Self {
        let yaml_str = fs::read_to_string(path).expect("Could not read YAML file");

        let instructions: SpecModifier =
            serde_yaml::from_str(&yaml_str).expect("Could not parse the YAML file");

        instructions
    }

    pub fn generate_spec(&self, mut json_obj_spec: Value) -> Spec {
        for path_to_clean in self.clean.iter() {
            let path_parts = path_to_clean.split('/').collect::<Vec<&str>>();
            delete_property(&mut json_obj_spec, &path_parts);
        }

        for path_to_replace in self.replace_property.iter() {
            let path_parts = path_to_replace.path.split('/').collect::<Vec<&str>>();
            rename_property(&mut json_obj_spec, &path_parts, &path_to_replace.new_name);
        }

        for entry_to_add in self.add.iter() {
            let path_parts = entry_to_add.path.split('/').collect::<Vec<&str>>();
            add_property(
                &mut json_obj_spec,
                &path_parts,
                &entry_to_add.new_entry,
                entry_to_add.replace_mode,
            );
        }

        for array_element_to_remove in self.remove_from_array.iter() {
            let path_parts = array_element_to_remove.path.split('/').collect::<Vec<&str>>();
            remove_array_element(&mut json_obj_spec, &path_parts, array_element_to_remove.index);
        }

        // Serialize serde_json::Value to string first and then deserialize to object Spec,
        // because if there is an error during deserialization, the error message will contain at
        // which line number and column, the deserialization failed.
        let json_spec_str = serde_json::to_string_pretty(&json_obj_spec)
            .expect("could not serialize the spec to string");

        // Parse the spec into a Spec struct
        match serde_json::from_str(&json_spec_str) {
            Ok(obj) => obj,
            Err(e) => {
                std::fs::write("failed_spec.json", json_spec_str).unwrap();
                panic!("Could not parse the JSON-RPC spec: {e:?}");
            }
        }
    }
}
