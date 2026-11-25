//! Construit l'arbre d'un fichier `.drc` à partir de ses jetons.
//!
//! ```text
//! fichier   = { item }
//! item      = "import" chaîne
//!           | [ "var" ] nom "=" expr
//!           | "fn" nom "(" [ nom { "," nom } ] ")" ( "=" expr | "{" { instr } "}" )
//!           | [ "app" ] nom objet               (`app "nom"` aussi)
//!           | section [ "=" ] objet           (section = docker | compose | k8s)
//! instr     = [ "var" ] nom "=" expr | "return" expr
//! expr      = terme { "+" terme }
//! terme     = chaîne | nombre | true | false | liste | objet
//!           | nom | nom "(" args ")" | nom "!" "(" args ")"
//! objet     = "{" { clé "=" expr [ "," ] } "}"
//! liste     = "[" { expr [ "," ] } "]"          (virgules facultatives)
//! ```

use super::lexer::{Tok, Token};
use super::{Error, Pos};

/// Une expression, avec sa position.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Expr>),
    Object(Vec<Entry>),
    Var(String),
    /// `a + b` : assemble des chaînes, additionne des nombres, met bout à bout des
    /// listes, fusionne des objets.
    Add(Box<Expr>, Box<Expr>),
    /// `nom(args)`, ou `nom!(args)` pour une macro (`format!`).
    Call {
        name: String,
        args: Vec<Expr>,
        macro_call: bool,
    },
}

/// Une entrée `clé = valeur` d'un objet.
#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: Expr,
    pub pos: Pos,
}

/// Les trois cibles de génération.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Docker,
    Compose,
    K8s,
}

impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Section::Docker => "docker",
            Section::Compose => "compose",
            Section::K8s => "k8s",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Import {
        path: String,
        pos: Pos,
    },
    Var {
        name: String,
        value: Expr,
        pos: Pos,
    },
    Fn(FnDef),
    /// `app web { … }` : une application, déclinée en Compose, Kubernetes et Dockerfile.
    App {
        name: String,
        body: Expr,
        pos: Pos,
    },
    Section {
        section: Section,
        body: Expr,
        pos: Pos,
    },
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Var { name: String, value: Expr },
    Return(Expr),
}

/// Les éléments d'un fichier, dans l'ordre.
pub fn parse(tokens: &[Token]) -> Result<Vec<Item>, Error> {
    let mut parser = Parser {
        tokens,
        at: 0,
        depth: 0,
    };
    let mut items = Vec::new();
    while parser.peek() != &Tok::Eof {
        items.push(parser.item()?);
        while parser.peek() == &Tok::Semi {
            parser.at += 1;
        }
    }
    Ok(items)
}

/// Les mots que l'on ne peut pas utiliser comme nom de variable ou de fonction.
const RESERVED: &[&str] = &[
    "var", "fn", "return", "import", "true", "false", "docker", "compose", "k8s",
];

/// Les mots qui commencent une déclaration.
pub const DECLARATIONS: &[&str] = &["app", "var", "fn", "import", "docker", "compose", "k8s"];

