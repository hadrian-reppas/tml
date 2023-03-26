use std::collections::{hash_map::Entry, HashMap, VecDeque};

use bimap::BiMap;

use crate::bytecode as bc;
use crate::error::Error;
use crate::lex::Span;
use crate::parse::{Arm, Name, Op, Pattern, State, Symbol, ToState};

pub struct Compiled {
    pub bytes: Vec<u8>,
    pub symbols: BiMap<String, u16>,
    pub states: HashMap<u32, String>,
    pub tape: Vec<u16>,
}

pub fn compile(unit: Vec<State>, symbols: Vec<Symbol>) -> Result<Compiled, Error> {
    let mut compiler = Compiler {
        bytes: vec![0, 0, 0xff, 0xff, 0xff, 0xff, bc::HALT],
        forward_refs: HashMap::new(),
        addresses: HashMap::new(),
        symbols: Symbols::new(),
        states: unit.into(),
        state_names: HashMap::new(),
    };

    compiler.compile()?;

    let mut tape = Vec::with_capacity(symbols.len());
    for symbol in symbols {
        tape.push(compiler.symbols.insert(symbol)?);
    }

    Ok(Compiled {
        bytes: compiler.bytes,
        symbols: compiler.symbols.0,
        states: compiler.state_names,
        tape,
    })
}

struct Compiler {
    bytes: Vec<u8>,
    forward_refs: HashMap<Signature, Vec<ForwardRef>>,
    addresses: HashMap<Signature, u32>,
    symbols: Symbols,
    states: VecDeque<State>,
    state_names: HashMap<u32, String>,
}

impl Compiler {
    fn compile(&mut self) -> Result<(), Error> {
        while let Some(state) = self.states.pop_front() {
            self.compile_state(state)?;
        }

        for (signature, refs) in &self.forward_refs {
            let span = refs[0].span;
            return Err(Error::new(
                format!("no function with signature `{}`", signature),
                Some(span),
            ));
        }

        let start_signature = Signature {
            name: "start",
            states: 0,
            symbols: 0,
        };

        if let Some(&start_address) = self.addresses.get(&start_signature) {
            self.bytes[2..6].copy_from_slice(&start_address.to_le_bytes());
        } else {
            return Err(Error::new("no `start` function".to_string(), None));
        }

        Ok(())
    }

    fn compile_state(
        &mut self,
        State {
            name,
            state_params,
            symbol_params,
            arms,
        }: State,
    ) -> Result<(), Error> {
        self.increment_count(name.span)?;

        let state_map = make_map(&state_params, "state")?;
        let symbol_map = make_map(&symbol_params, "symbol")?;

        let address = self.bytes.len() as u32;
        self.state_names.insert(address, name.name.to_string());
        let signature = Signature {
            name: name.name,
            states: state_map.len() as u8,
            symbols: symbol_map.len() as u8,
        };

        if let Some((_, refs)) = self.forward_refs.remove_entry(&signature) {
            let bytes = address.to_le_bytes();
            for f_ref in refs {
                self.bytes[f_ref.location..f_ref.location + 4].copy_from_slice(&bytes);
            }
        }
        if self.addresses.insert(signature, address).is_some() {
            return Err(Error::new(
                format!("a function with signature `{}` already exists", signature),
                Some(name.span),
            ));
        }

        let arm_count = arms.len();
        for (i, arm) in arms.into_iter().enumerate() {
            self.compile_arm(arm, &state_map, &symbol_map, i == arm_count - 1)?;
        }

        Ok(())
    }

    fn compile_arm(
        &mut self,
        Arm {
            pattern,
            ops,
            to_state,
        }: Arm,
        state_map: &HashMap<&'static str, u8>,
        symbol_map: &HashMap<&'static str, u8>,
        is_last_arm: bool,
    ) -> Result<(), Error> {
        let pattern_span = match &pattern {
            Pattern::Symbol(symbol) => symbol.span,
            Pattern::Name(name) => name.span,
        };

        let bound = self.compile_pattern(pattern, symbol_map, is_last_arm)?;

        let location = self.bytes.len();
        if !is_last_arm {
            self.bytes.extend(u16::MAX.to_le_bytes());
        }

        self.compile_ops(OpIter(ops.into()), symbol_map, bound)?;

        let mut counts: HashMap<_, _> = state_map.keys().map(|&name| (name, 0)).collect();
        count_state_args(&to_state, &mut counts)?;
        self.compile_to_state(to_state, state_map, symbol_map, &mut counts, bound, true)?;

        if !is_last_arm {
            let jump_size = self.bytes.len() - location - 2;
            match TryInto::<u16>::try_into(jump_size) {
                Ok(jump_size) => {
                    let bytes = jump_size.to_le_bytes();
                    self.bytes[location..location + 2].copy_from_slice(&bytes);
                }
                Err(_) => {
                    return Err(Error::new(
                        "this arm is too complicated".to_string(),
                        Some(pattern_span),
                    ))
                }
            }
        }

        Ok(())
    }

