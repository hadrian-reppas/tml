use std::iter::Peekable;

use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use unicode_segmentation::UnicodeSegmentation;

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

// TODO: fix this
pub fn print_decimal(tape: &[&str], radix: u32, start: usize, stride: usize) {
    let tape: String = tape
        .iter()
        .copied()
        .skip(start)
        .step_by(stride)
        .map_while(|symbol| to_char_radix(symbol, radix))
        .collect();

    if tape.is_empty() {
        print!("0.0");
    } else {
        let int = BigInt::parse_bytes(tape.as_bytes(), radix).unwrap();
        let decimal = BigDecimal::new(int, 0).with_prec(1000);
        let power = BigInt::from(radix).pow(tape.len() as u32);
        let coeff = BigDecimal::new(power, 0).with_prec(1000);
        let result = format!("{}", decimal / coeff);
        if result
            .chars()
            .filter(|c| c.is_ascii_digit())
            .all(|c| c == '0')
        {
            print!("0.0");
        } else {
            print!("{result}");
        }
    }
}

fn to_char_radix(symbol: &str, radix: u32) -> Option<char> {
    if symbol.len() == 1 && u32::from_str_radix(symbol, radix).is_ok() {
        symbol.chars().next()
    } else {
        None
    }
}
