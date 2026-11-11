//! Vérifie chaque section (les éléments obligatoires, les types) et écrit les fichiers.

use super::eval::SectionValue;
use super::parser::Section;
use super::yaml::to_yaml;
use super::{Error, GeneratedFile, Pos, Value};

const HEADER: &str = "Généré par Rust-Compose : modifiez le fichier .drc, pas celui-ci.";

pub fn files(sections: &[SectionValue]) -> Result<Vec<GeneratedFile>, Error> {
    let mut out = Vec::new();
    // Toujours le même ordre : Dockerfile, Compose, puis Kubernetes.
    for wanted in [Section::Docker, Section::Compose, Section::K8s] {
        let Some(section) = sections.iter().find(|s| s.section == wanted) else {
            continue;
        };
        let check = Check { pos: section.pos };
        let entries = check.object(&section.value, wanted.name())?;
        match wanted {
            Section::Docker => out.push(GeneratedFile {
                name: "Dockerfile".into(),
                content: dockerfile(&check, entries)?,
            }),
            Section::Compose => out.push(GeneratedFile {
                name: "docker-compose.yml".into(),
                content: compose(&check, &section.value, entries)?,
            }),
            Section::K8s => out.extend(kubernetes(&check, entries)?),
        }
    }
    Ok(out)
}

/// Les vérifications de type, rapportées à la position de la section.
struct Check {
    pos: Pos,
}

impl Check {
    fn err(&self, message: impl Into<String>) -> Error {
        Error::at(self.pos, message)
    }

    fn object<'a>(&self, value: &'a Value, path: &str) -> Result<&'a [(String, Value)], Error> {
        match value {
            Value::Object(entries) => Ok(entries),
            other => Err(self.err(format!(
                "`{path}` doit être un objet `{{ … }}`, pas {}",
                other.type_name()
            ))),
        }
    }

    fn list<'a>(&self, value: &'a Value, path: &str) -> Result<&'a [Value], Error> {
        match value {
            Value::List(items) => Ok(items),
            other => Err(self.err(format!(
                "`{path}` doit être une liste `[ … ]`, pas {}",
                other.type_name()
            ))),
        }
    }

    fn string<'a>(&self, value: &'a Value, path: &str) -> Result<&'a str, Error> {
        match value {
            Value::Str(s) if !s.trim().is_empty() => Ok(s),
            Value::Str(_) => Err(self.err(format!("`{path}` ne peut pas être vide"))),
            other => Err(self.err(format!(
                "`{path}` doit être une chaîne, pas {}",
                other.type_name()
            ))),
        }
    }

    /// Une chaîne ou un nombre (un port, une version).
    fn scalar(&self, value: &Value, path: &str) -> Result<String, Error> {
        match value {
            Value::Int(n) => Ok(n.to_string()),
            Value::Float(n) => Ok(n.to_string()),
            other => self.string(other, path).map(str::to_owned),
        }
    }

    /// Une chaîne seule ou une liste de chaînes.
    fn strings(&self, value: &Value, path: &str) -> Result<Vec<String>, Error> {
        match value {
            Value::List(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| self.scalar(item, &format!("{path}[{i}]")))
                .collect(),
            other => Ok(vec![self.scalar(other, path)?]),
        }
    }

    fn unknown(&self, path: &str, key: &str, allowed: &[&str]) -> Error {
        self.err(format!(
            "directive inconnue `{path}.{key}` (attendu : {})",
            allowed.join(", ")
        ))
    }

    fn required<'a>(
        &self,
        entries: &'a [(String, Value)],
        key: &str,
        path: &str,
    ) -> Result<&'a Value, Error> {
        get(entries, key).ok_or_else(|| self.err(format!("`{path}.{key}` est obligatoire")))
    }
}

fn get<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Une liste au format exec de Docker : `["./myapp", "--port", "80"]`.
fn exec_form(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| json_string(s)).collect();
    format!("[{}]", quoted.join(", "))
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Une valeur de Dockerfile qui contient des espaces passe entre guillemets.
fn docker_word(s: &str) -> String {
    if s.is_empty() || s.contains(|c: char| c.is_whitespace() || c == '"' || c == '\\') {
        json_string(s)
    } else {
        s.to_owned()
    }
}

