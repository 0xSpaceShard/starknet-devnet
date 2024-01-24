use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{Common, Schema};
use crate::api::json_rpc::spec_reader::data_generator::{Acceptor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ObjectPrimitive {
    #[serde(flatten)]
    pub common: Common,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub properties: HashMap<String, Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl Acceptor for ObjectPrimitive {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String> {
        visitor.do_for_object_primitive(self)
    }
}
