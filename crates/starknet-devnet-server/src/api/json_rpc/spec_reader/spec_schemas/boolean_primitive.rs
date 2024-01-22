use serde::{Deserialize, Serialize};

use super::Common;
use crate::api::json_rpc::spec_reader::data_generator::{Acceptor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BooleanPrimitive {
    #[serde(flatten)]
    pub common: Common,
    #[serde(skip)]
    pub generated_value: Option<bool>,
}

impl Acceptor for BooleanPrimitive {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String> {
        visitor.do_for_boolean_primitive()
    }
}
