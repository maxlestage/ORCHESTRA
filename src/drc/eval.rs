//! Évalue un projet : imports, variables, fonctions, puis la valeur de chaque section.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::lexer::tokenize;
use super::parser::{Expr, ExprKind, FnDef, Item, Section, Stmt, parse};
use super::{Error, Pos, Resolver, Value};

/// Les fonctions intégrées, en plus de `format!`.
pub const BUILTINS: &[&str] = &["concat", "upper", "lower", "join", "replace", "str"];

/// Profondeur maximale d'appels de fonctions (contre la récursion infinie).
const MAX_DEPTH: usize = 64;

/// Quantité de données qu'un programme peut produire (un élément ou un octet = 1),
/// pour qu'un fichier de quelques lignes ne puisse pas épuiser la mémoire.
pub const BUDGET: usize = 4_000_000;

/// Le poids d'une valeur pour [`BUDGET`].
fn weight(value: &Value) -> usize {
    match value {
        Value::Str(s) => 1 + s.len(),
        Value::Int(_) | Value::Float(_) | Value::Bool(_) => 1,
        Value::List(items) => 1 + items.iter().map(weight).sum::<usize>(),
        Value::Object(entries) => {
            1 + entries
                .iter()
                .map(|(k, v)| k.len() + weight(v))
                .sum::<usize>()
        }
    }
}

/// Une section évaluée (les sections du même nom sont fusionnées).
#[derive(Debug, Clone, PartialEq)]
pub struct SectionValue {
    pub section: Section,
    pub value: Value,
    pub pos: Pos,
}

pub struct Evaluator<'r> {
    globals: HashMap<String, Value>,
    fns: HashMap<String, Rc<FnDef>>,
    pub sections: Vec<SectionValue>,
    /// Les fichiers en cours d'import, pour repérer les cycles.
    stack: Vec<String>,
    loaded: HashSet<String>,
    spent: Cell<usize>,
    resolver: &'r mut Resolver<'r>,
}

impl<'r> Evaluator<'r> {
    /// `resolver(depuis, chemin)` renvoie le nom canonique et le contenu d'un import.
    pub fn new(resolver: &'r mut Resolver<'r>) -> Self {
        Self {
            globals: HashMap::new(),
            fns: HashMap::new(),
            sections: Vec::new(),
            stack: Vec::new(),
            loaded: HashSet::new(),
            spent: Cell::new(0),
            resolver,
        }
    }

    /// Évalue un fichier (et, récursivement, ses imports).
    pub fn file(&mut self, name: &str, source: &str) -> Result<(), Error> {
        let in_file = |e: Error| e.in_file(name);
        let items = parse(&tokenize(source).map_err(in_file)?).map_err(in_file)?;
        self.stack.push(name.to_owned());
        self.loaded.insert(name.to_owned());
        // Les fonctions d'un fichier sont visibles dans tout le fichier.
        for item in &items {
            if let Item::Fn(def) = item {
                self.define_fn(def).map_err(in_file)?;
            }
        }
        for item in items {
            self.item(name, item).map_err(in_file)?;
        }
        self.stack.pop();
        Ok(())
    }

    fn define_fn(&mut self, def: &FnDef) -> Result<(), Error> {
        if BUILTINS.contains(&def.name.as_str()) || def.name == "format" {
            return Err(Error::at(
                def.pos,
                format!(
                    "`{}` est une fonction intégrée, choisissez un autre nom",
                    def.name
                ),
            ));
        }
        if self.fns.contains_key(&def.name) {
            return Err(Error::at(
                def.pos,
                format!("la fonction `{}` est déjà définie", def.name),
            ));
        }
        self.fns.insert(def.name.clone(), Rc::new(def.clone()));
        Ok(())
    }

    fn item(&mut self, file: &str, item: Item) -> Result<(), Error> {
        match item {
            Item::Fn(_) => Ok(()),
            Item::Import { path, pos } => {
                let (canonical, source) =
                    (self.resolver)(file, &path).map_err(|e| Error::at(pos, e))?;
                if self.stack.contains(&canonical) {
                    return Err(Error::at(
                        pos,
                        format!(
                            "import circulaire : {} → {canonical}",
                            self.stack.join(" → ")
                        ),
                    ));
                }
                if self.loaded.contains(&canonical) {
                    return Ok(());
                }
                self.file(&canonical, &source)
            }
            Item::Var { name, value, pos } => {
                if self.globals.contains_key(&name) {
                    return Err(Error::at(
                        pos,
                        format!("la variable `{name}` est déjà définie"),
                    ));
                }
                let value = self.eval(&value, &HashMap::new(), 0)?;
                self.globals.insert(name, value);
                Ok(())
            }
            Item::Section { section, body, pos } => {
                let value = self.eval(&body, &HashMap::new(), 0)?;
                match self.sections.iter_mut().find(|s| s.section == section) {
                    Some(existing) => merge(&mut existing.value, value),
                    None => self.sections.push(SectionValue {
                        section,
                        value,
                        pos,
                    }),
                }
                Ok(())
            }
        }
    }

