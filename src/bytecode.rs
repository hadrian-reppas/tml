use termion::{color, style};

pub const COMPARE_ARG: u8 = 0;
pub const COMPARE_VAL: u8 = 1;
pub const OTHER: u8 = 2;
pub const HALT: u8 = 3;

pub const LEFT: u8 = 4;
pub const RIGHT: u8 = 5;
pub const LEFT_N: u8 = 6;
pub const RIGHT_N: u8 = 7;
pub const WRITE_ARG: u8 = 8;
pub const WRITE_VAL: u8 = 9;
pub const WRITE_BOUND: u8 = 10;

pub const SYMBOL_ARG: u8 = 11;
pub const SYMBOL_VAL: u8 = 12;
pub const SYMBOL_BOUND: u8 = 13;
pub const TAKE_ARG: u8 = 14;
pub const CLONE_ARG: u8 = 15;
pub const FREE_ARG: u8 = 16;
pub const MAKE_STATE: u8 = 17;
pub const FINAL_STATE: u8 = 18;
pub const FINAL_ARG: u8 = 19;

pub const HALT_ADDRESS: u32 = 6;

pub fn dump(bytes: &mut dyn Iterator<Item = u8>, no_color: bool) {
    let mut dumper = Dumper {
        bytes,
        no_color,
        address: 0,
    };

    dumper.dump();
}

struct Dumper<'a> {
    bytes: &'a mut dyn Iterator<Item = u8>,
    address: u32,
    no_color: bool,
}

macro_rules! text {
    ($self:expr, $text:expr, $color:ident) => {
        if $self.no_color {
            print!("{}", $text);
        } else {
            print!(
                "{}{}{}{}{}",
                style::Bold,
                color::Fg(color::$color),
                $text,
                style::Reset,
                color::Fg(color::Reset)
            );
        }
    };
}

macro_rules! textln {
    ($self: expr, $text:expr, $color:ident) => {{
        text!($self, $text, $color);
        println!();
    }};
}

impl Dumper<'_> {
    fn dump(&mut self) {
        let count = self.next_u16();

        if self.no_color {
            println!("\nnumber of states: {count}");
        } else {
            println!(
                "\n{}{}number of states:{}{} {count}",
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset)
            );
        }

        if self.no_color {
            println!("start address: {:#010x}\n", self.next_u32());
        } else {
            println!(
                "{}{}start address:{}{} {:#010x}\n",
                style::Bold,
                color::Fg(color::Blue),
                style::Reset,
                color::Fg(color::Reset),
                self.next_u32()
            );
        }

        assert_eq!(self.next_u8(), HALT, "invalid bytecode");

        for i in 0..count {
            if self.no_color {
                println!(
                    "========== state {i: <5} ({:#010x}) ==========",
                    self.address
                );
            } else {
                println!(
                    "{}{}========== state {i: <5} ({:#010x}) =========={}{}",
                    style::Bold,
                    color::Fg(color::Green),
                    self.address,
                    style::Reset,
                    color::Fg(color::Reset)
                );
            }
            self.state();
        }

        assert!(self.bytes.next().is_none(), "invalid bytecode");
    }

    fn state(&mut self) {
        let mut i = 0;
        while self.arm(i) {
            i += 1;
        }
        println!();
    }

    fn arm(&mut self, i: u32) -> bool {
        if self.no_color {
            println!("arm {i}:");
        } else {
            println!(
                "{}{}arm {i}:{}{}",
                style::Bold,
                color::Fg(color::Red),
                style::Reset,
                color::Fg(color::Reset)
            );
        }

        let is_last_arm = self.pattern();
        let mut seen_state = false;

        textln!(self, "instructions:", Blue);

        macro_rules! state_instr {
            () => {
                #[allow(unused_assignments)]
                {
                    if !seen_state {
                        seen_state = true;
                        if self.no_color {
                            println!("--");
                        } else {
                            println!(
                                "{}{}--{}{}",
                                style::Bold,
                                color::Fg(color::Blue),
                                style::Reset,
                                color::Fg(color::Reset)
                            );
                        }
                    }
                }
            };
        }

        loop {
            match self.next_u8() {
                LEFT => textln!(self, "    LEFT", Green),
                RIGHT => textln!(self, "    RIGHT", Green),
                LEFT_N => {
                    text!(self, "    LEFT_N", Green);
                    println!(" ({})", self.next_u8());
                }
                RIGHT_N => {
                    text!(self, "    RIGHT_N", Green);
                    println!(" (n: {})", self.next_u8());
                }
                WRITE_ARG => {
                    text!(self, "    WRITE_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                }
                WRITE_VAL => {
                    text!(self, "    WRITE_VAL", Green);
                    println!(" (value: {})", self.next_u16());
                }
                WRITE_BOUND => textln!(self, "    WRITE_BOUND", Green),

                SYMBOL_ARG => {
                    state_instr!();
                    text!(self, "    SYMBOL_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                }
                SYMBOL_VAL => {
                    state_instr!();
                    text!(self, "    SYMBOL_VAL", Green);
                    println!(" (value: {})", self.next_u16());
                }
                SYMBOL_BOUND => {
                    state_instr!();
                    textln!(self, "    SYMBOL_BOUND", Green);
                }
                TAKE_ARG => {
                    state_instr!();
                    text!(self, "    TAKE_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                }
                CLONE_ARG => {
                    state_instr!();
                    text!(self, "    CLONE_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                }
                FREE_ARG => {
                    state_instr!();
                    text!(self, "    FREE_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                }
                MAKE_STATE => {
                    state_instr!();
                    text!(self, "    MAKE_STATE", Green);
                    println!(
                        " (args: {}) (addr: {:#010x})",
                        self.next_u8(),
                        self.next_u32()
                    );
                }
                FINAL_STATE => {
                    state_instr!();
                    text!(self, "    FINAL_STATE", Green);
                    println!(" (addr: {:#010x})", self.next_u32());
                    return !is_last_arm;
                }
                FINAL_ARG => {
                    state_instr!();
                    text!(self, "    FINAL_ARG", Green);
                    println!(" (arg: {})", self.next_u8());
                    return !is_last_arm;
                }

                _ => panic!("invalid bytecode"),
            }
        }
    }

    fn pattern(&mut self) -> bool {
        match self.next_u8() {
            COMPARE_ARG => {
                text!(self, "    COMPARE_ARG", Green);
                println!(" (arg: {}) (skip: {})", self.next_u8(), self.next_u16());
                false
            }
            COMPARE_VAL => {
                text!(self, "    COMPARE_VAL", Green);
                println!(" (value: {}) (skip: {})", self.next_u16(), self.next_u16());
                false
            }
            OTHER => {
                textln!(self, "    OTHER", Green);
                true
            }
            _ => panic!("invalid bytecode"),
        }
    }

    fn next_u8(&mut self) -> u8 {
        self.address += 1;
        self.bytes.next().expect("invalid bytecode")
    }

    fn next_u16(&mut self) -> u16 {
        u16::from_le_bytes([self.next_u8(), self.next_u8()])
    }

    fn next_u32(&mut self) -> u32 {
        u32::from_le_bytes([
            self.next_u8(),
            self.next_u8(),
            self.next_u8(),
            self.next_u8(),
        ])
    }
}
