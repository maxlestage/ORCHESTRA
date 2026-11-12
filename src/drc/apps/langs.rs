//! Les langages qu'une pile sait construire : un nom suffit (`node`), la version et la
//! variante s'écrivent sans guillemets (`node = 26-alpine`). Chacun a son Dockerfile ;
//! pour les langages compilés, l'image d'exécution est de la même famille que celle
//! de construction.

use super::{list, obj, s};
use crate::drc::Value;

/// Les langages, dans l'ordre où la documentation les présente.
pub const LANGS: &[&str] = &["rust", "go", "node", "python", "java", "php"];

/// La variante prise quand seul le nom est écrit : la dernière stable, en image légère.
pub fn default_tag(lang: &str) -> &'static str {
    match lang {
        "rust" => super::RUST_DEFAULT,
        "go" => "1.27",
        "node" => "26-slim",
        "python" => "3.15-slim",
        "java" => "25",
        "php" => "8.5-apache",
        _ => "latest",
    }
}

/// Le port où l'application écoute quand il n'est pas écrit.
pub fn default_port(lang: &str) -> i64 {
    match lang {
        "php" => 80,
        _ => 8080,
    }
}

/// La commande de démarrage quand `command` n'est pas écrite.
fn default_command(lang: &str) -> &'static [&'static str] {
    match lang {
        "rust" | "go" => &["./server"],
        "node" => &["npm", "start"],
        "python" => &["python", "main.py"],
        "java" => &["java", "-jar", "app.jar"],
        // L'image php-apache démarre Apache toute seule.
        _ => &[],
    }
}

/// L'image d'exécution d'un binaire construit sur `tag` : alpine, bookworm, bullseye
/// ou, par défaut, trixie (la distribution des images officielles actuelles).
fn native_runtime(tag: &str) -> String {
    super::rust_images(tag).1
}

/// Le Dockerfile du code de `path`, écrit en `lang`, avec la variante `tag`.
///
/// `stores` sont les bases de la pile : PHP reçoit les extensions qui les joignent.
pub fn dockerfile(
    lang: &str,
    path: &str,
    tag: &str,
    port: i64,
    command: &[String],
    stores: &[&str],
) -> Value {
    let command: Vec<String> = if command.is_empty() {
        default_command(lang)
            .iter()
            .map(|c| (*c).to_owned())
            .collect()
    } else {
        command.to_vec()
    };
    let copy = Value::List(vec![obj([("src", s(path)), ("dst", s("."))])]);
    let mut docker = match lang {
        "rust" => {
            let (base, run) = super::rust_images(tag);
            vec![
                ("base", s(base)),
                ("workdir", s("/app")),
                ("copy", copy),
                (
                    "build",
                    list(&[
                        "cargo install --path . --root /out".to_owned(),
                        "mv /out/bin/* /app/server".to_owned(),
                    ]),
                ),
                ("from", s(run)),
                ("artifacts", list(&["server".to_owned()])),
            ]
        }
        "go" => vec![
            ("base", s(format!("golang:{tag}"))),
            ("workdir", s("/app")),
            ("copy", copy),
            (
                "build",
                list(&["CGO_ENABLED=0 go build -o /app/server .".to_owned()]),
            ),
            ("from", s(native_runtime(tag))),
            ("artifacts", list(&["server".to_owned()])),
        ],
        "java" => {
            // `25` ou `25-alpine` : Maven pour construire, le JRE de la même famille pour tourner.
            let (version, variant) = tag.split_once('-').unwrap_or((tag, ""));
            let suffix = if variant.is_empty() {
                String::new()
            } else {
                format!("-{variant}")
            };
            vec![
                ("base", s(format!("maven:3-eclipse-temurin-{version}{suffix}"))),
                ("workdir", s("/app")),
                ("copy", copy),
                (
                    "build",
                    list(&[
                        "mvn -q -B package -DskipTests".to_owned(),
                        "cp \"$(ls target/*.jar | grep -v -e -sources -e -javadoc -e original- | head -n 1)\" /app/app.jar".to_owned(),
                    ]),
                ),
                ("from", s(format!("eclipse-temurin:{version}-jre{suffix}"))),
                ("artifacts", list(&["app.jar".to_owned()])),
            ]
        }
        "node" => vec![
            ("base", s(format!("node:{tag}"))),
            ("workdir", s("/app")),
            ("copy", copy),
            // `npm ci` avec un package-lock.json, `npm install` sinon.
            (
                "build",
                list(&["npm ci --omit=dev || npm install --omit=dev".to_owned()]),
            ),
            ("env", obj([("NODE_ENV", s("production"))])),
        ],
        "python" => vec![
            ("base", s(format!("python:{tag}"))),
            ("workdir", s("/app")),
            ("copy", copy),
            (
                "build",
                list(&["if [ -f requirements.txt ]; then pip install --no-cache-dir -r requirements.txt; fi".to_owned()]),
            ),
            ("env", obj([("PYTHONUNBUFFERED", s("1"))])),
        ],
        // php-apache sert le dossier /var/www/html ; les extensions de la base de la pile
        // sont installées (pdo_mysql, pdo_pgsql).
        _ => {
            let alpine = tag.contains("alpine");
            let mut build = Vec::new();
            if stores.iter().any(|st| matches!(*st, "mysql" | "mariadb")) {
                build.push("docker-php-ext-install pdo_mysql mysqli".to_owned());
            }
            if stores.contains(&"postgres") {
                build.push(if alpine {
                    "apk add --no-cache postgresql-dev && docker-php-ext-install pdo_pgsql pgsql".to_owned()
                } else {
                    "apt-get update && apt-get install -y --no-install-recommends libpq-dev && docker-php-ext-install pdo_pgsql pgsql && rm -rf /var/lib/apt/lists/*".to_owned()
                });
            }
            let mut php = vec![("base", s(format!("php:{tag}"))), ("workdir", s("/var/www/html"))];
            if !build.is_empty() {
                php.push(("build", list(&build)));
            }
            php.push(("copy", copy));
            php
        }
    };
    // Le port, donné aussi au programme dans PORT.
    match docker.iter_mut().find(|(k, _)| *k == "env") {
        Some((_, Value::Object(env))) => env.push(("PORT".to_owned(), Value::Int(port))),
        _ => docker.push(("env", obj([("PORT", Value::Int(port))]))),
    }
    docker.push(("expose", Value::List(vec![Value::Int(port)])));
    if !command.is_empty() {
        docker.push(("run", list(&command)));
    }
    Value::Object(docker.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

/// Une variante valide : `26-alpine`, `3.15-slim`, `1.27`…
pub fn valid_tag(tag: &str) -> bool {
    super::rust_tag(tag)
}
