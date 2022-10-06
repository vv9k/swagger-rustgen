use crate::v2::codegen::backend::rust::format_type_name;
use crate::v2::{trim_reference, Schema, Swagger};

use log::trace;
use std::fmt;

#[derive(Clone)]
pub enum RustType {
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
    Vec(Box<RustType>),
    Object(Box<RustType>),
    Option(Box<RustType>),
    Custom(String),
    Value,
}

impl fmt::Display for RustType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RustType::*;
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

impl RustType {
    pub fn from_integer_format(format: &str) -> Option<Self> {
        let ty = match format {
            "int" => RustType::ISize,
            "uint" => RustType::USize,
            "int64" => RustType::I64,
            "uint64" => RustType::U64,
            "int32" => RustType::I32,
            "uint32" => RustType::U32,
            "int16" => RustType::I16,
            "uint16" => RustType::U16,
            "int8" => RustType::I8,
            "uint8" => RustType::U8,
            _ => return None,
        };

        Some(ty)
    }
}

impl crate::v2::Type for RustType {
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
                .and_then(|format| RustType::from_integer_format(format))
                .unwrap_or(RustType::ISize),
            "string" => match schema
                .format
                .as_ref()
                .map(|fmt| fmt.to_lowercase())
                .as_deref()
            {
                Some("date-time") | Some("datetime") | Some("date time") => RustType::DateTime,
                Some("binary") => RustType::Vec(Box::new(RustType::U8)),
                _ => RustType::String,
            },
            "boolean" => RustType::Bool,
            "array" => {
                let ty = if let Some(ref_) = ref_ {
                    RustType::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        RustType::Vec(Box::new(ty))
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
                    RustType::Custom(trim_reference(ref_).to_string())
                } else if let Some(item) = &schema.additional_properties {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        RustType::Object(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if let Some(item) = &schema.items {
                    if let Some(ty) = Self::map_item_type(item, true, parent_name, swagger) {
                        RustType::Object(Box::new(ty))
                    } else {
                        return None;
                    }
                } else if schema.properties.is_some() {
                    if let Some(name) = schema.name() {
                        RustType::Custom(name)
                    } else if let Some(parent_name) = &parent_name {
                        RustType::Custom(format!("{parent_name}InlineItem"))
                    } else {
                        RustType::Value
                    }
                } else {
                    RustType::Value
                };

                ty
            }
            "number" => {
                let ty = match schema.format.as_deref() {
                    Some("double") => RustType::F64,
                    Some("float") => RustType::F32,
                    _ => return None,
                };
                ty
            }
            _ => return None,
        };
        if !is_required {
            ty = RustType::Option(Box::new(ty));
        }
        trace!("mapped to {ty}");
        Some(ty)
    }
}