    /// Compte une valeur produite ; erreur au-delà du budget.
    fn charge(&self, value: Value, pos: Pos) -> Result<Value, Error> {
        let spent = self.spent.get().saturating_add(weight(&value));
        self.spent.set(spent);
        if spent > BUDGET {
            return Err(Error::at(
                pos,
                "le programme produit trop de données (plus de 4 millions d'éléments)",
            ));
        }
        Ok(value)
    }

    fn eval(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Value>,
        depth: usize,
    ) -> Result<Value, Error> {
        Ok(match &expr.kind {
            ExprKind::Str(s) => Value::Str(s.clone()),
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Float(n) => Value::Float(*n),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::List(items) => Value::List(
                items
                    .iter()
                    .map(|item| self.eval(item, locals, depth))
                    .collect::<Result<_, _>>()?,
            ),
            ExprKind::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|entry| Ok((entry.key.clone(), self.eval(&entry.value, locals, depth)?)))
                    .collect::<Result<_, Error>>()?,
            ),
            ExprKind::Var(name) => match locals.get(name).or_else(|| self.globals.get(name)) {
                Some(value) => self.charge(value.clone(), expr.pos)?,
                None => {
                    let hint = if self.fns.contains_key(name) {
                        format!(" (`{name}` est une fonction : appelez-la avec `{name}(…)`)")
                    } else {
                        String::new()
                    };
                    return Err(Error::at(
                        expr.pos,
                        format!("variable inconnue `{name}`{hint}"),
                    ));
                }
            },
            ExprKind::Call {
                name,
                args,
                macro_call,
            } => {
                let args = args
                    .iter()
                    .map(|arg| self.eval(arg, locals, depth))
                    .collect::<Result<Vec<_>, _>>()?;
                let value = self.call(name, *macro_call, args, expr.pos, depth)?;
                self.charge(value, expr.pos)?
            }
        })
    }

    fn call(
        &self,
        name: &str,
        macro_call: bool,
        args: Vec<Value>,
        pos: Pos,
        depth: usize,
    ) -> Result<Value, Error> {
        if macro_call {
            return match name {
                "format" => format_macro(&args, pos),
                other => Err(Error::at(
                    pos,
                    format!("macro inconnue `{other}!` (seule `format!` existe)"),
                )),
            };
        }
        if let Some(value) = builtin(name, &args, pos)? {
            return Ok(value);
        }
        let Some(def) = self.fns.get(name) else {
            let hint = if name == "format" {
                " (écrivez `format!(…)`)"
            } else {
                ""
            };
            return Err(Error::at(pos, format!("fonction inconnue `{name}`{hint}")));
        };
        if args.len() != def.params.len() {
            return Err(Error::at(
                pos,
                format!(
                    "`{name}` attend {} argument(s), {} donné(s)",
                    def.params.len(),
                    args.len()
                ),
            ));
        }
        if depth >= MAX_DEPTH {
            return Err(Error::at(
                pos,
                format!("trop d'appels imbriqués en appelant `{name}` (récursion infinie ?)"),
            ));
        }
        let mut locals: HashMap<String, Value> = def.params.iter().cloned().zip(args).collect();
        for stmt in &def.body {
            match stmt {
                Stmt::Var { name, value } => {
                    let value = self.eval(value, &locals, depth + 1)?;
                    locals.insert(name.clone(), value);
                }
                Stmt::Return(value) => return self.eval(value, &locals, depth + 1),
            }
        }
        Err(Error::at(
            def.pos,
            format!("la fonction `{name}` se termine sans `return`"),
        ))
    }
}

/// Fusionne `other` dans `base` : objet par objet, la dernière valeur l'emporte.
pub fn merge(base: &mut Value, other: Value) {
    match (base, other) {
        (Value::Object(base), Value::Object(other)) => {
            for (key, value) in other {
                match base.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, existing)) => merge(existing, value),
                    None => base.push((key, value)),
                }
            }
        }
        (base, other) => *base = other,
    }
}

