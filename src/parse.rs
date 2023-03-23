use std::collections::VecDeque;

use crate::error::Error;
use crate::lex::{Span, Token, TokenKind, Tokens};

#[derive(Clone, Debug)]
pub struct Upper {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Lower {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct State {
    pub name: Lower,
    pub params: Vec<Param>,
    pub arms: Vec<Arm>,
}

#[derive(Clone, Debug)]
pub enum Param {
    Upper(Upper),
    Lower(Lower),
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: Pattern,
    pub ops: Vec<Op>,
    pub to_state: ToState,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Symbol(String),
    Lower(Lower),
    Under,
}

#[derive(Clone, Debug)]
pub struct Op {
    pub kind: OpKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum OpKind {
    Left,
    Right,
    Lower(Lower),
    Symbol(String),
}

#[derive(Clone, Debug)]
pub struct ToState {
    pub name: Lower,
    pub args: Vec<Arg>,
}

#[derive(Clone, Debug)]
pub enum Arg {
    Upper(Upper),
    ToState(ToState),
    Symbol(String),
}

pub fn parse(tokens: Tokens) -> Result<Vec<State>, Error> {
    let mut parser = Parser {
        tokens,
        peek: VecDeque::new(),
    };
    parser.unit()
}

struct Parser {
    tokens: Tokens,
    peek: VecDeque<Token>,
}

impl Parser {
    fn peek(&mut self) -> Result<&TokenKind, Error> {
        if self.peek.is_empty() {
            self.peek.push_back(self.tokens.next()?);
        }
        Ok(&self.peek[0].kind)
    }

    fn peek_two(&mut self) -> Result<[&TokenKind; 2], Error> {
        while self.peek.len() < 2 {
            self.peek.push_back(self.tokens.next()?);
        }
        Ok([&self.peek[0].kind, &self.peek[1].kind])
    }

    fn next(&mut self) -> Result<Token, Error> {
        if let Some(token) = self.peek.pop_front() {
            Ok(token)
        } else {
            self.tokens.next()
        }
    }