    fn compile_pattern(
        &mut self,
        pattern: Pattern,
        symbol_map: &HashMap<&'static str, u8>,
        is_last_arm: bool,
    ) -> Result<&'static str, Error> {
        match pattern {
            Pattern::Symbol(symbol) => {
                if is_last_arm {
                    Err(Error::new(
                        "last arm must be a catchall".to_string(),
                        Some(symbol.span),
                    ))
                } else {
                    let value = self.symbols.insert(symbol)?;
                    self.bytes.push(bc::COMPARE_VAL);
                    self.bytes.extend(value.to_le_bytes());
                    Ok("")
                }
            }
            Pattern::Name(name) => {
                if let Some(&arg_index) = symbol_map.get(name.name) {
                    if is_last_arm {
                        Err(Error::new(
                            "last arm must be a catchall".to_string(),
                            Some(name.span),
                        ))
                    } else {
                        self.bytes.push(bc::COMPARE_ARG);
                        self.bytes.push(arg_index);
                        Ok("")
                    }
                } else {
                    if is_last_arm {
                        self.bytes.push(bc::OTHER);
                        Ok(name.name)
                    } else {
                        Err(Error::new(
                            "only the last arm can be a catchall".to_string(),
                            Some(name.span),
                        ))
                    }
                }
            }
        }
    }

    fn compile_ops(
        &mut self,
        ops: OpIter,
        symbol_map: &HashMap<&'static str, u8>,
        bound: &str,
    ) -> Result<(), Error> {
        for op in ops {
            match op {
                MultiOp::Left(mut n) => {
                    while n >= 255 {
                        self.bytes.push(bc::LEFT_N);
                        self.bytes.push(255);
                        n -= 255;
                    }

                    if n == 1 {
                        self.bytes.push(bc::LEFT);
                    } else if n > 0 {
                        self.bytes.push(bc::LEFT_N);
                        self.bytes.push(n as u8);
                    }
                }
                MultiOp::Right(mut n) => {
                    while n >= 255 {
                        self.bytes.push(bc::RIGHT_N);
                        self.bytes.push(255);
                        n -= 255;
                    }

                    if n == 1 {
                        self.bytes.push(bc::RIGHT);
                    } else if n > 0 {
                        self.bytes.push(bc::RIGHT_N);
                        self.bytes.push(n as u8);
                    }
                }
                MultiOp::Name(name) => {
                    if let Some(&arg_index) = symbol_map.get(name.name) {
                        self.bytes.push(bc::WRITE_ARG);
                        self.bytes.push(arg_index);
                    } else if name.name == bound {
                        self.bytes.push(bc::WRITE_BOUND);
                    } else {
                        return Err(Error::new(
                            format!("no value with name `{}`", name.name),
                            Some(name.span),
                        ));
                    }
                }
                MultiOp::Symbol(symbol) => {
                    let value = self.symbols.insert(symbol)?;
                    self.bytes.push(bc::WRITE_VAL);
                    self.bytes.extend(value.to_le_bytes());
                }
            }
        }
        Ok(())
    }

    fn compile_to_state(
        &mut self,
        to_state: ToState,
        state_map: &HashMap<&'static str, u8>,
        symbol_map: &HashMap<&'static str, u8>,
        arg_counts: &mut HashMap<&'static str, usize>,
        bound: &str,
        is_outer: bool,
    ) -> Result<(), Error> {
        match to_state {
            ToState::State {
                name,
                state_args,
                symbol_args,
            } => match arg_counts.get(name.name) {
                Some(1) => {
                    if is_outer {
                        for (arg, count) in arg_counts {
                            if *count == 0 {
                                self.bytes.push(bc::FREE_ARG);
                                self.bytes.push(state_map[arg]);
                            }
                        }
                        self.bytes.push(bc::FINAL_ARG);
                        self.bytes.push(state_map[name.name]);
                        Ok(())
                    } else {
                        self.bytes.push(bc::TAKE_ARG);
                        self.bytes.push(state_map[name.name]);
                        Ok(())
                    }
                }
                Some(&count) => {
                    self.bytes.push(bc::CLONE_ARG);
                    self.bytes.push(state_map[name.name]);
                    arg_counts.insert(name.name, count - 1);
                    Ok(())
                }
                None => {
                    let signature = Signature {
                        name: name.name,
                        states: state_args.len() as u8,
                        symbols: symbol_args.len() as u8,
                    };

                    for state_arg in state_args {
                        self.compile_to_state(
                            state_arg, state_map, symbol_map, arg_counts, bound, false,
                        )?;
                    }

                    for symbol_arg in symbol_args {
                        match symbol_arg {
                            Pattern::Symbol(symbol) => {
                                let value = self.symbols.insert(symbol)?;
                                self.bytes.push(bc::SYMBOL_VAL);
                                self.bytes.extend(&value.to_le_bytes());
                            }
                            Pattern::Name(name) => {
                                if let Some(&arg_index) = symbol_map.get(name.name) {
                                    self.bytes.push(bc::SYMBOL_ARG);
                                    self.bytes.push(arg_index);
                                } else if name.name == bound {
                                    self.bytes.push(bc::SYMBOL_BOUND);
                                } else {
                                    return Err(Error::new(
                                        format!("no value with name `{}`", name.name),
                                        Some(name.span),
                                    ));
                                }
                            }
                        }
                    }

                    match (self.addresses.get(&signature), is_outer) {
                        (Some(&address), false) => {
                            self.bytes.push(bc::MAKE_STATE);
                            self.bytes.push(signature.states);
                            self.bytes.extend(&address.to_le_bytes());
                        }
                        (Some(&address), true) => {
                            for (arg, count) in arg_counts {
                                if *count == 0 {
                                    self.bytes.push(bc::FREE_ARG);
                                    self.bytes.push(state_map[arg]);
                                }
                            }

                            self.bytes.push(bc::FINAL_STATE);
                            self.bytes.extend(&address.to_le_bytes());
                        }
                        (None, false) => {
                            self.bytes.push(bc::MAKE_STATE);
                            self.bytes.push(signature.states);

                            let forward_ref = ForwardRef {
                                location: self.bytes.len(),
                                span: name.span,
                            };
                            self.bytes.extend(&u32::MAX.to_le_bytes());

                            match self.forward_refs.entry(signature) {
                                Entry::Occupied(mut o) => o.get_mut().push(forward_ref),
                                Entry::Vacant(v) => {
                                    v.insert(vec![forward_ref]);
                                }
                            }
                        }
                        (None, true) => {
                            for (arg, count) in arg_counts {
                                if *count == 0 {
                                    self.bytes.push(bc::FREE_ARG);
                                    self.bytes.push(state_map[arg]);
                                }
                            }

                            self.bytes.push(bc::FINAL_STATE);

                            let forward_ref = ForwardRef {
                                location: self.bytes.len(),
                                span: name.span,
                            };

                            self.bytes.extend(&u32::MAX.to_le_bytes());

                            match self.forward_refs.entry(signature) {
                                Entry::Occupied(mut o) => o.get_mut().push(forward_ref),
                                Entry::Vacant(v) => {
                                    v.insert(vec![forward_ref]);
                                }
                            }
                        }
                    }

                    Ok(())
                }
            },
            ToState::Halt { .. } => {
                if is_outer {
                    self.bytes.push(bc::FINAL_STATE);
                    self.bytes.extend(bc::HALT_ADDRESS.to_le_bytes());
                    Ok(())
                } else {
                    self.bytes.push(bc::MAKE_STATE);
                    self.bytes.push(0);
                    self.bytes.extend(bc::HALT_ADDRESS.to_le_bytes());
                    Ok(())
                }
            }
        }
    }

    fn increment_count(&mut self, span: Span) -> Result<(), Error> {
        let bytes = [self.bytes[0], self.bytes[1]];
        let count = u16::from_le_bytes(bytes);
        if let Some(new_count) = count.checked_add(1) {
            let bytes = new_count.to_le_bytes();
            self.bytes[..2].copy_from_slice(&bytes);
            Ok(())
        } else {
            Err(Error::new(
                "too many states in program (max is 65536)".to_string(),
                Some(span),
            ))
        }
    }
}