const DOCKER_KEYS: &[&str] = &[
    "base",
    "workdir",
    "copy",
    "build",
    "from",
    "artifacts",
    "env",
    "labels",
    "user",
    "expose",
    "entrypoint",
    "run",
];

fn dockerfile(check: &Check, entries: &[(String, Value)]) -> Result<String, Error> {
    for (key, _) in entries {
        if !DOCKER_KEYS.contains(&key.as_str()) {
            return Err(check.unknown("docker", key, DOCKER_KEYS));
        }
    }
    let base = check.string(check.required(entries, "base", "docker")?, "docker.base")?;
    let workdir = get(entries, "workdir")
        .map(|v| check.string(v, "docker.workdir"))
        .transpose()?;
    let from = get(entries, "from")
        .map(|v| check.string(v, "docker.from"))
        .transpose()?;

    let mut out = format!("# syntax=docker/dockerfile:1\n# {HEADER}\n\n");
    match from {
        Some(_) => out.push_str(&format!("FROM {base} AS build\n")),
        None => out.push_str(&format!("FROM {base}\n")),
    }
    if let Some(dir) = workdir {
        out.push_str(&format!("WORKDIR {}\n", docker_word(dir)));
    }
    if let Some(copies) = get(entries, "copy") {
        for (i, copy) in check.list(copies, "docker.copy")?.iter().enumerate() {
            let path = format!("docker.copy[{i}]");
            let (src, dst) = match copy {
                Value::Object(fields) => {
                    for (key, _) in fields {
                        if key != "src" && key != "dst" {
                            return Err(check.unknown(&path, key, &["src", "dst"]));
                        }
                    }
                    let src = check.string(check.required(fields, "src", &path)?, &path)?;
                    let dst = check.string(check.required(fields, "dst", &path)?, &path)?;
                    (src, dst)
                }
                Value::Str(s) => (check.string(copy, &path)?, s.as_str()),
                other => {
                    return Err(check.err(format!(
                        "`{path}` doit être `{{ src = \"…\", dst = \"…\" }}`, pas {}",
                        other.type_name()
                    )));
                }
            };
            out.push_str(&format!("COPY {} {}\n", docker_word(src), docker_word(dst)));
        }
    }
    if let Some(build) = get(entries, "build") {
        for command in check.strings(build, "docker.build")? {
            out.push_str(&format!("RUN {command}\n"));
        }
    }
    if let Some(image) = from {
        out.push_str(&format!("\nFROM {image}\n"));
        if let Some(dir) = workdir {
            out.push_str(&format!("WORKDIR {}\n", docker_word(dir)));
        }
        let artifacts = get(entries, "artifacts")
            .map(|v| check.strings(v, "docker.artifacts"))
            .transpose()?;
        match (artifacts, workdir) {
            (Some(paths), _) => {
                for path in paths {
                    let src = match workdir {
                        Some(dir) if !path.starts_with('/') => {
                            format!(
                                "{}/{}",
                                dir.trim_end_matches('/'),
                                path.trim_start_matches("./")
                            )
                        }
                        _ => path.clone(),
                    };
                    out.push_str(&format!(
                        "COPY --from=build {} {}\n",
                        docker_word(&src),
                        docker_word(&path)
                    ));
                }
            }
            (None, Some(dir)) => {
                out.push_str(&format!(
                    "COPY --from=build {} {}\n",
                    docker_word(dir),
                    docker_word(dir)
                ));
            }
            (None, None) => {
                return Err(check.err(
                    "`docker.from` crée une image finale : indiquez `workdir` ou `artifacts` pour savoir quoi y copier",
                ));
            }
        }
    } else if get(entries, "artifacts").is_some() {
        return Err(check.err("`docker.artifacts` n'a de sens qu'avec `docker.from`"));
    }
    if let Some(env) = get(entries, "env") {
        for (key, value) in check.object(env, "docker.env")? {
            let value = check.scalar(value, &format!("docker.env.{key}"))?;
            out.push_str(&format!("ENV {key}={}\n", json_string(&value)));
        }
    }
    if let Some(labels) = get(entries, "labels") {
        for (key, value) in check.object(labels, "docker.labels")? {
            let value = check.scalar(value, &format!("docker.labels.{key}"))?;
            out.push_str(&format!(
                "LABEL {}={}\n",
                json_string(key),
                json_string(&value)
            ));
        }
    }
    if let Some(expose) = get(entries, "expose") {
        let ports = check.strings(expose, "docker.expose")?;
        for port in &ports {
            let valid = port
                .split_once('/')
                .map_or(port.as_str(), |(number, proto)| {
                    if proto == "tcp" || proto == "udp" {
                        number
                    } else {
                        ""
                    }
                })
                .parse::<u16>()
                .is_ok_and(|n| n > 0);
            if !valid {
                return Err(check.err(format!(
                    "`docker.expose` : port invalide `{port}` (exemples : 8080, \"8080/tcp\", \"53/udp\")"
                )));
            }
        }
        out.push_str(&format!("EXPOSE {}\n", ports.join(" ")));
    }
    if let Some(user) = get(entries, "user") {
        out.push_str(&format!("USER {}\n", check.scalar(user, "docker.user")?));
    }
    if let Some(entrypoint) = get(entries, "entrypoint") {
        let items = check.strings(entrypoint, "docker.entrypoint")?;
        out.push_str(&format!("ENTRYPOINT {}\n", exec_form(&items)));
    }
    if let Some(run) = get(entries, "run") {
        let items = check.strings(run, "docker.run")?;
        out.push_str(&format!("CMD {}\n", exec_form(&items)));
    }
    Ok(out)
}

