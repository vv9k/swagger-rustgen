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
    pub required: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub items: Option<Item>,
    pub properties: Option<Items>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Item>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<Value>>,

    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}

impl Schema {
    pub fn merge_all_of_schema(self) -> Schema {
        if let Some(all_of) = self.all_of {
            let base_schema = Schema {
                description: self.description.clone(),
                title: self.title.clone(),
                properties: Some(Items::default()),
                ..Default::default()
            };
            all_of.into_iter().fold(base_schema, |mut acc, schema| {
                if let Some(props) = &mut acc.properties {
                    if let Some(new_props) = &schema.properties {
                        props
                            .0
                            .extend(new_props.0.iter().map(|(k, v)| (k.clone(), v.clone())));
                    }
                }
                macro_rules! add_if_not_set {
                    ($($field:ident),+) => {
                        $(
                        if acc.$field.is_none() && schema.$field.is_some() {
                            acc.$field = schema.$field;
                        }
                        )+
                    };
                }
                add_if_not_set!(format, title, description, required, type_, enum_);

                acc
            })
        } else {
            self
        }
    }
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
