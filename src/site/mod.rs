//! Le site : des pages rendues par le serveur avec Paquets. Les animations sont en CSS
//! et en SVG ; le bac à sable compile côté serveur, ou dans le navigateur avec
//! `orchestra.wasm` (seule façon de compiler sur GitHub Pages, voir [`export`]).

mod art;
mod examples;
pub mod export;
mod guide;
mod home;
mod language;
mod sandbox;

use orchestra::drc::GeneratedFile;
use orchestra::render::{self, highlight};
use paquets::prelude::*;

/// La feuille de style, écrite dans chaque page (et autorisée par sa CSP).
const CSS: &str = include_str!("style.css");

/// L'adresse publique du site. Les messageries (WhatsApp, Signal, Telegram, iMessage,
/// Slack…) ne lisent l'image d'aperçu qu'à une adresse complète.
const SITE: &str = "https://maxlestage.github.io/ORCHESTRA";

/// La couleur du fond et de la barre du navigateur (`--bg`).
const THEME: &str = "#0b0d17";

/// Le site complet.
pub fn app() -> App {
    let mut app = App::new();
    app.get("/", home::page);
    app.get("/guide", guide::page);
    app.get("/langage", language::page);
    app.get("/bac-a-sable", sandbox::page);
    app.get("/bac-a-sable/:exemple", sandbox::example);
    app.post("/bac-a-sable", sandbox::submit);
    app.post("/api/compile", sandbox::api);
    app.health("/health");
    app.fallback(not_found);
    app.pwa(pwa());
    app
}

/// L'application installable : le manifeste (`/manifest.webmanifest`) et le service
/// worker (`/sw.js`) de Paquets ; les pages déjà vues restent lisibles hors ligne.
/// Les icônes sont dessinées par `ci/images.mjs`.
fn pwa() -> Pwa {
    Pwa::new("ORCHESTRA")
        .short_name("ORCHESTRA")
        .description(
            "Rust‑Compose : un fichier .drc compilé en Dockerfile, docker-compose.yml et manifests Kubernetes.",
        )
        .id("/")
        .start_url("/")
        .scope("/")
        .lang("fr")
        .theme_color(THEME)
        .background_color(THEME)
        .categories(["developer", "productivity", "utilities"])
        .icon("/icon-192.png", "192x192", "image/png")
        .icon("/icon-512.png", "512x512", "image/png")
        .maskable_icon("/icon-maskable-512.png", "512x512", "image/png")
}

/// L'aperçu d'un lien partagé : l'image (1200 × 630, dessinée par `ci/images.mjs`) et
/// le nom du site ; le titre et la description sont ceux de la page.
fn share() -> Seo {
    Seo::new()
        .image(format!("{SITE}/partage.jpg"))
        .site_name("ORCHESTRA")
        .locale("fr_FR")
        .meta_property("og:image:type", "image/jpeg")
        .meta_property("og:image:width", "1200")
        .meta_property("og:image:height", "630")
        .meta_property(
            "og:image:alt",
            "ORCHESTRA : un fichier .drc, et Docker, Compose et Kubernetes suivent.",
        )
}

/// Les pages du menu : (chemin, titre).
const NAV: &[(&str, &str)] = &[
    ("/#aide-memoire", "Aide‑mémoire"),
    ("/guide", "Guide"),
    ("/langage", "Langage"),
    ("/bac-a-sable", "Bac à sable"),
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
        .seo(share())
        .style(CSS)
        // La couleur du thème et le manifeste viennent de `pwa()`.
        .head(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml"/><link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png"/>"#)
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
            "/manifest.webmanifest",
            "/sw.js",
            "/partage.jpg",
            "/icon-maskable-512.png",
        ] {
            assert_eq!(client.get(path).await.status(), 200, "{path}");
        }
        assert_eq!(client.get("/introuvable").await.status(), 404);
        assert_eq!(client.get("/bac-a-sable/inconnu").await.status(), 404);

        // L'exemple de l'accueil est compilé à chaque rendu.
        let home = client.get("/").await.text();
        assert!(home.contains("docker-compose.yml"));
        assert!(home.contains("nginx:1.30"));
        assert!(home.contains("deployment.yaml"));

        // L'aperçu dans les messageries et l'application installable.
        assert!(home.contains(
            r#"<meta property="og:image" content="https://maxlestage.github.io/ORCHESTRA/partage.jpg"/>"#
        ));
        assert!(home.contains(r#"<meta name="twitter:card" content="summary_large_image"/>"#));
        assert!(home.contains(r#"<link rel="manifest" href="/manifest.webmanifest"/>"#));
        assert_eq!(home.matches(r#"name="theme-color""#).count(), 1);
        let manifest = client.get("/manifest.webmanifest").await;
        assert_eq!(manifest.json_path("start_url"), Some(json!("/")));
        assert_eq!(
            manifest.json_path("icons.2.purpose"),
            Some(json!("maskable"))
        );

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
                &json!({ "source": "compose { services = { web = { image = \"nginx:1.30\" } } }" }),
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
