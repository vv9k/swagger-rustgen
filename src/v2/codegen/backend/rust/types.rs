use crate::v2::codegen::backend::rust::format_type_name;
use crate::v2::{trim_reference, Schema, Swagger};

use log::trace;
use std::fmt;

#[derive(Clone)]
pub enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    ISize,
    USize,
    F32,
    F64,
    String,
    DateTime,
    Bool,
    Vec(Box<Type>),
    Object(Box<Type>),
    Option(Box<Type>),
    Custom(String),
    Value,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Type::*;
        match self {
            I8 => write!(f, "i8"),
            U8 => write!(f, "u8"),
            I16 => write!(f, "i16"),
            U16 => write!(f, "u16"),
            I32 => write!(f, "i32"),
            U32 => write!(f, "u32"),
            I64 => write!(f, "i64"),
            U64 => write!(f, "u64"),
            F32 => write!(f, "f32"),
            F64 => write!(f, "f64"),
            ISize => write!(f, "isize"),
            USize => write!(f, "usize"),
            String => write!(f, "String"),
            DateTime => write!(f, "DateTime<Utc>"),
            Bool => write!(f, "bool"),
            Vec(ty) => write!(f, "Vec<{ty}>"),
            Object(ty) => write!(f, "HashMap<String, {ty}>"),
            Option(ty) => write!(f, "Option<{ty}>"),
            Custom(ty) => write!(f, "{}", format_type_name(ty)),
            Value => write!(f, "Value"),
        }
    }
}

impl Type {
    pub fn from_integer_format(format: &str) -> Option<Self> {
        let ty = match format {
            "int" => Type::ISize,
            "uint" => Type::USize,
            "int64" => Type::I64,
            "uint64" => Type::U64,
            "int32" => Type::I32,
            "uint32" => Type::U32,
            "int16" => Type::I16,
            "uint16" => Type::U16,
            "int8" => Type::I8,
            "uint8" => Type::U8,
            _ => return None,
        };

        Some(ty)
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
        let mut ty = match ty {
            "integer" => schema
                .format
                .as_ref()
                .and_then(|format| Type::from_integer_format(format))
                .unwrap_or(Type::ISize),
            "string" => match schema
                .format
                .as_ref()
                .map(|fmt| fmt.to_lowercase())
                .as_deref()
            {
                Some("date-time") | Some("datetime") | Some("date time") => Type::DateTime,
                Some("binary") => Type::Vec(Box::new(Type::U8)),
                _ => Type::String,
            },
            "boolean" => Type::Bool,
            "array" => {
                let ty = if let Some(ref_) = ref_ {
                    Type::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        Type::Vec(Box::new(ty))
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
                        Type::Object(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        Type::Object(Box::new(ty))
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
                    Some("double") => Type::F64,
                    Some("float") => Type::F32,
                    _ => return None,
                };
                ty
            }
            _ => return None,
        };
        if !is_required {
            ty = Type::Option(Box::new(ty));
        }
        trace!("mapped to {ty}");
        Some(ty)
    }
}
