use serde::{Deserialize, Serialize};

use super::Common;
use crate::api::json_rpc::spec_reader::data_generator::{Acceptor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IntegerPrimitive {
    #[serde(flatten)]
    pub common: Common,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i32>,
}

impl Acceptor for IntegerPrimitive {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String> {
        visitor.do_for_integer_primitive(self)
    }
}
