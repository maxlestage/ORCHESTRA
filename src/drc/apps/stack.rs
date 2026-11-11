//! Une pile web : l'application et ce qui l'entoure, une ligne par technologie.
//!
//! ```text
//! web {
//!     rust     = "."        # le code de l'application
//!     nginx    = "1.29"     # devant, renvoie vers l'application
//!     postgres = "18"
//!     redis    = "8"
//! }
//! ```
//!
//! Le compilateur relie tout : configuration nginx, `DATABASE_URL` et `REDIS_URL` dans
//! l'application, volumes de données, StatefulSet et Secret Kubernetes, Ingress pour `host`.

use super::{Fields, list, obj, push, s, scalar, valid_name};
use crate::drc::eval::AppValue;
use crate::drc::{Error, Value};

/// Les technologies qu'une pile sait assembler.
pub const TECHS: &[&str] = &[
    "rust", "nginx", "postgres", "mysql", "mariadb", "mongo", "redis", "valkey",
];

/// Toutes les clés d'une pile.
pub const KEYS: &[&str] = &[
    "rust", "src", "image", "version", "port", "replicas", "env", "host", "password", "nginx",
    "postgres", "mysql", "mariadb", "mongo", "redis", "valkey",
];

/// Une pile : un bloc qui nomme au moins une technologie.
pub fn is_stack(app: &AppValue) -> bool {
    matches!(&app.value, Value::Object(fields)
        if fields.iter().any(|(k, _)| TECHS.contains(&k.as_str())))
}

/// Une base de données ou un cache de la pile.
struct Store {
    tech: &'static str,
    image: &'static str,
    port: i64,
    /// Où sont les données (pas de volume pour un cache).
    data: Option<&'static str>,
    /// La variable donnée à l'application.
    url_var: &'static str,
}

const STORES: &[Store] = &[
    store(
        "postgres",
        "postgres",
        5432,
        Some("/var/lib/postgresql/data"),
        "DATABASE_URL",
    ),
    store(
        "mysql",
        "mysql",
        3306,
        Some("/var/lib/mysql"),
        "DATABASE_URL",
    ),
    store(
        "mariadb",
        "mariadb",
        3306,
        Some("/var/lib/mysql"),
        "DATABASE_URL",
    ),
    store("mongo", "mongo", 27017, Some("/data/db"), "MONGODB_URL"),
    store("redis", "redis", 6379, None, "REDIS_URL"),
    store("valkey", "valkey/valkey", 6379, None, "REDIS_URL"),
];

const fn store(
    tech: &'static str,
    image: &'static str,
    port: i64,
    data: Option<&'static str>,
    url_var: &'static str,
) -> Store {
    Store {
        tech,
        image,
        port,
        data,
        url_var,
    }
}

/// Ce qu'une pile ajoute au projet.
#[derive(Default)]
pub struct Output {
    pub services: Vec<(String, Value)>,
    pub k8s: Vec<(String, Value)>,
    pub volumes: Vec<(String, Value)>,
    pub configs: Vec<(String, Value)>,
    pub dockerfile: Option<Value>,
    /// Les ports publiés par Compose : (public, intérieur).
    pub published: Vec<(i64, i64)>,
}

/// Le port de l'application quand il n'est pas écrit.
const DEFAULT_PORT: i64 = 8080;

