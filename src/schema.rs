use crate::{items::Items, trim_reference, types::RustType, Value};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Schema {
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,
    pub format: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub items: Option<Items>,
    pub properties: Option<Items>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<Value>>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}

impl Schema {
    pub fn map_type(&self, ref_: Option<&str>) -> Option<RustType> {
        let ty = self.type_.as_deref()?;
        match ty {
            "integer" => self
                .format
                .as_ref()
                .and_then(|format| RustType::from_integer_format(format))
                .or(Some(RustType::USize)),
            "string" => Some(RustType::String),
            "boolean" => Some(RustType::Bool),
            "array" => {
                if let Some(ref_) = ref_ {
                    return Some(RustType::Vec(Box::new(RustType::Custom(
                        trim_reference(ref_).to_string(),
                    ))));
                }
                None
            }
            "object" => {
                if let Some(ref_) = ref_ {
                    return Some(RustType::Custom(trim_reference(ref_).to_string()));
                }
                None
            }
            "number" => {
                let ty = match self.format.as_deref() {
                    Some("double") => RustType::F64,
                    Some("float") => RustType::F32,
                    _ => return None,
                };
                Some(ty)
            }
            _ => None,
        }
    }
}
