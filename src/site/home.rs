//! La page d'accueil.

use orchestra::drc;
use paquets::prelude::*;

use super::{art, code_block, examples, layout, outputs};

pub async fn page(_req: Request) -> Page {
    layout(
        "ORCHESTRA · Rust‑Compose, un fichier pour Docker, Compose et Kubernetes",
        "Rust‑Compose est un langage déclaratif, modulaire et typé : un fichier .drc génère votre Dockerfile, votre docker-compose.yml et vos manifests Kubernetes.",
        "/",
        fragment([
            Node::from(hero()),
            Node::from(how()),
            Node::from(principles()),
            Node::from(demo()),
            Node::from(cli()),
        ]),
    )
}

fn hero() -> Element {
    section().class("hero").child(
        div()
            .class("wrap")
            .child(
                div()
                    .child(span().class("eyebrow").text("Rust‑Compose 1.0"))
                    .child(h1().html(
                        "Un fichier <span class=\"grad\">.drc</span>.<br/>Trois cibles.",
                    ))
                    .child(p().class("lead").text(
                        "Décrivez votre application une seule fois. ORCHESTRA compile la partition en Dockerfile, docker-compose.yml et manifests Kubernetes, vérifiés avant d'être écrits.",
                    ))
                    .child(
                        div()
                            .class("actions")
                            .child(a().class("btn primary").href("/bac-a-sable").text("Essayer en ligne"))
                            .child(a().class("btn").href("/langage").text("Lire la spécification")),
                    )
                    .child(ul().class("badges").children(
                        ["Déclaratif", "Modulaire", "Typé", "Écrit en Rust"]
                            .into_iter()
                            .map(|b| li().text(b)),
                    )),
            )
            .child(div().html(art::HERO)),
    )
}

fn how() -> Element {
    let step = |art: &'static str, title: &'static str, text: &'static str| {
        li().class("step reveal")
            .child(div().html(art))
            .child(div().child(h3().text(title)).child(p().html(text)))
    };
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("Comment ça marche"))
            .child(h2().class("section-title").text("De la partition à la scène"))
            .child(p().class("lead").text(
                "Un seul format lisible, influencé par TOML et YAML, que l'outil Rust transforme en fichiers standards.",
            ))
            .child(
                ol().class("steps")
                    .child(step(
                        art::STEP_WRITE,
                        "Écrire",
                        "Des variables, des fonctions et trois sections : <code>docker</code>, <code>compose</code> et <code>k8s</code>.",
                    ))
                    .child(step(
                        art::STEP_COMPILE,
                        "Compiler",
                        "<code>orchestra build app.drc</code> valide la syntaxe et les types, remplace les variables, exécute les fonctions.",
                    ))
                    .child(step(
                        art::STEP_DEPLOY,
                        "Déployer",
                        "<code>docker build</code>, <code>docker compose up</code> ou <code>kubectl apply -f</code> : les fichiers sont prêts.",
                    )),
            ),
    )
}

fn principles() -> Element {
    let card = |icon: &'static str, title: &'static str, text: &'static str| {
        article()
            .class("card reveal")
            .child(div().html(icon))
            .child(h3().text(title))
            .child(p().html(text))
    };
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("Principes"))
            .child(h2().class("section-title").text("Cinq règles, aucune surprise"))
            .child(
                div()
                    .class("cards")
                    .child(card(
                        art::ICON_DECLARATIVE,
                        "Déclaratif",
                        "Tout est une déclaration : on dit ce que l'on veut, pas comment le construire.",
                    ))
                    .child(card(
                        art::ICON_MODULAR,
                        "Modulaire",
                        "Des fichiers séparés réunis par <code>import</code>, des sections fusionnées.",
                    ))
                    .child(card(
                        art::ICON_FUNCTIONS,
                        "Variables et fonctions",
                        "<code>var</code>, <code>fn</code> et <code>format!</code> pour ne rien écrire deux fois.",
                    ))
                    .child(card(
                        art::ICON_TYPED,
                        "Typé",
                        "Les éléments obligatoires et les types sont vérifiés à la compilation, avec la ligne et la colonne.",
                    ))
                    .child(card(
                        art::ICON_READABLE,
                        "Lisible",
                        "Une syntaxe entre TOML et YAML, agréable à lire et simple à analyser en Rust.",
                    )),
            ),
    )
}

fn demo() -> Element {
    // L'exemple est compilé à chaque rendu : ce que montre la page est exactement
    // ce que produit le compilateur.
    let files = drc::compile_str(examples::HOME).unwrap_or_default();
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("En direct"))
            .child(h2().class("section-title").text("Ce que produit le compilateur"))
            .child(p().class("lead").text(
                "À gauche, le fichier .drc. À droite, les fichiers générés à l'instant par le compilateur du site.",
            ))
            .child(
                div()
                    .class("demo")
                    .child(
                        div()
                            .class("reveal")
                            .child(div().class("file-tag").html("<span class=\"dot\"></span>app.drc"))
                            .child(code_block("app.drc", examples::HOME)),
                    )
                    .child(div().class("reveal").child(outputs(&files))),
            )
            .child(
                div()
                    .class("actions")
                    .child(a().class("btn primary").href("/bac-a-sable").text("Modifier cet exemple")),
            ),
    )
}

fn cli() -> Element {
    let lines = [
        "$ orchestra build app.drc --out deploy",
        "✓ deploy/Dockerfile",
        "✓ deploy/docker-compose.yml",
        "✓ deploy/service.yaml",
        "$ kubectl apply -f deploy/service.yaml",
        "service/myapp created",
    ];
    let html: String = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            format!(
                "<span class=\"term-line\" style=\"--d:{:.1}s\">{}</span>",
                0.6 + i as f32 * 0.7,
                super::highlight::shell(line)
            )
        })
        .collect();
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("En ligne de commande"))
            .child(h2().class("section-title").text("Un binaire, trois commandes"))
            .child(p().class("lead").html(
                "<code>orchestra</code> lance ce site, <code>orchestra check</code> vérifie un fichier, <code>orchestra build</code> écrit les fichiers.",
            ))
            .child(
                div()
                    .class("terminal reveal")
                    .attr("role", "img")
                    .attr("aria-label", "Démonstration : orchestra build écrit trois fichiers, puis kubectl applique le service.")
                    .child(div().class("terminal-bar").html("<i></i><i></i><i></i>"))
                    .child(pre().class("code").child(code().html(html))),
            )
            .child(div().class("actions").child(a().class("btn").href("/langage#cli").text("Installer l'outil"))),
    )
}