fn scalar(value: &Value, pos: Pos, context: &str) -> Result<String, Error> {
    match value {
        Value::Str(s) => Ok(s.clone()),
        Value::Int(n) => Ok(n.to_string()),
        Value::Float(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        other => Err(Error::at(
            pos,
            format!(
                "{context} n'accepte que des chaînes, nombres et booléens, pas {}",
                other.type_name()
            ),
        )),
    }
}

fn string_arg<'a>(args: &'a [Value], i: usize, name: &str, pos: Pos) -> Result<&'a str, Error> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s),
        Some(other) => Err(Error::at(
            pos,
            format!(
                "`{name}` : l'argument {} doit être une chaîne, pas {}",
                i + 1,
                other.type_name()
            ),
        )),
        None => Err(Error::at(
            pos,
            format!("`{name}` : il manque l'argument {}", i + 1),
        )),
    }
}

fn arity(args: &[Value], n: usize, name: &str, pos: Pos) -> Result<(), Error> {
    if args.len() == n {
        Ok(())
    } else {
        Err(Error::at(
            pos,
            format!("`{name}` attend {n} argument(s), {} donné(s)", args.len()),
        ))
    }
}

fn builtin(name: &str, args: &[Value], pos: Pos) -> Result<Option<Value>, Error> {
    let value = match name {
        "concat" => Value::Str(
            args.iter()
                .map(|a| scalar(a, pos, "`concat`"))
                .collect::<Result<String, _>>()?,
        ),
        "upper" => {
            arity(args, 1, name, pos)?;
            Value::Str(string_arg(args, 0, name, pos)?.to_uppercase())
        }
        "lower" => {
            arity(args, 1, name, pos)?;
            Value::Str(string_arg(args, 0, name, pos)?.to_lowercase())
        }
        "str" => {
            arity(args, 1, name, pos)?;
            Value::Str(scalar(&args[0], pos, "`str`")?)
        }
        "replace" => {
            arity(args, 3, name, pos)?;
            let s = string_arg(args, 0, name, pos)?;
            let from = string_arg(args, 1, name, pos)?;
            let to = string_arg(args, 2, name, pos)?;
            if from.is_empty() {
                return Err(Error::at(pos, "`replace` : le motif ne peut pas être vide"));
            }
            Value::Str(s.replace(from, to))
        }
        "join" => {
            arity(args, 2, name, pos)?;
            let Value::List(items) = &args[0] else {
                return Err(Error::at(
                    pos,
                    format!(
                        "`join` : l'argument 1 doit être une liste, pas {}",
                        args[0].type_name()
                    ),
                ));
            };
            let sep = string_arg(args, 1, name, pos)?;
            let parts = items
                .iter()
                .map(|item| scalar(item, pos, "`join`"))
                .collect::<Result<Vec<_>, _>>()?;
            Value::Str(parts.join(sep))
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

/// `format!("{}:{}", a, b)` : `{}` dans l'ordre, `{0}` par position, `{{` et `}}`.
fn format_macro(args: &[Value], pos: Pos) -> Result<Value, Error> {
    let Some(Value::Str(template)) = args.first() else {
        return Err(Error::at(
            pos,
            "`format!` attend une chaîne de format en premier argument",
        ));
    };
    let values = args[1..]
        .iter()
        .map(|v| scalar(v, pos, "`format!`"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = String::new();
    let mut next = 0;
    let mut used = vec![false; values.len()];
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let mut index = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(d) if d.is_ascii_digit() => index.push(d),
                        _ => {
                            return Err(Error::at(
                                pos,
                                "`format!` : seuls `{}` et `{0}`, `{1}`… sont permis",
                            ));
                        }
                    }
                }
                let i = if index.is_empty() {
                    next += 1;
                    next - 1
                } else {
                    index.parse().unwrap_or(usize::MAX)
                };
                let Some(value) = values.get(i) else {
                    return Err(Error::at(
                        pos,
                        format!(
                            "`format!` : il manque une valeur pour l'emplacement {} ({} donnée(s))",
                            i + 1,
                            values.len()
                        ),
                    ));
                };
                used[i] = true;
                out.push_str(value);
            }
            '}' => {
                return Err(Error::at(
                    pos,
                    "`format!` : `}` seul, écrivez `}}` pour une accolade",
                ));
            }
            c => out.push(c),
        }
    }
    if let Some(unused) = used.iter().position(|u| !u) {
        return Err(Error::at(
            pos,
            format!("`format!` : la valeur {} n'est jamais utilisée", unused + 1),
        ));
    }
    Ok(Value::Str(out))
}