/// Imbrication maximale des listes, objets et appels.
const MAX_NESTING: usize = 64;

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.tokens[self.at].kind
    }

    fn peek_at(&self, offset: usize) -> &Tok {
        let last = self.tokens.len() - 1;
        &self.tokens[(self.at + offset).min(last)].kind
    }

    fn pos(&self) -> Pos {
        self.tokens[self.at].pos
    }

    fn next(&mut self) -> &Token {
        let token = &self.tokens[self.at];
        if token.kind != Tok::Eof {
            self.at += 1;
        }
        token
    }

    fn expect(&mut self, tok: Tok, context: &str) -> Result<Pos, Error> {
        let token = self.next();
        if token.kind == tok {
            Ok(token.pos)
        } else {
            Err(Error::at(
                token.pos,
                format!(
                    "{} attendu {context}, trouvé {}",
                    tok.describe(),
                    token.kind.describe()
                ),
            ))
        }
    }

    fn name(&mut self, what: &str) -> Result<(String, Pos), Error> {
        let token = self.next().clone();
        match token.kind {
            Tok::Ident(name) if RESERVED.contains(&name.as_str()) => Err(Error::at(
                token.pos,
                format!("`{name}` est un mot réservé, il ne peut pas servir de {what}"),
            )),
            Tok::Ident(name) => Ok((name, token.pos)),
            other => Err(Error::at(
                token.pos,
                format!("{what} attendu, trouvé {}", other.describe()),
            )),
        }
    }

    fn item(&mut self) -> Result<Item, Error> {
        let pos = self.pos();
        let keyword = match self.peek() {
            Tok::Ident(word) => word.clone(),
            other => {
                return Err(Error::at(
                    pos,
                    format!(
                        "déclaration attendue (`app`, `var`, `fn`, `import`, `docker`, `compose` ou `k8s`), trouvé {}",
                        other.describe()
                    ),
                ));
            }
        };
        let section = match keyword.as_str() {
            "docker" => Some(Section::Docker),
            "compose" => Some(Section::Compose),
            "k8s" => Some(Section::K8s),
            _ => None,
        };
        if let Some(section) = section {
            self.at += 1;
            if self.peek() == &Tok::Eq {
                self.at += 1;
            }
            if self.peek() != &Tok::LBrace {
                return Err(Error::at(
                    self.pos(),
                    format!(
                        "`{{` attendu après `{}`, trouvé {}",
                        section.name(),
                        self.peek().describe()
                    ),
                ));
            }
            let body = self.expr()?;
            return Ok(Item::Section { section, body, pos });
        }
        // Sans mot-clé : `web { … }` est une application, `VERSION = "1.0"` une variable.
        let bare =
            !DECLARATIONS.contains(&keyword.as_str()) && !RESERVED.contains(&keyword.as_str());
        let next = self.peek_at(1).clone();
        if bare && matches!(next, Tok::LBrace | Tok::Eq) {
            // `dokcer { … }` est une faute de frappe, pas une application nommée dokcer.
            // (une seule lettre d'écart, et des mots d'au moins 4 lettres : `api`, `worker`
            // ou `db` sont des noms d'application).
            let hint = ["docker", "compose", "import"]
                .into_iter()
                .find(|k| keyword.chars().count() >= 4 && super::distance(&keyword, k) == 1)
                .map(|k| format!(" (vouliez-vous dire `{k}` ?)"))
                .unwrap_or_default();
            if !hint.is_empty() {
                let explicit = if next == Tok::LBrace {
                    format!("`app {keyword} {{ … }}`")
                } else {
                    format!("`var {keyword} = …`")
                };
                return Err(Error::at(
                    pos,
                    format!("`{keyword}`{hint} ; si ce nom est voulu, écrivez {explicit}"),
                ));
            }
            self.at += 1;
            if next == Tok::LBrace {
                let body = self.expr()?;
                return Ok(Item::App {
                    name: keyword,
                    body,
                    pos,
                });
            }
            self.at += 1;
            let value = self.expr()?;
            return Ok(Item::Var {
                name: keyword,
                value,
                pos,
            });
        }
        match keyword.as_str() {
            "app" => {
                self.at += 1;
                let token = self.next().clone();
                let name = match token.kind {
                    Tok::Ident(name) | Tok::Str(name) => name,
                    other => {
                        return Err(Error::at(
                            token.pos,
                            format!(
                                "nom de l'application attendu après `app` (ex. `app web {{ … }}`), trouvé {}",
                                other.describe()
                            ),
                        ));
                    }
                };
                if self.peek() != &Tok::LBrace {
                    return Err(Error::at(
                        self.pos(),
                        format!(
                            "`{{` attendu après `app {name}`, trouvé {}",
                            self.peek().describe()
                        ),
                    ));
                }
                let body = self.expr()?;
                Ok(Item::App { name, body, pos })
            }
            "import" => {
                self.at += 1;
                let token = self.next().clone();
                match token.kind {
                    Tok::Str(path) => Ok(Item::Import { path, pos }),
                    other => Err(Error::at(
                        token.pos,
                        format!(
                            "chemin entre guillemets attendu après `import`, trouvé {}",
                            other.describe()
                        ),
                    )),
                }
            }
            "var" => {
                self.at += 1;
                let (name, _) = self.name("nom de variable")?;
                self.expect(Tok::Eq, "après le nom de la variable")?;
                let value = self.expr()?;
                Ok(Item::Var { name, value, pos })
            }
            "fn" => {
                self.at += 1;
                let (name, _) = self.name("nom de fonction")?;
                self.expect(Tok::LParen, "après le nom de la fonction")?;
                let mut params = Vec::new();
                while self.peek() != &Tok::RParen {
                    let (param, ppos) = self.name("nom de paramètre")?;
                    if params.contains(&param) {
                        return Err(Error::at(ppos, format!("paramètre `{param}` en double")));
                    }
                    params.push(param);
                    if self.peek() == &Tok::Comma {
                        self.at += 1;
                    } else {
                        break;
                    }
                }
                self.expect(Tok::RParen, "pour fermer les paramètres")?;
                // `fn tag(v) = IMAGE + ":" + v` : une fonction d'une seule expression.
                if self.peek() == &Tok::Eq {
                    self.at += 1;
                    let value = self.expr()?;
                    return Ok(Item::Fn(FnDef {
                        name,
                        params,
                        body: vec![Stmt::Return(value)],
                        pos,
                    }));
                }
                self.expect(Tok::LBrace, "pour ouvrir le corps de la fonction (ou `=`)")?;
                let mut body = Vec::new();
                while self.peek() != &Tok::RBrace {
                    body.push(self.stmt()?);
                    while self.peek() == &Tok::Semi {
                        self.at += 1;
                    }
                }
                self.expect(Tok::RBrace, "pour fermer la fonction")?;
                Ok(Item::Fn(FnDef {
                    name,
                    params,
                    body,
                    pos,
                }))
            }
            other => Err(Error::at(
                pos,
                format!(
                    "déclaration inconnue `{other}`{} : attendu `app`, `var`, `fn`, `import`, `docker`, `compose` ou `k8s`",
                    super::suggest(other, DECLARATIONS.iter().copied())
                ),
            )),
        }
    }

    fn stmt(&mut self) -> Result<Stmt, Error> {
        let pos = self.pos();
        match self.peek() {
            Tok::Ident(word) if word == "return" => {
                self.at += 1;
                Ok(Stmt::Return(self.expr()?))
            }
            // `x = …` ou `var x = …`
            Tok::Ident(word)
                if word == "var"
                    || (self.peek_at(1) == &Tok::Eq && !RESERVED.contains(&word.as_str())) =>
            {
                if word == "var" {
                    self.at += 1;
                }
                let (name, _) = self.name("nom de variable")?;
                self.expect(Tok::Eq, "après le nom de la variable")?;
                Ok(Stmt::Var {
                    name,
                    value: self.expr()?,
                })
            }
            Tok::Eof => Err(Error::at(pos, "fonction jamais fermée (il manque `}`)")),
            other => Err(Error::at(
                pos,
                format!(
                    "`var` ou `return` attendu dans une fonction, trouvé {}",
                    other.describe()
                ),
            )),
        }
    }

    fn expr(&mut self) -> Result<Expr, Error> {
        if self.depth >= MAX_NESTING {
            return Err(Error::at(
                self.pos(),
                format!("valeurs imbriquées sur plus de {MAX_NESTING} niveaux"),
            ));
        }
        self.depth += 1;
        let expr = self.sum();
        self.depth -= 1;
        expr
    }

    /// `terme + terme + …`
    fn sum(&mut self) -> Result<Expr, Error> {
        let mut left = self.term()?;
        while self.peek() == &Tok::Plus {
            let pos = self.pos();
            self.at += 1;
            let right = self.term()?;
            left = Expr {
                kind: ExprKind::Add(Box::new(left), Box::new(right)),
                pos,
            };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, Error> {
        let token = self.next().clone();
        let pos = token.pos;
        let kind = match token.kind {
            Tok::Str(s) => ExprKind::Str(s),
            Tok::Int(n) => ExprKind::Int(n),
            Tok::Float(n) => ExprKind::Float(n),
            Tok::LBracket => {
                let mut items = Vec::new();
                while self.peek() != &Tok::RBracket {
                    if self.peek() == &Tok::Eof {
                        return Err(Error::at(pos, "liste jamais fermée (il manque `]`)"));
                    }
                    items.push(self.expr()?);
                    // Les virgules sont facultatives : `["a" "b"]` vaut `["a", "b"]`.
                    if self.peek() == &Tok::Comma {
                        self.at += 1;
                    }
                }
                self.at += 1;
                ExprKind::List(items)
            }
            Tok::LBrace => ExprKind::Object(self.entries(pos)?),
            Tok::Ident(word) => match word.as_str() {
                "true" => ExprKind::Bool(true),
                "false" => ExprKind::Bool(false),
                _ if self.peek() == &Tok::LParen => {
                    self.at += 1;
                    ExprKind::Call {
                        name: word,
                        args: self.args()?,
                        macro_call: false,
                    }
                }
                _ if self.peek() == &Tok::Bang && self.peek_at(1) == &Tok::LParen => {
                    self.at += 2;
                    ExprKind::Call {
                        name: word,
                        args: self.args()?,
                        macro_call: true,
                    }
                }
                _ if RESERVED.contains(&word.as_str()) => {
                    return Err(Error::at(
                        pos,
                        format!("valeur attendue, trouvé le mot réservé `{word}`"),
                    ));
                }
                _ => ExprKind::Var(word),
            },
            other => {
                return Err(Error::at(
                    pos,
                    format!("valeur attendue, trouvé {}", other.describe()),
                ));
            }
        };
        Ok(Expr { kind, pos })
    }

    /// Les arguments d'un appel, après `(`.
    fn args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        while self.peek() != &Tok::RParen {
            args.push(self.expr()?);
            if self.peek() == &Tok::Comma {
                self.at += 1;
            } else {
                break;
            }
        }
        self.expect(Tok::RParen, "pour fermer l'appel")?;
        Ok(args)
    }

    /// Les entrées d'un objet, après `{`.
    fn entries(&mut self, open: Pos) -> Result<Vec<Entry>, Error> {
        let mut entries: Vec<Entry> = Vec::new();
        loop {
            let token = self.next().clone();
            let key = match token.kind {
                Tok::RBrace => return Ok(entries),
                Tok::Ident(key) | Tok::Str(key) => key,
                Tok::Eof => return Err(Error::at(open, "objet jamais fermé (il manque `}`)")),
                other => {
                    return Err(Error::at(
                        token.pos,
                        format!("clé ou `}}` attendu, trouvé {}", other.describe()),
                    ));
                }
            };
            if entries.iter().any(|e| e.key == key) {
                return Err(Error::at(token.pos, format!("clé `{key}` en double")));
            }
            // Un nom seul vaut `true` : `web { rust nginx postgres }`.
            let value = if matches!(
                self.peek(),
                Tok::Ident(_) | Tok::RBrace | Tok::Comma | Tok::Semi
            ) {
                Expr {
                    kind: ExprKind::Bool(true),
                    pos: token.pos,
                }
            } else {
                self.expect(Tok::Eq, &format!("après la clé `{key}`"))?;
                self.expr()?
            };
            entries.push(Entry {
                key,
                value,
                pos: token.pos,
            });
            if matches!(self.peek(), Tok::Comma | Tok::Semi) {
                self.at += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::lexer::tokenize;
    use super::*;

    fn items(source: &str) -> Vec<Item> {
        parse(&tokenize(source).unwrap()).unwrap()
    }

    #[test]
    fn newlines_or_commas_separate_entries() {
        let parsed = items("docker { base = \"a\"\n workdir = \"/app\", run = [\"x\"] }");
        let Item::Section { body, .. } = &parsed[0] else {
            panic!()
        };
        let ExprKind::Object(entries) = &body.kind else {
            panic!()
        };
        let keys: Vec<_> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["base", "workdir", "run"]);
    }

    #[test]
    fn functions_and_macros() {
        let parsed = items("fn tag(v) { var t = v\n return format!(\"{}:{}\", NAME, t) }");
        let Item::Fn(def) = &parsed[0] else { panic!() };
        assert_eq!(def.params, ["v"]);
        assert_eq!(def.body.len(), 2);
    }

    #[test]
    fn simple_forms() {
        let parsed = items("fn tag(v) = NAME + \":\" + v\napp web { port = 80 }\nvar L = [1 2, 3]");
        let Item::Fn(def) = &parsed[0] else { panic!() };
        assert!(matches!(
            def.body[..],
            [Stmt::Return(Expr {
                kind: ExprKind::Add(..),
                ..
            })]
        ));
        assert!(matches!(&parsed[1], Item::App { name, .. } if name == "web"));
        let Item::Var { value, .. } = &parsed[2] else {
            panic!()
        };
        assert!(matches!(&value.kind, ExprKind::List(items) if items.len() == 3));
        let err = parse(&tokenize("dokcer { }").unwrap()).unwrap_err();
        assert!(
            err.message.contains("vouliez-vous dire `docker`"),
            "{}",
            err.message
        );
    }

    #[test]
    fn errors_point_at_the_problem() {
        let err = parse(&tokenize("var = 3").unwrap()).unwrap_err();
        assert_eq!(err.pos.col, 5);
        let err = parse(&tokenize("k8s { a = 1 a = 2 }").unwrap()).unwrap_err();
        assert!(err.message.contains("en double"));
        let err = parse(&tokenize("docker { base = }").unwrap()).unwrap_err();
        assert!(err.message.contains("valeur attendue"));
        let deep = format!("var X = {}{}", "[".repeat(10_000), "]".repeat(10_000));
        let err = parse(&tokenize(&deep).unwrap()).unwrap_err();
        assert!(err.message.contains("imbriquées"));
    }
}
