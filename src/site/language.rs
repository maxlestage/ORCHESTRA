//! La spécification du langage Rust-Compose.

use orchestra::drc::{BUILTINS, KINDS};
use paquets::prelude::*;

use super::{code_block, layout, shell_block};

/// Le sommaire : (ancre, titre).
const TOC: &[(&str, &str)] = &[
    ("objectif", "Objectif et principes"),
    ("app", "L'essentiel : app"),
    ("pile", "Un site web : une pile"),
    ("structure", "Structure du projet"),
    ("syntaxe", "Syntaxe générale"),
    ("docker", "Section docker"),
    ("compose", "Section compose"),
    ("k8s", "Section k8s"),
    ("fonctions", "Variables et fonctions"),
    ("modules", "Modules et imports"),
    ("verifications", "Vérifications"),
    ("cli", "Compilation et génération"),
    ("api", "API HTTP"),
];

pub async fn page(_req: Request) -> Page {
    let toc = details()
        .class("toc")
        .attr("open", "")
        .child(summary().text("Sommaire"))
        .child(
            ol().children(
                TOC.iter()
                    .map(|(id, title)| li().child(a().href(format!("#{id}")).text(*title))),
            ),
        );
    let article = article()
        .class("prose")
        .child(span().class("eyebrow").text("Spécification · version 1.0"))
        .child(h1().text("Le langage Rust‑Compose"))
        .child(p().class("lead").html(
            "Un format lisible et compilable par un outil Rust, qui génère à la fois le <code>Dockerfile</code>, le <code>docker-compose.yml</code> et les manifests Kubernetes (Deployment, Service, ConfigMap…).",
        ))
        .children(objective())
        .children(apps())
        .children(stack())
        .children(structure())
        .children(syntax())
        .children(docker())
        .children(compose())
        .children(kubernetes())
        .children(functions())
        .children(modules())
        .children(checks())
        .children(cli())
        .children(api());
    layout(
        "Le langage Rust‑Compose",
        "La spécification complète de Rust‑Compose : syntaxe, sections docker, compose et k8s, variables, fonctions, imports, vérifications et génération.",
        "/langage",
        div()
            .class("wrap doc-layout")
            .child(div().child(toc))
            .child(article),
    )
}

fn h2_id(id: &str, title: &str) -> Node {
    h2().id(id.to_owned()).text(title.to_owned()).into()
}

fn para(html: &str) -> Node {
    p().html(html.to_owned()).into()
}

fn drc(source: &str) -> Node {
    code_block("exemple.drc", source).into()
}

/// Un tableau ; chaque cellule porte le titre de sa colonne pour l'affichage mobile.
fn table(headers: &[&str], rows: &[Vec<String>]) -> Node {
    let head = thead().child(tr().children(headers.iter().map(|h| th().text((*h).to_owned()))));
    let body = tbody().children(rows.iter().map(|row| {
        tr().children(row.iter().zip(headers).map(|(cell, header)| {
            td().attr("data-label", (*header).to_owned())
                .html(cell.clone())
        }))
    }));
    table_el().class("table").child(head).child(body).into()
}

fn table_el() -> Element {
    html::table()
}

fn rows(data: &[&[&str]]) -> Vec<Vec<String>> {
    data.iter()
        .map(|row| row.iter().map(|c| (*c).to_owned()).collect())
        .collect()
}

fn objective() -> Vec<Node> {
    vec![
        h2_id("objectif", "Objectif et principes"),
        para("Rust‑Compose fournit un format lisible et compilable par un outil Rust, qui génère à la fois des Dockerfile, des <code>docker-compose.yml</code> et des manifests Kubernetes."),
        ol().children(
            [
                "Le langage est <strong>déclaratif</strong> : tout est une déclaration.",
                "Il est <strong>modulaire</strong> : fichiers séparés, imports.",
                "Il supporte <strong>les variables et les fonctions</strong>, pour la réutilisation.",
                "Il est <strong>typé</strong> : les éléments obligatoires sont vérifiés à la compilation.",
                "Sa syntaxe est <strong>influencée par TOML et YAML</strong> : lisible par les humains, facile à analyser en Rust.",
            ]
            .into_iter()
            .map(|item| li().html(item)),
        )
        .into(),
    ]
}