pub fn expand(app: &AppValue) -> Result<Output, Error> {
    let name = app.name.as_str();
    if !valid_name(name) {
        return Err(Error::at(
            app.pos,
            format!("`{name}` : le nom s'écrit en minuscules, chiffres et `-` (ex. `web {{ … }}`)"),
        ));
    }
    let Value::Object(fields) = &app.value else {
        unreachable!("une pile est un objet");
    };
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
    for (key, _) in fields {
        if !KEYS.contains(&key.as_str()) {
            let hint = if super::KEYS.contains(&key.as_str()) {
                format!(" : `{key}` sert aux applications simples, pas aux piles")
            } else {
                super::super::suggest(key, KEYS.iter().copied())
            };
            return Err(f.err(
                key,
                format!(
                    "clé inconnue `{key}`{hint} (possibles : {})",
                    KEYS.join(", ")
                ),
            ));
        }
    }

    // L'application : du code Rust à construire, ou une image toute faite.
    // `rust = "."` (un dossier) ou `rust = "1.99-alpine"` (une variante de Rust) ;
    // `src` donne le dossier quand la variante est écrite.
    let rust = match f.get("rust") {
        None => None,
        Some(Value::Bool(true)) => Some((".".to_owned(), super::RUST_DEFAULT.to_owned())),
        Some(Value::Str(path)) if path.starts_with(['.', '/']) => {
            Some((path.clone(), super::RUST_DEFAULT.to_owned()))
        }
        Some(Value::Str(tag)) if super::rust_tag(tag) => {
            let src = f.text("src")?.unwrap_or(".").to_owned();
            Some((src, tag.clone()))
        }
        Some(_) => {
            return Err(f.err(
                "rust",
                "`rust` est le dossier du code (`rust = \".\"`) ou une variante de Rust (`rust = \"1.99-alpine\"`, `rust = \"1.99-slim-bookworm\"`)",
            ));
        }
    };
    if f.get("src").is_some()
        && !matches!(f.get("rust"), Some(Value::Str(t)) if !t.starts_with(['.', '/']))
    {
        return Err(f.err(
            "src",
            "`src` est le dossier du code quand `rust` donne une variante (ex. `rust = \"1.99-alpine\"`, `src = \"./api\"`)",
        ));
    }
    if rust.is_none() && f.get("image").is_none() {
        return Err(f.err(
            "image",
            "une pile a besoin de son application : `rust = \".\"` pour construire votre code, ou `image = \"…\"`",
        ));
    }
    let image = f.image(rust.as_ref().map(|_| name.to_owned()))?;
    let port = match f.get("port") {
        None => DEFAULT_PORT,
        Some(Value::Int(p)) if (1..=65535).contains(p) => *p,
        Some(_) => {
            return Err(f.err(
                "port",
                "`port` est le port de l'application, un nombre entre 1 et 65535 (8080 par défaut)",
            ));
        }
    };
    let replicas = match f.get("replicas") {
        None => 1,
        Some(Value::Int(n)) if *n >= 0 => *n,
        Some(_) => return Err(f.err("replicas", "`replicas` est un nombre entier positif")),
    };
    let password = match f.get("password") {
        None => "change-me".to_owned(),
        Some(Value::Str(p)) if !p.is_empty() => p.clone(),
        Some(_) => return Err(f.err("password", "`password` est un texte non vide")),
    };
    let host = f.text("host")?;

    // Les technologies présentes, avec leur version.
    let version = |tech: &str| -> Result<Option<String>, Error> {
        match f.get(tech) {
            None => Ok(None),
            Some(Value::Bool(true)) => Ok(Some("latest".to_owned())),
            Some(value) => match scalar(value) {
                Some(v) if !v.is_empty() && !v.contains([' ', ':', '/']) => Ok(Some(v)),
                _ => Err(f.err(
                    tech,
                    format!(
                        "`{tech}` est une version, par exemple `{tech} = \"{}\"`",
                        example(tech)
                    ),
                )),
            },
        }
    };
    let nginx = version("nginx")?;
    let mut stores = Vec::new();
    for store in STORES {
        if let Some(v) = version(store.tech)? {
            if let Some((other, _)) = stores
                .iter()
                .find(|(s, _): &&(&Store, String)| s.url_var == store.url_var)
            {
                return Err(f.err(
                    store.tech,
                    format!(
                        "`{}` et `{}` donneraient tous deux `{}` à l'application : gardez-en un",
                        other.tech, store.tech, store.url_var
                    ),
                ));
            }
            stores.push((store, v));
        }
    }

    let mut out = Output::default();
    let host_of = |tech: &str| format!("{name}-{tech}");

    // L'environnement de l'application : son port, l'adresse de chaque base, puis le
    // sien. Ce qui contient un mot de passe va dans le Secret Kubernetes.
    let mut env: Vec<(String, String)> = vec![("PORT".into(), port.to_string())];
    let mut secret: Vec<(String, Value)> = Vec::new();
    for (store, _) in &stores {
        let host = host_of(store.tech);
        let pw = encode(&password);
        let url = match store.tech {
            "postgres" => format!("postgres://postgres:{pw}@{host}:5432/{name}"),
            "mysql" | "mariadb" => format!("mysql://root:{pw}@{host}:3306/{name}"),
            "mongo" => format!("mongodb://root:{pw}@{host}:27017/{name}?authSource=admin"),
            _ => format!("redis://{host}:6379"),
        };
        if store.data.is_some() {
            secret.push((store.url_var.to_owned(), s(url.clone())));
        }
        env.push((store.url_var.to_owned(), url));
    }
    env.extend(f.env()?);

    // --- L'application.
    let mut service = Vec::new();
    if rust.is_some() {
        service.push(("build".to_owned(), s(".")));
    }
    service.push(("image".to_owned(), s(image.clone())));
    if nginx.is_none() {
        service.push(("ports".to_owned(), list(&[format!("{port}:{port}")])));
        out.published.push((port, port));
    }
    service.push(("environment".to_owned(), environment(&env)));
    let depends: Vec<String> = stores.iter().map(|(st, _)| host_of(st.tech)).collect();
    if !depends.is_empty() {
        service.push(("depends_on".to_owned(), list(&depends)));
    }
    out.services.push((name.to_owned(), Value::Object(service)));

    let plain: Vec<(String, String)> = env
        .iter()
        .filter(|(k, _)| !secret.iter().any(|(sk, _)| sk == k))
        .cloned()
        .collect();
    let mut main = container(name, &image, &[port], &plain);
    if !secret.is_empty() {
        envfrom(&mut main, name);
    }
    push(
        &mut out.k8s,
        "deployment",
        workload(name, replicas, vec![main], vec![], None),
    );
    push(&mut out.k8s, "service", k8s_service(name, port, port));

    if let Some((path, tag)) = &rust {
        out.dockerfile = Some(dockerfile(path, tag, port));
    }

    // --- nginx, devant l'application.
    let mut entry = (name.to_owned(), port);
    if let Some(v) = &nginx {
        let svc = host_of("nginx");
        let conf = nginx_conf(name, port);
        out.configs
            .push((svc.clone(), obj([("content", s(conf.replace('$', "$$")))])));
        out.services.push((
            svc.clone(),
            obj([
                ("image", s(format!("nginx:{v}"))),
                ("ports", list(&["80:80".to_owned()])),
                (
                    "configs",
                    Value::List(vec![obj([
                        ("source", s(svc.clone())),
                        ("target", s("/etc/nginx/conf.d/default.conf")),
                    ])]),
                ),
                ("depends_on", list(&[name.to_owned()])),
            ]),
        ));
        out.published.push((80, 80));
        push(
            &mut out.k8s,
            "configmap",
            obj([
                ("name", s(svc.clone())),
                ("data", obj([("default.conf", s(conf))])),
            ]),
        );
        let mut nginx_container = container(&svc, &format!("nginx:{v}"), &[80], &[]);
        if let Value::Object(fields) = &mut nginx_container {
            fields.push((
                "volumeMounts".to_owned(),
                Value::List(vec![obj([
                    ("name", s("conf")),
                    ("mountPath", s("/etc/nginx/conf.d")),
                ])]),
            ));
        }
        push(
            &mut out.k8s,
            "deployment",
            workload(
                &svc,
                1,
                vec![nginx_container],
                vec![obj([
                    ("name", s("conf")),
                    ("configMap", obj([("name", s(svc.clone()))])),
                ])],
                None,
            ),
        );
        push(&mut out.k8s, "service", k8s_service(&svc, 80, 80));
        entry = (svc, 80);
    }

    // --- Bases de données et caches.
    for (store, v) in &stores {
        let svc = host_of(store.tech);
        let image = format!("{}:{v}", store.image);
        let (open, private) = store_env(store.tech, name, &password);
        let mut service = vec![("image".to_owned(), s(image.clone()))];
        let all: Vec<(String, String)> = open.iter().chain(&private).cloned().collect();
        if !all.is_empty() {
            service.push(("environment".to_owned(), environment(&all)));
        }
        if let Some(data) = store.data {
            service.push(("volumes".to_owned(), list(&[format!("{svc}:{data}")])));
            out.volumes.push((svc.clone(), Value::Object(Vec::new())));
        }
        out.services.push((svc.clone(), Value::Object(service)));

        let mut c = container(&svc, &image, &[store.port], &open);
        for (key, value) in &private {
            secret.push((key.clone(), s(value.clone())));
        }
        if !private.is_empty() {
            envfrom(&mut c, name);
        }
        match store.data {
            // Une base garde ses données : un StatefulSet et son volume persistant.
            Some(data) => {
                if let Value::Object(fields) = &mut c {
                    fields.push((
                        "volumeMounts".to_owned(),
                        Value::List(vec![obj([
                            ("name", s("data")),
                            ("mountPath", s(data)),
                            ("subPath", s(store.tech)),
                        ])]),
                    ));
                }
                push(
                    &mut out.k8s,
                    "statefulset",
                    workload(&svc, 1, vec![c], vec![], Some(&svc)),
                );
            }
            None => push(
                &mut out.k8s,
                "deployment",
                workload(&svc, 1, vec![c], vec![], None),
            ),
        }
        push(
            &mut out.k8s,
            "service",
            k8s_service(&svc, store.port, store.port),
        );
    }

    if !secret.is_empty() {
        push(
            &mut out.k8s,
            "secret",
            obj([("name", s(name)), ("stringData", Value::Object(secret))]),
        );
    }
    if let Some(host) = host {
        push(
            &mut out.k8s,
            "ingress",
            ingress(name, host, &entry.0, entry.1),
        );
    }
    Ok(out)
}

