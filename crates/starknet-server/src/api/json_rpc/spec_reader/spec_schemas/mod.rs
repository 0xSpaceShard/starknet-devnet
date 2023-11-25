use serde::{Deserialize, Serialize};

use self::all_of_schema::AllOf;
use self::array_primitive::ArrayPrimitive;
use self::boolean_primitive::BooleanPrimitive;
use self::integer_primitive::IntegerPrimitive;
use self::object_primitive::ObjectPrimitive;
use self::one_of_schema::OneOf;
use self::ref_schema::Reference;
use self::string_primitive::StringPrimitive;

pub(crate) mod all_of_schema;
pub(crate) mod array_primitive;
pub(crate) mod boolean_primitive;
pub(crate) mod integer_primitive;
pub(crate) mod object_primitive;
pub(crate) mod one_of_schema;
pub(crate) mod ref_schema;
pub(crate) mod string_primitive;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Common {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Primitive {
    Array(ArrayPrimitive),
    Boolean(BooleanPrimitive),
    Integer(IntegerPrimitive),
    Number(IntegerPrimitive),
    Object(ObjectPrimitive),
    String(StringPrimitive),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Schema {
    Ref(Reference),
    OneOf(OneOf),
    AllOf(AllOf),
    Primitive(Primitive),
}