fn make_map(params: &[Name], kind: &str) -> Result<HashMap<&'static str, u8>, Error> {
    let mut map = HashMap::new();

    for name in params {
        if map.contains_key(name.name) {
            return Err(Error::new(
                format!("duplicate {kind} parameter `{}`", name.name),
                Some(name.span),
            ));
        } else if map.len() == 256 {
            return Err(Error::new(
                format!("too many {kind} parameters (max is 256)"),
                Some(name.span),
            ));
        } else {
            map.insert(name.name, map.len() as u8);
        }
    }

    Ok(map)
}

fn count_state_args(
    state: &ToState,
    counts: &mut HashMap<&'static str, usize>,
) -> Result<(), Error> {
    match state {
        ToState::State {
            name,
            state_args,
            symbol_args,
        } => {
            if let Some(&count) = counts.get(name.name) {
                if state_args.is_empty() && symbol_args.is_empty() {
                    counts.insert(name.name, count + 1);
                    Ok(())
                } else {
                    Err(Error::new(
                        format!(
                            "`{}` is a state parameter, so it can't take arguments",
                            name.name
                        ),
                        Some(name.span),
                    ))
                }
            } else {
                for arg in state_args {
                    count_state_args(arg, counts)?;
                }
                Ok(())
            }
        }
        ToState::Halt { .. } => Ok(()),
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Signature {
    name: &'static str,
    states: u8,
    symbols: u8,
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        match (self.states, self.symbols) {
            (0, 0) => Ok(()),
            (_, 0) => {
                write!(f, "(")?;
                for i in 0..self.states {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "_")?;
                }
                write!(f, ")")
            }
            (_, _) => {
                write!(f, "(")?;
                for i in 0..self.states {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "_")?;
                }
                write!(f, "; ")?;
                for i in 0..self.symbols {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "_")?;
                }
                write!(f, ")")
            }
        }
    }
}

