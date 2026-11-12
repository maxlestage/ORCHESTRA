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
            // L'aide-mémoire tout de suite : ce que l'on vient chercher.
            Node::from(cheats()),
            Node::from(demo()),
            Node::from(web()),
            Node::from(how()),
            Node::from(principles()),
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
                        "Une ligne.<br/><span class=\"grad\">Trois cibles.</span>",
                    ))
                    .child(p().class("lead").text(
                        "Un nom et une image : c'est tout ce qu'il faut écrire. ORCHESTRA en déduit le docker-compose.yml, les manifests Kubernetes et, si besoin, le Dockerfile.",
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
                "Une application s'écrit en une ligne ; le port, le Deployment, le Service et le service Compose en sont déduits.",
            ))
            .child(
                ol().class("steps")
                    .child(step(
                        art::STEP_WRITE,
                        "Écrire",
                        "<code>web { image = \"nginx:1.29\" }</code>. Le reste est déduit.",
                    ))
                    .child(step(
                        art::STEP_COMPILE,
                        "Compiler",
                        "<code>orchestra build app.drc</code> vérifie tout et propose la correction d'une faute de frappe.",
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
                        "Sans répétition",
                        "<code>var</code>, <code>fn tag(v) = IMAGE + \":\" + v</code> : chaque valeur s'écrit une seule fois.",
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
    live(
        "En direct",
        "Ce que produit le compilateur",
        "Une ligne à gauche ; à droite, les fichiers générés à l'instant par le compilateur du site.",
        examples::HOME,
        "/bac-a-sable",
    )
}

fn web() -> Element {
    live(
        "Un site web complet",
        "Rust, nginx, une base, un cache",
        "Une ligne par technologie. Le compilateur relie tout : nginx renvoie vers l'application, qui reçoit DATABASE_URL et REDIS_URL ; la base garde ses données, son mot de passe reste secret.",
        examples::WEB,
        "/bac-a-sable/web",
    )
}

/// Un exemple et les fichiers que le compilateur en tire, à chaque affichage : ce que
/// montre la page est exactement ce que produit le compilateur.
fn live(
    eyebrow: &'static str,
    title: &'static str,
    lead: &'static str,
    source: &'static str,
    edit: &'static str,
) -> Element {
    let files = drc::compile_str(source).unwrap_or_default();
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text(eyebrow))
            .child(h2().class("section-title").text(title))
            .child(p().class("lead").text(lead))
            .child(
                div()
                    .class("demo")
                    .child(
                        div()
                            .class("reveal")
                            .child(
                                div()
                                    .class("file-tag")
                                    .html("<span class=\"dot\"></span>app.drc"),
                            )
                            .child(code_block("app.drc", source)),
                    )
                    .child(div().class("reveal").child(outputs(&files))),
            )
            .child(
                div().class("actions").child(
                    a().class("btn primary")
                        .href(edit)
                        .text("Modifier cet exemple"),
                ),
            ),
    )
}

/// L'aide-mémoire : un besoin, la ligne à écrire, ce qu'elle donne.
const CHEATS: &[(&str, &str, &str)] = &[
    (
        "Une application",
        r#"web { image = "nginx:1.29" }"#,
        "Service Compose, Deployment et Service Kubernetes. Le port des images courantes est déduit (80 pour nginx).",
    ),
    (
        "Un site complet",
        r#"web { rust = ".", nginx = "1.29", postgres = "18", redis = "8" }"#,
        "nginx devant votre code, <code>DATABASE_URL</code> et <code>REDIS_URL</code> fournies, données gardées, mot de passe dans un Secret.",
    ),
    (
        "La variante",
        r#"web { rust = "1.99-alpine", postgres = "18-alpine" }"#,
        "La valeur est le tag. Pour Rust, l'image d'exécution de la même famille suit : alpine, bookworm…",
    ),
    (
        "La version à part",
        r#"db { image = "postgres", version = "18" }"#,
        "La même chose que <code>postgres:18</code>, pratique avec une variable : <code>version = API</code>.",
    ),
    (
        "Les ports",
        r#"web { image = "nginx:1.29", port = "8080:80" }"#,
        "Publié sur 8080, l'application écoute sur 80. Plusieurs : <code>ports = [80, \"8443:443\"]</code>.",
    ),
    (
        "Un nom de domaine",
        r#"web { image = "nginx:1.29", host = "monsite.fr" }"#,
        "Un Ingress Kubernetes vers l'application.",
    ),
    (
        "Plusieurs conteneurs",
        r#"api { image = "acme/api:2", with = { proxy = { image = "envoyproxy/envoy:v1.36" } } }"#,
        "Dans le même pod Kubernetes ; dans Compose, sur le même réseau que l'application.",
    ),
    (
        "Une valeur réutilisée",
        "VERSION = \"2.1.0\"\nfn image(n) = \"ghcr.io/acme/\" + n + \":\" + VERSION\napi { image = image(\"api\") }",
        "Une variable, une fonction d'une ligne, <code>+</code> pour assembler.",
    ),
];

fn cheats() -> Element {
    let cards = CHEATS.iter().map(|(title, source, effect)| {
        article()
            .class("card cheat reveal")
            .child(h3().text(*title))
            .child(code_block("aide.drc", source))
            .child(p().html(*effect))
    });
    section().id("aide-memoire").child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("Aide-mémoire"))
            .child(h2().class("section-title").text("Un besoin, une ligne"))
            .child(p().class("lead").html(
                "Tout ce que l'on écrit le plus souvent. Les fautes de frappe sont signalées avec leur correction, à la ligne et à la colonne près.",
            ))
            .child(div().class("cards cheats").children(cards))
            .child(
                div()
                    .class("actions")
                    .child(a().class("btn primary").href("/bac-a-sable").text("Essayer"))
                    .child(a().class("btn").href("/langage").text("Toute la spécification")),
            ),
    )
}

fn cli() -> Element {
    let lines = [
        "$ orchestra build app.drc --out deploy --set API=2.2.0",
        "✓ deploy/Dockerfile",
        "✓ deploy/docker-compose.yml",
        "✓ deploy/deployment.yaml",
        "$ kubectl apply -f deploy/",
        "deployment.apps/web created",
    ];
    let html: String = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            format!(
                "<span class=\"term-line\" style=\"--d:{:.1}s\">{}</span>",
                0.6 + i as f32 * 0.7,
                orchestra::render::highlight::shell(line)
            )
        })
        .collect();
    section().child(
        div()
            .class("wrap")
            .child(span().class("eyebrow").text("En ligne de commande"))
            .child(h2().class("section-title").text("Un binaire, trois commandes"))
            .child(p().class("lead").html(
                "<code>orchestra check</code> vérifie un fichier, <code>orchestra build</code> écrit les fichiers (<code>--set</code> choisit une version au passage), <code>orchestra</code> seul lance ce site.",
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

#[cfg(test)]
mod tests {
    #[test]
    fn every_cheat_compiles() {
        for (title, source, _) in super::CHEATS {
            if let Err(e) = orchestra::drc::compile_str(source) {
                panic!("{title} : {e}");
            }
        }
    }
}
