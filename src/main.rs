use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use termion::{color, style};

mod bytecode;
mod compile;
mod error;
mod ffi;
mod lex;
mod parse;
mod tape;
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

    /// Don't print the decimal interpretation of the final tape
    #[arg(long = "hide-decimal")]
    hide_decimal: bool,

    /// Radix for the final decimal
    #[arg(short = 'r', long = "decimal-radix", default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=36))]
    decimal_radix: u32,

    /// Start position for the final decimal
    #[arg(short = 's', long = "decimal-start", default_value_t = 2)]
    decimal_start: u32,

    /// Stride for the final decimal
    #[arg(short = 'S', long = "decimal-stride", default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..))]
    decimal_stride: u32,

    /// Don't color output
    #[arg(long = "no-color")]
    no_color: bool,

    /// Allow tab characters in machine and tape files
    #[arg(long = "allow-tabs")]
    allow_tabs: bool,

    /// Dump bytecode
    #[arg(short = 'd', long = "dump-bytecode")]
    dump_bytecode: bool,

    /// Use Rust VM
    #[arg(long = "rust-vm")]
    rust_vm: bool,

    /// Time execution
    #[arg(short = 't', long = "time")]
    time: bool,

    /// Maximum width when printing the final tape
    #[arg(short = 'w', long = "terminal_width", value_parser = clap::value_parser!(u16).range(5..))]
    terminal_width: Option<u16>,
}

fn main() -> ExitCode {
    let args = Arguments::parse();
    let no_color = args.no_color;
    match do_it(args) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            error.print(no_color);
            ExitCode::FAILURE
        }
    }
}

fn do_it(args: Arguments) -> Result<(), error::Error> {
    let start = Instant::now();

    let tokens = lex::Tokens::from_path_buf(args.file, args.allow_tabs)?;
    let unit = parse::parse(tokens)?;

    let compiled = if let Some(path) = args.tape {
        let tokens = lex::Tokens::from_path_buf(path, args.allow_tabs)?;
        let symbols = parse::parse_tape(tokens)?;
        compile::compile(unit, symbols)?
    } else {
        compile::compile(unit, Vec::new())?
    };

    let compile_time = start.elapsed();

    if args.dump_bytecode {
        bytecode::dump(&mut compiled.bytes.iter().copied(), args.no_color);
    }

    let start = Instant::now();

    let max_moves = args.max_moves.unwrap_or(usize::MAX);
    let simulated = if args.rust_vm {
        vm::simulate(&compiled.bytes, compiled.tape, max_moves)
    } else {
        ffi::simulate(&compiled.bytes, &compiled.tape, max_moves)
    };

    let exec_time = start.elapsed();

    if args.time && args.no_color {
        println!("compile time: {compile_time:?}");
        println!("execution time: {exec_time:?}\n");
    } else if args.time {
        println!(
            "{}{}compile time:{}{} {compile_time:?}",
            style::Bold,
            color::Fg(color::Green),
            style::Reset,
            color::Fg(color::Reset)
        );
        println!(
            "{}{}execution time:{}{} {exec_time:?}\n",
            style::Bold,
            color::Fg(color::Green),
            style::Reset,
            color::Fg(color::Reset)
        );
    }

    let tape: Vec<_> = simulated
        .tape
        .iter()
        .map(|&i| compiled.symbols[i as usize].as_str())
        .collect();

    if !args.hide_tape {
        let terminal_width = if let Some(width) = args.terminal_width {
            width as usize
        } else if let Ok((width, _)) = termion::terminal_size() {
            width as usize
        } else {
            80
        };

        if args.no_color {
            println!("final tape:");
        } else {
            println!(
                "{}{}final tape:{}{}",
                style::Bold,
                color::Fg(color::Green),
                style::Reset,
                color::Fg(color::Reset)
            );
        }

        tape::dump(&tape, terminal_width);
    }

    if !args.hide_decimal {
        if args.no_color {
            print!("decimal: ");
        } else {
            print!(
                "{}{}decimal:{}{} ",
                style::Bold,
                color::Fg(color::Green),
                style::Reset,
                color::Fg(color::Reset)
            );
        }
        tape::print_decimal(
            &tape,
            args.decimal_radix,
            args.decimal_start as usize,
            args.decimal_stride as usize,
        );
        println!("\n");
    }

    println!("moves: {}", simulated.moves);
    println!("tape head: {}", simulated.head_position);
    println!("final address: {}", simulated.final_address);
    Ok(())
}
