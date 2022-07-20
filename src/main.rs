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
pub enum Item {
    Reference(String),
    Object(Box<ItemsObject>),
}

#[derive(Debug)]
pub enum Items {
    Reference(String),
    Objects(HashMap<String, Item>),
}

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
    pub enum_: Option<Vec<serde_yaml::Value>>,

    // Extensions
    #[serde(rename = "x-go-name")]
    pub x_go_name: Option<String>,
    #[serde(rename = "x-go-package")]
    pub x_go_package: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ItemsObject {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub required: Option<Vec<String>>,
    pub items: Option<Items>,
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

impl<'de> de::Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Item, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: serde_yaml::Value = de::Deserialize::deserialize(deserializer)?;

        match v {
            serde_yaml::Value::String(s) => Ok(Item::Reference(s)),
            v => serde_yaml::from_value(v.clone())
                .map(|mut prop: ItemsObject| {
                    prop.extra = v;

                    Item::Object(Box::new(prop))
                })
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

impl<'de> de::Deserialize<'de> for Items {
    fn deserialize<D>(deserializer: D) -> Result<Items, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v: serde_yaml::Value = de::Deserialize::deserialize(deserializer)?;

        let ref_key = "$ref".into();
        match v {
            serde_yaml::Value::String(s) => Ok(Items::Reference(s)),
            serde_yaml::Value::Mapping(map) if map.contains_key(&ref_key) => {
                match map.get(&ref_key) {
                    Some(serde_yaml::Value::String(s)) => Ok(Items::Reference(s.to_string())),
                    Some(v) => Err(de::Error::custom(format!("invalid $ref value `{:?}`", v))),
                    None => unreachable!(),
                }
            }
            v => serde_yaml::from_value(v)
                .map(|prop: HashMap<String, Item>| Items::Objects(prop))
                .map_err(|e| de::Error::custom(e.to_string())),
        }
    }
}

fn main() {
    let yaml = std::fs::read_to_string("/home/wojtek/Downloads/swagger-v4.2.yaml").unwrap();
    let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();
    //println!("{:#?}", swagger);

    for (name, schema) in swagger.definitions.unwrap().0 {
        if let Some(description) = schema.description {
            description.lines().for_each(|line| println!("/// {line}"));
        }

        println!("pub struct {name} {{");

        if let Some(props) = schema.properties {
            match props {
                Items::Reference(_) => todo!(),
                Items::Objects(props) => {
                    props.iter().for_each(|(prop, item)| match item {
                        Item::Reference(_) => todo!(),
                        Item::Object(item) => {
                            if let Some(descr) = &item.description {
                                descr.lines().for_each(|line| println!("    /// {line}"));
                            }
                            print!("    {prop}: ");
                            if let Some(ty) = &item.type_ {
                                print!("{ty}");
                            }
                            println!(",");
                        }
                    });
                }
            }
        }

        println!(" }}\n");
    }
}