fn apps() -> Vec<Node> {
    let keys: Vec<Vec<String>> = [
        ("image", "texte", "L'image du conteneur, avec ou sans version : <code>\"postgres:18-alpine\"</code> ou <code>\"postgres\"</code>. Obligatoire, sauf avec <code>build</code>."),
        ("version", "texte", "La version de l'image, écrite à part : <code>image = \"postgres\"</code>, <code>version = \"18-alpine\"</code>. Avec <code>build</code>, la version de l'image construite (<code>latest</code> sinon)."),
        ("port", "nombre ou texte", "<code>3000</code> : le même dehors et dedans. <code>\"8080:80\"</code> : publié sur 8080, l'application écoute sur 80. Déduit pour les images courantes (nginx 80, postgres 5432, redis 6379…), où un nombre seul est le port public : <code>port = 8080</code> publie nginx sur 8080. <code>false</code> pour aucun."),
        ("ports", "liste", "Plusieurs ports : <code>[80, \"8443:443\"]</code>. Deux applications ne peuvent pas publier le même port : le compilateur le refuse et en propose un libre."),
        ("build", "texte", "<code>\"rust\"</code> : un Dockerfile Rust en deux étapes (binaire du même nom que l'app). Un autre texte : le dossier de construction Compose."),
        ("replicas", "nombre", "Le nombre d'instances Kubernetes (1 par défaut)."),
        ("env", "objet", "Les variables d'environnement : <code>{ CLE = \"valeur\" }</code>."),
        ("command", "texte ou liste", "La commande lancée à la place de celle de l'image."),
        ("volumes", "liste", "Des volumes Compose : <code>[\"./data:/data\"]</code>."),
        ("depends_on", "liste", "Les applications à démarrer avant (Compose)."),
        ("host", "texte", "Un nom de domaine : un Ingress Kubernetes vers le port."),
        ("with", "objet", "D'autres conteneurs dans le même pod : <code>{ proxy = { image = \"envoyproxy/envoy\", version = \"v1.36\", port = 9901 } }</code>, avec <code>image</code>, <code>version</code>, <code>port</code>, <code>ports</code>, <code>env</code> et <code>command</code>. Dans Compose, chacun devient un service qui partage le réseau de l'application."),
    ]
    .iter()
    .map(|(key, kind, role)| vec![format!("<code>{key}</code>"), (*kind).to_owned(), (*role).to_owned()])
    .collect();
    vec![
        h2_id("app", "L'essentiel : app"),
        para("Une application s'écrit en une ligne : un nom, une image. Le compilateur en déduit le port, le service Compose, le Deployment et le Service Kubernetes, avec leurs labels et leur selector :"),
        drc("web { image = \"nginx:1.29\" }"),
        para("Aucun mot-clé n'est nécessaire : un nom suivi de <code>{ … }</code> est une application, un nom suivi de <code>=</code> une variable. Les formes longues <code>app web { … }</code> et <code>var X = …</code> restent valides."),
        para("Plusieurs applications, un Dockerfile Rust, un nom de domaine :"),
        drc(include_str!("../../examples/boutique.drc")),
        table(&["Clé", "Type", "Rôle"], &keys),
        blockquote().child(p().html(
            "Besoin d'un réglage que <code>app</code> ne prévoit pas ? Ajoutez une section <code>docker</code>, <code>compose</code> ou <code>k8s</code> à côté : elle complète ce que les applications génèrent.",
        ))
        .into(),
    ]
}

