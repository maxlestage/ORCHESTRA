//! Le compilateur Rust-Compose de bout en bout, sur les exemples du dépôt.

use std::path::Path;

use orchestra::drc::{self, GeneratedFile};

fn file<'a>(files: &'a [GeneratedFile], name: &str) -> &'a str {
    &files
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} n'a pas été généré"))
        .content
}

#[test]
fn the_complete_example_generates_every_file() {
    let source = include_str!("../examples/complet.drc");
    let files = drc::compile_str(source).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Dockerfile",
            "docker-compose.yml",
            "deployment.yaml",
            "service.yaml",
            "configmap.yaml"
        ]
    );

    let dockerfile = file(&files, "Dockerfile");
    assert!(dockerfile.contains("FROM rust:1.99-slim AS build\nWORKDIR /app\n"));
    assert!(dockerfile.contains("COPY ./src src\nCOPY ./Cargo.toml .\n"));
    assert!(
        dockerfile.contains("RUN cargo build --release\nRUN cp target/release/myapp ./myapp\n")
    );
    assert!(
        dockerfile.contains(
            "FROM debian:trixie-slim\nWORKDIR /app\nCOPY --from=build /app/myapp myapp\n"
        )
    );
    assert!(dockerfile.contains("EXPOSE 8080/tcp\nCMD [\"./myapp\"]\n"));

    let compose = file(&files, "docker-compose.yml");
    assert!(
        compose.contains(
            "services:\n  web:\n    image: myapp:latest\n    ports:\n      - \"8080:80\"\n"
        )
    );
    assert!(compose.contains("    environment:\n      ENV: prod\n"));
    assert!(compose.contains("networks:\n  default:\n    driver: bridge\n"));

    let deployment = file(&files, "deployment.yaml");
    assert!(deployment.contains("apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: myapp-deploy\n  labels:\n    app: myapp\nspec:\n  replicas: 3\n"));
    assert!(deployment.contains("        - name: myapp\n          image: myapp:1.0.0\n"));
    assert!(deployment.contains("          emptyDir: {}\n"));
    assert!(deployment.contains("            - name: DEBUG\n              value: \"false\"\n"));

    let service = file(&files, "service.yaml");
    assert!(service.contains("kind: Service\n"));
    assert!(service.contains(
        "  type: ClusterIP\n  ports:\n    - port: 80\n      targetPort: 80\n      protocol: TCP\n"
    ));

    let configmap = file(&files, "configmap.yaml");
    assert!(configmap.contains("kind: ConfigMap\nmetadata:\n  name: myapp-config\ndata:\n  app.conf: |\n    log_level=info\n"));
}

#[test]
fn a_project_split_into_files_with_imports() {
    let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/projet/main.drc");
    let files = orchestra_files(&main);
    assert!(file(&files, "Dockerfile").contains("FROM debian:trixie-slim"));
    assert!(file(&files, "docker-compose.yml").contains("image: myapp:latest"));
    let deployment = file(&files, "deployment.yaml");
    // Le selector, absent du fichier, est déduit des labels du modèle de pod.
    assert!(deployment.contains("  selector:\n    matchLabels:\n      app: myapp\n  template:\n"));
    assert!(deployment.contains("image: myapp:1.0.0"));
}

fn orchestra_files(main: &Path) -> Vec<GeneratedFile> {
    let source = std::fs::read_to_string(main).unwrap();
    drc::compile(&main.display().to_string(), &source, &mut |from, path| {
        let path = Path::new(from).parent().unwrap().join(path);
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
        let source = std::fs::read_to_string(&canonical).map_err(|e| e.to_string())?;
        Ok((canonical.display().to_string(), source))
    })
    .unwrap()
}

fn error(source: &str) -> String {
    drc::compile_str(source).unwrap_err().to_string()
}

