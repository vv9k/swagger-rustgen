use crate::v2::{
    items::{Item, Items},
    Value,
};

use serde::Deserialize;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,
    pub format: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub items: Option<Item>,
    pub properties: Option<Items>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Item>,
    #[serde(rename = "enum")]
    #[serde(default)]
    pub enum_: Vec<Value>,

    #[serde(rename = "allOf")]
    #[serde(default)]
    pub all_of: Vec<Schema>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}

impl Schema {
    pub fn type_(&self) -> Option<&str> {
        self.type_.as_deref()
    }

    pub fn is_of_type(&self, type_: impl AsRef<str>) -> bool {
        self.type_() == Some(type_.as_ref())
    }

    pub fn is_object(&self) -> bool {
        self.is_of_type("object")
    }

    pub fn is_array(&self) -> bool {
        self.is_of_type("array")
    }

    pub fn is_string_enum(&self) -> bool {
        self.is_of_type("string") && !self.enum_.is_empty()
    }

    pub fn name(&self) -> Option<String> {
        if let Some(title) = &self.x_go_name {
            Some(title.to_string())
        } else if let Some(title) = &self.title {
            Some(title.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::Schema;

    #[test]
    fn schema_types() {
        let s = Schema::default();
        assert_eq!(s.type_(), None);
        assert!(!s.is_array());
        assert!(!s.is_object());
        let s = Schema {
            type_: Some("array".into()),
            ..Default::default()
        };
        assert!(s.is_array());
        assert!(!s.is_object());
        let s = Schema {
            type_: Some("object".into()),
            ..Default::default()
        };
        assert!(!s.is_array());
        assert!(s.is_object());
    }
}
