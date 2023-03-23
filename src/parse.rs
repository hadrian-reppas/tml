use crate::error::Error;
use crate::lex::{Span, Token, TokenKind, Tokens};

#[derive(Clone, Debug)]
pub struct Name {
    pub name: &'static str,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct State {
    pub name: Name,
    pub state_params: Vec<Name>,
    pub symbol_params: Vec<Name>,
    pub arms: Vec<Arm>,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: Pattern,
    pub ops: Vec<Op>,
    pub to_state: ToState,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Symbol(Symbol),
    Name(Name),
}

#[derive(Clone, Debug)]
pub enum Op {
    Left(Span),
    Right(Span),
    Name(Name),
    Symbol(Symbol),
}

#[derive(Clone, Debug)]
pub enum ToState {
    State {
        name: Name,
        state_args: Vec<ToState>,
        symbol_args: Vec<Pattern>,
    },
    Halt {
        span: Span,
    },
}

pub fn parse(mut tokens: Tokens) -> Result<Vec<State>, Error> {
    let peek_one = tokens.next()?;
    let peek_two = tokens.next()?;
    let mut parser = Parser {
        tokens,
        peek_one,
        peek_two,
    };

    parser.unit()
}

struct Parser {
    tokens: Tokens,
    peek_one: Token,
    peek_two: Token,
}

impl Parser {
    fn peek(&self) -> &TokenKind {
        &self.peek_one.kind
    }

    fn peek_two(&self) -> [&TokenKind; 2] {
        [&self.peek_one.kind, &self.peek_two.kind]
    }

    fn next(&mut self) -> Result<Token, Error> {
        let peek_two = self.tokens.next()?;
        let peek_one = std::mem::replace(&mut self.peek_two, peek_two);
        let next = std::mem::replace(&mut self.peek_one, peek_one);
        Ok(next)
    }

    fn peek_span(&mut self) -> Span {
        self.peek_one.span
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, Error> {
        if self.peek() == &kind {
            self.next()
        } else {
            Err(Error::new(
                format!("expected {}, found {}", kind.desc(), self.peek().desc()),
                Some(self.peek_span()),
            ))
        }
    }

    fn name(&mut self) -> Result<Name, Error> {
        let token = self.expect(TokenKind::Name)?;
        Ok(Name {
            name: token.span.text,
            span: token.span,
        })
    }

    fn symbol(&mut self) -> Result<Symbol, Error> {
        if matches!(self.peek(), TokenKind::Symbol(_)) {
            match self.next()? {
                Token {
                    kind: TokenKind::Symbol(symbol),
                    span,
                } => Ok(Symbol { symbol, span }),
                _ => unreachable!(),
            }
        } else {
            Err(Error::new(
                format!("expected symbol, found {}", self.peek().desc()),
                Some(self.peek_span()),
            ))
        }
    }

    fn unit(&mut self) -> Result<Vec<State>, Error> {
        let mut unit = Vec::new();
        while self.peek() != &TokenKind::Eof {
            unit.push(self.state()?);
        }
        Ok(unit)
    }

    fn parens<T, U>(
        &mut self,
        parse_before: impl Fn(&mut Self) -> Result<T, Error>,
        parse_after: impl Fn(&mut Self) -> Result<U, Error>,
    ) -> Result<(Vec<T>, Vec<U>), Error> {
        if self.peek() != &TokenKind::LParen {
            return Ok((Vec::new(), Vec::new()));
        }
        self.expect(TokenKind::LParen)?;

        let mut before = Vec::new();
        let mut after = Vec::new();

        if self.peek() == &TokenKind::Semi {
            self.expect(TokenKind::Semi)?;
        } else if self.peek() != &TokenKind::RParen {
            before.push(parse_before(self)?);

            while self.peek() != &TokenKind::Semi
                && self.peek() != &TokenKind::RParen
                && self.peek_two() != [&TokenKind::Comma, &TokenKind::RParen]
            {
                self.expect(TokenKind::Comma)?;
                before.push(parse_before(self)?);
            }

            if self.peek() == &TokenKind::Semi {
                self.expect(TokenKind::Semi)?;
            } else if self.peek() == &TokenKind::Comma {
                self.expect(TokenKind::Comma)?;
            }
        }

        if self.peek() != &TokenKind::RParen {
            after.push(parse_after(self)?);

            while self.peek() != &TokenKind::RParen
                && self.peek_two() != [&TokenKind::Comma, &TokenKind::RParen]
            {
                self.expect(TokenKind::Comma)?;
                after.push(parse_after(self)?);
            }

            if self.peek() == &TokenKind::Comma {
                self.expect(TokenKind::Comma)?;
            }
        }

        self.expect(TokenKind::RParen)?;
        Ok((before, after))
    }

    fn state(&mut self) -> Result<State, Error> {
        let name = self.name()?;

        let (state_params, symbol_params) = self.parens(Parser::name, Parser::name)?;

        let mut arms = Vec::new();
        self.expect(TokenKind::LBrace)?;
        while self.peek() != &TokenKind::RBrace
            && self.peek_two() != [&TokenKind::Comma, &TokenKind::RBrace]
        {
            if !arms.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            arms.push(self.arm()?);
        }
        if self.peek() == &TokenKind::Comma {
            self.expect(TokenKind::Comma)?;
        }
        self.expect(TokenKind::RBrace)?;

        Ok(State {
            name,
            state_params,
            symbol_params,
            arms,
        })
    }

    fn arm(&mut self) -> Result<Arm, Error> {
        let pattern = self.pattern()?;

        self.expect(TokenKind::Bar)?;
        let mut ops = Vec::new();
        while self.peek() != &TokenKind::Bar {
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
        match self.peek() {
            TokenKind::Name => Ok(Pattern::Name(self.name()?)),
            TokenKind::Symbol(_) => Ok(Pattern::Symbol(self.symbol()?)),
            _ => Err(Error::new(
                format!("expected name or symbol, found {}", self.peek().desc()),
                Some(self.peek_span()),
            )),
        }
    }

    fn op(&mut self) -> Result<Op, Error> {
        match self.peek() {
            TokenKind::Left => Ok(Op::Left(self.expect(TokenKind::Left)?.span)),
            TokenKind::Right => Ok(Op::Right(self.expect(TokenKind::Right)?.span)),
            TokenKind::Name => Ok(Op::Name(self.name()?)),
            TokenKind::Symbol(_) => Ok(Op::Symbol(self.symbol()?)),
            _ => Err(Error::new(
                format!(
                    "expected `<`, `>`, name or symbol, found {}",
                    self.peek().desc()
                ),
                Some(self.peek_span()),
            )),
        }
    }

    fn to_state(&mut self) -> Result<ToState, Error> {
        match self.peek() {
            TokenKind::Name => {
                let name = self.name()?;
                let (state_args, symbol_args) = self.parens(Parser::to_state, Parser::pattern)?;
                Ok(ToState::State {
                    name,
                    state_args,
                    symbol_args,
                })
            }
            TokenKind::Bang => Ok(ToState::Halt {
                span: self.expect(TokenKind::Bang)?.span,
            }),
            _ => Err(Error::new(
                format!("expected name or `!`, found {}", self.peek().desc()),
                Some(self.peek_span()),
            )),
        }
    }
}
