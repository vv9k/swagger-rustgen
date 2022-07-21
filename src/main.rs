// https://swagger.io/specification/v2/#definitionsObject
// https://tools.ietf.org/html/draft-zyp-json-schema-04
use serde::{de, Deserialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Swagger {
    pub swagger: String,
    pub definitions: Option<Definitions>,
}

const DEFINITIONS_REF: &str = "#/definitions/";

#[derive(Debug, Deserialize)]
pub struct Definitions(pub HashMap<String, Schema>);

impl Definitions {
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Schema> {
        let key = key.as_ref().trim_start_matches(DEFINITIONS_REF);
        self.0.get(key)
    }
}

#[derive(Debug)]
pub enum Item {
    Reference(String),
    Object(Box<ItemsObject>),
}

#[derive(Debug, Deserialize)]
pub struct Items(HashMap<String, Item>);

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

enum RustType {
    I32,
    U32,
    I64,
    U64,
    ISize,
    USize,
    F32,
    F64,
    String,
    Bool,
    Vec(Box<RustType>),
    Object(Box<RustType>),
    Option(Box<RustType>),
    Custom(String),
}

impl fmt::Display for RustType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RustType::*;
        match self {
            I32 => write!(f, "i32"),
            U32 => write!(f, "u32"),
            I64 => write!(f, "i64"),
            U64 => write!(f, "u64"),
            F32 => write!(f, "f32"),
            F64 => write!(f, "f64"),
            ISize => write!(f, "isize"),
            USize => write!(f, "usize"),
            String => write!(f, "String"),
            Bool => write!(f, "bool"),
            Vec(ty) => write!(f, "Vec<{ty}>"),
            Object(ty) => write!(f, "HashMap<String, {ty}>"),
            Option(ty) => write!(f, "Option<{ty}>"),
            Custom(ty) => write!(f, "{ty}"),
        }
    }
}

fn map_schema_type(schema: &Schema, ref_: Option<&str>) -> Option<RustType> {
    let ty = schema.type_.as_deref()?;
    match ty {
        "integer" => {
            let ty = match schema.format.as_deref() {
                Some("int64") => RustType::I64,
                Some("uint64") => RustType::U64,
                Some("int32") => RustType::I32,
                Some("uint32") => RustType::U32,
                Some("int") => RustType::ISize,
                Some("uint") => RustType::USize,
                _ => return None,
            };
            Some(ty)
        }
        "string" => Some(RustType::String),
        "boolean" => Some(RustType::Bool),
        "array" => {
            println!("got array schema {:?}", schema);
            None
        }
        "object" => {
            if let Some(ref_) = ref_ {
                return Some(RustType::Custom(
                    ref_.trim_start_matches(DEFINITIONS_REF).to_string(),
                ));
            }
            None
        }
        "number" => {
            let ty = match schema.format.as_deref() {
                Some("double") => RustType::F64,
                Some("float") => RustType::F32,
                _ => return None,
            };
            Some(ty)
        }
        _ => None,
    }
}

fn map_item_type(item: &Item, is_required: bool, definitions: &Definitions) -> Option<RustType> {
    match item {
        Item::Reference(ref_) => {
            let schema = definitions.get(ref_)?;
            map_schema_type(schema, Some(ref_))
        }
        Item::Object(item) => {
            let ty = item.type_.as_deref()?;
            let mut ty = match ty {
                "integer" => match item.format.as_deref() {
                    Some("int64") => RustType::I64,
                    Some("uint64") => RustType::U64,
                    Some("int32") => RustType::I32,
                    Some("uint32") => RustType::U32,
                    Some("int") => RustType::ISize,
                    Some("uint") => RustType::USize,
                    _ => return None,
                },
                "string" => RustType::String,
                "boolean" => RustType::Bool,
                "array" => match &item.items {
                    Some(Item::Reference(ref_)) => {
                        let schema = definitions.get(ref_)?;
                        let ty = map_schema_type(schema, Some(ref_))?;
                        RustType::Vec(Box::new(ty))
                    }
                    Some(item) => {
                        let ty = map_item_type(&item, true, definitions)?;
                        RustType::Vec(Box::new(ty))
                    }
                    None => return None,
                },
                "object" => match &item.additional_properties {
                    Some(Item::Reference(ref_)) => {
                        let schema = definitions.get(ref_)?;
                        let ty = map_schema_type(schema, Some(ref_))?;
                        RustType::Object(Box::new(ty))
                    }
                    Some(item) => {
                        let ty = map_item_type(&item, true, definitions)?;
                        RustType::Object(Box::new(ty))
                    }
                    None => {
                        eprintln!("skipping, object has no additional props {:?}", item);
                        return None;
                    }
                },
                "number" => match item.format.as_deref() {
                    Some("double") => RustType::F64,
                    Some("float") => RustType::F32,
                    _ => return None,
                },
                _ => return None,
            };

            if !is_required {
                ty = RustType::Option(Box::new(ty));
            }

            Some(ty)
        }
    }
}

fn main() {
    let yaml = std::fs::read_to_string("/home/wojtek/Downloads/swagger-v4.2.yaml").unwrap();
    let swagger: Swagger = serde_yaml::from_str(&yaml).unwrap();
    println!("{:#?}", swagger);

    let definitions = swagger.definitions.unwrap();
    for (name, schema) in definitions.0.iter() {
        if let Some(description) = &schema.description {
            description.lines().for_each(|line| println!("/// {line}"));
        }

        println!("pub struct {name} {{");
        let required = schema.required.clone().unwrap_or_default();

        if let Some(props) = &schema.properties {
            props.0.iter().for_each(|(prop, item)| match item {
                Item::Reference(ref_) => {
                    eprintln!("skipping5 {ref_} for {prop}",);
                }
                it @ Item::Object(item) => {
                    if let Some(descr) = &item.description {
                        descr.lines().for_each(|line| println!("    /// {line}"));
                    }
                    print!("    {prop}: ");

                    let is_required = required.contains(prop);

                    if let Some(ty) = map_item_type(it, is_required, &definitions) {
                        print!("{ty}");
                    }
                    println!(",");
                }
            });
        }

        println!(" }}\n");
    }
}