/// L'exemple de version d'une technologie, pour les messages d'erreur.
fn example(tech: &str) -> &'static str {
    match tech {
        "nginx" => "1.29",
        "postgres" => "18",
        "mysql" => "9.5",
        "mariadb" => "12",
        "mongo" => "8",
        "redis" => "8",
        "valkey" => "9",
        _ => "1",
    }
}

/// Les variables d'une base : (visibles, secrètes).
type Vars = Vec<(String, String)>;
fn store_env(tech: &str, db: &str, password: &str) -> (Vars, Vars) {
    let pair = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    match tech {
        // `PGDATA` dans le volume : depuis Postgres 18, l'image range sinon ses données
        // ailleurs (`/var/lib/postgresql/18/docker`), hors du volume.
        "postgres" => (
            vec![
                pair("POSTGRES_DB", db),
                pair("PGDATA", "/var/lib/postgresql/data"),
            ],
            vec![pair("POSTGRES_PASSWORD", password)],
        ),
        "mysql" => (
            vec![pair("MYSQL_DATABASE", db)],
            vec![pair("MYSQL_ROOT_PASSWORD", password)],
        ),
        "mariadb" => (
            vec![pair("MARIADB_DATABASE", db)],
            vec![pair("MARIADB_ROOT_PASSWORD", password)],
        ),
        "mongo" => (
            vec![pair("MONGO_INITDB_ROOT_USERNAME", "root")],
            vec![pair("MONGO_INITDB_ROOT_PASSWORD", password)],
        ),
        _ => (Vec::new(), Vec::new()),
    }
}

