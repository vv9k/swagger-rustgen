use crate::v2::{
    items::{Item, Items},
    Value,
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,
    pub format: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub items: Option<Item>,
    pub properties: Option<Items>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Item>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<Value>>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}