fn stack() -> Vec<Node> {
    let techs: Vec<Vec<String>> = [
        ("rust", "<code>\".\"</code>", "Votre code : un Dockerfile Rust (le binaire, quel que soit son nom), construit par Compose. À la place : <code>image = \"…\"</code> pour une application déjà construite."),
        ("nginx", "<code>\"1.29\"</code>", "Devant l'application, sur le port 80 : sa configuration de proxy est écrite pour vous (ConfigMap dans Kubernetes)."),
        ("postgres", "<code>\"18\"</code>", "<code>DATABASE_URL</code> pour l'application ; données dans un volume (StatefulSet et volume persistant dans Kubernetes)."),
        ("mysql, mariadb", "<code>\"9.5\"</code>, <code>\"12\"</code>", "<code>DATABASE_URL</code> (<code>mysql://…</code>), données gardées de même."),
        ("mongo", "<code>\"8\"</code>", "<code>MONGODB_URL</code>, données gardées de même."),
        ("redis, valkey", "<code>\"8\"</code>, <code>\"9\"</code>", "<code>REDIS_URL</code> pour l'application."),
    ]
    .iter()
    .map(|(tech, example, role)| vec![format!("<code>{tech}</code>"), (*example).to_owned(), (*role).to_owned()])
    .collect();
    let options: Vec<Vec<String>> = [
        ("port", "Le port de l'application (8080 par défaut, donné aussi dans <code>PORT</code>)."),
        ("version", "La version de l'image de l'application."),
        ("env", "D'autres variables pour l'application."),
        ("replicas", "Le nombre d'instances de l'application."),
        ("password", "Le mot de passe des bases (<code>change-me</code> par défaut : changez-le, par exemple avec <code>--set</code>). Il va dans un Secret Kubernetes."),
        ("host", "Un nom de domaine : un Ingress vers nginx, ou vers l'application sans nginx."),
    ]
    .iter()
    .map(|(key, role)| vec![format!("<code>{key}</code>"), (*role).to_owned()])
    .collect();
    vec![
        h2_id("pile", "Un site web : une pile"),
        para(
            "Pour un site, écrivez une ligne par technologie, avec sa version. Le compilateur relie tout :",
        ),
        drc(include_str!("../../examples/web.drc")),
        para(
            "nginx renvoie vers l'application ; l'application reçoit <code>DATABASE_URL</code> et <code>REDIS_URL</code> ; seul nginx est publié (port 80), la base et le cache restent internes. Dans Kubernetes, la base devient un StatefulSet avec son volume, et son mot de passe un Secret.",
        ),
        table(&["Technologie", "Valeur", "Ce qu'elle apporte"], &techs),
        table(&["Option", "Rôle"], &options),
    ]
}

fn structure() -> Vec<Node> {
    vec![
        h2_id("structure", "Structure du projet"),
        pre().class("code tree").child(code().text(
            "project/\n├── docker/\n│   └── app.drc          ← fichier Rust-Compose Docker\n├── k8s/\n│   └── deployment.drc   ← fichier Rust-Compose Kubernetes\n├── config.drc           ← variables globales, fonctions\n├── docker-compose.yml   ← généré\n├── Dockerfile           ← généré\n└── deployment.yaml      ← généré",
        ))
        .into(),
        blockquote().child(p().html(
            "Le suffixe <code>.drc</code> (pour <em>Docker‑Rust‑Compose</em>) est la convention de nommage. Les sections peuvent aussi être réunies dans un même fichier : le compilateur sait les distinguer.",
        ))
        .into(),
    ]
}

