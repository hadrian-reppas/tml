# `tml`: A simple language for describing Turing machines

I recently read [The Annotated Turing](https://www.amazon.com/Annotated-Turing-Through-Historic-Computability/dp/0470229055),
a book by Charles Petzold. It goes through Turing's 1936 paper [On Computable Numbers](https://www.cs.virginia.edu/~robins/Turing_Paper_1936.pdf).
To simulate some of the Turing machines described in the book, I created `tml`. 
(I was going to call it Hadrian's Turing Machine Language and use `.html` file
extension but that confuses text editors.) I designed the language to share
most of syntax Turing uses in his paper.

## Examples

Let's say we want to create a machine that alternates printing `0`'s and `1`'s.
We could use this machine:

```
start {
    '' | '0' | f,
}

f {
    '0' | > '1' | f,
    '1' | > '0' | f,
}
```

This machines contains two states: `start` and `f`. Each state contains zero
or more arms. Each arm contains three parts: a pattern, zero or more instructions
and a final state. The three parts are separated by `|`'s. `'0'`, `'1'` and `''`
are symbol literals. The `>` characters are instructions that tell the machine's
head to move right. 

You can read the `f` state like this:
 1. If the symbol under the machine's head is `'0'`, then move the head to the
    right, print `'1'` and go to state `f`.
 2. If The symbol under the machine's head is `'1`', then move the head to the
    right, print '`0`' and go to state `f`.
 3. Otherwise, halt.

Every machine must contain a state named `start`. When we run the machine, it
starts at the `start` state with an infinite tape filled with `''` symbols. The
`start` state looks at the symbol under the machine's head. Since the symbol is
`''`, we move into the that arm, print a `'0'` and go to state `f`. State `f`
looks at the current symbol, moves to the right, prints the other symbol and
returns to state `f`.

If we put our machine in a file called `simple.tml`, we can simulate 10 moves with

```
cargo run -- simple.tml -m 10
```

This produces the following output:

```
final tape:
┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬
│ 0 │ 1 │ 0 │ 1 │ 0 │ 1 │ 0 │ 1 │ 0 │ 1 │
┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴

decimal: 0.000

number of moves: 10
final head position: 9
```

## Functions

Just like in Turing's paper, you can define functions. Consider the following
state:

```
match(A, B; x) {
    x | | A,
    _ | | B,
}
```

This function has two state parameters (`A` and `B`) and one symbol parameter
(`x`). It checks if the symbol at the machine's head is the symbol argument `x`.
If it is, then the machine transitions to the state argument `A`. Otherwise, it
transitions to the state argument `B`. We can "call" the `match` function like this:

```
start {
    '' | 'a' | match(f, g; 'a'),
}

f {}
g {}
```

Here, the match function checks if the tape contains the symbol `'a'` and goes
to state `f` if it does and `g` if it does not.

## Other things

You can "bind" the symbol at the machine's head to a variable like this:

```
copy(A) {
    x | > x | A,
}
```

The `copy` function copies the symbol at the current position to the next position
on the tape then goes to state parameter `A`.

You can also halt with the special `!` state:

```
halt_if(; a) {
    a | | !,
    _ | | f,
}
```

So if this function is called with `halt_if(; 'x')`, the machine halts if the
symbol at the machine's head is `'x'`.

## The final decimal

Because Turing's paper focuses on computable numbers, `tmi` automaticaly
interprets the final tape as a number between 0 and 1. Consider the following
machine (that can be found in `examples/turing_2.tml`):

```
start {
    _ | 'ə' > 'ə' > | b,
}

b {
    ''  | '0'    | b,
    '0' | >> '1' | b,
    '1' | >> '0' | b,
}
```

If we run with it with

```
cargo run -- examples/turing_2.tml -m 20
```

we get

```
final tape:
┬───┬───┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬
│ ə │ ə │ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │  │ 1 │  │
┴───┴───┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴
┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬──┬───┬
│ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │  │ 1 │  │ 0 │
┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴──┴───┴

decimal: 0.333332

number of moves: 20
final head position: 38
```

By default, the final decimal is interpreted as a base 2 number that starts at
position 2 and has a digit in every other square on the tape. So the tape above
is interpreted as the base 2 number `0.0101010101010101010`. In base 10, we get
`0.333332` (which is what `tml` prints out for us).

You can control the radix with the `-r` or `--decimal-radix` flags. The radix
defaults to 2. You can control the start position with the `-s` or
`--decimal-start` flags. The start position defaults to 2. You can control the
stride with the `-S` or `--decimal-stride` flags. The stride defaults to 2.

## How it works

The `.tml` file is interpreted in two steps. First, it is compiled into a
bytecode. Then the bytecode is interpreted by a virtual machine. The default
VM is written in C, but you can use a VM written in safe Rust with the 
`--rust-vm` flag. The Rust VM is about 10% slower. You can inspect the generated
bytecode with the `-b` or `--dump-bytecode` flags.

The fact that machines are compiled to bytecode means they are actually pretty
fast. The Turing machine that Petzold describes to calculate $\sqrt{2}/2$
is implemented in `examples/sqrt2.tml`. On my computer, I can simulate
1,000,000,000 moves in 15 seconds. 1,000,000,000 moves is enough to calculate
$\sqrt{2}/2$ to 50 decimal places. To run it yourself, use

```
cargo run --release -- examples/sqrt2.tml -m 1000000000 --hide-tape
```

Make sure to use the `--release` flag so the code is optimized. When run, it
outputs this:

```
compile time: 437.334µs
execution time: 15.237713143s

decimal: 0.70710678118654752440084436210484903928483593768847

number of moves: 1000000000
final head position: 307
```

## Usage

```
Usage: tml [OPTIONS] <FILE> [TAPE]

Arguments:
  <FILE>  File containing the Turing machine
  [TAPE]  File containing the initial tape

Options:
  -m, --max-moves <MAX_MOVES>            Maximum number of moves
      --hide-tape                        Don't print the final tape
      --hide-decimal                     Don't print the decimal interpretation of the final tape
  -r, --decimal-radix <DECIMAL_RADIX>    Radix for the final decimal [default: 2]
  -d, --decimal-digits <DECIMAL_DIGITS>  Digits in the final decimal
  -s, --decimal-start <DECIMAL_START>    Start position for the final decimal [default: 2]
  -S, --decimal-stride <DECIMAL_STRIDE>  Stride for the final decimal [default: 2]
      --no-color                         Don't color output
      --allow-tabs                       Allow tab characters in machine and tape files
  -b, --dump-bytecode                    Dump bytecode
      --rust-vm                          Use Rust VM
  -t, --time                             Time execution
  -w, --terminal_width <TERMINAL_WIDTH>  Maximum width when printing the final tape
  -h, --help                             Print help
```

Note that you can initialize the tape by passing in a file that contains a
sequence of symbols. Like this, for example:

```
'a' 'b' 'c'
'' 'xyz' ''
'symbol' 'a'
```
