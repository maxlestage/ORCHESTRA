//! `web { image = "nginx:1.29" }` : une application en une ligne, déclinée en service
//! Compose, Deployment, Service et Ingress Kubernetes, et Dockerfile pour `build = "rust"`.
//! Le port des images courantes (nginx, postgres, redis…) est déduit.
//!
//! - Les versions : `image = "postgres:18-alpine"`, ou `image = "postgres"` et
//!   `version = "18-alpine"`.
//! - Les ports : `port = 3000` (le même dehors et dedans), `port = "8080:80"` (publié sur
//!   8080, l'application écoute sur 80), `ports = [80, "8443:443"]` pour plusieurs. Pour
//!   une image connue, un nombre seul est le port public : `port = 8080` publie sur 8080
//!   le port 80 de nginx.
//! - Plusieurs conteneurs : `with = { proxy = { image = "envoyproxy/envoy", … } }`, dans le
//!   même pod Kubernetes ; dans Compose, des services qui partagent le réseau de
//!   l'application (`network_mode: service:…`), pour se parler par `localhost`.
//!
//! Les sections `docker`, `compose` et `k8s` écrites à la main restent possibles : elles
//! complètent ce que les applications génèrent, ou le remplacent clé par clé.

mod stack;

use super::eval::{AppValue, SectionValue, merge};
use super::parser::Section;
use super::{Error, Pos, Value};

/// Les clés d'une application.
pub const KEYS: &[&str] = &[
    "image",
    "version",
    "build",
    "port",
    "ports",
    "replicas",
    "env",
    "command",
    "volumes",
    "depends_on",
    "host",
    "with",
];

/// Les clés d'un conteneur d'accompagnement (`with`).
pub const WITH_KEYS: &[&str] = &["image", "version", "port", "ports", "env", "command"];

/// Les images du Dockerfile de `build = "rust"`.
const RUST_BUILD: &str = "rust:1.99-slim";
const RUST_RUN: &str = "debian:trixie-slim";

