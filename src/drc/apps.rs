//! `web { image = "nginx:1.29" }` : une application en une ligne, déclinée en service
//! Compose, Deployment, Service et Ingress Kubernetes, et Dockerfile pour `build = "rust"`.
//! Le port des images courantes (nginx, postgres, redis…) est déduit.
//!
//! Les sections `docker`, `compose` et `k8s` écrites à la main restent possibles : elles
//! complètent ce que les applications génèrent, ou le remplacent clé par clé.

use super::eval::{AppValue, SectionValue, merge};
use super::parser::Section;
use super::{Error, Pos, Value};

/// Les clés d'une application.
pub const KEYS: &[&str] = &[
    "image",
    "build",
    "port",
    "replicas",
    "env",
    "command",
    "volumes",
    "depends_on",
    "host",
];

/// Les images du Dockerfile de `build = "rust"`.
const RUST_BUILD: &str = "rust:1.99-slim";
const RUST_RUN: &str = "debian:trixie-slim";

/// Les sections générées par les applications, complétées par les sections écrites.
pub fn expand(apps: &[AppValue], explicit: Vec<SectionValue>) -> Result<Vec<SectionValue>, Error> {
    let mut docker: Option<(Value, Pos)> = None;
    let mut services = Vec::new();
    let mut k8s: Vec<(String, Value)> = Vec::new();
    for app in apps {
        let a = App::read(app, apps)?;
        if let Some(dockerfile) = a.dockerfile() {
            if docker.is_some() {
                return Err(Error::at(
                    app.pos,
                    "une seule application peut avoir `build = \"rust\"` (un seul Dockerfile)",
                ));
            }
            docker = Some((dockerfile, app.pos));
        }
        services.push((a.name.to_owned(), a.service()));
        push(&mut k8s, "deployment", a.deployment());
        if let Some(service) = a.k8s_service() {
            push(&mut k8s, "service", service);
        }
        if let Some(ingress) = a.ingress() {
            push(&mut k8s, "ingress", ingress);
        }
    }

    let first = apps.first().map(|a| a.pos).unwrap_or_default();
    let mut sections = Vec::new();
    if let Some((value, pos)) = docker {
        sections.push(SectionValue {
            section: Section::Docker,
            value,
            pos,
        });
    }
    if !services.is_empty() {
        sections.push(SectionValue {
            section: Section::Compose,
            value: obj([("services", Value::Object(services))]),
            pos: first,
        });
        sections.push(SectionValue {
            section: Section::K8s,
            value: Value::Object(k8s),
            pos: first,
        });
    }

    // Les sections écrites à la main : fusionnées (docker, compose), ou ajoutées aux
    // ressources générées (k8s : un Deployment de plus ne remplace pas ceux des apps).
    for section in explicit {
        let Some(generated) = sections.iter_mut().find(|s| s.section == section.section) else {
            sections.push(section);
            continue;
        };
        match (section.section, &mut generated.value, section.value) {
            (Section::K8s, Value::Object(resources), Value::Object(written)) => {
                for (kind, value) in written {
                    match resources.iter_mut().find(|(k, _)| *k == kind) {
                        Some((_, Value::List(list))) => match value {
                            Value::List(more) => list.extend(more),
                            one => list.push(one),
                        },
                        _ => resources.push((kind, value)),
                    }
                }
            }
            (_, value, written) => merge(value, written),
        }
        generated.pos = section.pos;
    }
    sections.sort_by_key(|s| s.section as u8);
    Ok(sections)
}

fn push(resources: &mut Vec<(String, Value)>, kind: &str, value: Value) {
    match resources.iter_mut().find(|(k, _)| k == kind) {
        Some((_, Value::List(list))) => list.push(value),
        _ => resources.push((kind.to_owned(), Value::List(vec![value]))),
    }
}

fn s(text: impl Into<String>) -> Value {
    Value::Str(text.into())
}

fn obj<const N: usize>(entries: [(&str, Value); N]) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

/// Une application lue et vérifiée.
struct App<'a> {
    name: &'a str,
    image: String,
    /// `build = "rust"` : un Dockerfile Rust ; un autre texte : le dossier de construction.
    build: Option<&'a str>,
    port: Option<i64>,
    replicas: i64,
    env: Vec<(String, String)>,
    command: Vec<String>,
    volumes: Vec<String>,
    depends_on: Vec<String>,
    host: Option<&'a str>,
}