#[test]
fn mistakes_are_reported_with_their_position() {
    assert_eq!(
        error("var A = 1\nvar B = C"),
        "main.drc:2:9 : variable inconnue `C`"
    );
    assert!(error("docker { workdir = \"/app\" }").contains("`docker.base` est obligatoire"));
    assert!(error("docker { base = \"a\", bse = 1 }").contains("directive inconnue `docker.bse`"));
    assert!(
        error("compose { services = { web = { ports = [] } } }")
            .contains("le service `web` a besoin de `image` ou de `build`")
    );
    assert!(error("k8s { deployment = { name = \"Mon App\" } }").contains("nom Kubernetes valide"));
    assert!(error("k8s { pod = { name = \"x\" } }").contains("directive inconnue `k8s.pod`"));
    assert!(
        error("k8s { service = { name = \"s\", ports = [{ port = 0 }] } }")
            .contains("entre 1 et 65535")
    );
    assert!(error("var X = 1").contains("rien à générer"));
    assert!(error("import \"autre.drc\"\ndocker { base = \"a\" }").contains("imports"));
}

#[test]
fn functions_builtins_and_formatting() {
    let files = drc::compile_str(
        r#"
        var REGISTRY = "ghcr.io/acme"
        fn image(name, tag) {
            var full = format!("{}/{}", REGISTRY, name)
            return format!("{0}:{1}", full, tag)
        }
        compose {
            services = {
                api = {
                    image = image(lower("API"), 2)
                    command = join(["serve", "--port", 80], " ")
                    labels = { kind = concat("a", 1, true), tag = upper(replace("x-y", "-", "_")) }
                }
            }
        }
        "#,
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("image: ghcr.io/acme/api:2\n"));
    assert!(compose.contains("command: serve --port 80\n"));
    assert!(compose.contains("kind: a1true\n"));
    assert!(compose.contains("tag: X_Y\n"));
}

#[test]
fn runaway_programs_are_stopped() {
    let recursion = "fn f(x) { return f(x) }\ndocker { base = f(1) }";
    assert!(error(recursion).contains("récursion"));

    let mut doubling = String::from("var A0 = \"xxxxxxxxxxxxxxxx\"\n");
    for i in 1..40 {
        doubling.push_str(&format!("var A{i} = [A{0}, A{0}]\n", i - 1));
    }
    doubling.push_str("docker { base = \"a\" }");
    assert!(error(&doubling).contains("trop de données"));

    let cycle = |_: &str, path: &str| Ok((path.to_owned(), format!("import \"{path}\"")));
    let err = drc::compile("a.drc", "import \"a.drc\"", &mut { cycle }).unwrap_err();
    assert!(err.message.contains("import circulaire"));
}

#[test]
fn an_app_in_one_line() {
    let files = drc::compile_str(include_str!("../examples/simple.drc")).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["docker-compose.yml", "deployment.yaml", "service.yaml"]
    );
    assert!(
        file(&files, "docker-compose.yml")
            .contains("  web:\n    image: nginx:1.30\n    ports:\n      - \"80:80\"\n")
    );
    let deployment = file(&files, "deployment.yaml");
    assert!(deployment.contains("  replicas: 1\n  selector:\n    matchLabels:\n      app: web\n"));
    assert!(deployment.contains("        - name: web\n          image: nginx:1.30\n          ports:\n            - containerPort: 80\n"));
    assert!(file(&files, "service.yaml").contains("    - port: 80\n      targetPort: 80\n"));
}

#[test]
fn several_apps_with_a_rust_build_and_a_host() {
    let files = drc::compile_str(include_str!("../examples/boutique.drc")).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Dockerfile",
            "docker-compose.yml",
            "deployment.yaml",
            "service.yaml",
            "ingress.yaml"
        ]
    );
    let dockerfile = file(&files, "Dockerfile");
    assert!(dockerfile.contains("FROM rust:1.99-slim AS build\n"));
    assert!(
        dockerfile.contains("COPY --from=build /app/target/release/site target/release/site\n")
    );
    assert!(dockerfile.contains("EXPOSE 8080\nCMD [\"./target/release/site\"]\n"));
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("  site:\n    build: .\n    image: site:latest\n"));
    assert!(compose.contains("  api:\n    image: ghcr.io/acme/api:2.1.0\n"));
    assert!(compose.contains("      POSTGRES_PASSWORD: secret\n"));
    assert!(compose.contains("    depends_on:\n      - db\n"));
    let deployment = file(&files, "deployment.yaml");
    assert_eq!(deployment.matches("kind: Deployment").count(), 3);
    assert!(deployment.contains("  replicas: 3\n"));
    // Le port de postgres est déduit : la base a aussi son Service.
    let services = file(&files, "service.yaml");
    assert_eq!(services.matches("kind: Service").count(), 3);
    assert!(services.contains(
        "    - port: 5432
"
    ));
    assert!(file(&files, "ingress.yaml").contains("    - host: boutique.example.com\n"));
}

