use std::cmp;
use std::iter::Peekable;

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use unicode_segmentation::UnicodeSegmentation;

use crate::decimal::Decimal;
use crate::int::Int;

pub fn dump(tape: &[&str], terminal_width: usize) {
    if tape.is_empty() {
        println!("┌──┬──┬");
        println!("│  │  │");
        println!("└──┴──┴");
    }

    let mut symbols = tape.iter().copied().peekable();
    while symbols.peek().is_some() {
        let line = next_line(&mut symbols, terminal_width);
        print_line(&line);
    }
    println!();
}

fn next_line<'a>(
    symbols: &mut Peekable<impl Iterator<Item = &'a str>>,
    width: usize,
) -> Vec<&'a str> {
    let mut len = 1;
    let mut line = Vec::new();
    while line.is_empty()
        || symbols
            .peek()
            .map_or(false, |s| len + s.graphemes(true).count() + 3 <= width)
    {
        let symbol = symbols.next().unwrap();
        line.push(symbol);
        len += symbol.graphemes(true).count() + 3;
    }
    line
}

fn print_line(symbols: &[&str]) {
    print!("┬");
    for symbol in symbols {
        print!("{}┬", "─".repeat(2 + symbol.graphemes(true).count()));
    }
    println!();

    print!("│");
    for symbol in symbols {
        print!(" {symbol} │");
    }
    println!();

    print!("┴");
    for symbol in symbols {
        print!("{}┴", "─".repeat(2 + symbol.graphemes(true).count()));
    }
    println!();
}

pub fn parse_decimal(
    tape: &[&str],
    radix: usize,
    digits: Option<usize>,
    start: usize,
    stride: usize,
) -> Decimal {
    let symbols: Vec<_> = tape
        .iter()
        .copied()
        .skip(start)
        .step_by(stride)
        .map_while(|symbol| to_char_radix(symbol, radix))
        .collect();

    let digits = if let Some(digits) = digits {
        digits as usize
    } else {
        let len = symbols.len() as f64;
        let radix = radix as f64;
        cmp::max(3, (len * radix.log(10.0)).ceil() as usize)
    };

    if tape.is_empty() {
        Decimal::zero(digits)
    } else {
        let mut decimal = Decimal::zero(2 * digits);
        let mut power = Int::from(radix);
        let radix = Int::from(radix);

        for symbol in symbols {
            let digit = Int::from_hex_digit(symbol);
            let coeff = power.inverse(2 * digits);
            let term = &coeff * &digit;
            decimal = &decimal + &term;
            power = &radix * &power;
        }

        decimal.trim(digits)
    }
}

fn to_char_radix(symbol: &str, radix: usize) -> Option<char> {
    if symbol.len() == 1 && u32::from_str_radix(symbol, radix as u32).is_ok() {
        symbol.chars().next()
    } else {
        None
    }
}
