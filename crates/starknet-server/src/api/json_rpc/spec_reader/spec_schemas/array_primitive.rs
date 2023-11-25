use serde::{Deserialize, Serialize};

use super::{Common, Schema};
use crate::api::json_rpc::spec_reader::data_generator::{Acceptor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArrayPrimitive {
    #[serde(flatten)]
    pub common: Common,
    pub items: Box<Schema>,
}
impl Acceptor for ArrayPrimitive {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String> {
        visitor.do_for_array_primitive(self)
    }
}
