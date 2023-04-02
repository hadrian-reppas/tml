use bigdecimal::{BigDecimal, ToPrimitive};
use num_bigint::BigInt;

pub fn dump(tape: &[&str], terminal_width: usize) {
    println!("terminal size: {:?}", termion::terminal_size());
    println!("tape: {tape:?}\n");
}

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
        if result.chars().filter(|c| c.is_ascii_digit()).all(|c| c == '0') {
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