const COMPOSE_KEYS: &[&str] = &[
    "name", "services", "networks", "volumes", "configs", "secrets",
];

fn compose(check: &Check, value: &Value, entries: &[(String, Value)]) -> Result<String, Error> {
    for (key, _) in entries {
        if !COMPOSE_KEYS.contains(&key.as_str()) {
            return Err(check.unknown("compose", key, COMPOSE_KEYS));
        }
    }
    let services = check.object(
        check.required(entries, "services", "compose")?,
        "compose.services",
    )?;
    if services.is_empty() {
        return Err(check.err("`compose.services` doit contenir au moins un service"));
    }
    for (name, service) in services {
        let path = format!("compose.services.{name}");
        let fields = check.object(service, &path)?;
        if get(fields, "image").is_none() && get(fields, "build").is_none() {
            return Err(check.err(format!(
                "le service `{name}` a besoin de `image` ou de `build`"
            )));
        }
        if let Some(image) = get(fields, "image") {
            check.string(image, &format!("{path}.image"))?;
        }
        for key in ["ports", "volumes"] {
            if let Some(v) = get(fields, key) {
                check.list(v, &format!("{path}.{key}"))?;
            }
        }
        if let Some(env) = get(fields, "environment")
            && !matches!(env, Value::Object(_) | Value::List(_))
        {
            return Err(check.err(format!(
                "`{path}.environment` doit être un objet ou une liste, pas {}",
                env.type_name()
            )));
        }
    }
    Ok(format!("# {HEADER}\n{}", to_yaml(value)))
}

/// Une ressource Kubernetes que la section `k8s` sait écrire.
pub struct Kind {
    /// La clé dans la section `k8s` (et le nom du fichier généré).
    pub key: &'static str,
    pub kind: &'static str,
    pub api_version: &'static str,
    /// Les ressources sans `spec` : leurs champs vont à la racine du manifest.
    pub flat: bool,
}

