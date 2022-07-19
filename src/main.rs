// https://swagger.io/specification/v2/#definitionsObject
// https://tools.ietf.org/html/draft-zyp-json-schema-04
use serde::{de, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Swagger {
    pub swagger: String,
    pub definitions: Option<Definitions>,
}

#[derive(Debug, Deserialize)]
pub struct Definitions(pub HashMap<String, Schema>);

#[derive(Debug)]
pub enum Schema {
    Reference(String),
    Object(SchemaObject),
}

#[derive(Debug, Deserialize)]
pub struct SchemaObject {
    pub description: Option<String>,
    pub required: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub format: Option<String>,
    pub items: Option<HashMap<String, Schema>>,
    pub properties: Option<HashMap<String, Schema>>,
    #[serde(rename = "collectionFormat")]
    pub collection_format: Option<String>,
    #[serde(rename = "uniqueItems")]
    pub unique_items: Option<bool>,
    #[serde(rename = "enum")]
    pub enum_: Option<Vec<serde_yaml::Value>>,
    #[serde(rename = "$ref")]
    pub ref_: Option<String>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,

    #[serde(skip_deserializing)]
    pub extra: serde_yaml::Value,
}

impl<'de> de::Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Schema, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: serde_yaml::Value = de::Deserialize::deserialize(deserializer)?;

        match v {
            serde_yaml::Value::String(s) => Ok(Schema::Reference(s)),
            v => serde_yaml::from_value(v.clone())
                .map(|mut prop: SchemaObject| {
                    prop.extra = v;

                    Schema::Object(prop)
                })
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

fn main() {
    let yaml = std::fs::read_to_string("/home/wojtek/Downloads/swagger-v4.2.yaml").unwrap();
    let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();
    println!("{:#?}", swagger);

    for (name, schema) in swagger.definitions.unwrap().0 {
        match schema {
            Schema::Reference(ref_) => {
                println!("{name} {ref_}")
            }
            Schema::Object(schema) => {
                let properties = schema
                    .properties
                    .map(|p| {
                        p.into_iter()
                            .map(|(name, prop)| match prop {
                                Schema::Reference(ref_) => {
                                    format!("\t{name}: {ref_}")
                                }
                                Schema::Object(obj) => {
                                    format!(
                                        "\t{name}: {}",
                                        obj.type_.or(obj.ref_).unwrap_or_default()
                                    )
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                println!(
                    "{name} ({}):\n  Description: {}\n  Fields:\n{}",
                    schema.type_.unwrap_or_default(),
                    &schema.description.unwrap_or_default(),
                    properties
                );
            }
        }
    }
}
