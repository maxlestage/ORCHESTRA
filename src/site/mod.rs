//! Le site : des pages rendues par le serveur avec Paquets. Les animations sont en CSS
//! et en SVG ; le bac à sable compile côté serveur, ou dans le navigateur avec
//! `orchestra.wasm` (seule façon de compiler sur GitHub Pages, voir [`export`]).

mod art;
mod examples;
pub mod export;
mod home;
mod language;
mod sandbox;

use orchestra::drc::GeneratedFile;
use orchestra::render::{self, highlight};
use paquets::prelude::*;

/// La feuille de style, écrite dans chaque page (et autorisée par sa CSP).
const CSS: &str = include_str!("style.css");

/// Le site complet.
pub fn app() -> App {
    let mut app = App::new();
    app.get("/", home::page);
    app.get("/langage", language::page);
    app.get("/bac-a-sable", sandbox::page);
    app.get("/bac-a-sable/:exemple", sandbox::example);
    app.post("/bac-a-sable", sandbox::submit);
    app.post("/api/compile", sandbox::api);
    app.health("/health");
    app.fallback(not_found);
    app
}

/// Les pages du menu : (chemin, titre).
const NAV: &[(&str, &str)] = &[
    ("/#aide-memoire", "Aide‑mémoire"),
    ("/langage", "Langage"),
    ("/bac-a-sable", "Bac à sable"),
    ("/langage#cli", "CLI"),
];

/// La page commune : en-tête, contenu, pied de page.
fn layout(title: &str, description: &str, current: &str, content: impl Into<Node>) -> Page {
    let full_title = if current == "/" {
        title.to_owned()
    } else {
        format!("{title} · ORCHESTRA")
    };
    Page::new(full_title)
        .lang("fr")
        .description(description)
        .plain()
        .style(CSS)
        .head(r##"<meta name="theme-color" content="#0b0d17"/><link rel="icon" href="/favicon.svg" type="image/svg+xml"/>"##)
        .body(a().class("skip").href("#contenu").text("Aller au contenu"))
        .body(header(current))
        .body(html::main().id("contenu").child(content))
        .body(footer())
}

fn header(current: &str) -> Element {
    let links = NAV.iter().map(|(href, label)| {
        let link = a().href(*href).text(*label);
        if *href == current {
            link.attr("aria-current", "page")
        } else {
            link
        }
    });
    html::header().class("site-header").child(
        div()
            .class("wrap")
            .child(
                a().class("brand")
                    .href("/")
                    .attr("aria-label", "ORCHESTRA, accueil")
                    .html(format!("{}<span>ORCHESTRA</span>", art::LOGO)),
            )
            .child(
                nav()
                    .class("site-nav")
                    .attr("aria-label", "Navigation principale")
                    .children(links),
            ),
    )
}

fn footer() -> Element {
    html::footer().class("site-footer").child(
        div()
            .class("wrap")
            .child(p().html(
                "Rust‑Compose · écrit en Rust avec <a href=\"https://github.com/maxlestage/Paquets\">Paquets</a>",
            ))
            .child(p().html(
                "<a href=\"/langage\">Spécification</a> · <a href=\"/bac-a-sable\">Bac à sable</a> · <a href=\"https://github.com/maxlestage/ORCHESTRA\">Code source</a>",
            )),
    )
}

/// Un bloc de code coloré.
fn code_block(name: &str, source: &str) -> Element {
    div().class("block").html(render::code_block(name, source))
}

/// Un bloc de commandes de terminal.
fn shell_block(source: &str) -> Element {
    pre()
        .class("code")
        .child(code().html(highlight::shell(source)))
}

/// Les fichiers générés, dépliables ; le premier est ouvert.
fn outputs(files: &[GeneratedFile]) -> Element {
    div().html(render::outputs(files))
}

async fn not_found(_req: Request) -> Page {
    missing()
}

/// La page 404.
fn missing() -> Page {
    layout(
        "Fausse note",
        "Cette page n'existe pas.",
        "",
        section().class("lost").child(
            div()
                .class("wrap")
                .child(div().html(art::LOST))
                .child(h1().text("Fausse note"))
                .child(
                    p().class("lead")
                        .attr("style", "margin-inline:auto")
                        .text("Cette page ne fait pas partie de la partition."),
                )
                .child(
                    div()
                        .class("actions")
                        .child(
                            a().class("btn primary")
                                .href("/")
                                .text("Retour à l'accueil"),
                        )
                        .child(
                            a().class("btn")
                                .href("/langage")
                                .text("Lire la spécification"),
                        ),
                ),
        ),
    )
    .status(404)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pages_render_and_the_compiler_answers() {
        let client = app().test().await;
        for path in [
            "/",
            "/langage",
            "/bac-a-sable",
            "/bac-a-sable?exemple=k8s",
            "/health",
        ] {
            assert_eq!(client.get(path).await.status(), 200, "{path}");
        }
        assert_eq!(client.get("/introuvable").await.status(), 404);
        assert_eq!(client.get("/bac-a-sable/inconnu").await.status(), 404);

        // L'exemple de l'accueil est compilé à chaque rendu.
        let home = client.get("/").await.text();
        assert!(home.contains("docker-compose.yml"));
        assert!(home.contains("nginx:1.29"));
        assert!(home.contains("deployment.yaml"));

        // Le formulaire du bac à sable.
        let res = client
            .post("/bac-a-sable")
            .form(&json!({ "source": include_str!("../../examples/complet.drc") }))
            .await;
        assert_eq!(res.status(), 200);
        assert!(res.text().contains("5 fichier(s) générés"));

        let res = client
            .post("/bac-a-sable")
            .form(&json!({ "source": "docker {\r\n  workdir = \"/app\"\r\n}" }))
            .await;
        assert_eq!(res.status(), 422);
        assert!(res.text().contains("est obligatoire"));

        // L'API JSON.
        let res = client
            .post("/api/compile")
            .json(
                &json!({ "source": "compose { services = { web = { image = \"nginx:1.29\" } } }" }),
            )
            .await;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.json_path("files.0.name"),
            Some(json!("docker-compose.yml"))
        );
        let res = client
            .post("/api/compile")
            .json(&json!({ "source": "var X = Y" }))
            .await;
        assert_eq!(res.status(), 422);
        assert_eq!(res.json_path("error.column"), Some(json!(9)));
    }

    #[test]
    fn every_example_but_the_broken_one_compiles() {
        for example in examples::ALL {
            let result = orchestra::drc::compile_str(example.source);
            assert_eq!(result.is_err(), example.id == "erreur", "{}", example.id);
        }
        assert!(orchestra::drc::compile_str(examples::HOME).is_ok());
    }
}
