use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{Rng, SeedableRng};
use serde_json::{Map, Value};

use super::spec_schemas::all_of_schema::AllOf;
use super::spec_schemas::array_primitive::ArrayPrimitive;
use super::spec_schemas::integer_primitive::IntegerPrimitive;
use super::spec_schemas::object_primitive::ObjectPrimitive;
use super::spec_schemas::one_of_schema::OneOf;
use super::spec_schemas::ref_schema::Reference;
use super::spec_schemas::string_primitive::StringPrimitive;
use super::spec_schemas::{Primitive, Schema};

const MAX_DEPTH: u8 = 5;

pub(crate) trait Visitor {
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
}

pub(crate) trait Acceptor {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String>;
}

pub(crate) struct RandDataGenerator<'a> {
    schemas: &'a HashMap<String, Schema>,
    depth: u8,
}

impl<'a> RandDataGenerator<'a> {
    pub(crate) fn new(schemas: &'a HashMap<String, Schema>, depth: u8) -> Self {
        Self { schemas, depth }
    }
}

impl<'a> Visitor for RandDataGenerator<'a> {
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

        if let Some(regex_pattern) = element.pattern.clone() {
            let mut buffer: Vec<u8> = vec![];
            let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let u128_bytes = duration.as_nanos().to_be_bytes();
            let mut u64_bytes = [0; 8];
            u64_bytes.copy_from_slice(&u128_bytes[8..16]);
            let seed = u64::from_be_bytes(u64_bytes);

            regex_generate::Generator::new(
                &regex_pattern,
                rand_chacha::ChaCha8Rng::seed_from_u64(seed),
                100,
            )
            .unwrap()
            .generate(&mut buffer)
            .unwrap();
            let random_string = String::from_utf8(buffer).unwrap();

            return Ok(serde_json::Value::String(random_string));
        }

        Ok(serde_json::Value::String("".to_string()))
    }

    fn do_for_integer_primitive(
        &self,
        element: &IntegerPrimitive,
    ) -> Result<serde_json::Value, String> {
        let num = rand::thread_rng().gen_range(element.minimum.unwrap_or_default()..i32::MAX);

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

        let number_of_elements = rand::thread_rng().gen_range(1..3);

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
        let schema = element.one_of.get(idx).ok_or("OneOf schema doesnt have entry".to_string())?;

        generate_schema_value(schema, self.schemas, self.depth)
    }

    fn do_for_all_of(&self, element: &AllOf) -> Result<serde_json::Value, String> {
        let mut accumulated_json_value = Map::new();

        for one in element.all_of.iter() {
            let generated_value = generate_schema_value(one, self.schemas, self.depth)?;

            if !generated_value.is_null() {
                let single_value = generated_value
                    .as_object()
                    .ok_or("Expected to be an object".to_string())?
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

        for (key, inner_schema) in
            element.properties.iter().filter(|(k, _)| match element.required.as_ref() {
                Some(required_fields) => required_fields.contains(k),
                None => true,
            })
        {
            let generated_value =
                generate_schema_value(inner_schema, self.schemas, self.depth + 1)?;

            if !generated_value.is_null() {
                accumulated_json_value.insert(key.to_string(), generated_value);
            }
        }

        if accumulated_json_value.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(Value::Object(accumulated_json_value))
        }
    }
}

pub(crate) fn generate_schema_value(
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
    }
}