pub const KINDS: &[Kind] = &[
    kind("namespace", "Namespace", "v1", true),
    kind("deployment", "Deployment", "apps/v1", false),
    kind("statefulset", "StatefulSet", "apps/v1", false),
    kind("daemonset", "DaemonSet", "apps/v1", false),
    kind("job", "Job", "batch/v1", false),
    kind("cronjob", "CronJob", "batch/v1", false),
    kind("service", "Service", "v1", false),
    kind("ingress", "Ingress", "networking.k8s.io/v1", false),
    kind(
        "networkpolicy",
        "NetworkPolicy",
        "networking.k8s.io/v1",
        false,
    ),
    kind("configmap", "ConfigMap", "v1", true),
    kind("secret", "Secret", "v1", true),
    kind("serviceaccount", "ServiceAccount", "v1", true),
    kind(
        "persistentvolumeclaim",
        "PersistentVolumeClaim",
        "v1",
        false,
    ),
    kind("hpa", "HorizontalPodAutoscaler", "autoscaling/v2", false),
    kind(
        "poddisruptionbudget",
        "PodDisruptionBudget",
        "policy/v1",
        false,
    ),
];

const fn kind(
    key: &'static str,
    kind: &'static str,
    api_version: &'static str,
    flat: bool,
) -> Kind {
    Kind {
        key,
        kind,
        api_version,
        flat,
    }
}

const METADATA_KEYS: &[&str] = &["name", "namespace", "labels", "annotations"];

fn kubernetes(check: &Check, entries: &[(String, Value)]) -> Result<Vec<GeneratedFile>, Error> {
    let keys: Vec<&str> = KINDS.iter().map(|k| k.key).collect();
    let mut files = Vec::new();
    for (key, value) in entries {
        let Some(kind) = KINDS.iter().find(|k| k.key == key) else {
            return Err(check.unknown("k8s", key, &keys));
        };
        // Une ressource, ou une liste de ressources du même type.
        let resources: Vec<&Value> = match value {
            Value::List(items) => items.iter().collect(),
            other => vec![other],
        };
        if resources.is_empty() {
            return Err(check.err(format!("`k8s.{key}` est une liste vide")));
        }
        let mut documents = Vec::new();
        for (i, resource) in resources.into_iter().enumerate() {
            let path = if matches!(value, Value::List(_)) {
                format!("k8s.{key}[{i}]")
            } else {
                format!("k8s.{key}")
            };
            documents.push(to_yaml(&manifest(check, kind, resource, &path)?));
        }
        files.push(GeneratedFile {
            name: format!("{key}.yaml"),
            content: format!("# {HEADER}\n{}", documents.join("---\n")),
        });
    }
    Ok(files)
}