struct ForwardRef {
    location: usize,
    span: Span,
}

struct Symbols(BiMap<String, u16>);

impl Symbols {
    fn new() -> Self {
        let mut map = BiMap::new();
        map.insert(String::new(), 0);
        Symbols(map)
    }

    fn insert(&mut self, symbol: Symbol) -> Result<u16, Error> {
        let Symbol { symbol, span } = symbol;
        let len = self.0.len();
        if let Some(&value) = self.0.get_by_left(&symbol) {
            Ok(value)
        } else {
            match len.try_into() {
                Ok(value) => {
                    self.0.insert(symbol, value);
                    Ok(value)
                }
                Err(_) => Err(Error::new(
                    "too many unique symbols in program (max is 65536)".to_string(),
                    Some(span),
                )),
            }
        }
    }
}

struct OpIter(VecDeque<Op>);

impl OpIter {
    fn count_moves(&mut self, mut offset: isize) -> Option<MultiOp> {
        while matches!(self.0.front(), Some(Op::Left(_) | Op::Right(_))) {
            match self.0.pop_front() {
                Some(Op::Left(_)) => offset -= 1,
                Some(Op::Right(_)) => offset += 1,
                _ => unreachable!(),
            }
        }

        if offset == 0 {
            self.next()
        } else if offset < 0 {
            Some(MultiOp::Left(-offset as usize))
        } else {
            Some(MultiOp::Right(offset as usize))
        }
    }
}

impl Iterator for OpIter {
    type Item = MultiOp;
    fn next(&mut self) -> Option<Self::Item> {
        match self.0.pop_front()? {
            Op::Left(_) => self.count_moves(-1),
            Op::Right(_) => self.count_moves(1),
            Op::Name(name) => Some(MultiOp::Name(name)),
            Op::Symbol(symbol) => Some(MultiOp::Symbol(symbol)),
        }
    }
}

enum MultiOp {
    Left(usize),
    Right(usize),
    Name(Name),
    Symbol(Symbol),
}
