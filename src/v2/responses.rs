use crate::v2::{schema::Schema, Value};

use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Response {
    Reference(String),
    Object(Box<ResponseObject>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseObject {
    pub description: Option<String>,
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

#[derive(Debug, Clone)]
pub struct Responses(pub HashMap<String, Response>);

impl<'de> de::Deserialize<'de> for Responses {
    fn deserialize<D>(deserializer: D) -> Result<Responses, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: Value = de::Deserialize::deserialize(deserializer)?;

        let mut responses = HashMap::new();
        match v {
            Value::Mapping(map) => {
                for (key, val) in map {
                    let k = match key {
                        Value::String(k) => k,
                        Value::Number(n) => n.to_string(),
                        _ => return Err(de::Error::custom("invalid responses key type")),
                    };
                    let val: Response = serde_yaml::from_value(val)
                        .map_err(|e| de::Error::custom(e.to_string()))?;
                    responses.insert(k, val);
                }

                Ok(Responses(responses))
            }
            _ => Err(de::Error::custom("invalid type for responses object")),
        }
    }
}
