//! Le bac à sable : un formulaire compilé côté serveur, et l'API JSON.

use orchestra::drc::{self, Error as DrcError, GeneratedFile};
use paquets::prelude::*;

use super::highlight::escape;
use super::{examples, layout, outputs};

/// Taille maximale d'un fichier envoyé.
pub const MAX_SOURCE: usize = 64 * 1024;

#[data]
pub struct SourceForm {
    pub source: String,
}

pub async fn page(req: Request) -> Page {
    let id = req.query("exemple").map(|id| id.into_owned());
    let example = examples::find(id.as_deref());
    let result = drc::compile_str(example.source);
    render(example.source, Some(example.id), Some(result))
}

pub async fn submit(req: Request) -> Result<Page> {
    let form: SourceForm = req.form().await?;
    let source = normalize(&form.source);
    let result = compile(&source);
    let status = if result.is_ok() { 200 } else { 422 };
    Ok(render(&source, None, Some(result)).status(status))
}

pub async fn api(req: Request) -> Result<Response> {
    let body: SourceForm = req.json().await?;
    let source = normalize(&body.source);
    Ok(match compile(&source) {
        Ok(files) => res::json(json!({
            "files": files
                .iter()
                .map(|f| json!({ "name": f.name, "content": f.content }))
                .collect::<Vec<_>>()
        })),
        Err(e) => res::json(json!({
            "error": { "line": e.pos.line, "column": e.pos.col, "message": e.message }
        }))
        .status(422),
    })
}

/// Les retours chariot des formulaires (`\r\n`) deviennent des `\n`.
fn normalize(source: &str) -> String {
    source.replace("\r\n", "\n")
}

fn compile(source: &str) -> std::result::Result<Vec<GeneratedFile>, DrcError> {
    if source.len() > MAX_SOURCE {
        return Err(DrcError::at(
            drc::Pos { line: 1, col: 1 },
            format!("fichier trop long : {} octets au plus", MAX_SOURCE),
        ));
    }
    drc::compile_str(source)
}

const CHECK: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path class="ok-check" d="M5 12.5l4.5 4.5L19 7.5"/></svg>"#;
const CROSS: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path class="err-cross" d="M6 6l12 12M18 6L6 18"/></svg>"#;
const EMPTY: &str = r#"<svg viewBox="0 0 120 80" aria-hidden="true"><g class="staff"><path d="M4 20H116M4 32H116M4 44H116M4 56H116M4 68H116"/></g></svg>"#;

fn render(
    source: &str,
    current: Option<&str>,
    result: Option<std::result::Result<Vec<GeneratedFile>, DrcError>>,
) -> Page {
    let chips =
        ul().class("chips")
            .attr("aria-label", "Exemples")
            .children(examples::ALL.iter().map(|example| {
                let link = a()
                    .href(format!("/bac-a-sable?exemple={}", example.id))
                    .text(example.title);
                li().child(if current == Some(example.id) {
                    link.attr("aria-current", "true")
                } else {
                    link
                })
            }));
    let editor = form()
        .class("editor")
        .attr("method", "post")
        .attr("action", "/bac-a-sable")
        .child(label().attr("for", "source").text("Votre fichier .drc"))
        .child(
            textarea()
                .id("source")
                .attr("name", "source")
                .attr("spellcheck", "false")
                .attr("autocapitalize", "off")
                .attr("autocomplete", "off")
                .attr("autocorrect", "off")
                .attr("maxlength", MAX_SOURCE.to_string())
                .attr("required", "")
                .text(source.to_owned()),
        )
        .child(
            div()
                .class("actions")
                .child(
                    button()
                        .class("btn primary")
                        .attr("type", "submit")
                        .text("Compiler"),
                )
                .child(
                    a().class("btn")
                        .href("/langage")
                        .text("Aide : la spécification"),
                ),
        );
    let results = match result {
        Some(Ok(files)) if !files.is_empty() => div()
            .child(div().class("status").attr("role", "status").html(format!(
                "{CHECK}<span>{} fichier(s) générés</span>",
                files.len()
            )))
            .child(outputs(&files)),
        Some(Err(error)) => div()
            .child(
                div()
                    .class("status error")
                    .attr("role", "alert")
                    .html(format!(
                        "{CROSS}<span>Ligne {}, colonne {} : {}</span>",
                        error.pos.line,
                        error.pos.col,
                        escape(&error.message)
                    )),
            )
            .child(snippet(source, &error)),
        _ => div().child(div().class("placeholder").html(format!(
            "{EMPTY}<p>Les fichiers générés s'afficheront ici.</p>"
        ))),
    };
    layout(
        "Bac à sable",
        "Écrivez un fichier Rust‑Compose et obtenez instantanément le Dockerfile, le docker-compose.yml et les manifests Kubernetes.",
        "/bac-a-sable",
        div()
            .class("wrap sandbox")
            .child(
                div()
                    .child(span().class("eyebrow").text("Bac à sable"))
                    .child(h1().attr("style", "font-size:clamp(1.8rem,7vw,2.6rem)").text("Compilez en ligne"))
                    .child(p().class("muted").text(
                        "Choisissez un exemple ou écrivez le vôtre, puis compilez : le compilateur du site génère les fichiers.",
                    ))
                    .child(chips)
                    .child(editor),
            )
            .child(
                div()
                    .attr("aria-live", "polite")
                    .child(h2().attr("style", "font-size:1.3rem").text("Fichiers générés"))
                    .child(results),
            ),
    )
}

/// Les lignes autour de l'erreur, la ligne fautive surlignée et le caractère fautif marqué.
fn snippet(source: &str, error: &DrcError) -> Element {
    let line = error.pos.line.max(1);
    let lines: Vec<&str> = source.lines().collect();
    let first = line.saturating_sub(3).max(1);
    let last = (line + 2).min(lines.len()).max(first);
    let width = last.to_string().len();
    let mut html = String::new();
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
    pre().class("code snippet").child(code().html(html))
}
