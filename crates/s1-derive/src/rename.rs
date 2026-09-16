use syn::{Error, Result};

pub fn apply(ident: &str, rule: &str, span: proc_macro2::Span) -> Result<String> {
    match rule {
        "snake_case" => Ok(to_snake_case(ident)),
        "lowercase" => Ok(ident.to_lowercase()),
        "UPPERCASE" => Ok(ident.to_uppercase()),
        "PascalCase" => Ok(ident.to_string()),
        "camelCase" => Ok(to_camel_case(ident)),
        "SCREAMING_SNAKE_CASE" => Ok(to_snake_case(ident).to_uppercase()),
        "kebab-case" => Ok(to_snake_case(ident).replace('_', "-")),
        "none" | "identity" => Ok(ident.to_string()),
        other => Err(Error::new(
            span,
            format!("unsupported rename_all = {other:?}; try \"snake_case\""),
        )),
    }
}

fn to_snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if i > 0 && (prev_lower || next_lower) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn to_camel_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}
