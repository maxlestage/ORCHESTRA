//! Découpe un fichier `.drc` en jetons.
//!
//! Les retours à la ligne ne sont pas significatifs : les entrées d'un objet se
//! séparent par une virgule ou par un simple saut de ligne, ce que l'analyseur
//! reconnaît sans aide du lexeur.

use super::{Error, Pos};

/// Un jeton et sa position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: Tok,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Str(String),
    Int(i64),
    Float(f64),
    /// `=`
    Eq,
    /// `,`
    Comma,
    /// `;`
    Semi,
    /// `!`
    Bang,
    /// `+`
    Plus,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Eof,
}

impl Tok {
    /// Le jeton tel qu'il apparaît dans les messages d'erreur.
    pub fn describe(&self) -> String {
        match self {
            Tok::Ident(name) => format!("`{name}`"),
            Tok::Str(_) => "une chaîne".into(),
            Tok::Int(n) => format!("`{n}`"),
            Tok::Float(n) => format!("`{n}`"),
            Tok::Eq => "`=`".into(),
            Tok::Comma => "`,`".into(),
            Tok::Semi => "`;`".into(),
            Tok::Bang => "`!`".into(),
            Tok::Plus => "`+`".into(),
            Tok::LBrace => "`{`".into(),
            Tok::RBrace => "`}`".into(),
            Tok::LBracket => "`[`".into(),
            Tok::RBracket => "`]`".into(),
            Tok::LParen => "`(`".into(),
            Tok::RParen => "`)`".into(),
            Tok::Eof => "la fin du fichier".into(),
        }
    }
}

/// Tous les jetons du texte, terminés par [`Tok::Eof`].
pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer {
        chars: source.chars().collect(),
        at: 0,
        line: 1,
        col: 1,
    };
    let mut tokens = Vec::new();
    loop {
        lexer.skip_blank()?;
        let pos = lexer.pos();
        let Some(c) = lexer.peek() else {
            tokens.push(Token {
                kind: Tok::Eof,
                pos,
            });
            return Ok(tokens);
        };
        let kind = match c {
            '=' => lexer.single(Tok::Eq),
            ',' => lexer.single(Tok::Comma),
            ';' => lexer.single(Tok::Semi),
            '!' => lexer.single(Tok::Bang),
            '+' => lexer.single(Tok::Plus),
            '{' => lexer.single(Tok::LBrace),
            '}' => lexer.single(Tok::RBrace),
            '[' => lexer.single(Tok::LBracket),
            ']' => lexer.single(Tok::RBracket),
            '(' => lexer.single(Tok::LParen),
            ')' => lexer.single(Tok::RParen),
            '"' => lexer.string()?,
            c if c.is_ascii_digit() || (c == '-' && lexer.next_is_digit()) => lexer.number()?,
            c if c.is_alphabetic() || c == '_' => lexer.ident(),
            other => {
                return Err(Error::at(pos, format!("caractère inattendu `{other}`")));
            }
        };
        tokens.push(Token { kind, pos });
    }
}

struct Lexer {
    chars: Vec<char>,
    at: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    fn next_is_digit(&self) -> bool {
        self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn single(&mut self, tok: Tok) -> Tok {
        self.bump();
        tok
    }

    /// Espaces, sauts de ligne et commentaires (`# …`, `// …`, `/* … */`).
    fn skip_blank(&mut self) -> Result<(), Error> {
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some(c), _) if c.is_whitespace() => {
                    self.bump();
                }
                (Some('#'), _) | (Some('/'), Some('/')) => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                }
                (Some('/'), Some('*')) => {
                    let start = self.pos();
                    self.bump();
                    self.bump();
                    loop {
                        match (self.peek(), self.peek_at(1)) {
                            (Some('*'), Some('/')) => {
                                self.bump();
                                self.bump();
                                break;
                            }
                            (Some(_), _) => {
                                self.bump();
                            }
                            (None, _) => {
                                return Err(Error::at(start, "commentaire `/*` jamais fermé"));
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn string(&mut self) -> Result<Tok, Error> {
        let start = self.pos();
        self.bump();
        let mut value = String::new();
        loop {
            let pos = self.pos();
            match self.bump() {
                None => return Err(Error::at(start, "chaîne jamais fermée (il manque `\"`)")),
                Some('"') => return Ok(Tok::Str(value)),
                Some('\\') => match self.bump() {
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    Some('r') => value.push('\r'),
                    Some('0') => value.push('\0'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(other) => {
                        return Err(Error::at(pos, format!("échappement inconnu `\\{other}`")));
                    }
                    None => return Err(Error::at(start, "chaîne jamais fermée (il manque `\"`)")),
                },
                Some(c) => value.push(c),
            }
        }
    }

    fn number(&mut self) -> Result<Tok, Error> {
        let start = self.pos();
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push('-');
            self.bump();
        }
        let mut float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    text.push(c);
                }
                self.bump();
            } else if c == '.' && !float && self.next_is_digit() {
                float = true;
                text.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if float {
            text.parse()
                .map(Tok::Float)
                .map_err(|_| Error::at(start, format!("nombre invalide `{text}`")))
        } else {
            text.parse()
                .map(Tok::Int)
                .map_err(|_| Error::at(start, format!("nombre trop grand `{text}`")))
        }
    }

    fn ident(&mut self) -> Tok {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' && self.ident_continues() {
                name.push(c);
                self.bump();
            } else {
                break;
            }
        }
        Tok::Ident(name)
    }

    /// Un `-` fait partie d'un identifiant (`image-pull`) s'il est suivi d'une lettre.
    fn ident_continues(&self) -> bool {
        self.peek_at(1).is_some_and(|c| c.is_alphabetic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<Tok> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn comments_and_values() {
        assert_eq!(
            kinds("# en-tête\nvar X = \"a\\n\" /* bloc\n */ // fin\n[1, -2.5, true]"),
            vec![
                Tok::Ident("var".into()),
                Tok::Ident("X".into()),
                Tok::Eq,
                Tok::Str("a\n".into()),
                Tok::LBracket,
                Tok::Int(1),
                Tok::Comma,
                Tok::Float(-2.5),
                Tok::Comma,
                Tok::Ident("true".into()),
                Tok::RBracket,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn positions_and_errors() {
        let tokens = tokenize("a\n  b").unwrap();
        assert_eq!(tokens[1].pos, Pos { line: 2, col: 3 });
        let err = tokenize("x = \"ouvert").unwrap_err();
        assert_eq!((err.pos.line, err.pos.col), (1, 5));
        assert!(tokenize("x = @").is_err());
    }
}
