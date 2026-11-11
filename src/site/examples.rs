//! Les exemples du site et du bac à sable.

/// Un exemple : (identifiant dans l'URL, titre, source).
pub struct Example {
    pub id: &'static str,
    pub title: &'static str,
    pub source: &'static str,
}

/// L'exemple de la page d'accueil : court, et les trois cibles.
pub const HOME: &str = r#"# Rust-Compose 1.0
var IMAGE_NAME = "myapp"

fn image_tag(tag) {
    return format!("{}:{}", IMAGE_NAME, tag)
}

docker {
    base = "rust:1.99-slim"
    workdir = "/app"
    copy = [{ src = ".", dst = "." }]
    build = ["cargo build --release"]
    from = "debian:trixie-slim"
    artifacts = ["target/release/myapp"]
    run = ["./target/release/myapp"]
    expose = [8080]
}

compose {
    services = {
        web = { image = image_tag("latest"), ports = ["8080:8080"] }
    }
}

k8s {
    service = {
        name = "myapp"
        selector = { app = "myapp" }
        ports = [{ port = 80, targetPort = 8080 }]
    }
}
"#;

pub const ALL: &[Example] = &[
    Example {
        id: "complet",
        title: "Complet",
        source: include_str!("../../examples/complet.drc"),
    },
    Example {
        id: "docker",
        title: "Docker",
        source: r#"# Rust-Compose 1.0
# Une construction en deux étapes : on compile avec Rust, on livre une image légère.
docker {
    base = "rust:1.99-slim"
    workdir = "/app"
    copy = [
        { src = "./Cargo.toml", dst = "." },
        { src = "./src", dst = "src" }
    ]
    build = ["cargo build --release"]
    from = "debian:trixie-slim"
    artifacts = ["target/release/api"]
    env = { RUST_LOG = "info", PORT = 8080 }
    labels = { "org.opencontainers.image.title" = "api" }
    user = "10001"
    expose = [8080, "9090/udp"]
    run = ["./target/release/api"]
}
"#,
    },
    Example {
        id: "compose",
        title: "Compose",
        source: r#"# Rust-Compose 1.0
var PROJECT = "boutique"

fn service_image(name) {
    return format!("ghcr.io/acme/{}-{}:latest", PROJECT, name)
}

compose {
    name = PROJECT
    services = {
        front = {
            image = service_image("front")
            ports = ["80:8080"]
            depends_on = ["api"]
        }
        api = {
            image = service_image("api")
            environment = { DATABASE_URL = "postgres://app:secret@db/app" }
        }
        db = {
            image = "postgres:18-alpine"
            environment = { POSTGRES_USER = "app", POSTGRES_PASSWORD = "secret" }
            volumes = ["pgdata:/var/lib/postgresql"]
        }
    }
    volumes = { pgdata = {} }
}
"#,
    },
    Example {
        id: "k8s",
        title: "Kubernetes",
        source: r#"# Rust-Compose 1.0
var APP = "api"
var LABELS = { app = APP, tier = "backend" }

k8s {
    namespace = { name = "prod" }

    deployment = {
        name = APP
        namespace = "prod"
        labels = LABELS
        replicas = 2
        # Pas de `selector` : il est déduit des labels du modèle de pod.
        template = {
            metadata = { labels = LABELS }
            spec = {
                containers = [{
                    name = APP
                    image = "ghcr.io/acme/api:2.1.0"
                    ports = [{ containerPort = 8080 }]
                    resources = { limits = { cpu = "500m", memory = "256Mi" } }
                }]
            }
        }
    }

    service = {
        name = APP
        namespace = "prod"
        selector = LABELS
        ports = [{ port = 80, targetPort = 8080, protocol = "TCP" }]
    }

    hpa = {
        name = APP
        namespace = "prod"
        scaleTargetRef = { apiVersion = "apps/v1", kind = "Deployment", name = APP }
        minReplicas = 2
        maxReplicas = 10
        metrics = [{
            type = "Resource"
            resource = { name = "cpu", target = { type = "Utilization", averageUtilization = 70 } }
        }]
    }
}
"#,
    },
    Example {
        id: "erreur",
        title: "Avec une erreur",
        source: r#"# Rust-Compose 1.0
# Le compilateur vérifie les éléments obligatoires : ici, l'image du conteneur manque.
k8s {
    deployment = {
        name = "web"
        labels = { app = "web" }
        template = {
            metadata = { labels = { app = "web" } }
            spec = {
                containers = [{ name = "web" }]
            }
        }
    }
}
"#,
    },
];

/// L'exemple d'identifiant `id`, ou le premier.
pub fn find(id: Option<&str>) -> &'static Example {
    id.and_then(|id| ALL.iter().find(|e| e.id == id))
        .unwrap_or(&ALL[0])
}