#[test]
fn apps_and_hand_written_sections_together() {
    let files = drc::compile_str(
        r#"
        app web { image = "nginx:1.30", port = 80 }
        compose { services = { web = { restart = "always" } } }
        k8s { configmap = { name = "web", data = { "a.conf" = "x" } } }
        "#,
    )
    .unwrap();
    assert!(
        file(&files, "docker-compose.yml").contains("      - \"80:80\"\n    restart: always\n")
    );
    assert!(file(&files, "configmap.yaml").contains("kind: ConfigMap"));
    assert!(file(&files, "deployment.yaml").contains("name: web"));
}

#[test]
fn keywords_are_optional_and_ports_are_known() {
    let files = drc::compile_str(
        "TAG = \"7\"\ncache { image = \"redis:\" + TAG }\nworker { image = \"acme/worker:1\" }\nlb { image = \"nginx:1.30\", port = false }",
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("  cache:\n    image: redis:7\n    ports:\n      - \"6379:6379\"\n"));
    assert!(compose.contains("  worker:\n    image: acme/worker:1\n  lb:"));
    let services = file(&files, "service.yaml");
    assert_eq!(services.matches("kind: Service").count(), 1);
    // `app` et `var` restent acceptés.
    assert!(drc::compile_str("var X = \"nginx:1.30\"\napp web { image = X }").is_ok());
    // Un nom proche d'un mot-clé est une faute de frappe, pas une application.
    assert!(error("dokcer { base = \"a\" }").contains("vouliez-vous dire `docker`"));
    assert!(
        drc::compile_str(
            "api { image = \"a:1\" }\nweb { image = \"b:1\" }\ndb { image = \"postgres:18\" }"
        )
        .is_ok()
    );
}

#[test]
fn writing_is_forgiving_and_errors_help() {
    // `+`, fonction d'une ligne, virgules facultatives.
    let files = drc::compile_str(
        "var TAG = 1 + 1\nfn img(n) = n + \":\" + TAG\napp web { image = img(\"nginx\"), command = [\"nginx\" \"-g\" \"daemon off;\"] }",
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("image: nginx:2\n"));
    assert!(compose.contains("      - nginx\n      - \"-g\"\n      - daemon off;\n"));

    assert!(
        error("app web {\n  imgae = \"nginx\"\n}").starts_with(
            "main.drc:2:3 : `web` : clé inconnue `imgae` (vouliez-vous dire `image` ?)"
        )
    );
    assert!(error("app web { port = 80 }").contains("il faut une image"));
    assert!(error("app Web { image = \"x\" }").contains("minuscules"));
    assert!(
        error("app a { image = \"x\", depends_on = [\"bd\"] }\napp db { image = \"y\" }")
            .contains("vouliez-vous dire `db`")
    );
    assert!(
        error("var IMAGE = \"x\"\napp a { image = IMAEG }").contains("vouliez-vous dire `IMAGE`")
    );
    assert!(error("docker { bsae = \"a\" }").contains("vouliez-vous dire `base`"));
    assert!(error("app a { image = \"x\", host = \"a.fr\" }").contains("il lui faut un `port`"));
    assert!(
        error("app a { build = \"rust\" }\napp b { build = \"rust\" }")
            .contains("un seul Dockerfile")
    );
}

