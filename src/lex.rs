use std::path::{Path, PathBuf};
use std::str::Lines;
use std::{cmp, fs};

use termion::{color, style};
use unicode_segmentation::UnicodeSegmentation;

use crate::error::Error;

#[derive(Clone, Copy)]
pub struct Span {
    pub text: &'static str,
    pub prefix: &'static str,
    pub suffix: &'static str,
    pub line: usize,
    pub column: usize,
    pub path: &'static Path,
}

impl Span {
    pub fn print(self, no_color: bool) {
        let prefix_len = self.prefix.graphemes(true).count();
        let text_len = cmp::max(1, self.text.graphemes(true).count());
        let line_str = format!("{}", self.line + 1);
        if no_color {
            println!(
                "{}--> {}:{}:{}",
                " ".repeat(line_str.len()),
                self.path.display(),
                self.line + 1,
                self.column + 1
            );
            println!("{} |", " ".repeat(line_str.len()));
            println!("{line_str} | {}{}{}", self.prefix, self.text, self.suffix);
            println!(
                "{} | {}{}",
                " ".repeat(line_str.len()),
                " ".repeat(prefix_len),
                "^".repeat(text_len)
            );
        } else {
            println!(
                "{}{}{}-->{}{} {}:{}:{}",
                " ".repeat(line_str.len()),
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset),
                self.path.display(),
                self.line + 1,
                self.column + 1
            );
            println!(
                "{} {}{}|{}{}",
                " ".repeat(line_str.len()),
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset)
            );
            println!(
                "{}{}{line_str} |{}{} {}{}{}",
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset),
                self.prefix,
                self.text,
                self.suffix
            );
            println!(
                "{} {}{}|{}{} {}{}{}{}{}{}",
                " ".repeat(line_str.len()),
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset),
                " ".repeat(prefix_len),
                style::Bold,
                color::Fg(color::Red),
                "^".repeat(text_len),
                style::Reset,
                color::Fg(color::Reset),
            );
        }
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Span({:?})", self.text)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Bar,
    Bang,
    Left,
    Right,
    Name,
    Symbol(String),
    Eof,
}