fn syntax() -> Vec<Node> {
    vec![
        h2_id("syntaxe", "Syntaxe générale"),
        para(
            "Un mélange de TOML (pour les sections) et de YAML (pour les blocs imbriqués). Chaque fichier peut commencer par l'en-tête facultatif <code># Rust-Compose 1.0</code>.",
        ),
        h3().text("Déclarations").into(),
        table(
            &["Élément", "Format", "Exemple"],
            &rows(&[
                &[
                    "Application",
                    "<code>&lt;nom&gt; { … }</code>",
                    "<code>web { image = \"nginx:1.29\" }</code>",
                ],
                &[
                    "Variable",
                    "<code>[var] &lt;nom&gt; = &lt;valeur&gt;</code>",
                    "<code>IMAGE_NAME = \"myapp\"</code>",
                ],
                &[
                    "Fonction",
                    "<code>fn &lt;nom&gt;(&lt;params&gt;) { … }</code>",
                    "<code>fn image_tag(v) { return format!(\"{}:{}\", IMAGE_NAME, v) }</code>",
                ],
                &[
                    "Import",
                    "<code>import \"&lt;chemin&gt;\"</code>",
                    "<code>import \"../config.drc\"</code>",
                ],
                &[
                    "Section Docker",
                    "<code>docker { … }</code>",
                    "<code>docker { base = \"rust:1.99-slim\" }</code>",
                ],
                &[
                    "Section Compose",
                    "<code>compose { … }</code>",
                    "<code>compose { services = { … } }</code>",
                ],
                &[
                    "Section Kubernetes",
                    "<code>k8s { … }</code>",
                    "<code>k8s { deployment = { … } }</code>",
                ],
            ]),
        ),
        h3().text("Types de valeurs").into(),
        table(
            &["Type", "Syntaxe", "Exemple", "Rôle"],
            &rows(&[
                &[
                    "Chaîne",
                    "<code>\"texte\"</code>",
                    "<code>\"myapp\"</code>",
                    "Noms, tags, URL",
                ],
                &[
                    "Booléen",
                    "<code>true</code> / <code>false</code>",
                    "<code>true</code>",
                    "Activer ou désactiver une option",
                ],
                &[
                    "Nombre",
                    "<code>42</code>, <code>2.5</code>",
                    "<code>8080</code>",
                    "Ports, réplicas",
                ],
                &[
                    "Liste",
                    "<code>[a, b, c]</code>",
                    "<code>[\"alpine\", \"debian\"]</code>",
                    "Tags multiples, volumes",
                ],
                &[
                    "Objet",
                    "<code>{ clé = valeur, … }</code>",
                    "<code>{ image = \"myapp\", port = 8080 }</code>",
                    "Configurations imbriquées",
                ],
            ]),
        ),
        para(
            "Dans un objet, les entrées se séparent par une virgule <em>ou</em> par un retour à la ligne. Une clé est un nom (<code>image</code>) ou une chaîne (<code>\"app.conf\"</code>). Les chaînes acceptent les échappements <code>\\n</code>, <code>\\t</code>, <code>\\\"</code> et <code>\\\\</code>.",
        ),
        h3().text("Commentaires").into(),
        drc(
            "# commentaire sur une ligne\n// aussi sur une ligne\n/* un bloc\n   sur plusieurs lignes */",
        ),
    ]
}

