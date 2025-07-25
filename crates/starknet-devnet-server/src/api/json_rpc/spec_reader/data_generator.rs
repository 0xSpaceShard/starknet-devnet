use std::collections::HashMap;

use rand::{Rng, SeedableRng};
use serde_json::{Map, Value};

use super::spec_schemas::all_of_schema::AllOf;
use super::spec_schemas::array_primitive::ArrayPrimitive;
use super::spec_schemas::integer_primitive::IntegerPrimitive;
use super::spec_schemas::object_primitive::ObjectPrimitive;
use super::spec_schemas::one_of_schema::OneOf;
use super::spec_schemas::ref_schema::Reference;
use super::spec_schemas::string_primitive::StringPrimitive;
use super::spec_schemas::tuple_schema::Tuple;
use super::spec_schemas::{Primitive, Schema};

const MAX_DEPTH: u8 = 5;
/// regex pattern for 1-10 characters
const DEFAULT_STRING_REGEX: &str = "^.{1,10}$";

pub trait Visitor {
    fn do_for_boolean_primitive(&self) -> Result<serde_json::Value, String>;
    fn do_for_string_primitive(
        &self,
        element: &StringPrimitive,
    ) -> Result<serde_json::Value, String>;
    fn do_for_integer_primitive(
        &self,
        element: &IntegerPrimitive,
    ) -> Result<serde_json::Value, String>;
    fn do_for_array_primitive(&self, element: &ArrayPrimitive)
    -> Result<serde_json::Value, String>;
    fn do_for_ref(&self, element: &Reference) -> Result<serde_json::Value, String>;
    fn do_for_one_of(&self, element: &OneOf) -> Result<serde_json::Value, String>;
    fn do_for_all_of(&self, element: &AllOf) -> Result<serde_json::Value, String>;
    fn do_for_object_primitive(
        &self,
        element: &ObjectPrimitive,
    ) -> Result<serde_json::Value, String>;

    fn do_for_tuple(&self, element: &Tuple) -> Result<serde_json::Value, String>;
}

pub trait Acceptor {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String>;
}

pub struct RandDataGenerator<'a> {
    schemas: &'a HashMap<String, Schema>,
    depth: u8,
}

impl<'a> RandDataGenerator<'a> {
    pub fn new(schemas: &'a HashMap<String, Schema>, depth: u8) -> Self {
        Self { schemas, depth }
    }
}

