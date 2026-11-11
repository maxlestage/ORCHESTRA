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
    for file in &files {
        let path = out.join(&file.name);
        if let Err(e) = std::fs::write(&path, &file.content) {
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
