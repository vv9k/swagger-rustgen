use crate::v2::{operation::Operation, Value};

use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Path {
    Item(Box<PathItemObject>),
    Extension(serde_yaml::Value),
}

impl<'de> de::Deserialize<'de> for Paths {
    fn deserialize<D>(deserializer: D) -> Result<Paths, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: Value = de::Deserialize::deserialize(deserializer)?;

        match v {
            Value::Mapping(map) => {
                let paths: HashMap<String, Path> = map
                    .into_iter()
                    .filter_map(|(key, val)| {
                        if key.is_string() {
                            let key = key.as_str().unwrap();
                            if key.starts_with("x-") {
                                Some((key.to_owned(), Path::Extension(val)))
                            } else {
                                serde_yaml::from_value(val).ok().map(|v: PathItemObject| {
                                    (key.to_owned(), Path::Item(Box::new(v)))
                                })
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Paths(paths))
            }
            v => Err(de::Error::custom(format!(
                "invalid object for paths `{:?}`",
                v
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathItemObject {
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,
    pub get: Option<Operation>,
    pub put: Option<Operation>,
    pub post: Option<Operation>,
    pub delete: Option<Operation>,
    pub options: Option<Operation>,
    pub head: Option<Operation>,
    pub patch: Option<Operation>,
}

#[derive(Debug, Clone)]
pub struct Paths(pub HashMap<String, Path>);
