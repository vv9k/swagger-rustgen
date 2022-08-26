use crate::v2::{items::Item, schema::Schema};

use serde::{de, Deserialize};
use serde_yaml::Value;

pub enum Parameter {
    Path(PathParameter),
    Query(QueryParameter),
    Body(BodyParameter),
}

impl<'de> de::Deserialize<'de> for Parameter {
    fn deserialize<D>(deserializer: D) -> Result<Parameter, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: Value = de::Deserialize::deserialize(deserializer)?;

        match v {
            Value::Mapping(map) => {
                if let Some(in_) = map.get(&Value::String("in".into())) {
                    if !in_.is_string() {
                        return Err(de::Error::custom(format!("invalid `in` - {in_:?}")));
                    } else {
                        let in_ = in_.as_str().unwrap();
                        match in_ {
                            "query" => serde_yaml::from_value(Value::Mapping(map))
                                .map(|param: QueryParameter| Parameter::Query(param))
                                .map_err(|e| de::Error::custom(e.to_string())),
                            "path" => serde_yaml::from_value(Value::Mapping(map))
                                .map(|param: PathParameter| Parameter::Path(param))
                                .map_err(|e| de::Error::custom(e.to_string())),
                            "body" => serde_yaml::from_value(Value::Mapping(map))
                                .map(|param: BodyParameter| Parameter::Body(param))
                                .map_err(|e| de::Error::custom(e.to_string())),
                            _ => Err(de::Error::custom("unexpected parameter type `{in_}`")),
                        }
                    }
                } else {
                    Err(de::Error::custom("expected `in` field for parameter"))
                }
            }
            v => Err(de::Error::custom(format!(
                "invalid object for param `{:?}`",
                v
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PathParameter {
    pub name: String,
    pub description: Option<String>,
    pub type_: String,
    #[serde(default)]
    pub required: bool,
    pub items: Option<Item>,
}

pub type QueryParameter = PathParameter;

#[derive(Debug, Clone, Deserialize)]
pub struct BodyParameter {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    pub schema: Schema,
}
