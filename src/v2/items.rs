use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemsObject {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub required: Option<Vec<String>>,
    pub items: Option<Item>,
    #[serde(rename = "collectionFormat")]
    pub collection_format: Option<String>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<Item>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<serde_yaml::Value>>,

    #[serde(skip_deserializing)]
    pub extra: serde_yaml::Value,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Reference(String),
    Object(Box<ItemsObject>),
}

impl<'de> de::Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Item, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: serde_yaml::Value = de::Deserialize::deserialize(deserializer)?;

        let ref_key = "$ref".into();
        match v {
            serde_yaml::Value::String(s) => Ok(Item::Reference(s)),
            serde_yaml::Value::Mapping(map) if map.contains_key(&ref_key) => {
                let ref_ = map.get(&ref_key).unwrap();
                if ref_.is_string() {
                    Ok(Item::Reference(ref_.as_str().unwrap().to_string()))
                } else {
                    Err(de::Error::custom(format!("invalid reference `{:?}`", ref_)))
                }
            }
            v => serde_yaml::from_value(v.clone())
                .map(|mut prop: ItemsObject| {
                    prop.extra = v;

                    Item::Object(Box::new(prop))
                })
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Items(pub HashMap<String, Item>);
