//! ORCHESTRA : le site de Rust‑Compose (Paquets) et l'outil `orchestra build`.
//!
//!     cargo run                                   # le site, http://localhost:3000
//!     cargo run -- build app.drc --out generated  # écrit Dockerfile, compose, manifests
//!     cargo run -- check app.drc                  # vérifie sans rien écrire
//!     cargo run -- export dist                    # le site en fichiers statiques (GitHub Pages)

mod site;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use orchestra::drc::{self, GeneratedFile};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("serve") => match site::app().run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("orchestra : {e}");
                ExitCode::FAILURE
            }
        },
        Some("build") => cli(&args[1..], true),
        Some("check") => cli(&args[1..], false),
        Some("export") => export(&args[1..]),
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("orchestra {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("orchestra : commande inconnue `{other}`\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "\
ORCHESTRA : Rust-Compose, un fichier .drc pour Docker, Compose et Kubernetes.

Usage :
  orchestra                         lance le site (PORT, 3000 par défaut)
  orchestra build <fichier.drc> [--out <dossier>] [--set NOM=valeur…]
                                    écrit Dockerfile, docker-compose.yml et les manifests
  orchestra check <fichier.drc> [--set NOM=valeur…]
                                    vérifie le fichier sans rien écrire

  --set NOM=valeur remplace une variable du fichier, par exemple --set API=2.2.0
  orchestra export <dossier>        le site en fichiers statiques (GitHub Pages)
                                    options : --base /chemin, --wasm fichier.wasm";

fn cli(args: &[String], write: bool) -> ExitCode {
    let mut input = None;
    let mut out = PathBuf::from(".");
    let mut overrides = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--set" => match args.next().and_then(|a| a.split_once('=')) {
                Some((name, value)) if !name.trim().is_empty() => {
                    overrides.push((name.trim().to_owned(), value.to_owned()));
                }
                _ => return usage_error("`--set` attend NOM=valeur (ex. --set API=2.2.0)"),
            },
            "--out" | "-o" if write => match args.next() {
                Some(dir) => out = PathBuf::from(dir),
                None => return usage_error("`--out` attend un dossier"),
            },
            flag if flag.starts_with('-') => {
                return usage_error(&format!("option inconnue `{flag}`"));
            }
            file if input.is_none() => input = Some(PathBuf::from(file)),
            extra => return usage_error(&format!("argument en trop `{extra}`")),
        }
    }
    let Some(input) = input else {
        return usage_error("indiquez un fichier .drc");
    };
    let files = match compile_file(&input, &overrides) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("erreur : {message}");
            return ExitCode::FAILURE;
        }
    };
    if !write {
        println!(
            "✓ {} : {} fichier(s) à générer",
            input.display(),
            files.len()
        );
        return ExitCode::SUCCESS;
    }
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("erreur : {} : {e}", out.display());
        return ExitCode::FAILURE;
    }
    // Le code est à côté du fichier .drc : si les fichiers sont écrits ailleurs
    // (`--out deploy`), le `build` de Compose doit encore y mener.
    let project = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let has_dockerfile = files.iter().any(|f| f.name == "Dockerfile");
    for file in &files {
        let path = out.join(&file.name);
        let content = if file.name == "docker-compose.yml" {
            relocate_builds(&file.content, project, &out, has_dockerfile)
        } else {
            file.content.clone()
        };
        if let Err(e) = std::fs::write(&path, &content) {
            eprintln!("erreur : {} : {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("✓ {}", path.display());
    }
    ExitCode::SUCCESS
}

/// `orchestra export <dossier>`. Dans GitHub Actions, le chemin du site (`/ORCHESTRA`)
/// est déduit du dépôt ; `--base` le remplace.
fn export(args: &[String]) -> ExitCode {
    let mut out = None;
    let mut base = site::export::default_base(std::env::var("GITHUB_REPOSITORY").ok().as_deref());
    let mut wasm = PathBuf::from(site::export::WASM);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => match args.next() {
                Some(value) => base = value.clone(),
                None => return usage_error("`--base` attend un chemin (ex. /ORCHESTRA)"),
            },
            "--wasm" => match args.next() {
                Some(value) => wasm = PathBuf::from(value),
                None => return usage_error("`--wasm` attend un fichier"),
            },
            flag if flag.starts_with('-') => {
                return usage_error(&format!("option inconnue `{flag}`"));
            }
            dir if out.is_none() => out = Some(PathBuf::from(dir)),
            extra => return usage_error(&format!("argument en trop `{extra}`")),
        }
    }
    let Some(out) = out else {
        return usage_error("indiquez le dossier de sortie");
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("erreur : {e}");
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(site::export::export(&out, &base, &wasm)) {
        Ok(report) => {
            println!("✓ {} fichier(s) dans {}", report.files, out.display());
            if !report.wasm {
                eprintln!(
                    "attention : {} absent, le bac à sable ne compilera pas en ligne (lancez d'abord `cargo wasm`)",
                    wasm.display()
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}

/// Réécrit les `build:` d'un fichier Compose écrit dans `out` pour un projet situé dans
/// `project` : le contexte mène au projet, et le Dockerfile généré est retrouvé dans `out`.
fn relocate_builds(compose: &str, project: &Path, out: &Path, has_dockerfile: bool) -> String {
    let (Ok(project), Ok(out)) = (project.canonicalize(), out.canonicalize()) else {
        return compose.to_owned();
    };
    if project == out {
        return compose.to_owned();
    }
    let mut result = String::with_capacity(compose.len());
    // `build:` sur plusieurs lignes : (indentation de `build:`, dossier du contexte).
    let mut block: Option<(usize, std::path::PathBuf)> = None;
    for line in compose.split_inclusive('\n') {
        let indent = line.len() - line.trim_start().len();
        if let Some((build_indent, context)) = &block {
            if indent > *build_indent {
                let pad = " ".repeat(indent);
                if let Some(dir) = line.trim().strip_prefix("context: ") {
                    let source = project.join(dir.trim_matches('"'));
                    result.push_str(&format!("{pad}context: {}\n", relative(&out, &source)));
                    block = Some((*build_indent, source));
                    continue;
                }
                if let Some(file) = line.trim().strip_prefix("dockerfile: ") {
                    let file = out.join(file.trim_matches('"'));
                    result.push_str(&format!("{pad}dockerfile: {}\n", relative(context, &file)));
                    continue;
                }
                result.push_str(line);
                continue;
            }
            block = None;
        }
        if line.trim() == "build:" {
            block = Some((indent, project.clone()));
            result.push_str(line);
            continue;
        }
        match line.trim().strip_prefix("build: ") {
            Some(dir) => {
                let pad = " ".repeat(indent);
                let source = project.join(dir.trim_matches('"'));
                let context = relative(&out, &source);
                result.push_str(&format!("{pad}build:\n{pad}  context: {context}\n"));
                if dir == "." && has_dockerfile {
                    let dockerfile = relative(&source, &out.join("Dockerfile"));
                    result.push_str(&format!("{pad}  dockerfile: {dockerfile}\n"));
                }
            }
            None => result.push_str(line),
        }
    }
    result
}

/// Le chemin de `to` vu depuis le dossier `from` (`../src`, `deploy/Dockerfile`…).
fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to_parts: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts: Vec<String> = vec!["..".to_owned(); from.len() - common];
    parts.extend(
        to_parts[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("orchestra : {message}\n\n{USAGE}");
    ExitCode::from(2)
}

/// Compile un fichier ; ses `import` sont lus par rapport à son dossier.
fn compile_file(path: &Path, overrides: &[(String, String)]) -> Result<Vec<GeneratedFile>, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let name = path.display().to_string();
    drc::compile_with(
        &name,
        &source,
        &mut |from, import| {
            let base = Path::new(from).parent().unwrap_or(Path::new("."));
            let target = base.join(import);
            let canonical = target
                .canonicalize()
                .map_err(|e| format!("import \"{import}\" : {e}"))?;
            let source = std::fs::read_to_string(&canonical)
                .map_err(|e| format!("import \"{import}\" : {e}"))?;
            Ok((canonical.display().to_string(), source))
        },
        overrides,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_still_lead_to_the_code() {
        assert_eq!(relative(Path::new("/p/deploy"), Path::new("/p")), "..");
        assert_eq!(
            relative(Path::new("/p"), Path::new("/p/deploy/Dockerfile")),
            "deploy/Dockerfile"
        );
        assert_eq!(relative(Path::new("/p/a/b"), Path::new("/p/c")), "../../c");
        assert_eq!(relative(Path::new("/p"), Path::new("/p")), ".");

        let dir = std::env::temp_dir().join(format!("orchestra-out-{}", std::process::id()));
        let out = dir.join("deploy");
        std::fs::create_dir_all(&out).unwrap();
        let compose =
            "services:\n  web:\n    build: .\n    image: web:latest\n  api:\n    build: ./api\n";
        let moved = relocate_builds(compose, &dir, &out, true);
        assert_eq!(
            moved,
            "services:\n  web:\n    build:\n      context: ..\n      dockerfile: deploy/Dockerfile\n    image: web:latest\n  api:\n    build:\n      context: ../api\n"
        );
        assert_eq!(relocate_builds(compose, &dir, &dir, true), compose);
        let several = "services:\n  api:\n    build:\n      context: .\n      dockerfile: Dockerfile.api\n    image: api:latest\n";
        assert_eq!(
            relocate_builds(several, &dir, &out, true),
            "services:\n  api:\n    build:\n      context: ..\n      dockerfile: deploy/Dockerfile.api\n    image: api:latest\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