#[test]
fn custom_ports_for_several_apps() {
    let files = drc::compile_str(
        r#"
        front { image = "nginx:1.30", port = 8080 }          # nginx écoute sur 80
        api   { image = "acme/api:2", port = "3001:3000" }
        proxy { image = "acme/proxy:1", ports = [80, "8443:443"] }
        db    { image = "postgres:18-alpine" }
        "#,
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("  front:\n    image: nginx:1.30\n    ports:\n      - \"8080:80\"\n"));
    assert!(compose.contains("      - \"3001:3000\"\n"));
    assert!(compose.contains(
        "  proxy:\n    image: acme/proxy:1\n    ports:\n      - \"80:80\"\n      - \"8443:443\"\n"
    ));
    assert!(compose.contains("      - \"5432:5432\"\n"));

    let deployment = file(&files, "deployment.yaml");
    assert!(deployment.contains(
        "          image: nginx:1.30\n          ports:\n            - containerPort: 80\n"
    ));
    assert!(
        deployment.contains("            - containerPort: 80\n            - containerPort: 443\n")
    );

    let services = file(&files, "service.yaml");
    assert!(services.contains("  name: front\nspec:\n  selector:\n    app: front\n  ports:\n    - port: 8080\n      targetPort: 80\n"));
    assert!(services.contains("    - port: 3001\n      targetPort: 3000\n"));
    // Plusieurs ports : chacun a un nom, comme Kubernetes l'exige.
    assert!(services.contains("    - name: p80\n      port: 80\n      targetPort: 80\n    - name: p8443\n      port: 8443\n      targetPort: 443\n"));
}

#[test]
fn port_mistakes_are_caught() {
    let collision =
        error("a { image = \"nginx:1.30\" }\nb {\n  image = \"nginx:1.30\"\n  port = 80\n}");
    assert_eq!(
        collision,
        "main.drc:4:3 : `b` : le port 80 est déjà publié par `a` ; choisissez un autre port public, par exemple `port = \"81:80\"`"
    );
    assert!(
        error("a { image = \"x:1\", port = \"80:abc\" }")
            .contains("`80:abc` n'est pas un port valide")
    );
    assert!(error("a { image = \"x:1\", port = 70000 }").contains("entre 1 et 65535"));
    assert!(error("a { image = \"x:1\", port = 80, ports = [81] }").contains("pas les deux"));
    assert!(error("a { image = \"x:1\", ports = 80 }").contains("`ports` est une liste"));
    assert!(error("a { image = \"x:1\", ports = [80, \"80:81\"] }").contains("publié deux fois"));
    // Deux nginx sans port : le second doit choisir.
    assert!(
        error("a { image = \"nginx:1.30\" }\nb { image = \"nginx:1.30\" }")
            .contains("déjà publié par `a`")
    );
}

#[test]
fn versions_apart_from_images() {
    let files = drc::compile_str(
        r#"
        db    { image = "postgres", version = "18-alpine" }
        cache { image = "redis", version = 8 }
        site  { build = "rust", version = "2.1.0", port = 8080 }
        "#,
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("image: postgres:18-alpine\n"));
    assert!(compose.contains("image: redis:8\n"));
    assert!(compose.contains("    build: .\n    image: site:2.1.0\n"));
    // Le port connu de l'image est déduit, version ou pas.
    assert!(compose.contains("      - \"5432:5432\"\n"));
    assert!(error("db { image = \"postgres:17\", version = \"18\" }").contains("pas les deux"));
    assert!(error("db { version = \"18\" }").contains("`version` a besoin d'une `image`"));
    // Un registre avec un port n'est pas une version.
    assert!(drc::compile_str("a { image = \"localhost:5000/api\", version = \"1\" }").is_ok());
}

fn with_overrides(source: &str, overrides: &[(&str, &str)]) -> Result<Vec<GeneratedFile>, String> {
    let overrides: Vec<(String, String)> = overrides
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    drc::compile_with(
        "main.drc",
        source,
        &mut |_, _| Err("pas d'import".into()),
        &overrides,
    )
    .map_err(|e| e.to_string())
}

