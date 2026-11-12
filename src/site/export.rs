//! `orchestra export <dossier>` : le site en fichiers statiques, pour GitHub Pages ou
//! tout hébergeur statique. Chaque page vient du serveur lui-même, par le client de
//! test de Paquets : les deux versions sont identiques.
//!
//! Ce qu'un hébergeur statique ne sait pas faire est remplacé :
//! - les pages s'écrivent `langage.html` (GitHub Pages sert `/langage` sans redirection) ;
//! - le bac à sable compile dans le navigateur avec `orchestra.wasm` (le formulaire et
//!   l'API `/api/compile` ont besoin du serveur) ;
//! - les liens passent derrière `base` (`/ORCHESTRA` pour un site de projet GitHub Pages).

use std::io;
use std::path::{Path, PathBuf};

use super::{app, examples};

/// Où `cargo wasm` écrit le module.
pub const WASM: &str = "target/wasm32-unknown-unknown/wasm/orchestra.wasm";

/// Ce que l'export a écrit.
pub struct Report {
    pub files: usize,
    pub wasm: bool,
}

/// Écrit le site dans `out`, liens derrière `base` (`""` ou `/ORCHESTRA`). `wasm` est
/// le compilateur du navigateur ; sans lui, le bac à sable n'affiche que les exemples.
pub async fn export(out: &Path, base: &str, wasm: &Path) -> io::Result<Report> {
    let base = normalize_base(base)?;
    let mut app = app();
    // Des fichiers statiques n'ont pas de serveur à protéger, et l'export demande toutes
    // les pages d'un coup (la limitation de débit l'arrêterait).
    app.security(None);
    let client = app.test().await;

    let mut pages: Vec<(String, String, u16)> = vec![
        ("/".into(), "index.html".into(), 200),
        ("/langage".into(), "langage.html".into(), 200),
        ("/guide".into(), "guide.html".into(), 200),
        ("/bac-a-sable".into(), "bac-a-sable.html".into(), 200),
        // `/bac-a-sable` et le dossier `bac-a-sable/` : GitHub Pages peut rediriger
        // vers le dossier, qui doit alors avoir sa page.
        ("/bac-a-sable".into(), "bac-a-sable/index.html".into(), 200),
        ("/404".into(), "404.html".into(), 404),
    ];
    let ids = examples::ALL
        .iter()
        .map(|e| e.id)
        .chain(super::guide::RECIPES.iter().map(|r| r.id));
    for id in ids {
        pages.push((
            format!("/bac-a-sable/{id}"),
            format!("bac-a-sable/{id}.html"),
            200,
        ));
    }

    let mut report = Report {
        files: 0,
        wasm: false,
    };
    let mut write = |path: &str, bytes: &[u8]| -> io::Result<()> {
        let file = out.join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, bytes)?;
        report.files += 1;
        Ok(())
    };
    for (url, file, expected) in &pages {
        let res = client.get(url).await;
        if res.status().as_u16() != *expected {
            return Err(io::Error::other(format!(
                "GET {url} : {} au lieu de {expected}",
                res.status()
            )));
        }
        write(file, with_base(&res.text(), &base).as_bytes())?;
    }
    for file in files(Path::new("public"))? {
        let relative = file.strip_prefix("public").map_err(io::Error::other)?;
        write(&relative.to_string_lossy(), &std::fs::read(&file)?)?;
    }
    // GitHub Pages : servir les fichiers tels quels, sans Jekyll.
    write(".nojekyll", b"")?;
    if wasm.is_file() {
        write("orchestra.wasm", &std::fs::read(wasm)?)?;
        report.wasm = true;
    }
    Ok(report)
}

/// Le chemin du site sur GitHub Pages, d'après le dépôt (`owner/nom`) : `/nom`, ou rien
/// pour le site principal `owner.github.io` et hors de GitHub Actions.
pub fn default_base(repository: Option<&str>) -> String {
    match repository.and_then(|r| r.split_once('/')) {
        Some((owner, name)) if !name.eq_ignore_ascii_case(&format!("{owner}.github.io")) => {
            format!("/{name}")
        }
        _ => String::new(),
    }
}

/// `ORCHESTRA`, `/ORCHESTRA/` → `/ORCHESTRA` ; `/` ou rien → `""`.
fn normalize_base(base: &str) -> io::Result<String> {
    let trimmed = base.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    {
        return Err(io::Error::other(format!(
            "chemin de base invalide `{base}`"
        )));
    }
    Ok(format!("/{trimmed}"))
}

/// Préfixe par `base` les adresses absolues du site (`href="/…"`, `src`, `action`,
/// `data-wasm`), pas celles d'un autre site (`//…`) ni le texte des exemples (échappé).
fn with_base(html: &str, base: &str) -> String {
    if base.is_empty() {
        return html.to_owned();
    }
    let mut out = html.to_owned();
    for attr in ["href", "src", "action", "data-wasm"] {
        let from = format!("{attr}=\"/");
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.find(&from) {
            let after = &rest[at + from.len()..];
            result.push_str(&rest[..at]);
            if after.starts_with('/') {
                result.push_str(&from);
            } else {
                result.push_str(&format!("{attr}=\"{base}/"));
            }
            rest = after;
        }
        result.push_str(rest);
        out = result;
    }
    out
}

fn files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    if !dir.is_dir() {
        return Ok(found);
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_move_behind_the_base_path() {
        let html = r#"<a href="/langage">L</a><a href="//cdn.example">x</a><form action="/bac-a-sable" data-wasm="/orchestra.wasm"></form><a href="https://github.com">g</a>&quot;/app&quot;"#;
        assert_eq!(
            with_base(html, "/ORCHESTRA"),
            r#"<a href="/ORCHESTRA/langage">L</a><a href="//cdn.example">x</a><form action="/ORCHESTRA/bac-a-sable" data-wasm="/ORCHESTRA/orchestra.wasm"></form><a href="https://github.com">g</a>&quot;/app&quot;"#
        );
        assert_eq!(with_base(html, ""), html);
        assert_eq!(normalize_base("ORCHESTRA/").unwrap(), "/ORCHESTRA");
        assert_eq!(normalize_base("/").unwrap(), "");
        assert!(normalize_base("a\"b").is_err());
        assert_eq!(default_base(Some("maxlestage/ORCHESTRA")), "/ORCHESTRA");
        assert_eq!(default_base(Some("maxlestage/maxlestage.github.io")), "");
        assert_eq!(default_base(None), "");
    }

    #[tokio::test]
    async fn the_whole_site_is_exported() {
        let out = std::env::temp_dir().join(format!("orchestra-export-{}", std::process::id()));
        let report = export(&out, "/ORCHESTRA", Path::new("absent.wasm"))
            .await
            .unwrap();
        assert!(!report.wasm);
        for file in [
            "index.html",
            "langage.html",
            "bac-a-sable.html",
            "bac-a-sable/k8s.html",
            "404.html",
            "favicon.svg",
            ".nojekyll",
        ] {
            assert!(out.join(file).is_file(), "{file}");
        }
        let home = std::fs::read_to_string(out.join("index.html")).unwrap();
        assert!(home.contains(r#"href="/ORCHESTRA/langage""#));
        assert!(!home.contains(r#"href="/langage""#));
        let missing = std::fs::read_to_string(out.join("404.html")).unwrap();
        assert!(missing.contains("Fausse note"));
        std::fs::remove_dir_all(out).unwrap();
    }
}
