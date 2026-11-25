//! Le bac à sable : un formulaire compilé côté serveur, l'API JSON et, quand
//! `orchestra.wasm` est publié à côté du site (GitHub Pages), la compilation dans le
//! navigateur pendant la frappe.

use orchestra::drc::{self, Error as DrcError, GeneratedFile};
use orchestra::render::{self, MAX_SOURCE};
use paquets::prelude::*;

use super::{examples, layout};

#[data]
pub struct SourceForm {
    pub source: String,
}

/// Le bac à sable, avec le premier exemple.
pub async fn page(_req: Request) -> Page {
    let example = &examples::ALL[0];
    render(
        example.source,
        Some(example.id),
        drc::compile_str(example.source),
    )
}

/// Le bac à sable avec l'exemple `/bac-a-sable/<id>`.
pub async fn example(req: Request) -> Page {
    let id = req.param("exemple").unwrap_or_default();
    match examples::ALL.iter().find(|e| e.id == id) {
        Some(example) => render(
            example.source,
            Some(example.id),
            drc::compile_str(example.source),
        ),
        // Les recettes du guide s'ouvrent aussi dans le bac à sable.
        None => match super::guide::RECIPES.iter().find(|r| r.id == id) {
            Some(recipe) => render(recipe.source, None, drc::compile_str(recipe.source)),
            None => super::missing(),
        },
    }
}

pub async fn submit(req: Request) -> Result<Page> {
    let form: SourceForm = req.form().await?;
    let source = render::normalize(&form.source);
    let result = render::compile(&source);
    let status = if result.is_ok() { 200 } else { 422 };
    Ok(render(&source, None, result).status(status))
}

pub async fn api(req: Request) -> Result<Response> {
    let body: SourceForm = req.json().await?;
    Ok(match render::compile(&body.source) {
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

/// Compile dans le navigateur si `orchestra.wasm` se charge (adresse dans `data-wasm`) :
/// à l'envoi du formulaire et pendant la frappe. Sinon, rien ne change : le formulaire
/// part au serveur.
const SCRIPT: &str = r#"<script>addEventListener("DOMContentLoaded",()=>{const f=document.getElementById("editor"),t=document.getElementById("source"),r=document.getElementById("results");if(!f||!t||!r||!window.WebAssembly)return;fetch(f.dataset.wasm).then(x=>{if(!x.ok)throw 0;return x.arrayBuffer()}).then(b=>WebAssembly.instantiate(b)).then(({instance:{exports:w}})=>{const e=new TextEncoder(),d=new TextDecoder();let last=r.innerHTML,timer;const run=()=>{const b=e.encode(t.value),p=w.alloc(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);w.render(p,b.length);const html=d.decode(new Uint8Array(w.memory.buffer,w.out_ptr(),w.out_len()));if(html!==last){r.innerHTML=html;last=html}};f.addEventListener("submit",x=>{x.preventDefault();run()});t.addEventListener("input",()=>{clearTimeout(timer);timer=setTimeout(run,250)});f.classList.add("live")}).catch(()=>{})})</script>"#;

fn render(
    source: &str,
    current: Option<&str>,
    result: std::result::Result<Vec<GeneratedFile>, DrcError>,
) -> Page {
    let chips =
        ul().class("chips")
            .attr("aria-label", "Exemples")
            .children(examples::ALL.iter().map(|example| {
                let link = a()
                    .href(format!("/bac-a-sable/{}", example.id))
                    .text(example.title);
                li().child(if current == Some(example.id) {
                    link.attr("aria-current", "true")
                } else {
                    link
                })
            }));
    let editor = form()
        .id("editor")
        .class("editor")
        .attr("data-wasm", "/orchestra.wasm")
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
    let results = div().id("results").html(render::results(source, &result));
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
                    .child(editor)
                    .child(p().class("muted live-hint").text(
                        "Compilation dans votre navigateur, pendant la frappe.",
                    )),
            )
            .child(
                div()
                    .attr("aria-live", "polite")
                    .child(h2().attr("style", "font-size:1.3rem").text("Fichiers générés"))
                    .child(results),
            ),
    )
    .head(SCRIPT)
}