impl<'a> App<'a> {
    fn read(app: &'a AppValue, all: &[AppValue]) -> Result<Self, Error> {
        let name = app.name.as_str();
        let err = |message: String| Error::at(app.pos, message);
        // Une erreur sur une clé est située sur cette clé.
        let at = |key: &str, message: String| {
            let pos = app
                .keys
                .iter()
                .find(|(k, _)| k == key)
                .map_or(app.pos, |(_, pos)| *pos);
            Error::at(pos, message)
        };
        let valid = name.len() <= 63
            && name.starts_with(|c: char| c.is_ascii_lowercase())
            && !name.ends_with('-')
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid {
            return Err(err(format!(
                "`{name}` : le nom s'écrit en minuscules, chiffres et `-` (ex. `mon-api {{ … }}`)"
            )));
        }
        let Value::Object(fields) = &app.value else {
            return Err(err(format!("`{name}` attend `{{ … }}`")));
        };
        for (key, _) in fields {
            if !KEYS.contains(&key.as_str()) {
                return Err(at(
                    key,
                    format!(
                        "`{name}` : clé inconnue `{key}`{} (possibles : {})",
                        super::suggest(key, KEYS.iter().copied()),
                        KEYS.join(", ")
                    ),
                ));
            }
        }
        let get = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| v);
        let text = |key: &str| -> Result<Option<&'a str>, Error> {
            match fields.iter().find(|(k, _)| k == key).map(|(_, v)| v) {
                None => Ok(None),
                Some(Value::Str(v)) if !v.trim().is_empty() => Ok(Some(v.as_str())),
                Some(other) => Err(at(
                    key,
                    format!(
                        "`{name}` : `{key}` doit être un texte non vide, pas {}",
                        other.type_name()
                    ),
                )),
            }
        };
        let words = |key: &str| -> Result<Vec<String>, Error> {
            match get(key) {
                None => Ok(Vec::new()),
                Some(Value::List(items)) => items
                    .iter()
                    .map(|item| match item {
                        Value::Str(v) => Ok(v.clone()),
                        other => Err(at(
                            key,
                            format!(
                                "`{name}` : `{key}` ne contient que des textes, pas {}",
                                other.type_name()
                            ),
                        )),
                    })
                    .collect(),
                Some(Value::Str(v)) => Ok(vec![v.clone()]),
                Some(other) => Err(at(
                    key,
                    format!(
                        "`{name}` : `{key}` est un texte ou une liste de textes, pas {}",
                        other.type_name()
                    ),
                )),
            }
        };

        let build = text("build")?;
        let image = match (text("image")?, build) {
            (Some(image), _) => image.to_owned(),
            (None, Some(_)) => format!("{name}:latest"),
            (None, None) => {
                return Err(err(format!(
                    "`{name}` a besoin d'une image (`image = \"nginx:1.29\"`) ou de `build = \"rust\"`"
                )));
            }
        };
        let port = match get("port") {
            None => known_port(&image),
            Some(Value::Bool(false)) => None,
            Some(Value::Int(p)) if (1..=65535).contains(p) => Some(*p),
            Some(_) => {
                return Err(at(
                    "port",
                    format!(
                        "`{name}` : `port` est un nombre entre 1 et 65535 (ex. `port = 8080`), ou `false` pour aucun"
                    ),
                ));
            }
        };
        let replicas = match get("replicas") {
            None => 1,
            Some(Value::Int(n)) if *n >= 0 => *n,
            Some(_) => {
                return Err(at(
                    "replicas",
                    format!("`{name}` : `replicas` est un nombre entier positif"),
                ));
            }
        };
        let env = match get("env") {
            None => Vec::new(),
            Some(Value::Object(vars)) => vars
                .iter()
                .map(|(k, v)| match v {
                    Value::Str(v) => Ok((k.clone(), v.clone())),
                    Value::Int(n) => Ok((k.clone(), n.to_string())),
                    Value::Float(n) => Ok((k.clone(), n.to_string())),
                    Value::Bool(b) => Ok((k.clone(), b.to_string())),
                    other => Err(at("env", format!(
                        "`{name}` : `env.{k}` doit être un texte, un nombre ou un booléen, pas {}",
                        other.type_name()
                    ))),
                })
                .collect::<Result<_, _>>()?,
            Some(other) => {
                return Err(at("env", format!(
                    "`{name}` : `env` s'écrit `{{ CLE = \"valeur\" }}`, pas {}",
                    other.type_name()
                )));
            }
        };
        let command = match get("command") {
            Some(Value::Str(line)) => line.split_whitespace().map(str::to_owned).collect(),
            _ => words("command")?,
        };
        let depends_on = words("depends_on")?;
        for other in &depends_on {
            if !all.iter().any(|a| a.name == *other) {
                return Err(at(
                    "depends_on",
                    format!(
                        "`{name}` dépend de `{other}`, qui n'existe pas{}",
                        super::suggest(other, all.iter().map(|a| a.name.as_str()))
                    ),
                ));
            }
        }
        let host = text("host")?;
        if host.is_some() && port.is_none() {
            return Err(at(
                "host",
                format!("`{name}` : `host` publie l'application, il lui faut un `port`"),
            ));
        }
        Ok(Self {
            name,
            image,
            build,
            port,
            replicas,
            env,
            command,
            volumes: words("volumes")?,
            depends_on,
            host,
        })
    }

    fn labels(&self) -> Value {
        obj([("app", s(self.name))])
    }

    fn dockerfile(&self) -> Option<Value> {
        if self.build != Some("rust") {
            return None;
        }
        let binary = format!("target/release/{}", self.name);
        let mut docker = vec![
            ("base".to_owned(), s(RUST_BUILD)),
            ("workdir".to_owned(), s("/app")),
            (
                "copy".to_owned(),
                Value::List(vec![obj([("src", s(".")), ("dst", s("."))])]),
            ),
            (
                "build".to_owned(),
                Value::List(vec![s("cargo build --release")]),
            ),
            ("from".to_owned(), s(RUST_RUN)),
            ("artifacts".to_owned(), Value::List(vec![s(binary.clone())])),
        ];
        if let Some(port) = self.port {
            docker.push(("expose".to_owned(), Value::List(vec![Value::Int(port)])));
        }
        docker.push((
            "run".to_owned(),
            Value::List(vec![s(format!("./{binary}"))]),
        ));
        Some(Value::Object(docker))
    }

    fn service(&self) -> Value {
        let mut service = Vec::new();
        match self.build {
            Some("rust") => service.push(("build".to_owned(), s("."))),
            Some(dir) => service.push(("build".to_owned(), s(dir))),
            None => {}
        }
        service.push(("image".to_owned(), s(self.image.clone())));
        if !self.command.is_empty() {
            service.push(("command".to_owned(), list(&self.command)));
        }
        if let Some(port) = self.port {
            service.push((
                "ports".to_owned(),
                Value::List(vec![s(format!("{port}:{port}"))]),
            ));
        }
        if !self.env.is_empty() {
            service.push((
                "environment".to_owned(),
                Value::Object(
                    self.env
                        .iter()
                        .map(|(k, v)| (k.clone(), s(v.clone())))
                        .collect(),
                ),
            ));
        }
        if !self.volumes.is_empty() {
            service.push(("volumes".to_owned(), list(&self.volumes)));
        }
        if !self.depends_on.is_empty() {
            service.push(("depends_on".to_owned(), list(&self.depends_on)));
        }
        Value::Object(service)
    }

    fn deployment(&self) -> Value {
        let mut container = vec![
            ("name".to_owned(), s(self.name)),
            ("image".to_owned(), s(self.image.clone())),
        ];
        if !self.command.is_empty() {
            container.push(("args".to_owned(), list(&self.command)));
        }
        if let Some(port) = self.port {
            container.push((
                "ports".to_owned(),
                Value::List(vec![obj([("containerPort", Value::Int(port))])]),
            ));
        }
        if !self.env.is_empty() {
            container.push((
                "env".to_owned(),
                Value::List(
                    self.env
                        .iter()
                        .map(|(k, v)| obj([("name", s(k.clone())), ("value", s(v.clone()))]))
                        .collect(),
                ),
            ));
        }
        obj([
            ("name", s(self.name)),
            ("labels", self.labels()),
            ("replicas", Value::Int(self.replicas)),
            (
                "template",
                obj([
                    ("metadata", obj([("labels", self.labels())])),
                    (
                        "spec",
                        obj([("containers", Value::List(vec![Value::Object(container)]))]),
                    ),
                ]),
            ),
        ])
    }

    fn k8s_service(&self) -> Option<Value> {
        let port = self.port?;
        Some(obj([
            ("name", s(self.name)),
            ("selector", self.labels()),
            (
                "ports",
                Value::List(vec![obj([
                    ("port", Value::Int(port)),
                    ("targetPort", Value::Int(port)),
                ])]),
            ),
        ]))
    }

    fn ingress(&self) -> Option<Value> {
        let (host, port) = (self.host?, self.port?);
        let backend = obj([(
            "service",
            obj([
                ("name", s(self.name)),
                ("port", obj([("number", Value::Int(port))])),
            ]),
        )]);
        Some(obj([
            ("name", s(self.name)),
            (
                "rules",
                Value::List(vec![obj([
                    ("host", s(host)),
                    (
                        "http",
                        obj([(
                            "paths",
                            Value::List(vec![obj([
                                ("path", s("/")),
                                ("pathType", s("Prefix")),
                                ("backend", backend),
                            ])]),
                        )]),
                    ),
                ])]),
            ),
        ]))
    }
}

/// Le port des images courantes, pour ne pas avoir à l'écrire.
pub const KNOWN_PORTS: &[(&str, i64)] = &[
    ("nginx", 80),
    ("httpd", 80),
    ("caddy", 80),
    ("traefik", 80),
    ("postgres", 5432),
    ("mysql", 3306),
    ("mariadb", 3306),
    ("redis", 6379),
    ("valkey", 6379),
    ("mongo", 27017),
    ("rabbitmq", 5672),
    ("memcached", 11211),
    ("elasticsearch", 9200),
    ("grafana", 3000),
    ("prometheus", 9090),
    ("minio", 9000),
];

/// `docker.io/library/postgres:18-alpine` → 5432.
fn known_port(image: &str) -> Option<i64> {
    let name = image.rsplit('/').next().unwrap_or(image);
    let name = name.split([':', '@']).next().unwrap_or(name);
    KNOWN_PORTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, p)| *p)
}

fn list(items: &[String]) -> Value {
    Value::List(items.iter().cloned().map(Value::Str).collect())
}
