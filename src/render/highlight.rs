//! Coloration syntaxique côté serveur : `.drc`, YAML, Dockerfile et shell.
//! Le HTML produit est échappé ; les couleurs viennent des classes `t-*` du CSS.

/// Échappe le texte pour l'insérer dans du HTML.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"t-");
    out.push_str(class);
    out.push_str("\">");
    out.push_str(&escape(text));
    out.push_str("</span>");
}

/// La coloration adaptée à un nom de fichier.
pub fn for_file(name: &str, source: &str) -> String {
    if name == "Dockerfile" {
        dockerfile(source)
    } else if name.ends_with(".yml") || name.ends_with(".yaml") {
        yaml(source)
    } else if name.ends_with(".drc") {
        drc(source)
    } else {
        escape(source)
    }
}

const KEYWORDS: &[&str] = &["app", "var", "fn", "return", "import"];
const SECTIONS: &[&str] = &["docker", "compose", "k8s"];

/// Un fichier Rust-Compose.
pub fn drc(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len() * 2);
    let mut i = 0;
    let collect = |from: usize, to: usize| chars[from..to].iter().collect::<String>();
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c == '#' || (c == '/' && chars.get(i + 1) == Some(&'/')) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            span(&mut out, "comment", &collect(start, i));
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            span(&mut out, "comment", &collect(start, i));
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' && chars[i] != '\n' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(chars.len());
            span(&mut out, "string", &collect(start, i));
        } else if c.is_ascii_digit()
            || (c == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_')
            {
                i += 1;
            }
            span(&mut out, "number", &collect(start, i));
        } else if c.is_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word = collect(start, i);
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let next = chars.get(j).copied();
            let class = if KEYWORDS.contains(&word.as_str()) {
                "keyword"
            } else if word == "true" || word == "false" {
                "number"
            } else if SECTIONS.contains(&word.as_str()) && next == Some('{') {
                "section"
            } else if next == Some('(') || (next == Some('!') && chars.get(j + 1) == Some(&'(')) {
                "call"
            } else if word
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
            {
                "const"
            } else if next == Some('=') {
                "key"
            } else {
                "ident"
            };
            span(&mut out, class, &word);
        } else if "{}[]()".contains(c) {
            i += 1;
            span(&mut out, "punct", &c.to_string());
        } else {
            i += 1;
            out.push_str(&escape(&c.to_string()));
        }
    }
    out
}

/// Un document YAML (Compose, Kubernetes).
pub fn yaml(source: &str) -> String {
    let mut out = String::with_capacity(source.len() * 2);
    for line in source.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        let trimmed = body.trim_start();
        let indent = &body[..body.len() - trimmed.len()];
        out.push_str(indent);
        if trimmed.starts_with('#') || trimmed == "---" {
            span(&mut out, "comment", trimmed);
        } else {
            let (dash, rest) = match trimmed.strip_prefix("- ") {
                Some(rest) => ("- ", rest),
                None => ("", trimmed),
            };
            if !dash.is_empty() {
                span(&mut out, "punct", dash);
            }
            match key_split(rest) {
                Some((key, value)) => {
                    span(&mut out, "key", key);
                    out.push(':');
                    yaml_value(&mut out, value);
                }
                None => yaml_value(&mut out, rest),
            }
        }
        out.push_str(newline);
    }
    out
}

/// `clé: valeur` → (`clé`, ` valeur`), en ignorant les `:` d'une valeur entre guillemets.
fn key_split(text: &str) -> Option<(&str, &str)> {
    if let Some(rest) = text.strip_prefix('"') {
        let end = rest.find('"')? + 2;
        return text[end..]
            .starts_with(':')
            .then(|| (&text[..end], &text[end + 1..]));
    }
    let at = text
        .find(": ")
        .or_else(|| text.ends_with(':').then(|| text.len() - 1))?;
    Some((&text[..at], &text[at + 1..]))
}

fn yaml_value(out: &mut String, value: &str) {
    let trimmed = value.trim_start();
    out.push_str(&value[..value.len() - trimmed.len()]);
    if trimmed.is_empty() {
        return;
    }
    let class = if trimmed.starts_with('"') {
        "string"
    } else if trimmed.parse::<f64>().is_ok() || matches!(trimmed, "true" | "false" | "null") {
        "number"
    } else if matches!(trimmed, "{}" | "[]" | "|" | "|-" | "|+") {
        "punct"
    } else {
        "value"
    };
    span(out, class, trimmed);
}

/// Un Dockerfile.
pub fn dockerfile(source: &str) -> String {
    let mut out = String::with_capacity(source.len() * 2);
    for line in source.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if body.starts_with('#') {
            span(&mut out, "comment", body);
        } else if let Some((word, rest)) = body.split_once(' ') {
            if word.chars().all(|c| c.is_ascii_uppercase()) {
                span(&mut out, "keyword", word);
                out.push(' ');
                for (i, part) in rest.split(' ').enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    if part.starts_with("--") || part == "AS" {
                        span(&mut out, "section", part);
                    } else if part.starts_with('"') || part.starts_with('[') {
                        span(&mut out, "string", part);
                    } else {
                        out.push_str(&escape(part));
                    }
                }
            } else {
                out.push_str(&escape(body));
            }
        } else {
            out.push_str(&escape(body));
        }
        out.push_str(newline);
    }
    out
}

/// Des commandes de terminal : `$ commande`, et les sorties en atténué.
pub fn shell(source: &str) -> String {
    let mut out = String::new();
    for line in source.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if let Some(command) = body.strip_prefix("$ ") {
            span(&mut out, "prompt", "$ ");
            let (cmd, rest) = command.split_once(' ').unwrap_or((command, ""));
            span(&mut out, "call", cmd);
            if !rest.is_empty() {
                out.push(' ');
                out.push_str(&escape(rest));
            }
        } else if body.starts_with('#') {
            span(&mut out, "comment", body);
        } else {
            span(&mut out, "value", body);
        }
        out.push_str(newline);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drc_tokens() {
        let html = drc("var X = \"a<b\" # note\nk8s { n = f(1) }");
        assert!(html.contains("<span class=\"t-keyword\">var</span>"));
        assert!(html.contains("<span class=\"t-const\">X</span>"));
        assert!(html.contains("<span class=\"t-string\">&quot;a&lt;b&quot;</span>"));
        assert!(html.contains("<span class=\"t-comment\"># note</span>"));
        assert!(html.contains("<span class=\"t-section\">k8s</span>"));
        assert!(html.contains("<span class=\"t-key\">n</span>"));
        assert!(html.contains("<span class=\"t-call\">f</span>"));
    }

    #[test]
    fn yaml_and_docker_lines() {
        let html = yaml("ports:\n  - \"8080:80\"\n  - port: 80\n");
        assert!(html.contains("<span class=\"t-key\">ports</span>:"));
        assert!(html.contains("<span class=\"t-string\">&quot;8080:80&quot;</span>"));
        assert!(
            html.contains("<span class=\"t-key\">port</span>: <span class=\"t-number\">80</span>")
        );
        let html = dockerfile("FROM rust AS build\n");
        assert!(html.starts_with(
            "<span class=\"t-keyword\">FROM</span> rust <span class=\"t-section\">AS</span>"
        ));
    }
}