    fn peek_span(&mut self) -> Result<Span, Error> {
        if self.peek.is_empty() {
            self.peek.push_back(self.tokens.next()?);
        }
        Ok(self.peek[0].span)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, Error> {
        if self.peek()? == &kind {
            self.next()
        } else {
            Err(Error::new(
                format!("expected {}, found {}", kind.desc(), self.peek()?.desc()),
                Some(self.peek_span()?),
            ))
        }
    }

    fn lower(&mut self) -> Result<Lower, Error> {
        let token = self.expect(TokenKind::Lower)?;
        Ok(Lower {
            name: token.span.text,
            span: token.span,
        })
    }

    fn upper(&mut self) -> Result<Upper, Error> {
        let token = self.expect(TokenKind::Upper)?;
        Ok(Upper {
            name: token.span.text,
            span: token.span,
        })
    }

    fn unit(&mut self) -> Result<Vec<State>, Error> {
        let mut unit = Vec::new();
        while self.peek()? != &TokenKind::Eof {
            unit.push(self.state()?);
        }
        Ok(unit)
    }

    fn state(&mut self) -> Result<State, Error> {
        let name = self.lower()?;

        let mut params = Vec::new();
        if self.peek_two()? == [&TokenKind::LParen, &TokenKind::RParen] {
            self.expect(TokenKind::LParen)?;
            self.expect(TokenKind::RParen)?;
        } else if self.peek()? == &TokenKind::LParen {
            self.expect(TokenKind::LParen)?;
            params.push(self.param()?);
            while self.peek()? != &TokenKind::RParen
                && self.peek_two()? != [&TokenKind::Comma, &TokenKind::RParen]
            {
                self.expect(TokenKind::Comma)?;
                params.push(self.param()?);
            }
            if self.peek()? == &TokenKind::Comma {
                self.expect(TokenKind::Comma)?;
            }
            self.expect(TokenKind::RParen)?;
        }

        let mut arms = Vec::new();
        self.expect(TokenKind::LBrace)?;
        while self.peek()? != &TokenKind::RBrace
            && self.peek_two()? != [&TokenKind::Comma, &TokenKind::RBrace]
        {
            if !arms.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            arms.push(self.arm()?);
        }
        if self.peek()? == &TokenKind::Comma {
            self.expect(TokenKind::Comma)?;
        }
        self.expect(TokenKind::RBrace)?;

        Ok(State { name, params, arms })
    }

    fn param(&mut self) -> Result<Param, Error> {
        if self.peek()? == &TokenKind::Lower {
            Ok(Param::Lower(self.lower()?))
        } else if self.peek()? == &TokenKind::Upper {
            Ok(Param::Upper(self.upper()?))
        } else {
            Err(Error::new(
                format!(
                    "expected uppercase or lowercase name, found {}",
                    self.peek()?.desc()
                ),
                Some(self.peek_span()?),
            ))
        }
    }

    fn arm(&mut self) -> Result<Arm, Error> {
        let pattern = self.pattern()?;

        self.expect(TokenKind::Bar)?;
        let mut ops = Vec::new();
        while self.peek()? != &TokenKind::Bar {
            ops.push(self.op()?);
        }

        self.expect(TokenKind::Bar)?;
        let to_state = self.to_state()?;

        Ok(Arm {
            pattern,
            ops,
            to_state,
        })
    }

    fn pattern(&mut self) -> Result<Pattern, Error> {
        if self.peek()? == &TokenKind::Under {
            self.expect(TokenKind::Under)?;
            Ok(Pattern::Under)
        } else if self.peek()? == &TokenKind::Lower {
            Ok(Pattern::Lower(self.lower()?))
        } else if matches!(self.peek()?, TokenKind::Symbol(_)) {
            match self.next()? {
                Token {
                    kind: TokenKind::Symbol(symbol),
                    ..
                } => Ok(Pattern::Symbol(symbol)),
                _ => unreachable!(),
            }
        } else {
            Err(Error::new(
                format!(
                    "expected `_`, symbol or lowercase name, found {}",
                    self.peek()?.desc()
                ),
                Some(self.peek_span()?),
            ))
        }
    }

    fn op(&mut self) -> Result<Op, Error> {
        if self.peek()? == &TokenKind::Left {
            let span = self.expect(TokenKind::Left)?.span;
            Ok(Op {
                kind: OpKind::Left,
                span,
            })
        } else if self.peek()? == &TokenKind::Right {
            let span = self.expect(TokenKind::Right)?.span;
            Ok(Op {
                kind: OpKind::Right,
                span,
            })
        } else if self.peek()? == &TokenKind::Lower {
            let lower = self.lower()?;
            let span = lower.span;
            Ok(Op {
                kind: OpKind::Lower(lower),
                span,
            })
        } else if matches!(self.peek()?, TokenKind::Symbol(_)) {
            match self.next()? {
                Token {
                    kind: TokenKind::Symbol(symbol),
                    span,
                } => Ok(Op {
                    kind: OpKind::Symbol(symbol),
                    span,
                }),
                _ => unreachable!(),
            }
        } else {
            Err(Error::new(
                format!(
                    "expected `<`, `>`, symbol or lowercase name, found {}",
                    self.peek()?.desc()
                ),
                Some(self.peek_span()?),
            ))
        }
    }

    fn to_state(&mut self) -> Result<ToState, Error> {
        let name = self.lower()?;

        let mut args = Vec::new();
        if self.peek_two()? == [&TokenKind::LParen, &TokenKind::RParen] {
            self.expect(TokenKind::LParen)?;
            self.expect(TokenKind::RParen)?;
        } else if self.peek()? == &TokenKind::LParen {
            self.expect(TokenKind::LParen)?;
            args.push(self.arg()?);
            while self.peek()? != &TokenKind::RParen
                && self.peek_two()? != [&TokenKind::Comma, &TokenKind::RParen]
            {
                self.expect(TokenKind::Comma)?;
                args.push(self.arg()?);
            }
            if self.peek()? == &TokenKind::Comma {
                self.expect(TokenKind::Comma)?;
            }
            self.expect(TokenKind::RParen)?;
        }

        Ok(ToState { name, args })
    }

    fn arg(&mut self) -> Result<Arg, Error> {
        if self.peek()? == &TokenKind::Lower {
            Ok(Arg::ToState(self.to_state()?))
        } else if self.peek()? == &TokenKind::Upper {
            Ok(Arg::Upper(self.upper()?))
        } else if matches!(self.peek()?, TokenKind::Symbol(_)) {
            match self.next()? {
                Token {
                    kind: TokenKind::Symbol(symbol),
                    ..
                } => Ok(Arg::Symbol(symbol)),
                _ => unreachable!(),
            }
        } else {
            Err(Error::new(
                format!(
                    "expected symbol, lowercase name or uppercase name, found {}",
                    self.peek()?.desc()
                ),
                Some(self.peek_span()?),
            ))
        }
    }
}