#[test]
fn versions_chosen_when_compiling() {
    let source = "API = \"2.1.0\"\nREPLICAS = 2\napi { image = \"acme/api:\" + API, port = 3000, replicas = REPLICAS }";
    let files = with_overrides(source, &[("API", "2.2.0"), ("REPLICAS", "5")]).unwrap();
    assert!(file(&files, "docker-compose.yml").contains("image: acme/api:2.2.0\n"));
    assert!(file(&files, "deployment.yaml").contains("  replicas: 5\n"));
    // « 1.10 » reste un texte : la variable est un texte dans le fichier.
    let files = with_overrides(source, &[("API", "1.10")]).unwrap();
    assert!(file(&files, "docker-compose.yml").contains("image: acme/api:1.10\n"));

    assert_eq!(
        with_overrides(source, &[("APi", "3")]).unwrap_err(),
        "`--set APi=…` : aucune variable `APi` dans le projet (vouliez-vous dire `API` ?)"
    );
    assert!(
        with_overrides(source, &[("REPLICAS", "beaucoup")])
            .unwrap_err()
            .contains("`REPLICAS` est un nombre entier dans le fichier")
    );
}

#[test]
fn several_containers_in_one_app() {
    let files = drc::compile_str(
        r#"
        api {
            image = "acme/api:2"
            port = 3000
            with = {
                proxy = { image = "envoyproxy/envoy", version = "v1.36", port = "9901" }
                logs  = { image = "fluent/fluent-bit:4.1", env = { LEVEL = "info" } }
            }
        }
        "#,
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    // L'application publie aussi les ports de ses conteneurs, qui partagent son réseau.
    assert!(compose.contains(
        "  api:\n    image: acme/api:2\n    ports:\n      - \"3000:3000\"\n      - \"9901:9901\"\n"
    ));
    assert!(compose.contains("  api-proxy:\n    image: envoyproxy/envoy:v1.36\n    network_mode: service:api\n    depends_on:\n      - api\n"));
    assert!(compose.contains("  api-logs:\n    image: fluent/fluent-bit:4.1\n    environment:\n      LEVEL: info\n    network_mode: service:api\n"));

    let deployment = file(&files, "deployment.yaml");
    assert_eq!(deployment.matches("kind: Deployment").count(), 1);
    assert!(deployment.contains("        - name: api\n          image: acme/api:2\n"));
    assert!(deployment.contains("        - name: proxy\n          image: envoyproxy/envoy:v1.36\n          ports:\n            - containerPort: 9901\n"));
    assert!(deployment.contains("        - name: logs\n"));
    let service = file(&files, "service.yaml");
    assert!(service.contains(
        "    - name: p3000\n      port: 3000\n      targetPort: 3000\n    - name: p9901\n"
    ));

    assert!(
        error(
            "api { image = \"a:1\", port = 80, with = { side = { image = \"b:1\", port = 80 } } }"
        )
        .contains("écoutent tous deux sur le port 80")
    );
    assert!(
        error("api { image = \"a:1\", with = { side = { imag = \"b:1\" } } }")
            .contains("`api.with.side` : clé inconnue `imag` (vouliez-vous dire `image` ?)")
    );
    assert!(
        error("api { image = \"a:1\", with = { side = { image = \"b:1\", replicas = 2 } } }")
            .contains("clé inconnue `replicas`")
    );
    assert!(error("api { image = \"a:1\", with = [\"b:1\"] }").contains("`with` s'écrit"));
}

#[test]
fn a_web_stack_in_one_block() {
    let files = drc::compile_str(include_str!("../examples/web.drc")).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Dockerfile",
            "docker-compose.yml",
            "deployment.yaml",
            "service.yaml",
            "configmap.yaml",
            "statefulset.yaml",
            "secret.yaml"
        ]
    );
    assert!(file(&files, "Dockerfile").contains("RUN cargo install --path . --root /out\n"));
    let compose = file(&files, "docker-compose.yml");
    // L'application reçoit l'adresse de la base et du cache ; nginx seul est publié.
    assert!(compose.contains("      DATABASE_URL: postgres://postgres:change-me@web-postgres:5432/web\n      REDIS_URL: redis://web-redis:6379\n"));
    assert!(
        compose.contains("  web-nginx:\n    image: nginx:1.30\n    ports:\n      - \"80:80\"\n")
    );
    assert!(!compose.contains("8080:8080"));
    assert!(compose.contains("      - web-postgres:/var/lib/postgresql/data\n"));
    // `$` doublé pour Compose, simple pour Kubernetes.
    assert!(compose.contains("proxy_pass http://web:8080;\n") && compose.contains("Host $$host;"));
    assert!(file(&files, "configmap.yaml").contains("Host $host;"));
    let statefulset = file(&files, "statefulset.yaml");
    assert!(statefulset.contains("  serviceName: web-postgres\n"));
    assert!(statefulset.contains("            storage: \"1Gi\"\n"));
    assert!(statefulset.contains("              value: /var/lib/postgresql/data\n"));
    let secret = file(&files, "secret.yaml");
    assert!(secret.contains("  POSTGRES_PASSWORD: change-me\n"));
    let deployment = file(&files, "deployment.yaml");
    assert!(
        !deployment.contains("DATABASE_URL"),
        "le mot de passe reste dans le Secret"
    );
    assert!(
        deployment
            .contains("          envFrom:\n            - secretRef:\n                name: web\n")
    );
}

