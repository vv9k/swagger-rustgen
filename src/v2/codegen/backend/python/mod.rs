mod backend;
mod types;

pub use backend::Codegen;
pub use types::Type;

use crate::{Case, Casing};

pub const KEYWORDS: &[&str] = &[
    "False", "await", "else", "import", "pass", "None", "break", "except", "in", "raise", "True",
    "class", "finally", "is", "return", "and", "continue", "for", "lambda", "try", "as", "def",
    "from", "nonlocal", "while", "assert", "del", "global", "not", "with", "async", "elif", "if",
    "or", "yield",
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
    let mut name = name.to_case(Case::Upper);
    name = name.replace(' ', "");
    fix_name_if_keyword(&mut name);

    if name.is_empty() {
        "EMPTY".into()
    } else if name
        .chars()
        .next()
        .map(|c| c.is_numeric())
        .unwrap_or_default()
    {
        format!("VALUE{name}")
    } else {
        name
    }
}