impl Visitor for RandDataGenerator<'_> {
    fn do_for_boolean_primitive(&self) -> Result<serde_json::Value, String> {
        Ok(serde_json::Value::Bool(rand::thread_rng().gen_bool(0.5)))
    }

    fn do_for_string_primitive(
        &self,
        element: &StringPrimitive,
    ) -> Result<serde_json::Value, String> {
        if let Some(enums) = element.possible_enums.clone() {
            let random_number = rand::thread_rng().gen_range(0..enums.len());
            return Ok(serde_json::Value::String(enums[random_number].clone()));
        }

        // If pattern is not set, then generate a string from the default pattern
        let regex_pattern = element.pattern.as_deref().unwrap_or(DEFAULT_STRING_REGEX);
        let mut buffer: Vec<u8> = vec![];
        let seed = rand::thread_rng().gen();

        regex_generate::Generator::new(
            regex_pattern,
            rand_chacha::ChaCha8Rng::seed_from_u64(seed),
            100,
        )
        .unwrap()
        .generate(&mut buffer)
        .unwrap();

        let random_string = String::from_utf8(buffer).unwrap();

        Ok(serde_json::Value::String(random_string))
    }

    fn do_for_integer_primitive(
        &self,
        element: &IntegerPrimitive,
    ) -> Result<serde_json::Value, String> {
        let num = rand::thread_rng()
            .gen_range(element.minimum.unwrap_or_default()..element.maximum.unwrap_or(i32::MAX));

        Ok(serde_json::Value::Number(serde_json::Number::from(num)))
    }

    fn do_for_array_primitive(
        &self,
        element: &ArrayPrimitive,
    ) -> Result<serde_json::Value, String> {
        let mut array = vec![];
        if self.depth >= MAX_DEPTH {
            return Ok(serde_json::Value::Array(array));
        }

        let min_items = element.min_items.unwrap_or(1);
        let max_items = element.max_items.unwrap_or(3);

        let number_of_elements = rand::thread_rng().gen_range(min_items..=max_items);

        for _ in 0..number_of_elements {
            let generated_value =
                generate_schema_value(element.items.as_ref(), self.schemas, self.depth + 1)?;

            if !generated_value.is_null() {
                array.push(generated_value);
            }
        }

        Ok(serde_json::Value::Array(array))
    }

    fn do_for_ref(&self, element: &Reference) -> Result<serde_json::Value, String> {
        let schema_name = element
            .ref_field
            .trim_start_matches("./")
            .split("#/components/schemas/")
            .filter(|entry| !entry.is_empty())
            .last()
            .unwrap_or_default();

        let schema = self
            .schemas
            .get(schema_name)
            .ok_or(format!("Missing schema in components {}", schema_name))?;

        generate_schema_value(schema, self.schemas, self.depth)
    }

    fn do_for_one_of(&self, element: &OneOf) -> Result<serde_json::Value, String> {
        let idx = rand::thread_rng().gen_range(0..element.one_of.len());
        let schema =
            element.one_of.get(idx).ok_or("OneOf schema doesn't have entry".to_string())?;

        generate_schema_value(schema, self.schemas, self.depth)
    }

    fn do_for_all_of(&self, element: &AllOf) -> Result<serde_json::Value, String> {
        let mut accumulated_json_value = Map::new();

        for one in element.all_of.iter() {
            let generated_value = generate_schema_value(one, self.schemas, self.depth)?;

            if !generated_value.is_null() {
                let single_value = generated_value
                    .as_object()
                    .ok_or(format!(
                        "Expected to be an object: {generated_value:?}. AllOf element: {element:?}"
                    ))?
                    .clone();

                accumulated_json_value.extend(single_value);
            }
        }

        if accumulated_json_value.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(serde_json::Value::Object(accumulated_json_value))
        }
    }

    fn do_for_object_primitive(
        &self,
        element: &ObjectPrimitive,
    ) -> Result<serde_json::Value, String> {
        if self.depth >= MAX_DEPTH {
            return Ok(Value::Null);
        }
        let mut accumulated_json_value = Map::new();

        // Collect all field names as references
        let all_fields: Vec<&String> = element.properties.keys().collect();

        // if there are no required json entry, then all propertries are required
        let required_fields: Vec<&String> = if let Some(required) = &element.required {
            required.iter().collect()
        } else {
            all_fields.clone()
        };

        // Determine optional fields by removing required fields from all fields
        let mut optional_fields: Vec<&String> =
            all_fields.iter().filter(|field| !required_fields.contains(field)).cloned().collect();

        // if there are no optional fields then all fields have to be included
        let fields_to_include = if optional_fields.is_empty() {
            required_fields
        } else {
            // decide the number of optional fields to remove
            let mut optional_fields_left_to_remove =
                rand::thread_rng().gen_range(0..=optional_fields.len());

            // remove the optional fields 1 by 1
            while optional_fields_left_to_remove > 0 {
                optional_fields_left_to_remove -= 1;

                let idx_to_remove = rand::thread_rng().gen_range(0..optional_fields.len());
                optional_fields.swap_remove(idx_to_remove);
            }

            // combine required and optional fields that will be part of the json object
            [required_fields.as_slice(), optional_fields.as_slice()].concat()
        };

        for (key, inner_schema) in
            element.properties.iter().filter(|(k, _)| fields_to_include.contains(k))
        {
            let generated_value =
                generate_schema_value(inner_schema, self.schemas, self.depth + 1)?;

            // this means that it reached max depth, but one last value must be generated,
            // because it the object will be generated without this property and further
            // deserialization will fail with missing field
            if generated_value.is_null() {
                let generated_value =
                    generate_schema_value(inner_schema, self.schemas, self.depth)?;
                accumulated_json_value.insert(key.to_string(), generated_value);
            } else {
                accumulated_json_value.insert(key.to_string(), generated_value);
            }
        }

        if accumulated_json_value.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(Value::Object(accumulated_json_value))
        }
    }

    fn do_for_tuple(&self, element: &Tuple) -> Result<serde_json::Value, String> {
        let mut array = vec![];
        if self.depth >= MAX_DEPTH {
            return Ok(serde_json::Value::Null);
        }

        for variant in element.variants.iter() {
            let generated_value = generate_schema_value(variant, self.schemas, self.depth + 1)?;

            if generated_value.is_null() {
                let generated_value = generate_schema_value(variant, self.schemas, self.depth)?;
                array.push(generated_value);
            } else {
                array.push(generated_value);
            }
        }

        Ok(serde_json::Value::Array(array))
    }
}

pub fn generate_schema_value(
    schema: &Schema,
    schemas: &HashMap<String, Schema>,
    depth: u8,
) -> core::result::Result<Value, String> {
    let generator = RandDataGenerator::new(schemas, depth);

    match schema {
        Schema::Ref(schema_ref) => schema_ref.accept(&generator),
        Schema::OneOf(one) => one.accept(&generator),
        Schema::AllOf(all) => all.accept(&generator),
        Schema::Primitive(Primitive::Integer(integer_primitive)) => {
            integer_primitive.accept(&generator)
        }
        Schema::Primitive(Primitive::Number(number_primitive)) => {
            number_primitive.accept(&generator)
        }
        Schema::Primitive(Primitive::String(string_primitive)) => {
            string_primitive.accept(&generator)
        }
        Schema::Primitive(Primitive::Array(array)) => array.accept(&generator),
        Schema::Primitive(Primitive::Boolean(boolean)) => boolean.accept(&generator),
        Schema::Primitive(Primitive::Object(obj)) => obj.accept(&generator),
        Schema::Tuple(tuple) => tuple.accept(&generator),
    }
}
