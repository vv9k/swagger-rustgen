use convert_case::{Case, Casing};

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while",
];

pub fn is_keyword(word: &str) -> bool {
    RUST_KEYWORDS.contains(&word)
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
    let mut name = name.to_case(Case::Snake);
    fix_name_if_keyword(&mut name);
    name
}
