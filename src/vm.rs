use std::ops::ControlFlow;

use crate::bytecode as bc;

const EXTRA_RESIZE_ROOM: usize = 256;

pub struct Simulated {
    pub tape: Vec<u16>,
    pub head_position: usize,
    pub final_address: u32,
    pub moves: usize,
}

#[derive(Debug, Clone)]
struct State {
    address: u32,
    states: Vec<State>,
    symbols: Vec<u16>,
}

pub fn simulate(bytes: &[u8], tape: Vec<u16>, max_moves: usize) -> Simulated {
    let mut bytes = Bytes { bytes, ip: 2 };
    bytes.goto();
    let address = bytes.ip as u32;

    let mut vm = Vm {
        bytes,
        tape: Tape { tape, head: 0 },
        state: State {
            address,
            states: Vec::new(),
            symbols: Vec::new(),
        },
        state_stack: Vec::new(),
        symbol_stack: Vec::new(),
        bound: 0,
        moves: 0,
        max_moves,
    };

    vm.run();

    let mut tape = vm.tape.tape;
    while let Some(0) = tape.last() {
        tape.pop();
    }

    Simulated {
        tape,
        head_position: vm.tape.head,
        final_address: vm.state.address,
        moves: vm.moves,
    }
}

struct Vm<'a> {
    bytes: Bytes<'a>,
    tape: Tape,
    state: State,
    state_stack: Vec<State>,
    symbol_stack: Vec<u16>,
    bound: u16,
    moves: usize,
    max_moves: usize,
}

impl Vm<'_> {
    fn run(&mut self) -> ControlFlow<()> {
        loop {
            if self.moves == self.max_moves {
                return ControlFlow::Break(());
            } else {
                self.run_move()?;
                self.moves += 1;
            }
        }
    }

    fn run_move(&mut self) -> ControlFlow<()> {
        loop {
            match self.bytes.next() {
                bc::COMPARE_ARG => {
                    let arg_index = self.bytes.next();
                    if self.tape.read() == self.state.symbols[arg_index as usize] {
                        self.bytes.next_u16();
                        self.rhs()?;
                        return ControlFlow::Continue(());
                    } else {
                        self.bytes.skip();
                    }
                }
                bc::COMPARE_VAL => {
                    let value = self.bytes.next_u16();
                    if self.tape.read() == value {
                        self.bytes.next_u16();
                        self.rhs()?;
                        return ControlFlow::Continue(());
                    } else {
                        self.bytes.skip();
                    }
                }
                bc::OTHER => {
                    self.bound = self.tape.read();
                    self.rhs()?;
                    return ControlFlow::Continue(());
                }
                bc::HALT => return ControlFlow::Break(()),
                _ => panic!("invalid bytecode"),
            }
        }
    }

    fn rhs(&mut self) -> ControlFlow<()> {
        loop {
            match self.bytes.next() {
                bc::LEFT => self.tape.left(1)?,
                bc::RIGHT => self.tape.right(1),
                bc::LEFT_N => self.tape.left(self.bytes.next())?,
                bc::RIGHT_N => self.tape.right(self.bytes.next()),
                bc::WRITE_ARG => {
                    let arg_index = self.bytes.next() as usize;
                    self.tape.write(self.state.symbols[arg_index]);
                }
                bc::WRITE_VAL => {
                    let value = self.bytes.next_u16();
                    self.tape.write(value);
                }
                bc::WRITE_BOUND => self.tape.write(self.bound),
                bc::SYMBOL_ARG => {
                    let arg_index = self.bytes.next() as usize;
                    self.symbol_stack.push(self.state.symbols[arg_index]);
                }
                bc::SYMBOL_VAL => {
                    let value = self.bytes.next_u16();
                    self.symbol_stack.push(value);
                }
                bc::SYMBOL_BOUND => self.symbol_stack.push(self.bound),
                bc::TAKE_ARG => {
                    let arg_index = self.bytes.next() as usize;
                    self.state_stack.push(self.state.states[arg_index].clone());
                }
                bc::CLONE_ARG => {
                    let arg_index = self.bytes.next() as usize;
                    self.state_stack.push(self.state.states[arg_index].clone());
                }
                bc::FREE_ARG => {
                    self.bytes.next();
                }
                bc::MAKE_STATE => {
                    let end = self.state_stack.len() - self.bytes.next() as usize;
                    let states = self.state_stack.drain(end..).collect();
                    let symbols = std::mem::take(&mut self.symbol_stack);
                    let address = self.bytes.next_u32();
                    self.state_stack.push(State {
                        address,
                        states,
                        symbols,
                    });
                }
                bc::FINAL_STATE => {
                    let states = std::mem::take(&mut self.state_stack);
                    let symbols = std::mem::take(&mut self.symbol_stack);
                    let address = self.bytes.goto();
                    self.state = State {
                        address,
                        states,
                        symbols,
                    };
                    return ControlFlow::Continue(());
                }
                bc::FINAL_ARG => {
                    let arg_index = self.bytes.next() as usize;
                    self.state = self.state.states[arg_index].clone();
                    self.bytes.ip = self.state.address as usize;
                    return ControlFlow::Continue(());
                }
                _ => panic!("invalid bytecode"),
            }
        }
    }
}

struct Tape {
    tape: Vec<u16>,
    head: usize,
}

impl Tape {
    fn left(&mut self, n: u8) -> ControlFlow<()> {
        if let Some(head) = self.head.checked_sub(n as usize) {
            self.head = head;
            ControlFlow::Continue(())
        } else {
            self.head = 0;
            ControlFlow::Break(())
        }
    }

    fn right(&mut self, n: u8) {
        self.head += n as usize;
    }

    fn read(&self) -> u16 {
        self.tape.get(self.head).copied().unwrap_or_default()
    }

    fn write(&mut self, value: u16) {
        if value != 0 {
            if self.head < self.tape.len() {
                self.tape[self.head] = value;
            } else {
                self.tape.resize(self.head + EXTRA_RESIZE_ROOM, 0);
                self.tape[self.head] = value;
            }
        }
    }
}

struct Bytes<'a> {
    bytes: &'a [u8],
    ip: usize,
}

impl Bytes<'_> {
    fn skip(&mut self) {
        let offset = self.next_u16();
        self.ip += offset as usize;
    }

    fn goto(&mut self) -> u32 {
        let address = self.next_u32();
        self.ip = address as usize;
        address
    }

    fn next(&mut self) -> u8 {
        let byte = *self.bytes.get(self.ip).expect("invalid bytecode");
        self.ip += 1;
        byte
    }

    fn next_u16(&mut self) -> u16 {
        let bytes = [self.next(), self.next()];
        u16::from_le_bytes(bytes)
    }

    fn next_u32(&mut self) -> u32 {
        let bytes = [self.next(), self.next(), self.next(), self.next()];
        u32::from_le_bytes(bytes)
    }
}
