use crate::v2::codegen::backend::python::format_type_name;
use crate::v2::{trim_reference, Schema, Swagger};

use log::trace;
use std::fmt;

#[derive(Clone)]
pub enum Type {
    String,
    Bool,
    Int,
    Float,
    List(Box<Type>),
    Map(Box<Type>),
    Custom(String),
    Value,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Type::*;
        match self {
            String => write!(f, "str"),
            Bool => write!(f, "bool"),
            Int => write!(f, "int"),
            Float => write!(f, "float"),
            List(ty) => write!(f, "List[{ty}]"),
            Map(ty) => write!(f, "Map[{ty}]"),
            Value => write!(f, "{}", Type::Map(Box::new(Type::String))),
            Custom(ty) => write!(f, "{}", format_type_name(ty)),
        }
    }
}

impl crate::v2::Type for Type {
    fn format_name(name: &str) -> String {
        format_type_name(name)
    }

    fn map_schema_type(
        schema: &Schema,
        ref_: Option<&str>,
        is_required: bool,
        parent_name: Option<&str>,
        swagger: &Swagger<Self>,
    ) -> Option<Self> {
        let ty = schema.type_()?;
        trace!(
            "mapping schema type, type: {ty}, ref: {ref_:?}, required: {is_required}, parent: {parent_name:?}"
        );
        let ty = match ty {
            "integer" => Type::Int,
            "string" => match schema
                .format
                .as_ref()
                .map(|fmt| fmt.to_lowercase())
                .as_deref()
            {
                //Some("date-time") | Some("datetime") | Some("date time") => Type::String,
                Some("binary") => Type::List(Box::new(Type::Int)),
                _ => Type::String,
            },
            "boolean" => Type::Bool,
            "array" => {
                let ty = if let Some(ref_) = ref_ {
                    Type::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        Type::List(Box::new(ty))
                    } else {
                        return None;
                    }
                } else {
                    return None;
                };

                ty
            }
            "object" => {
                let ty = if let Some(ref_) = ref_ {
                    Type::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.additional_properties {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        Type::Map(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        Type::Map(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if schema.properties.is_some() {
                    if let Some(name) = schema.name() {
                        Type::Custom(name)
                    } else if let Some(parent_name) = &parent_name {
                        Type::Custom(format!("{parent_name}InlineItem"))
                    } else {
                        Type::Value
                    }
                } else {
                    Type::Value
                };

                ty
            }
            "number" => {
                let ty = match schema.format.as_deref() {
                    Some("double") | Some("float") => Type::Float,
                    _ => return None,
                };
                ty
            }
            _ => return None,
        };
        trace!("mapped to {ty}");
        Some(ty)
    }
}