/// Les sections générées par les applications, complétées par les sections écrites.
pub fn expand(apps: &[AppValue], explicit: Vec<SectionValue>) -> Result<Vec<SectionValue>, Error> {
    let mut docker: Option<(Value, Pos)> = None;
    let mut services = Vec::new();
    let mut k8s: Vec<(String, Value)> = Vec::new();
    // Compose publie chaque port sur la machine : deux applications ne peuvent pas
    // prendre le même.
    let mut published: Vec<(i64, String)> = Vec::new();
    let mut volumes: Vec<(String, Value)> = Vec::new();
    let mut configs: Vec<(String, Value)> = Vec::new();
    for app in apps {
        // Une pile (`web { rust = ".", nginx = "1.29", postgres = "18" }`).
        if stack::is_stack(app) {
            let out = stack::expand(app)?;
            for (public, inside) in out.published {
                if let Some((_, owner)) = published.iter().find(|(p, _)| *p == public) {
                    return Err(Error::at(
                        app.pos,
                        format!(
                            "`{}` publie le port {public}, déjà publié par `{owner}` (port {inside} à l'intérieur)",
                            app.name
                        ),
                    ));
                }
                published.push((public, app.name.clone()));
            }
            if let Some(dockerfile) = out.dockerfile {
                if docker.is_some() {
                    return Err(Error::at(
                        app.pos,
                        "une seule application peut être construite en Rust (un seul Dockerfile)",
                    ));
                }
                docker = Some((dockerfile, app.pos));
            }
            services.extend(out.services);
            for (kind, value) in out.k8s {
                if let Value::List(items) = value {
                    for item in items {
                        push(&mut k8s, &kind, item);
                    }
                }
            }
            volumes.extend(out.volumes);
            configs.extend(out.configs);
            continue;
        }
        let a = App::read(app, apps)?;
        for (public, inside) in a.all_ports() {
            if let Some((_, owner)) = published.iter().find(|(p, _)| *p == public) {
                return Err(Error::at(
                    a.port_pos(),
                    format!(
                        "`{}` : le port {public} est déjà publié par `{owner}` ; choisissez un autre port public, par exemple `port = \"{}:{inside}\"`",
                        a.main.name,
                        free_port(public, &published)
                    ),
                ));
            }
            published.push((public, a.main.name.clone()));
        }
        if let Some(dockerfile) = a.dockerfile() {
            if docker.is_some() {
                return Err(Error::at(
                    app.pos,
                    "une seule application peut être construite en Rust (un seul Dockerfile)",
                ));
            }
            docker = Some((dockerfile, app.pos));
        }
        services.extend(a.services());
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
        let mut compose = vec![("services".to_owned(), Value::Object(services))];
        if !volumes.is_empty() {
            compose.push(("volumes".to_owned(), Value::Object(volumes)));
        }
        if !configs.is_empty() {
            compose.push(("configs".to_owned(), Value::Object(configs)));
        }
        sections.push(SectionValue {
            section: Section::Compose,
            value: Value::Object(compose),
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

fn list(items: &[String]) -> Value {
    Value::List(items.iter().cloned().map(Value::Str).collect())
}

/// Un nom d'application ou de conteneur : minuscules, chiffres et `-`.
fn valid_name(name: &str) -> bool {
    name.len() <= 63
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Les champs d'un objet, avec ce qu'il faut pour situer et nommer les erreurs.
struct Fields<'a, 'p> {
    /// `web`, ou `web.with.proxy`.
    label: String,
    fields: &'a [(String, Value)],
    pos_of: &'p dyn Fn(&str) -> Pos,
}

impl<'a> Fields<'a, '_> {
    fn err(&self, key: &str, message: impl std::fmt::Display) -> Error {
        Error::at((self.pos_of)(key), format!("`{}` : {message}", self.label))
    }

    fn get(&self, key: &str) -> Option<&'a Value> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn check_keys(&self, allowed: &[&str]) -> Result<(), Error> {
        for (key, _) in self.fields {
            if !allowed.contains(&key.as_str()) {
                return Err(self.err(
                    key,
                    format!(
                        "clé inconnue `{key}`{} (possibles : {})",
                        super::suggest(key, allowed.iter().copied()),
                        allowed.join(", ")
                    ),
                ));
            }
        }
        Ok(())
    }

    fn text(&self, key: &str) -> Result<Option<&'a str>, Error> {
        match self.get(key) {
            None => Ok(None),
            Some(Value::Str(v)) if !v.trim().is_empty() => Ok(Some(v.as_str())),
            Some(other) => Err(self.err(
                key,
                format!(
                    "`{key}` doit être un texte non vide, pas {}",
                    other.type_name()
                ),
            )),
        }
    }

    fn words(&self, key: &str) -> Result<Vec<String>, Error> {
        match self.get(key) {
            None => Ok(Vec::new()),
            Some(Value::List(items)) => items
                .iter()
                .map(|item| match item {
                    Value::Str(v) => Ok(v.clone()),
                    other => Err(self.err(
                        key,
                        format!(
                            "`{key}` ne contient que des textes, pas {}",
                            other.type_name()
                        ),
                    )),
                })
                .collect(),
            Some(Value::Str(v)) => Ok(vec![v.clone()]),
            Some(other) => Err(self.err(
                key,
                format!(
                    "`{key}` est un texte ou une liste de textes, pas {}",
                    other.type_name()
                ),
            )),
        }
    }

    /// `command = "nginx -g 'daemon off;'"` (découpé aux espaces) ou une liste.
    fn command(&self) -> Result<Vec<String>, Error> {
        match self.get("command") {
            Some(Value::Str(line)) => Ok(line.split_whitespace().map(str::to_owned).collect()),
            _ => self.words("command"),
        }
    }

    fn env(&self) -> Result<Vec<(String, String)>, Error> {
        match self.get("env") {
            None => Ok(Vec::new()),
            Some(Value::Object(vars)) => vars
                .iter()
                .map(|(k, v)| match scalar(v) {
                    Some(v) => Ok((k.clone(), v)),
                    None => Err(self.err(
                        "env",
                        format!(
                            "`env.{k}` doit être un texte, un nombre ou un booléen, pas {}",
                            v.type_name()
                        ),
                    )),
                })
                .collect(),
            Some(other) => Err(self.err(
                "env",
                format!(
                    "`env` s'écrit `{{ CLE = \"valeur\" }}`, pas {}",
                    other.type_name()
                ),
            )),
        }
    }

    /// L'image et sa version (`image = "postgres"`, `version = "18-alpine"`). `default`
    /// est l'image d'une application construite (`build`), sans version.
    fn image(&self, default: Option<String>) -> Result<String, Error> {
        let version = match self.get("version") {
            None => None,
            Some(value) => match scalar(value).filter(|v| !v.trim().is_empty()) {
                Some(v) => Some(v),
                None => {
                    return Err(self.err(
                        "version",
                        format!("`version` est un texte, pas {}", value.type_name()),
                    ));
                }
            },
        };
        match (self.text("image")?, version, default) {
            (Some(image), Some(version), _) => {
                if has_version(image) {
                    return Err(self.err(
                        "version",
                        format!(
                            "l'image `{image}` a déjà une version ; écrivez-la dans `image` ou dans `version`, pas les deux"
                        ),
                    ));
                }
                Ok(format!("{image}:{version}"))
            }
            (Some(image), None, _) => Ok(image.to_owned()),
            (None, version, Some(default)) => Ok(format!(
                "{default}:{}",
                version.as_deref().unwrap_or("latest")
            )),
            (None, Some(_), None) => Err(self.err(
                "version",
                "`version` a besoin d'une `image` (ex. `image = \"postgres\"`)",
            )),
            (None, None, None) => Err(self.err(
                "image",
                "il faut une image (`image = \"nginx:1.29\"`) ou `build = \"rust\"`",
            )),
        }
    }

    /// `port`, `ports`, ou le port connu de l'image.
    fn ports(&self, image: &str) -> Result<Vec<(i64, i64)>, Error> {
        let known = known_port(image);
        let one = |value: &Value, single: bool| -> Result<(i64, i64), String> {
            match value {
                // Un nombre seul, pour une image connue : le port public de son port.
                Value::Int(p) if valid_port(*p) => match known {
                    Some(inside) if single => Ok((*p, inside)),
                    _ => Ok((*p, *p)),
                },
                Value::Str(text) => {
                    let parse = |t: &str| t.trim().parse::<i64>().ok().filter(|p| valid_port(*p));
                    match text.split_once(':') {
                        Some((public, inside)) => match (parse(public), parse(inside)) {
                            (Some(public), Some(inside)) => Ok((public, inside)),
                            _ => Err(format!("`{text}` n'est pas un port valide")),
                        },
                        None => parse(text)
                            .map(|p| (p, p))
                            .ok_or_else(|| format!("`{text}` n'est pas un port valide")),
                    }
                }
                _ => Err(String::new()),
            }
        };
        let explain = |key: &str, why: String| {
            let help = "un port s'écrit `3000`, ou `\"8080:80\"` (publié sur 8080, l'application écoute sur 80), entre 1 et 65535";
            let sep = if why.is_empty() { "" } else { " ; " };
            self.err(key, format!("{why}{sep}{help}"))
        };
        let ports = match (self.get("port"), self.get("ports")) {
            (Some(_), Some(_)) => {
                return Err(self.err("ports", "écrivez `port` ou `ports`, pas les deux"));
            }
            (Some(Value::Bool(false)), None) => Vec::new(),
            (Some(value), None) => vec![one(value, true).map_err(|why| explain("port", why))?],
            (None, Some(Value::List(items))) => items
                .iter()
                .map(|item| one(item, false).map_err(|why| explain("ports", why)))
                .collect::<Result<_, _>>()?,
            (None, Some(_)) => {
                return Err(self.err(
                    "ports",
                    "`ports` est une liste, par exemple `ports = [80, \"8443:443\"]`",
                ));
            }
            (None, None) => known.map(|p| vec![(p, p)]).unwrap_or_default(),
        };
        for (i, (public, _)) in ports.iter().enumerate() {
            if ports[..i].iter().any(|(p, _)| p == public) {
                return Err(self.err("ports", format!("le port {public} est publié deux fois")));
            }
        }
        Ok(ports)
    }
}

/// Un texte, un nombre ou un booléen, écrit en texte.
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Str(v) => Some(v.clone()),
        Value::Int(n) => Some(n.to_string()),
        Value::Float(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// `postgres:18` ou `nginx@sha256:…` ont une version ; `localhost:5000/app` non.
fn has_version(image: &str) -> bool {
    image.contains('@') || image.rsplit('/').next().unwrap_or(image).contains(':')
}

/// Un conteneur : celui de l'application, ou l'un de ceux qui l'accompagnent.
struct Container {
    name: String,
    image: String,
    /// (port publié, port du conteneur), dans l'ordre écrit.
    ports: Vec<(i64, i64)>,
    env: Vec<(String, String)>,
    command: Vec<String>,
}

impl Container {
    /// Les ports sur lesquels le conteneur écoute, sans doublon.
    fn inside(&self) -> Vec<i64> {
        let mut inside: Vec<i64> = Vec::new();
        for (_, port) in &self.ports {
            if !inside.contains(port) {
                inside.push(*port);
            }
        }
        inside
    }

    fn k8s(&self) -> Value {
        let mut container = vec![
            ("name".to_owned(), s(self.name.clone())),
            ("image".to_owned(), s(self.image.clone())),
        ];
        if !self.command.is_empty() {
            container.push(("args".to_owned(), list(&self.command)));
        }
        if !self.ports.is_empty() {
            container.push((
                "ports".to_owned(),
                Value::List(
                    self.inside()
                        .into_iter()
                        .map(|port| obj([("containerPort", Value::Int(port))]))
                        .collect(),
                ),
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
        Value::Object(container)
    }

    fn environment(&self) -> Value {
        Value::Object(
            self.env
                .iter()
                .map(|(k, v)| (k.clone(), s(v.clone())))
                .collect(),
        )
    }
}

/// Une application lue et vérifiée.
struct App<'a> {
    main: Container,
    /// Les conteneurs du même pod (`with`), dans l'ordre écrit.
    with: Vec<Container>,
    /// `build = "rust"` : un Dockerfile Rust ; un autre texte : le dossier de construction.
    build: Option<&'a str>,
    replicas: i64,
    volumes: Vec<String>,
    depends_on: Vec<String>,
    host: Option<&'a str>,
    source: &'a AppValue,
}

impl<'a> App<'a> {
    fn read(app: &'a AppValue, all: &[AppValue]) -> Result<Self, Error> {
        let name = app.name.as_str();
        if !valid_name(name) {
            return Err(Error::at(
                app.pos,
                format!(
                    "`{name}` : le nom s'écrit en minuscules, chiffres et `-` (ex. `mon-api {{ … }}`)"
                ),
            ));
        }
        let Value::Object(fields) = &app.value else {
            return Err(Error::at(app.pos, format!("`{name}` attend `{{ … }}`")));
        };
        // Une erreur sur une clé est située sur cette clé.
        let pos_of = |key: &str| {
            app.keys
                .iter()
                .find(|(k, _)| k == key)
                .map_or(app.pos, |(_, pos)| *pos)
        };
        let f = Fields {
            label: name.to_owned(),
            fields,
            pos_of: &pos_of,
        };
        f.check_keys(KEYS)?;

        let build = f.text("build")?;
        let image = f.image(build.map(|_| name.to_owned()))?;
        let main = Container {
            name: name.to_owned(),
            ports: f.ports(&image)?,
            env: f.env()?,
            command: f.command()?,
            image,
        };

        let with_pos = |_: &str| pos_of("with");
        let mut with = Vec::new();
        match f.get("with") {
            None => {}
            Some(Value::Object(containers)) => {
                for (side, value) in containers {
                    let label = format!("{name}.with.{side}");
                    if !valid_name(side) || side == name {
                        return Err(Error::at(
                            with_pos(""),
                            format!(
                                "`{label}` : le nom d'un conteneur s'écrit en minuscules, chiffres et `-`, et diffère de celui de l'application"
                            ),
                        ));
                    }
                    if all.iter().any(|a| a.name == format!("{name}-{side}")) {
                        return Err(Error::at(
                            with_pos(""),
                            format!(
                                "`{label}` : Compose l'appellerait `{name}-{side}`, nom déjà pris par une application"
                            ),
                        ));
                    }
                    let Value::Object(fields) = value else {
                        return Err(Error::at(
                            with_pos(""),
                            format!(
                                "`{label}` s'écrit `{{ image = \"…\" }}`, pas {}",
                                value.type_name()
                            ),
                        ));
                    };
                    let g = Fields {
                        label,
                        fields,
                        pos_of: &with_pos,
                    };
                    g.check_keys(WITH_KEYS)?;
                    let image = g.image(None)?;
                    with.push(Container {
                        name: side.clone(),
                        ports: g.ports(&image)?,
                        env: g.env()?,
                        command: g.command()?,
                        image,
                    });
                }
            }
            Some(other) => {
                return Err(f.err(
                    "with",
                    format!(
                        "`with` s'écrit `{{ proxy = {{ image = \"…\" }} }}`, pas {}",
                        other.type_name()
                    ),
                ));
            }
        }
        // Les conteneurs d'un pod partagent `localhost` : un port n'y sert qu'une fois.
        let mut used: Vec<(i64, &str)> = Vec::new();
        for container in std::iter::once(&main).chain(&with) {
            for port in container.inside() {
                if let Some((_, other)) = used.iter().find(|(p, _)| *p == port) {
                    return Err(f.err(
                        "with",
                        format!(
                            "`{other}` et `{}` écoutent tous deux sur le port {port}, alors qu'ils partagent le même réseau",
                            container.name
                        ),
                    ));
                }
                used.push((port, &container.name));
            }
        }

        let replicas = match f.get("replicas") {
            None => 1,
            Some(Value::Int(n)) if *n >= 0 => *n,
            Some(_) => return Err(f.err("replicas", "`replicas` est un nombre entier positif")),
        };
        let depends_on = f.words("depends_on")?;
        for other in &depends_on {
            if !all.iter().any(|a| a.name == *other) {
                return Err(f.err(
                    "depends_on",
                    format!(
                        "dépend de `{other}`, qui n'existe pas{}",
                        super::suggest(other, all.iter().map(|a| a.name.as_str()))
                    ),
                ));
            }
        }
        let host = f.text("host")?;
        if host.is_some() && main.ports.is_empty() {
            return Err(f.err("host", "`host` publie l'application, il lui faut un `port`"));
        }
        Ok(Self {
            main,
            with,
            build,
            replicas,
            volumes: f.words("volumes")?,
            depends_on,
            host,
            source: app,
        })
    }

    /// Tous les ports publiés : ceux de l'application, puis ceux de ses conteneurs.
    fn all_ports(&self) -> Vec<(i64, i64)> {
        std::iter::once(&self.main)
            .chain(&self.with)
            .flat_map(|c| c.ports.iter().copied())
            .collect()
    }

    /// Où situer une erreur de port : `port`, `ports`, `with`, ou l'application.
    fn port_pos(&self) -> Pos {
        ["port", "ports", "with"]
            .iter()
            .find_map(|key| {
                self.source
                    .keys
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, pos)| *pos)
            })
            .unwrap_or(self.source.pos)
    }

    fn labels(&self) -> Value {
        obj([("app", s(self.main.name.clone()))])
    }

    fn dockerfile(&self) -> Option<Value> {
        if self.build != Some("rust") {
            return None;
        }
        let binary = format!("target/release/{}", self.main.name);
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
        if !self.main.ports.is_empty() {
            docker.push((
                "expose".to_owned(),
                Value::List(self.main.inside().into_iter().map(Value::Int).collect()),
            ));
        }
        docker.push((
            "run".to_owned(),
            Value::List(vec![s(format!("./{binary}"))]),
        ));
        Some(Value::Object(docker))
    }

    /// Le service Compose de l'application, puis un service par conteneur `with`.
    fn services(&self) -> Vec<(String, Value)> {
        let name = &self.main.name;
        let mut service = Vec::new();
        match self.build {
            Some("rust") => service.push(("build".to_owned(), s("."))),
            Some(dir) => service.push(("build".to_owned(), s(dir))),
            None => {}
        }
        service.push(("image".to_owned(), s(self.main.image.clone())));
        if !self.main.command.is_empty() {
            service.push(("command".to_owned(), list(&self.main.command)));
        }
        // Les conteneurs `with` partagent le réseau de l'application : leurs ports sont
        // publiés par elle.
        let ports = self.all_ports();
        if !ports.is_empty() {
            service.push((
                "ports".to_owned(),
                Value::List(
                    ports
                        .iter()
                        .map(|(public, inside)| s(format!("{public}:{inside}")))
                        .collect(),
                ),
            ));
        }
        if !self.main.env.is_empty() {
            service.push(("environment".to_owned(), self.main.environment()));
        }
        if !self.volumes.is_empty() {
            service.push(("volumes".to_owned(), list(&self.volumes)));
        }
        if !self.depends_on.is_empty() {
            service.push(("depends_on".to_owned(), list(&self.depends_on)));
        }
        let mut services = vec![(name.clone(), Value::Object(service))];
        for side in &self.with {
            let mut service = vec![("image".to_owned(), s(side.image.clone()))];
            if !side.command.is_empty() {
                service.push(("command".to_owned(), list(&side.command)));
            }
            if !side.env.is_empty() {
                service.push(("environment".to_owned(), side.environment()));
            }
            service.push(("network_mode".to_owned(), s(format!("service:{name}"))));
            service.push(("depends_on".to_owned(), Value::List(vec![s(name.clone())])));
            services.push((format!("{name}-{}", side.name), Value::Object(service)));
        }
        services
    }

    fn deployment(&self) -> Value {
        let containers = std::iter::once(&self.main)
            .chain(&self.with)
            .map(Container::k8s)
            .collect();
        obj([
            ("name", s(self.main.name.clone())),
            ("labels", self.labels()),
            ("replicas", Value::Int(self.replicas)),
            (
                "template",
                obj([
                    ("metadata", obj([("labels", self.labels())])),
                    ("spec", obj([("containers", Value::List(containers))])),
                ]),
            ),
        ])
    }

    fn k8s_service(&self) -> Option<Value> {
        let all = self.all_ports();
        if all.is_empty() {
            return None;
        }
        // Kubernetes exige un nom pour chaque port quand il y en a plusieurs.
        let named = all.len() > 1;
        let ports = all
            .iter()
            .map(|(public, inside)| {
                let mut port = Vec::new();
                if named {
                    port.push(("name".to_owned(), s(format!("p{public}"))));
                }
                port.push(("port".to_owned(), Value::Int(*public)));
                port.push(("targetPort".to_owned(), Value::Int(*inside)));
                Value::Object(port)
            })
            .collect();
        Some(obj([
            ("name", s(self.main.name.clone())),
            ("selector", self.labels()),
            ("ports", Value::List(ports)),
        ]))
    }

    fn ingress(&self) -> Option<Value> {
        // L'Ingress mène au premier port de l'application.
        let (host, port) = (self.host?, self.main.ports.first()?.0);
        let backend = obj([(
            "service",
            obj([
                ("name", s(self.main.name.clone())),
                ("port", obj([("number", Value::Int(port))])),
            ]),
        )]);
        Some(obj([
            ("name", s(self.main.name.clone())),
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

fn valid_port(port: i64) -> bool {
    (1..=65535).contains(&port)
}

/// Le premier port libre à partir de `port + 1`, pour la suggestion d'une collision.
fn free_port(port: i64, taken: &[(i64, String)]) -> i64 {
    (port + 1..=65535)
        .find(|p| !taken.iter().any(|(t, _)| t == p))
        .unwrap_or(port)
}

/// `docker.io/library/postgres:18-alpine` → 5432.
fn known_port(image: &str) -> Option<i64> {
    let name = image.rsplit('/').next().unwrap_or(image);
    let name = name.split([':', '@']).next().unwrap_or(name);
    KNOWN_PORTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, p)| *p)
}
