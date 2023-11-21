use serde::{Deserialize, Serialize};

use super::Common;
use crate::api::json_rpc::spec_reader::data_generator::{Acceptor, Visitor};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Reference {
    #[serde(rename = "$comment")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "$ref")]
    pub ref_field: String,
    #[serde(flatten)]
    pub common: Common,
}

impl Acceptor for Reference {
    fn accept(&self, visitor: &impl Visitor) -> Result<serde_json::Value, String> {
        visitor.do_for_ref(self)
    }
}
