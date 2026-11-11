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
    assert!(error("var X = 1").contains("aucune section"));
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
