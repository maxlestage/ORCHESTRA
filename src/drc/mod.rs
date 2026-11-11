//! Rust‑Compose : un langage déclaratif (`.drc`) compilé en Dockerfile,
//! `docker-compose.yml` et manifests Kubernetes.
//!
//! ```
//! use orchestra::drc;
//!
//! let files = drc::compile_str(r#"
//!     var IMAGE_NAME = "myapp"
//!     fn image_tag(tag) { return format!("{}:{}", IMAGE_NAME, tag) }
//!     compose { services = { web = { image = image_tag("latest"), ports = ["8080:80"] } } }
//! "#).unwrap();
//! assert_eq!(files[0].name, "docker-compose.yml");
//! assert!(files[0].content.contains("image: myapp:latest"));
//! ```

mod eval;
mod generate;
pub mod lexer;
pub mod parser;
pub mod yaml;

use std::fmt;

pub use eval::BUILTINS;

/// Lit un import : `resolver(depuis, chemin)` renvoie le nom canonique du fichier
/// (pour repérer les cycles) et son contenu.
pub type Resolver<'a> = dyn FnMut(&str, &str) -> Result<(String, String), String> + 'a;
pub use generate::KINDS;

/// Une position dans un fichier (à partir de 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

/// Une erreur de compilation, avec sa position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub file: Option<String>,
    pub pos: Pos,
    pub message: String,
}

impl Error {
    pub fn at(pos: Pos, message: impl Into<String>) -> Self {
        Self {
            file: None,
            pos,
            message: message.into(),
        }
    }

    fn in_file(mut self, file: &str) -> Self {
        if self.file.is_none() {
            self.file = Some(file.to_owned());
        }
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{file}:")?;
        }
        write!(f, "{}:{} : {}", self.pos.line, self.pos.col, self.message)
    }
}

impl std::error::Error for Error {}

/// Une valeur évaluée. Les objets gardent l'ordre de leurs clés.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Str(_) => "une chaîne",
            Value::Int(_) | Value::Float(_) => "un nombre",
            Value::Bool(_) => "un booléen",
            Value::List(_) => "une liste",
            Value::Object(_) => "un objet",
        }
    }

    /// La valeur d'une clé, si c'est un objet.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Un fichier généré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub name: String,
    pub content: String,
}

/// Compile un texte seul : `import` y est refusé.
pub fn compile_str(source: &str) -> Result<Vec<GeneratedFile>, Error> {
    compile("main.drc", source, &mut |_, path| {
        Err(format!(
            "`import \"{path}\"` : les imports ne sont possibles qu'avec des fichiers (orchestra build)"
        ))
    })
}

/// Compile un projet. `resolver(depuis, chemin)` lit chaque import et renvoie son
/// nom canonique (pour repérer les cycles) et son contenu.
pub fn compile(
    name: &str,
    source: &str,
    resolver: &mut Resolver<'_>,
) -> Result<Vec<GeneratedFile>, Error> {
    let mut evaluator = eval::Evaluator::new(resolver);
    evaluator.file(name, source)?;
    let sections = std::mem::take(&mut evaluator.sections);
    if sections.is_empty() {
        return Err(Error::at(
            Pos { line: 1, col: 1 },
            "aucune section : ajoutez `docker { … }`, `compose { … }` ou `k8s { … }`",
        )
        .in_file(name));
    }
    generate::files(&sections).map_err(|e| e.in_file(name))
}
