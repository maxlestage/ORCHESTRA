//! Écrit une [`Value`] en YAML lisible, dans l'ordre des clés du fichier source.

use super::Value;

/// Le document YAML de `value` (un objet, en général).
pub fn to_yaml(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(entries) if !entries.is_empty() => map(&mut out, entries, 0),
        Value::List(items) if !items.is_empty() => list(&mut out, items, 0),
        other => {
            out.push_str(&inline(other));
            out.push('\n');
        }
    }
    out
}

fn pad(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

fn map(out: &mut String, entries: &[(String, Value)], indent: usize) {
    for (key, value) in entries {
        pad(out, indent);
        out.push_str(&string(key));
        out.push(':');
        match value {
            Value::Object(inner) if !inner.is_empty() => {
                out.push('\n');
                map(out, inner, indent + 2);
            }
            Value::List(items) if !items.is_empty() => {
                out.push('\n');
                list(out, items, indent + 2);
            }
            Value::Str(s) if block_friendly(s) => {
                let chomp = if s.ends_with("\n\n") {
                    "+"
                } else if s.ends_with('\n') {
                    ""
                } else {
                    "-"
                };
                out.push_str(" |");
                out.push_str(chomp);
                out.push('\n');
                let body = s.strip_suffix('\n').unwrap_or(s);
                for line in body.split('\n') {
                    if !line.is_empty() {
                        pad(out, indent + 2);
                        out.push_str(line);
                    }
                    out.push('\n');
                }
            }
            other => {
                out.push(' ');
                out.push_str(&inline(other));
                out.push('\n');
            }
        }
    }
}

fn list(out: &mut String, items: &[Value], indent: usize) {
    for item in items {
        match item {
            Value::Object(inner) if !inner.is_empty() => {
                // `- clé: valeur`, puis les autres clés alignées sous la première.
                let mut nested = String::new();
                map(&mut nested, inner, indent + 2);
                pad(out, indent);
                out.push_str("- ");
                out.push_str(&nested[indent + 2..]);
            }
            Value::List(inner) if !inner.is_empty() => {
                let mut nested = String::new();
                list(&mut nested, inner, indent + 2);
                pad(out, indent);
                out.push_str("- ");
                out.push_str(&nested[indent + 2..]);
            }
            other => {
                pad(out, indent);
                out.push_str("- ");
                out.push_str(&inline(other));
                out.push('\n');
            }
        }
    }
}

/// Une valeur sur une ligne : scalaire, `{}` ou `[]`.
fn inline(value: &Value) -> String {
    match value {
        Value::Str(s) => string(s),
        Value::Int(n) => n.to_string(),
        Value::Float(n) if n.is_finite() && n.fract() == 0.0 => format!("{n:.1}"),
        Value::Float(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::List(items) if items.is_empty() => "[]".into(),
        Value::Object(entries) if entries.is_empty() => "{}".into(),
        // Les collections non vides sont écrites par `map` et `list`.
        Value::List(_) | Value::Object(_) => unreachable!("collection non vide en ligne"),
    }
}

/// Une chaîne multi-ligne s'écrit en bloc `|` si ses lignes s'y prêtent.
fn block_friendly(s: &str) -> bool {
    s.contains('\n')
        && !s.starts_with([' ', '\t', '\n'])
        && !s.chars().any(|c| c.is_control() && c != '\n')
        && s.lines().all(|l| !l.ends_with([' ', '\t']))
}

/// Une chaîne, entre guillemets seulement quand YAML pourrait la lire autrement.
pub fn string(s: &str) -> String {
    if plain_safe(s) {
        s.to_owned()
    } else {
        quoted(s)
    }
}

fn plain_safe(s: &str) -> bool {
    let Some(first) = s.chars().next() else {
        return false;
    };
    const RESERVED: &[&str] = &[
        "true", "false", "yes", "no", "on", "off", "y", "n", "null", "~",
    ];
    if RESERVED.contains(&s.to_ascii_lowercase().as_str()) {
        return false;
    }
    // Les nombres, et ce que YAML 1.1 lit comme nombre (`8080:80`, `1.0`, `.5`, `+1`).
    let numeric_start = first.is_ascii_digit()
        || (matches!(first, '-' | '+' | '.')
            && s[1..].starts_with(|c: char| c.is_ascii_digit() || c == '.'));
    if numeric_start || s == ".inf" || s == ".nan" {
        return false;
    }
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) || first.is_whitespace() {
        return false;
    }
    if s.ends_with([' ', ':']) || s.contains(": ") || s.contains(" #") {
        return false;
    }
    s.chars()
        .all(|c| !c.is_control() && !matches!(c, '[' | ']' | '{' | '}' | ',' | '"' | '\''))
}

fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(entries: &[(&str, Value)]) -> Value {
        Value::Object(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn quoting() {
        assert_eq!(string("myapp"), "myapp");
        assert_eq!(string("postgres:15-alpine"), "postgres:15-alpine");
        assert_eq!(string("./data:/data"), "./data:/data");
        assert_eq!(string("8080:80"), "\"8080:80\"");
        assert_eq!(string("true"), "\"true\"");
        assert_eq!(string("No"), "\"No\"");
        assert_eq!(string(""), "\"\"");
        assert_eq!(string("- x"), "\"- x\"");
        assert_eq!(string("a: b"), "\"a: b\"");
        assert_eq!(string("1.0.0"), "\"1.0.0\"");
    }

    #[test]
    fn nested_structures() {
        let value = obj(&[
            ("name", Value::Str("web".into())),
            (
                "ports",
                Value::List(vec![obj(&[
                    ("port", Value::Int(80)),
                    ("protocol", Value::Str("TCP".into())),
                ])]),
            ),
            ("emptyDir", obj(&[])),
            ("conf", Value::Str("a=1\nb=2\n".into())),
        ]);
        assert_eq!(
            to_yaml(&value),
            "name: web\nports:\n  - port: 80\n    protocol: TCP\nemptyDir: {}\nconf: |\n  a=1\n  b=2\n"
        );
    }
}