/// Un mot de passe dans une URL : les caractères réservés sont encodés.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn environment(env: &[(String, String)]) -> Value {
    Value::Object(env.iter().map(|(k, v)| (k.clone(), s(v.clone()))).collect())
}

fn container(name: &str, image: &str, ports: &[i64], env: &[(String, String)]) -> Value {
    let mut c = vec![
        ("name".to_owned(), s(name)),
        ("image".to_owned(), s(image)),
        (
            "ports".to_owned(),
            Value::List(
                ports
                    .iter()
                    .map(|p| obj([("containerPort", Value::Int(*p))]))
                    .collect(),
            ),
        ),
    ];
    if !env.is_empty() {
        c.push((
            "env".to_owned(),
            Value::List(
                env.iter()
                    .map(|(k, v)| obj([("name", s(k.clone())), ("value", s(v.clone()))]))
                    .collect(),
            ),
        ));
    }
    Value::Object(c)
}

/// Les variables secrètes de la pile, lues dans son Secret.
fn envfrom(container: &mut Value, secret: &str) {
    if let Value::Object(fields) = container {
        fields.push((
            "envFrom".to_owned(),
            Value::List(vec![obj([("secretRef", obj([("name", s(secret))]))])]),
        ));
    }
}

/// Un Deployment, ou un StatefulSet avec son volume quand `stateful` nomme son Service.
fn workload(
    name: &str,
    replicas: i64,
    containers: Vec<Value>,
    volumes: Vec<Value>,
    stateful: Option<&str>,
) -> Value {
    let labels = obj([("app", s(name))]);
    let mut spec = vec![("containers".to_owned(), Value::List(containers))];
    if !volumes.is_empty() {
        spec.push(("volumes".to_owned(), Value::List(volumes)));
    }
    let mut w = vec![
        ("name".to_owned(), s(name)),
        ("labels".to_owned(), labels.clone()),
    ];
    if let Some(service) = stateful {
        w.push(("serviceName".to_owned(), s(service)));
    }
    w.push(("replicas".to_owned(), Value::Int(replicas)));
    w.push((
        "template".to_owned(),
        obj([
            ("metadata", obj([("labels", labels)])),
            ("spec", Value::Object(spec)),
        ]),
    ));
    if stateful.is_some() {
        w.push((
            "volumeClaimTemplates".to_owned(),
            Value::List(vec![obj([
                ("metadata", obj([("name", s("data"))])),
                (
                    "spec",
                    obj([
                        ("accessModes", list(&["ReadWriteOnce".to_owned()])),
                        (
                            "resources",
                            obj([("requests", obj([("storage", s("1Gi"))]))]),
                        ),
                    ]),
                ),
            ])]),
        ));
    }
    Value::Object(w)
}