fn docker() -> Vec<Node> {
    vec![
        h2_id("docker", "Section docker"),
        drc(r#"docker {
    base = "rust:1.99-slim"          # image de construction (FROM)
    workdir = "/app"                 # WORKDIR
    copy = [
        { src = "./src", dst = "src" },
        { src = "./Cargo.toml", dst = "." }
    ]
    build = [
        "cargo build --release",
        "cp target/release/myapp ./myapp"
    ]
    from = "debian:trixie-slim"      # image finale (FROM)
    artifacts = ["myapp"]            # copié de la construction vers l'image finale
    run = ["./myapp"]                # CMD
    expose = ["8080/tcp"]            # EXPOSE
}"#),
        table(
            &["Directive", "Type", "Description"],
            &rows(&[
                &["<code>base</code>", "chaîne, <strong>obligatoire</strong>", "Image de base (<code>FROM</code>)."],
                &["<code>workdir</code>", "chaîne", "<code>WORKDIR</code>, répété dans l'image finale."],
                &["<code>copy</code>", "liste de <code>{ src, dst }</code>", "Copies <code>COPY</code>, dans l'ordre."],
                &["<code>build</code>", "liste de chaînes", "Commandes <code>RUN</code>, exécutées dans l'ordre."],
                &["<code>from</code>", "chaîne", "Image finale : la construction devient l'étape <code>build</code>."],
                &["<code>artifacts</code>", "liste de chaînes", "Ce que l'image finale reprend de la construction (par défaut, tout <code>workdir</code>)."],
                &["<code>env</code>", "objet", "Variables <code>ENV</code>."],
                &["<code>labels</code>", "objet", "Étiquettes <code>LABEL</code>."],
                &["<code>user</code>", "chaîne ou nombre", "<code>USER</code>."],
                &["<code>expose</code>", "liste", "Ports exposés : <code>8080</code>, <code>\"8080/tcp\"</code>, <code>\"53/udp\"</code>."],
                &["<code>entrypoint</code>", "liste de chaînes", "<code>ENTRYPOINT</code>, au format exec."],
                &["<code>run</code>", "liste de chaînes", "Commande par défaut (<code>CMD</code>), au format exec."],
            ]),
        ),
        blockquote().child(p().html(
            "<code>build</code> peut contenir autant de commandes que nécessaire : chaque élément devient une instruction <code>RUN</code>, exécutée dans l'ordre.",
        ))
        .into(),
    ]
}

fn compose() -> Vec<Node> {
    vec![
        h2_id("compose", "Section compose"),
        drc(r#"compose {
    services = {
        web = {
            image = image_tag("latest")      # appel à une fonction
            ports = ["8080:80"]
            environment = { "ENV" = "prod" }
            volumes = ["./data:/data"]
        }
        db = {
            image = "postgres:18-alpine"
            environment = { "POSTGRES_PASSWORD" = "secret" }
            volumes = ["./dbdata:/var/lib/postgresql"]
            ports = ["5432:5432"]
        }
    }
    networks = {
        default = { driver = "bridge" }
    }
}"#),
        para(
            "La section suit la <a href=\"https://compose-spec.io\">spécification Compose</a> : <code>name</code>, <code>services</code> (obligatoire), <code>networks</code>, <code>volumes</code>, <code>configs</code> et <code>secrets</code>. Chaque service a besoin d'une <code>image</code> ou d'un <code>build</code> ; <code>ports</code> et <code>volumes</code> sont des listes, <code>environment</code> un objet ou une liste. Les autres clés des services sont écrites telles quelles.",
        ),
    ]
}

fn kubernetes() -> Vec<Node> {
    let kinds: Vec<Vec<String>> = KINDS
        .iter()
        .map(|k| {
            vec![
                format!("<code>{}</code>", k.key),
                format!(
                    "{} <span class=\"muted\">({})</span>",
                    k.kind, k.api_version
                ),
                format!("<code>{}.yaml</code>", k.key),
            ]
        })
        .collect();
    vec![
        h2_id("k8s", "6. Section k8s"),
        drc(r#"k8s {
    deployment = {
        name = "myapp-deploy"
        labels = { app = "myapp" }
        replicas = 3
        selector = { matchLabels = { app = "myapp" } }
        template = {
            metadata = { labels = { app = "myapp" } }
            spec = {
                containers = [
                    {
                        name = "myapp"
                        image = image_tag("latest")
                        ports = [{ containerPort = 80 }]
                        env = [
                            { name = "ENV", value = "prod" },
                            { name = "DEBUG", value = "false" }
                        ]
                        volumeMounts = [{ name = "data", mountPath = "/app/data" }]
                    }
                ]
                volumes = [{ name = "data", emptyDir = {} }]
                imagePullSecrets = [{ name = "regcred" }]
            }
        }
    }

    service = {
        name = "myapp-service"
        selector = { app = "myapp" }
        type = "ClusterIP"
        ports = [{ port = 80, targetPort = 80, protocol = "TCP" }]
    }

    configmap = {
        name = "myapp-config"
        data = { "app.conf" = "log_level=info\n" }
    }
}"#),
        para(
            "Chaque clé de la section est un type de ressource ; sa valeur est un objet, ou une liste d'objets pour plusieurs ressources du même type (écrites dans le même fichier, séparées par <code>---</code>). <code>name</code>, <code>namespace</code>, <code>labels</code> et <code>annotations</code> vont dans <code>metadata</code>, le reste dans <code>spec</code> (ou à la racine pour ConfigMap, Secret, ServiceAccount et Namespace).",
        ),
        table(&["Clé", "Ressource", "Fichier"], &kinds),
        para(
            "Pour un Deployment, un StatefulSet ou un DaemonSet sans <code>selector</code>, le compilateur le déduit des labels du modèle de pod (ou de la ressource).",
        ),
    ]
}

