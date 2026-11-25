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

mod apps;
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
        // Une erreur sans position (un argument `--set`) : le message seul.
        if self.pos.line == 0 {
            return write!(f, "{}", self.message);
        }
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

/// ` (vouliez-vous dire `x` ?)` quand `word` ressemble à l'un des `candidates`.
pub(crate) fn suggest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> String {
    // Au plus une faute pour trois lettres, et jamais tout le mot (`C` ne suggère pas `A`).
    let length = word.chars().count();
    let limit = (length / 3).max(1).min(length.saturating_sub(1));
    candidates
        .into_iter()
        .filter(|c| *c != word)
        .map(|c| (distance(&word.to_lowercase(), &c.to_lowercase()), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| format!(" (vouliez-vous dire `{c}` ?)"))
        .unwrap_or_default()
}

/// La distance d'édition entre deux mots (insertions, suppressions, substitutions et
/// inversions de deux lettres voisines).
pub(crate) fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
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
    compile_with(name, source, resolver, &[])
}

/// Comme [`compile`], avec des variables remplacées (`--set API=2.2.0`) : chaque nom doit
/// être une variable du projet, et la valeur garde le type de celle du fichier.
pub fn compile_with(
    name: &str,
    source: &str,
    resolver: &mut Resolver<'_>,
    overrides: &[(String, String)],
) -> Result<Vec<GeneratedFile>, Error> {
    let mut evaluator = eval::Evaluator::new(resolver);
    evaluator.set_overrides(overrides);
    evaluator.file(name, source)?;
    for (variable, _) in overrides {
        if !evaluator.overridden.contains(variable) {
            return Err(Error::at(
                Pos::default(),
                format!(
                    "`--set {variable}=…` : aucune variable `{variable}` dans le projet{}",
                    suggest(variable, evaluator.variables())
                ),
            ));
        }
    }
    let apps = std::mem::take(&mut evaluator.apps);
    let explicit = std::mem::take(&mut evaluator.sections);
    if apps.is_empty() && explicit.is_empty() {
        return Err(Error::at(
            Pos { line: 1, col: 1 },
            "rien à générer : ajoutez une application, par exemple `web { image = \"nginx:1.30\" }`",
        )
        .in_file(name));
    }
    let sections = apps::expand(&apps, explicit).map_err(|e| e.in_file(name))?;
    generate::files(&sections).map_err(|e| e.in_file(name))
}