fn manifest(check: &Check, kind: &Kind, value: &Value, path: &str) -> Result<Value, Error> {
    let fields = check.object(value, path)?;
    let name = check.string(
        check.required(fields, "name", path)?,
        &format!("{path}.name"),
    )?;
    let valid_name = name.len() <= 253
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name.ends_with(|c: char| c.is_ascii_alphanumeric());
    if !valid_name {
        return Err(check.err(format!(
            "`{path}.name` : `{name}` n'est pas un nom Kubernetes valide (minuscules, chiffres, `-` et `.`)"
        )));
    }
    let mut metadata = Vec::new();
    let mut rest = Vec::new();
    for (key, value) in fields {
        if METADATA_KEYS.contains(&key.as_str()) {
            if matches!(key.as_str(), "labels" | "annotations") {
                for (label, v) in check.object(value, &format!("{path}.{key}"))? {
                    check.scalar(v, &format!("{path}.{key}.{label}"))?;
                }
            }
            metadata.push((key.clone(), value.clone()));
        } else {
            rest.push((key.clone(), value.clone()));
        }
    }
    let labels = get(fields, "labels").cloned();
    match kind.key {
        "deployment" | "statefulset" | "daemonset" => {
            workload(check, &mut rest, labels.as_ref(), path)?;
        }
        "service" => {
            let ports = check.list(
                check.required(fields, "ports", path)?,
                &format!("{path}.ports"),
            )?;
            if ports.is_empty() {
                return Err(check.err(format!("`{path}.ports` doit contenir au moins un port")));
            }
            for (i, port) in ports.iter().enumerate() {
                let p = format!("{path}.ports[{i}]");
                let port = check.object(port, &p)?;
                match check.required(port, "port", &p)? {
                    Value::Int(n) if (1..=65535).contains(n) => {}
                    _ => {
                        return Err(
                            check.err(format!("`{p}.port` doit être un nombre entre 1 et 65535"))
                        );
                    }
                }
            }
            if let Some(kind) = get(fields, "type") {
                let kind = check.string(kind, &format!("{path}.type"))?;
                const TYPES: &[&str] = &["ClusterIP", "NodePort", "LoadBalancer", "ExternalName"];
                if !TYPES.contains(&kind) {
                    return Err(check.err(format!(
                        "`{path}.type` : `{kind}` inconnu (attendu : {})",
                        TYPES.join(", ")
                    )));
                }
            }
        }
        "configmap" | "secret" => {
            for key in ["data", "stringData"] {
                if let Some(data) = get(fields, key) {
                    for (entry, v) in check.object(data, &format!("{path}.{key}"))? {
                        if !matches!(v, Value::Str(_)) {
                            return Err(check.err(format!(
                                "`{path}.{key}.{entry}` doit être une chaîne, pas {}",
                                v.type_name()
                            )));
                        }
                    }
                }
            }
        }
        _ => {}
    }
    let mut manifest = vec![
        ("apiVersion".to_owned(), Value::Str(kind.api_version.into())),
        ("kind".to_owned(), Value::Str(kind.kind.into())),
        ("metadata".to_owned(), Value::Object(metadata)),
    ];
    if kind.flat {
        manifest.extend(rest);
    } else if !rest.is_empty() {
        manifest.push(("spec".to_owned(), Value::Object(rest)));
    }
    Ok(Value::Object(manifest))
}

/// Deployment, StatefulSet, DaemonSet : des conteneurs nommés avec une image, et un
/// `selector` déduit des labels quand il manque.
fn workload(
    check: &Check,
    spec: &mut Vec<(String, Value)>,
    labels: Option<&Value>,
    path: &str,
) -> Result<(), Error> {
    if let Some(replicas) = get(spec, "replicas")
        && !matches!(replicas, Value::Int(n) if *n >= 0)
    {
        return Err(check.err(format!(
            "`{path}.replicas` doit être un nombre entier positif"
        )));
    }
    let template = check.object(
        check.required(spec, "template", path)?,
        &format!("{path}.template"),
    )?;
    let pod = check.object(
        check.required(template, "spec", &format!("{path}.template"))?,
        &format!("{path}.template.spec"),
    )?;
    let containers = check.list(
        check.required(pod, "containers", &format!("{path}.template.spec"))?,
        &format!("{path}.template.spec.containers"),
    )?;
    if containers.is_empty() {
        return Err(check.err(format!(
            "`{path}.template.spec.containers` doit contenir au moins un conteneur"
        )));
    }
    for (i, container) in containers.iter().enumerate() {
        let p = format!("{path}.template.spec.containers[{i}]");
        let container = check.object(container, &p)?;
        check.string(check.required(container, "name", &p)?, &format!("{p}.name"))?;
        check.string(
            check.required(container, "image", &p)?,
            &format!("{p}.image"),
        )?;
    }
    let template_labels = template
        .iter()
        .find(|(k, _)| k == "metadata")
        .and_then(|(_, m)| m.get("labels"))
        .cloned();
    if get(spec, "selector").is_none() {
        let Some(match_labels) = template_labels.or_else(|| labels.cloned()) else {
            return Err(check.err(format!(
                "`{path}.selector` est obligatoire (ou donnez des `labels` pour le déduire)"
            )));
        };
        let selector = Value::Object(vec![("matchLabels".into(), match_labels)]);
        let at = spec
            .iter()
            .position(|(k, _)| k == "template")
            .unwrap_or(spec.len());
        spec.insert(at, ("selector".into(), selector));
    }
    Ok(())
}
