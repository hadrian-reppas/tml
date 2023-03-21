use termion::{color, style};

use crate::lex::Span;

#[derive(Debug)]
pub struct Error {
    msg: String,
    span: Option<Span>,
}

impl Error {
    pub fn new(msg: String, span: Option<Span>) -> Self {
        Error { msg, span }
    }

    pub fn print(&self, no_color: bool) {
        if no_color {
            println!("error: {}", self.msg);
        } else {
            println!(
                "{}{}error:{}{} {}",
                style::Bold,
                color::Fg(color::Red),
                style::Reset,
                color::Fg(color::Reset),
                self.msg
            );
        }

        if let Some(span) = self.span {
            span.print(no_color);
        }
    }
}
