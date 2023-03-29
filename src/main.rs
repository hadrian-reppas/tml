use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod bytecode;
mod compile;
mod error;
mod ffi;
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

    /// Use Rust VM
    #[arg(long = "rust-vm")]
    rust_vm: bool,
}

fn main() -> ExitCode {
    let Arguments {
        file,
        tape,
        max_moves,
        no_color,
        allow_tabs,
        dump_bytecode,
        rust_vm,
        ..
    } = Arguments::parse();

    match do_it(
        file,
        tape,
        max_moves.unwrap_or(usize::MAX),
        no_color,
        allow_tabs,
        dump_bytecode,
        rust_vm,
    ) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            error.print(no_color);
            ExitCode::FAILURE
        }
    }
}

fn do_it(
    path: PathBuf,
    tape: Option<PathBuf>,
    max_moves: usize,
    no_color: bool,
    allow_tabs: bool,
    dump_bytecode: bool,
    rust_vm: bool,
) -> Result<(), error::Error> {
    let tokens = lex::Tokens::from_path_buf(path, allow_tabs)?;
    let unit = parse::parse(tokens)?;

    let compiled = if let Some(path) = tape {
        let tokens = lex::Tokens::from_path_buf(path, allow_tabs)?;
        let symbols = parse::parse_tape(tokens)?;
        compile::compile(unit, symbols)?
    } else {
        compile::compile(unit, Vec::new())?
    };

    if dump_bytecode {
        bytecode::dump(&mut compiled.bytes.iter().copied(), no_color);
    }

    let simulated = if rust_vm {
        vm::simulate(&compiled.bytes, compiled.tape, max_moves)
    } else {
        ffi::simulate(&compiled.bytes, &compiled.tape, max_moves)
    };

    let mut symbol_tape = Vec::new();
    for i in simulated.tape {
        symbol_tape.push(&compiled.symbols[i as usize]);
    }
    println!("{:?}", symbol_tape);
    println!("{}", simulated.moves);
    Ok(())
}