#[test]
fn stack_options_and_mistakes() {
    let files = drc::compile_str(
        "shop { image = \"acme/shop\", version = \"3\", port = 3000, mariadb = \"12\", password = \"p@ss\", host = \"shop.fr\" }",
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    // Sans nginx, l'application est publiée elle-même.
    assert!(
        compose.contains("  shop:\n    image: acme/shop:3\n    ports:\n      - \"3000:3000\"\n")
    );
    assert!(compose.contains("mysql://root:p%40ss@shop-mariadb:3306/shop"));
    assert!(compose.contains("      MARIADB_ROOT_PASSWORD: p@ss\n"));
    assert!(file(&files, "ingress.yaml").contains(
        "                name: shop\n                port:\n                  number: 3000\n"
    ));

    assert!(error("web { port = 3000, postgres }").contains("ajoutez `rust`"));
    assert!(
        error("web { rust = \".\", postgres = \"18\", mysql = \"9\" }")
            .contains("`postgres` et `mysql` donneraient tous deux `DATABASE_URL`")
    );
    assert!(
        error("web { rust = \".\", redis = \"8\", with = {} }")
            .contains("sert aux applications simples")
    );
    assert!(
        error("web { rust = \".\", postgress = \"18\" }").contains("vouliez-vous dire `postgres`")
    );
    assert!(
        error("web { rust = \".\", nginx = \"nginx:1.30\" }").contains("`nginx` est une version")
    );
    assert!(
        error("web { rust = \".\", nginx = \"1.30\" }\nfront { image = \"nginx:1.30\" }")
            .contains("déjà publié par `web`")
    );
}

#[test]
fn rust_variants_choose_the_matching_runtime() {
    let froms = |source: &str| -> String {
        let files = drc::compile_str(source).unwrap();
        file(&files, "Dockerfile")
            .lines()
            .filter(|l| l.starts_with("FROM"))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    assert_eq!(
        froms("web { rust = \".\" }"),
        "FROM rust:1.99-slim AS build | FROM debian:trixie-slim"
    );
    assert_eq!(
        froms("web { rust = \"1.99-slim-bookworm\" }"),
        "FROM rust:1.99-slim-bookworm AS build | FROM debian:bookworm-slim"
    );
    assert_eq!(
        froms("web { rust = \"1.99-alpine\" }"),
        "FROM rust:1.99-alpine AS build | FROM alpine:3.24"
    );
    assert_eq!(
        froms("web { rust = \"1.99-alpine3.22\" }"),
        "FROM rust:1.99-alpine3.22 AS build | FROM alpine:3.22"
    );
    assert_eq!(
        froms("web { rust = \"1.99-bullseye\" }"),
        "FROM rust:1.99-bullseye AS build | FROM debian:bullseye-slim"
    );
    // Le dossier du code, quand la variante est écrite.
    let files = drc::compile_str("web { rust = \"1.99-alpine\", src = \"./api\" }").unwrap();
    assert!(file(&files, "Dockerfile").contains("COPY ./api .\n"));
    // Les applications simples : `build = "rust:…"`.
    assert_eq!(
        froms("site { build = \"rust:1.99-slim-bookworm\", port = 8080 }"),
        "FROM rust:1.99-slim-bookworm AS build | FROM debian:bookworm-slim"
    );
    let files = drc::compile_str("site { build = \"rust:1.99-alpine\", port = 8080 }").unwrap();
    assert!(
        file(&files, "docker-compose.yml")
            .contains("  site:\n    build: .\n    image: site:latest\n")
    );

    // Les variantes des services : la valeur est le tag.
    let files = drc::compile_str(
        "web { rust = \".\", nginx = \"1.30-alpine\", postgres = \"18-alpine\", redis = \"8-alpine\" }",
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    for image in ["nginx:1.30-alpine", "postgres:18-alpine", "redis:8-alpine"] {
        assert!(compose.contains(&format!("image: {image}\n")), "{image}");
    }

    assert!(error("web { rust = \"1.99 alpine\" }").contains("variante de Rust"));
    assert!(
        error("web { rust = \".\", src = \"./api\" }")
            .contains("`src` est le dossier du code quand")
    );
    assert!(error("site { build = \"rust:\" }").contains("attend une variante de Rust"));
}

#[test]
fn names_alone_and_versions_without_quotes() {
    // Un nom seul : la dernière version stable, choisie par le compilateur.
    let files = drc::compile_str("web { rust nginx postgres redis }").unwrap();
    let compose = file(&files, "docker-compose.yml");
    for image in ["nginx:1.30", "postgres:18", "redis:8"] {
        assert!(compose.contains(&format!("image: {image}\n")), "{image}");
    }
    assert!(compose.contains("DATABASE_URL: postgres://"));
    assert!(file(&files, "Dockerfile").contains("FROM rust:1.99-slim AS build\n"));

    // Des versions sans guillemets, gardées telles qu'écrites.
    let files = drc::compile_str(
        "web {\n    rust     = 1.99-alpine\n    nginx    = 1.30-alpine\n    postgres = 18-alpine\n    redis    = 8\n}",
    )
    .unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("image: nginx:1.30-alpine\n"));
    assert!(compose.contains("image: postgres:18-alpine\n"));
    assert!(file(&files, "Dockerfile").contains("FROM alpine:3.24\n"));
    let files = drc::compile_str("db { image = \"postgres\", version = 17.10 }").unwrap();
    assert!(file(&files, "docker-compose.yml").contains("image: postgres:17.10\n"));

    // Sans application : juste les services.
    let files = drc::compile_str("data { postgres redis }").unwrap();
    let compose = file(&files, "docker-compose.yml");
    assert!(compose.contains("  data-postgres:\n") && compose.contains("  data-redis:\n"));
    assert!(!compose.contains("  data:\n"));
    assert!(file(&files, "statefulset.yaml").contains("name: data-postgres"));
    let files = drc::compile_str("site { nginx, host = \"monsite.fr\" }").unwrap();
    assert!(
        file(&files, "docker-compose.yml")
            .contains("  site-nginx:\n    image: nginx:1.30\n    ports:\n      - \"80:80\"\n")
    );
    assert!(!file(&files, "docker-compose.yml").contains("configs:"));
    assert!(file(&files, "ingress.yaml").contains("name: site-nginx"));

    assert!(
        error("data { postgres, host = \"a.fr\" }")
            .contains("`host` publie l'application ou nginx")
    );
    assert!(error("web { nginx = 1.30- }").contains("version incomplète"));
}