impl TokenKind {
    pub fn desc(&self) -> &'static str {
        match self {
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::Comma => "`,`",
            TokenKind::Semi => "`;`",
            TokenKind::Bar => "`|`",
            TokenKind::Bang => "`!`",
            TokenKind::Left => "`<`",
            TokenKind::Right => "`>`",
            TokenKind::Name => "name",
            TokenKind::Symbol(_) => "symbol",
            TokenKind::Eof => "end of file",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Tokens {
    suffix: &'static str,
    line: &'static str,
    lines: Lines<'static>,
    line_number: usize,
    column_number: usize,
    path: &'static Path,
    eof: Option<Span>,
    allow_tabs: bool,
}

impl Tokens {
    pub fn from_path_buf(path: PathBuf, allow_tabs: bool) -> Result<Self, Error> {
        let path: &'static Path = Box::leak(Box::new(path));

        let Ok(code) = fs::read_to_string(path) else {
            return Err(Error::new(format!("cannot read file {}", path.display()), None));
        };
        let code = Box::leak(Box::new(code));

        Tokens::new(code, path, allow_tabs)
    }

    pub fn new(code: &'static str, path: &'static Path, allow_tabs: bool) -> Result<Self, Error> {
        let mut lines = code.lines();
        let line = lines
            .next()
            .ok_or_else(|| Error::new(format!("file {} is empty", path.display()), None))?;
        Ok(Tokens {
            suffix: line,
            line,
            lines,
            line_number: 0,
            column_number: 0,
            path,
            eof: None,
            allow_tabs,
        })
    }

    fn make_span(&mut self, len: usize) -> Span {
        let offset = self.suffix.as_ptr() as usize - self.line.as_ptr() as usize;
        let prefix = &self.line[..offset];
        let text = &self.suffix[..len];
        self.suffix = &self.suffix[len..];
        let column = self.column_number;

        self.column_number += text.chars().count();

        Span {
            text,
            prefix,
            suffix: self.suffix,
            line: self.line_number,
            column,
            path: self.path,
        }
    }

    pub fn next(&mut self) -> Result<Token, Error> {
        if let Some(span) = self.eof {
            return Ok(Token {
                kind: TokenKind::Eof,
                span,
            });
        }

        macro_rules! token {
            ($kind:ident) => {
                Ok(Token {
                    kind: TokenKind::$kind,
                    span: self.make_span(1),
                })
            };
        }

        self.strip_whitespace()?;
        match self.suffix.chars().next() {
            None => self.eol(),
            Some('(') => token!(LParen),
            Some(')') => token!(RParen),
            Some('{') => token!(LBrace),
            Some('}') => token!(RBrace),
            Some(',') => token!(Comma),
            Some(';') => token!(Semi),
            Some('|') => token!(Bar),
            Some('!') => token!(Bang),
            Some('<') => token!(Left),
            Some('>') => token!(Right),
            Some('\'') => self.symbol(),
            Some('_') => self.name(),
            Some(c) if c.is_alphabetic() => self.name(),
            Some(c) => Err(Error::new(
                format!("unexpected character {c:?}"),
                Some(self.make_span(1)),
            )),
        }
    }

    fn eol(&mut self) -> Result<Token, Error> {
        if let Some(line) = self.lines.next() {
            self.suffix = line;
            self.line = line;
            self.line_number += 1;
            self.column_number = 0;
        } else {
            self.eof = Some(self.make_span(0));
        }
        self.next()
    }

    fn strip_whitespace(&mut self) -> Result<(), Error> {
        let mut len = 0;
        let mut chars = self.suffix.chars();
        while let Some(c) = chars.next() {
            if !c.is_whitespace() {
                break;
            } else if !self.allow_tabs && c == '\t' {
                self.make_span(len);
                return Err(Error::new(
                    "tab characters are not allowed".to_string(),
                    Some(self.make_span(1)),
                ));
            }
            len += c.len_utf8();
        }
        self.make_span(len);
        Ok(())
    }

    fn name(&mut self) -> Result<Token, Error> {
        let mut len = 0;
        let mut chars = self.suffix.chars();
        while let Some(c) = chars.next() {
            if c.is_alphanumeric() || c == '_' {
                len += c.len_utf8();
            } else {
                break;
            }
        }

        let span = self.make_span(len);
        Ok(Token {
            kind: TokenKind::Name,
            span,
        })
    }

    fn symbol(&mut self) -> Result<Token, Error> {
        let mut start = 1;
        let mut string = String::new();

        while let Some((c, len)) = self.char(start)? {
            string.push(c);
            start += len;
        }

        Ok(Token {
            kind: TokenKind::Symbol(string),
            span: self.make_span(start + 1),
        })
    }

    fn char(&mut self, start: usize) -> Result<Option<(char, usize)>, Error> {
        if self.suffix[start..].starts_with('\'') {
            Ok(None)
        } else if self.suffix[start..].starts_with("\\\\") {
            Ok(Some(('\\', 2)))
        } else if self.suffix[start..].starts_with("\\\'") {
            Ok(Some(('\'', 2)))
        } else if self.suffix[start..].starts_with('\\') {
            self.make_span(start);
            let len = 1 + (self.suffix.len() > start + 1) as usize;
            Err(Error::new(
                "invalid escape sequence (only '\\'' and '\\\\' are supported)".to_string(),
                Some(self.make_span(len)),
            ))
        } else if let Some(c) = self.suffix[start..].chars().next() {
            if c.escape_debug().count() == 1 {
                Ok(Some((c, c.len_utf8())))
            } else {
                self.make_span(start);
                Err(Error::new(
                    format!("weird character {c:?} is not allowed"),
                    Some(self.make_span(1)),
                ))
            }
        } else {
            Err(Error::new(
                "unterminated symbol".to_string(),
                Some(self.make_span(start)),
            ))
        }
    }
}
