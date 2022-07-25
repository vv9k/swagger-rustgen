use crate::{schema::Schema, Value};

use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Response {
    Reference(String),
    Object(Box<ResponseObject>),
}

#[derive(Debug, Deserialize)]
pub struct ResponseObject {
    pub description: String,
    pub schema: Option<Schema>,
}

impl<'de> de::Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Response, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: Value = de::Deserialize::deserialize(deserializer)?;

        let ref_key = "$ref".into();
        let schema_key = "schema".into();
        match v {
            Value::Mapping(map) if map.contains_key(&schema_key) => {
                let schema = map.get(&schema_key).unwrap();
                if schema.is_mapping() {
                    if let Some(Value::String(ref_)) = schema.as_mapping().unwrap().get(&ref_key) {
                        Ok(Response::Reference(ref_.to_string()))
                    } else {
                        serde_yaml::from_value(Value::Mapping(map))
                            .map(|resp: ResponseObject| Response::Object(Box::new(resp)))
                            .map_err(|e| de::Error::custom(e.to_string()))
                    }
                } else {
                    Err(de::Error::custom("invalid schema, expected mapping"))
                }
            }
            v => serde_yaml::from_value(v)
                .map(|resp: ResponseObject| Response::Object(Box::new(resp)))
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Responses(pub HashMap<String, Response>);
