use crate::v2::{parameter::Parameter, responses::Responses};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Operation {
    #[serde(default)]
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub consumes: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    pub responses: Responses,
    #[serde(default)]
    pub depracated: bool,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}