fn k8s_service(name: &str, port: i64, target: i64) -> Value {
    obj([
        ("name", s(name)),
        ("selector", obj([("app", s(name))])),
        (
            "ports",
            Value::List(vec![obj([
                ("port", Value::Int(port)),
                ("targetPort", Value::Int(target)),
            ])]),
        ),
    ])
}

fn ingress(name: &str, host: &str, service: &str, port: i64) -> Value {
    let backend = obj([(
        "service",
        obj([
            ("name", s(service)),
            ("port", obj([("number", Value::Int(port))])),
        ]),
    )]);
    obj([
        ("name", s(name)),
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
    ])
}

/// La configuration nginx : tout est renvoyé vers l'application.
fn nginx_conf(app: &str, port: i64) -> String {
    format!(
        "server {{
    listen 80;
    location / {{
        proxy_pass http://{app}:{port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}
"
    )
}

/// Le Dockerfile du code Rust : `cargo install` construit le binaire, quel que soit son nom.
fn dockerfile(path: &str, tag: &str, port: i64) -> Value {
    let (base, run) = super::rust_images(tag);
    obj([
        ("base", s(base)),
        ("workdir", s("/app")),
        (
            "copy",
            Value::List(vec![obj([("src", s(path)), ("dst", s("."))])]),
        ),
        (
            "build",
            list(&[
                "cargo install --path . --root /out".to_owned(),
                "mv /out/bin/* /app/server".to_owned(),
            ]),
        ),
        ("from", s(run)),
        ("artifacts", list(&["server".to_owned()])),
        ("env", obj([("PORT", Value::Int(port))])),
        ("expose", Value::List(vec![Value::Int(port)])),
        ("run", list(&["./server".to_owned()])),
    ])
}
