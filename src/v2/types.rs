use std::fmt;

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
    Bool,
    Vec(Box<RustType>),
    Object(Box<RustType>),
    Option(Box<RustType>),
    Custom(String),
    JsonValue,
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
            Bool => write!(f, "bool"),
            Vec(ty) => write!(f, "Vec<{ty}>"),
            Object(ty) => write!(f, "HashMap<String, {ty}>"),
            Option(ty) => write!(f, "Option<{ty}>"),
            Custom(ty) => write!(f, "{ty}"),
            JsonValue => write!(f, "serde_json::Value"),
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
