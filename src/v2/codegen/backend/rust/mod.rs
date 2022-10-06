mod backend;
mod types;

pub use backend::Codegen;
pub use types::Type;

use crate::{Case, Casing};

pub const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while",
];

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

pub fn fix_name_if_keyword(name: &mut String) {
    let is_keyword = is_keyword(name.as_str());
    if is_keyword {
        name.push('_');
    }
}

pub fn format_type_name(name: &str) -> String {
    let mut name = name.to_case(Case::UpperCamel);
    fix_name_if_keyword(&mut name);
    name
}

pub fn format_var_name(name: &str) -> String {
    let name = name.replace('-', "_");
    let name = name.replace('.', "_");
    let name = name.replace('/', "_");
    let mut name = name.to_case(Case::Snake);
    fix_name_if_keyword(&mut name);
    name
}

pub fn format_enum_value_name(name: &str) -> String {
    let name = name.replace('-', " ");
    let name = name.replace('.', " ");
    let name = name.replace('/', " ");
    let mut name = name.to_case(Case::UpperCamel);
    name = name.replace(' ', "");
    fix_name_if_keyword(&mut name);

    if name.is_empty() {
        "Empty".into()
    } else if name
        .chars()
        .next()
        .map(|c| c.is_numeric())
        .unwrap_or_default()
    {
        format!("Value{name}")
    } else {
        name
    }
}
