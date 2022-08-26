pub mod codegen;
pub mod definitions;
pub mod items;
pub mod operation;
pub mod parameter;
pub mod path;
pub mod responses;
pub mod schema;

pub const DEFINITIONS_REF: &str = "#/definitions/";
pub const RESPONSES_REF: &str = "#/responses/";

use serde::Deserialize;

pub(crate) use serde_yaml::Value;

fn trim_reference(ref_: &str) -> &str {
    ref_.trim_start_matches(DEFINITIONS_REF)
        .trim_start_matches(RESPONSES_REF)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Swagger {
    pub swagger: String,
    pub definitions: Option<definitions::Definitions>,
    pub paths: Option<path::Paths>,
    pub responses: Option<responses::Responses>,
}
