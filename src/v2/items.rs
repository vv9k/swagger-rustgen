use crate::v2::schema::Schema;

use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Item {
    Reference(String),
    Object(Box<Schema>),
}

impl Item {
    pub fn is_reference(&self) -> bool {
        matches!(self, Item::Reference(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Item::Object(_))
    }
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
                .map(|schema: Schema| Item::Object(Box::new(schema)))
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Items(pub HashMap<String, Item>);
