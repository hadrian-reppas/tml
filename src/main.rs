use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod bytecode;
mod compile;
mod error;
mod lex;
mod parse;
mod vm;

#[derive(Parser, Debug)]
struct Arguments {
    /// File containing the Turing machine
    file: PathBuf,
    /// File containing the initial tape
    tape: Option<PathBuf>,

    /// Maximum number of moves
    #[arg(short = 'm', long = "max-moves")]
    max_moves: Option<usize>,

    /// Don't print the final tape
    #[arg(long = "hide-tape")]
    hide_tape: bool,

    /// Don't print the final state
    #[arg(long = "hide-state")]
    hide_state: bool,

    /// Don't print the decimal interpretation of the final tape
    #[arg(long = "hide-decimal")]
    hide_decimal: bool,

    /// Number of printed digits in the final decimal
    #[arg(short = 'd', long = "decimal-digits")]
    decimal_digits: Option<usize>,

    /// Radix for the final decimal
    #[arg(short = 'r', long = "decimal-radix", default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=36))]
    decimal_radix: u32,

    /// Start position for the final decimal
    #[arg(short = 's', long = "decimal-start", default_value_t = 2)]
    decimal_start: usize,

    /// Stride for the final decimal
    #[arg(short = 'S', long = "decimal-stride", default_value_t = 2)]
    decimal_stride: usize,

    /// Length (in squares) of the final decimal
    #[arg(short = 'l', long = "decimal-length")]
    decimal_length: Option<usize>,

    /// Don't color output
    #[arg(long = "no-color")]
    no_color: bool,

    /// Allow tab characters in machine and tape files
    #[arg(long = "allow-tabs")]
    allow_tabs: bool,

    /// Dump bytecode
    #[arg(short = 'b', long = "dump-bytecode")]
    dump_bytecode: bool,
}

fn main() -> ExitCode {
    let Arguments {
        file,
        max_moves,
        no_color,
        allow_tabs,
        dump_bytecode,
        ..
    } = Arguments::parse();

    match do_it(file, max_moves, no_color, allow_tabs, dump_bytecode) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            error.print(no_color);
            ExitCode::FAILURE
        }
    }
}

fn do_it(
    path: PathBuf,
    max_moves: Option<usize>,
    no_color: bool,
    allow_tabs: bool,
    dump_bytecode: bool,
) -> Result<(), error::Error> {
    let tokens = lex::Tokens::from_path_buf(path, allow_tabs)?;
    let unit = parse::parse(tokens)?;
    let compiled = compile::compile(unit)?;
    if dump_bytecode {
        bytecode::dump(&mut compiled.bytes.iter().copied(), no_color);
    }
    let simulated = vm::simulate(&compiled.bytes, Vec::new(), max_moves.unwrap_or(usize::MAX));
    println!("{:?}", simulated.tape);
    println!("{}", simulated.moves);
    Ok(())
}