fn functions() -> Vec<Node> {
    let builtins: Vec<Vec<String>> = BUILTINS
        .iter()
        .map(|name| {
            let (signature, description) = match *name {
                "concat" => (
                    "concat(a, b, …)",
                    "Met bout à bout des chaînes, nombres et booléens.",
                ),
                "upper" => ("upper(s)", "La chaîne en majuscules."),
                "lower" => ("lower(s)", "La chaîne en minuscules."),
                "join" => (
                    "join(liste, séparateur)",
                    "Les éléments d'une liste, séparés.",
                ),
                "replace" => (
                    "replace(s, motif, remplacement)",
                    "Remplace chaque occurrence du motif.",
                ),
                "str" => ("str(v)", "Une valeur simple en chaîne."),
                _ => ("", ""),
            };
            vec![format!("<code>{signature}</code>"), description.to_owned()]
        })
        .collect();
    let mut all = vec![vec![
        "<code>format!(\"{}:{}\", a, b)</code>".to_owned(),
        "Remplace chaque <code>{}</code> dans l'ordre (ou <code>{0}</code>, <code>{1}</code>… par position) ; <code>{{</code> et <code>}}</code> pour les accolades.".to_owned(),
    ]];
    all.extend(builtins);
    vec![
        h2_id("fonctions", "Variables globales et fonctions"),
        drc(r#"var IMAGE_NAME = "myapp"
var VERSION = "1.0.0"

fn image_tag(tag) = IMAGE_NAME + ":" + tag      # une fonction d'une ligne

fn image_for(env) {
    var suffix = lower(env)          # variable locale
    return concat(image_tag(VERSION), "-", suffix)
}"#),
        para(
            "Une variable se déclare avant d'être utilisée et ne se redéfinit pas. Les fonctions sont visibles dans tout leur fichier ; leur corps contient des <code>var</code> locales et se termine par <code>return</code>. Fonctions intégrées :",
        ),
        table(&["Appel", "Résultat"], &all),
    ]
}

fn modules() -> Vec<Node> {
    vec![
        h2_id("modules", "Modules et imports"),
        drc(r#"# docker/app.drc
import "../config.drc"      # chemin relatif au fichier qui importe

docker {
    base = "rust:1.99-slim"
    run = ["./myapp"]
}"#),
        para(
            "Un import apporte les variables, les fonctions et les sections d'un autre fichier. Un fichier importé deux fois n'est lu qu'une fois ; un import circulaire est une erreur. Les sections du même nom sont fusionnées objet par objet, la dernière valeur l'emportant.",
        ),
    ]
}

fn checks() -> Vec<Node> {
    vec![
        h2_id("verifications", "Vérifications"),
        para("Le compilateur refuse un fichier incorrect avant d'écrire quoi que ce soit, en donnant le fichier, la ligne et la colonne :"),
        ul().children(
            [
                "la syntaxe : accolades, crochets, guillemets, clés en double ;",
                "les fautes de frappe : une variable, une clé ou une application mal écrite reçoit sa correction (<em>vouliez-vous dire `image` ?</em>) ;",
                "les noms : variable ou fonction inconnue, nombre d'arguments, mots réservés ;",
                "les directives : une clé inconnue de <code>docker</code>, <code>compose</code> ou <code>k8s</code> est une erreur (pas une faute de frappe silencieuse) ;",
                "les éléments obligatoires : <code>docker.base</code>, <code>compose.services</code> avec une image ou un build par service, le <code>name</code> de chaque ressource, l'<code>image</code> de chaque conteneur, les <code>ports</code> d'un Service ;",
                "les types et les valeurs : ports entre 1 et 65535, nombre de réplicas, noms Kubernetes valides, type de Service ;",
                "les limites : récursion infinie, imbrication excessive, programme qui produirait trop de données.",
            ]
            .into_iter()
            .map(|item| li().html(item)),
        )
        .into(),
        pre().class("code").child(code().html(
            "<span class=\"t-comment\">$ orchestra check app.drc</span>\nerreur : app.drc:2:5 : `web` : clé inconnue `imgae` (vouliez-vous dire `image` ?)",
        ))
        .into(),
    ]
}

fn cli() -> Vec<Node> {
    vec![
        h2_id("cli", "Compilation et génération"),
        para("L'outil Rust <code>orchestra</code> lit le fichier <code>.drc</code>, valide la syntaxe, remplace les variables, exécute les fonctions, puis écrit :"),
        ul().children(
            [
                "<code>Dockerfile</code>, si la section <code>docker</code> est présente ;",
                "<code>docker-compose.yml</code>, si la section <code>compose</code> est présente ;",
                "<code>deployment.yaml</code>, <code>service.yaml</code>, <code>configmap.yaml</code>… si <code>k8s</code> est présente.",
            ]
            .into_iter()
            .map(|item| li().html(item)),
        )
        .into(),
        shell_block(
            "# Installer l'outil (Rust 1.99 ou plus récent)\n$ cargo install --git https://github.com/maxlestage/ORCHESTRA --locked\n\n$ orchestra check app.drc\n✓ app.drc : 5 fichier(s) à générer\n$ orchestra build app.drc --out deploy\n✓ deploy/Dockerfile\n✓ deploy/docker-compose.yml\n✓ deploy/deployment.yaml\n\n$ docker build -f deploy/Dockerfile .\n$ docker compose -f deploy/docker-compose.yml up\n$ kubectl apply -f deploy/",
        )
        .into(),
        para("Les fichiers générés respectent les conventions Docker et Kubernetes et s'utilisent directement avec <code>docker build</code>, <code>docker compose up</code> ou <code>kubectl apply -f</code>."),
        para("<code>--set NOM=valeur</code> remplace une variable du fichier au moment de compiler, sans le modifier : pratique en intégration continue pour déployer la version qui vient d'être construite. La valeur garde le type de la variable (<code>--set REPLICAS=5</code> reste un nombre, <code>--set API=1.10</code> reste un texte), et un nom inconnu est une erreur."),
        shell_block("$ orchestra build app.drc --out deploy --set API=2.2.0 --set REPLICAS=5").into(),
    ]
}

fn api() -> Vec<Node> {
    vec![
        h2_id("api", "API HTTP"),
        para("Le serveur expose le compilateur : envoyez un fichier, recevez les fichiers générés (ou l'erreur, avec un statut 422). Sur la version GitHub Pages, sans serveur, le bac à sable compile dans le navigateur avec le même compilateur, en WebAssembly."),
        shell_block(
            "$ curl -s https://<votre-site>/api/compile \\\n    -H 'content-type: application/json' \\\n    -d '{\"source\": \"docker { base = \\\"alpine:3.22\\\" }\"}'\n{\"files\":[{\"name\":\"Dockerfile\",\"content\":\"…\"}]}",
        )
        .into(),
    ]
}
