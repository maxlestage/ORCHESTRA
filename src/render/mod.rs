//! Le HTML des résultats d'une compilation : fichiers générés colorés, ou erreur
//! située dans le fichier. Partagé par le serveur et par la version WebAssembly
//! (le bac à sable de GitHub Pages compile dans le navigateur), pour un rendu identique.

pub mod highlight;

use crate::drc::{self, Error, GeneratedFile, Pos};
use highlight::escape;

/// Taille maximale d'un fichier compilé par le site.
pub const MAX_SOURCE: usize = 64 * 1024;

/// Compile un texte venu d'un formulaire : `\r\n` devient `\n`, et la taille est limitée.
pub fn compile(source: &str) -> Result<Vec<GeneratedFile>, Error> {
    if source.len() > MAX_SOURCE {
        return Err(Error::at(
            Pos { line: 1, col: 1 },
            format!("fichier trop long : {MAX_SOURCE} octets au plus"),
        ));
    }
    drc::compile_str(&normalize(source))
}

/// Les retours chariot des formulaires (`\r\n`) deviennent des `\n`.
pub fn normalize(source: &str) -> String {
    source.replace("\r\n", "\n")
}

const CHECK: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path class="ok-check" d="M5 12.5l4.5 4.5L19 7.5"/></svg>"#;
const CROSS: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path class="err-cross" d="M6 6l12 12M18 6L6 18"/></svg>"#;
const EMPTY: &str = r#"<svg viewBox="0 0 120 80" aria-hidden="true"><g class="staff"><path d="M4 20H116M4 32H116M4 44H116M4 56H116M4 68H116"/></g></svg>"#;

/// Un bloc de code coloré selon le nom du fichier.
pub fn code_block(name: &str, source: &str) -> String {
    format!(
        "<pre class=\"code\"><code>{}</code></pre>",
        highlight::for_file(name, source)
    )
}

/// La pastille de couleur d'un fichier.
pub fn dot_class(name: &str) -> &'static str {
    if name == "Dockerfile" {
        "dot docker"
    } else if name.starts_with("docker-compose") {
        "dot compose"
    } else if name.ends_with(".drc") {
        "dot"
    } else {
        "dot k8s"
    }
}

/// Les fichiers générés, dépliables ; le premier est ouvert.
pub fn outputs(files: &[GeneratedFile]) -> String {
    let mut html = String::from("<div class=\"outputs\">");
    for (i, file) in files.iter().enumerate() {
        html.push_str(&format!(
            "<details class=\"file\"{}><summary><span class=\"{}\"></span><span>{}</span><span class=\"lines\">{} lignes</span></summary>{}</details>",
            if i == 0 { " open" } else { "" },
            dot_class(&file.name),
            escape(&file.name),
            file.content.lines().count(),
            code_block(&file.name, &file.content),
        ));
    }
    html.push_str("</div>");
    html
}

/// Le panneau de résultats du bac à sable.
pub fn results(source: &str, result: &Result<Vec<GeneratedFile>, Error>) -> String {
    match result {
        Ok(files) if !files.is_empty() => format!(
            "<div class=\"status\" role=\"status\">{CHECK}<span>{} fichier(s) générés</span></div>{}",
            files.len(),
            outputs(files)
        ),
        Ok(_) => format!(
            "<div class=\"placeholder\">{EMPTY}<p>Les fichiers générés s'afficheront ici.</p></div>"
        ),
        Err(error) => format!(
            "<div class=\"status error\" role=\"alert\">{CROSS}<span>Ligne {}, colonne {} : {}</span></div>{}",
            error.pos.line,
            error.pos.col,
            escape(&error.message),
            snippet(&normalize(source), error)
        ),
    }
}

/// Les lignes autour de l'erreur, la ligne fautive surlignée et le caractère fautif marqué.
pub fn snippet(source: &str, error: &Error) -> String {
    let line = error.pos.line.max(1);
    let lines: Vec<&str> = source.lines().collect();
    let first = line.saturating_sub(3).max(1);
    let last = (line + 2).min(lines.len()).max(first);
    let width = last.to_string().len();
    let mut html = String::from("<pre class=\"code snippet\"><code>");
    for n in first..=last {
        let text = lines.get(n - 1).copied().unwrap_or("");
        if n == line {
            let chars: Vec<char> = text.chars().collect();
            let at = error.pos.col.saturating_sub(1).min(chars.len());
            let before: String = chars[..at].iter().collect();
            let marked: String = chars.get(at).map_or(" ".to_owned(), char::to_string);
            let after: String = chars
                .get(at + 1..)
                .map_or(String::new(), |c| c.iter().collect());
            html.push_str(&format!(
                "<span class=\"hl\">{n:>width$} │ {}<mark>{}</mark>{}</span>",
                escape(&before),
                escape(&marked),
                escape(&after)
            ));
        } else {
            html.push_str(&format!("{n:>width$} │ {}\n", escape(text)));
        }
    }
    html.push_str("</code></pre>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_for_files_and_errors() {
        let ok = results("x", &compile("docker { base = \"alpine:3.22\" }"));
        assert!(ok.contains("1 fichier(s) générés"));
        assert!(ok.contains("<details class=\"file\" open>"));
        let source = "docker {\r\n  base = <\r\n}";
        let err = results(source, &compile(source));
        assert!(err.contains("Ligne 2, colonne 10"));
        assert!(err.contains("<mark>&lt;</mark>"));
        let long = "#".repeat(MAX_SOURCE + 1);
        assert!(compile(&long).unwrap_err().message.contains("trop long"));
    }
}
