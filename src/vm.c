#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include <stdio.h>

#define COMPARE_ARG 0
#define COMPARE_VAL 1
#define OTHER 2
#define HALT 3

#define LEFT 4
#define RIGHT 5
#define LEFT_N 6
#define RIGHT_N 7
#define WRITE_ARG 8
#define WRITE_VAL 9
#define WRITE_BOUND 10

#define SYMBOL_ARG 11
#define SYMBOL_VAL 12
#define SYMBOL_BOUND 13
#define TAKE_ARG 14
#define CLONE_ARG 15
#define FREE_ARG 16
#define MAKE_STATE 17
#define FINAL_STATE 18
#define FINAL_ARG 19

#define INTIAL_TAPE_CAPACITY 256
#define TAPE_GROWTH_FACTOR 2
#define STATE_STACK_CAPACITY 1024

#define ControlFlow bool
#define STOP true
#define CONTINUE false

typedef struct State {
  uint32_t address;
  struct State *states;
  size_t state_count;
  uint16_t *symbols;
  size_t symbol_count;
} State;

// tape
uint16_t *tape;
uint16_t *tape_end;
uint16_t *tape_head;

// current state
uint32_t address;
State states[256];
size_t state_count;
uint16_t symbols[256];
size_t symbol_count;

// stacks
State state_stack[STATE_STACK_CAPACITY];
State *state_stack_top = &state_stack[0];
uint16_t symbol_stack[256];
uint16_t *symbol_stack_top = &symbol_stack[0];

// bytes
uint8_t *bytes_start;
uint8_t *ip;

// misc
size_t max_moves;
size_t moves;
uint16_t bound;

void free_state(State *state) {
  if (state->state_count) {
    free(state->states);
  }
  if (state->symbol_count) {
    free(state->symbols);
  }
}

State clone_state(State *state) {
  State cloned;
  cloned.address = state->address;
  cloned.state_count = state->state_count;
  cloned.symbol_count = state->symbol_count;
  if (cloned.state_count) {
    cloned.states = malloc(cloned.state_count * sizeof(State));
    for (size_t i = 0; i < cloned.state_count; i++) {
      cloned.states[i] = clone_state(&state->states[i]);
    }
  }
  if (cloned.symbol_count) {
    cloned.symbols = malloc(cloned.symbol_count * sizeof(uint16_t));
    memcpy(cloned.symbols, state->symbols,
           cloned.symbol_count * sizeof(uint16_t));
  }
  return cloned;
}

void print_state(State *state) {
  printf("State(0x%08x", state->address);
  for (size_t i = 0; i < state->state_count; i++) {
    if (i) {
      printf(", ");
    } else {
      printf("; ");
    }
    print_state(&state->states[i]);
  }
  if (state->symbol_count) {
    for (size_t i = 0; i < state->symbol_count; i++) {
      if (i) {
        printf(", ");
      } else {
        printf("; ");
      }
      printf("%hu", state->symbols[i]);
    }
  }
  printf(")");
}

void init_tape(uint16_t *symbols, size_t len) {
  if (len < INTIAL_TAPE_CAPACITY) {
    tape = calloc(INTIAL_TAPE_CAPACITY, sizeof(uint16_t));
    tape_end = &tape[INTIAL_TAPE_CAPACITY];
  } else {
    tape = calloc(len, sizeof(uint16_t));
    tape_end = &tape[len];
  }
  tape_head = tape;
  memcpy(tape, symbols, len * sizeof(uint16_t));
}

ControlFlow tape_left(size_t n) {
  if (tape_head - tape < (long)n) {
    tape_head = tape;
    return STOP;
  } else {
    tape_head -= n;
    return CONTINUE;
  }
}

void tape_right(size_t n) { tape_head += n; }

uint16_t read_tape() {
  if (tape_head >= tape_end) {
    return 0;
  } else {
    return *tape_head;
  }
}

void write_tape(uint16_t value) {
  if (tape_head < tape_end) {
    *tape_head = value;
  } else {
    if (value) {
      size_t head_offset = tape_head - tape;
      size_t old_len = tape_end - tape;
      size_t new_len = TAPE_GROWTH_FACTOR * head_offset;

      tape = realloc(tape, new_len * sizeof(uint16_t));
      memset(&tape[old_len], 0, (new_len - old_len)*sizeof(uint16_t));
      tape_head = &tape[head_offset];
      tape_end = &tape[new_len];

      *tape_head = value;
    }
  }
}

uint8_t next() { return *ip++; }

uint16_t next_u16() {
  uint16_t low = next();
  uint16_t high = next();
  return low | (high << 8);
}

uint32_t next_u32() {
  uint32_t a = next();
  uint32_t b = next();
  uint32_t c = next();
  uint32_t d = next();
  return a | (b << 8) | (c << 16) | (d << 24);
}

void go_to(uint32_t address) { ip = bytes_start + address; }

void skip(uint16_t skip) { ip += skip; }

void push_symbol(uint16_t value) {
  *symbol_stack_top = value;
  symbol_stack_top++;
}

void push_state(State state) {
  *state_stack_top = state;
  state_stack_top++;
}

ControlFlow run_rhs() {
  while (true) {
    switch (next()) {
    case LEFT: {
      if (tape_left(1) == STOP) {
        return STOP;
      }
      break;
    }
    case RIGHT: {
      tape_right(1);
      break;
    }
    case LEFT_N: {
      if (tape_left(next()) == STOP) {
        return STOP;
      }
      break;
    }
    case RIGHT_N: {
      tape_right(next());
      break;
    }
    case WRITE_ARG: {
      uint8_t arg_index = next();
      write_tape(symbols[arg_index]);
      break;
    }
    case WRITE_VAL: {
      uint16_t value = next_u16();
      write_tape(value);
      break;
    }
    case WRITE_BOUND: {
      write_tape(bound);
      break;
    }
    case SYMBOL_ARG: {
      uint8_t arg_index = next();
      push_symbol(symbols[arg_index]);
      break;
    }
    case SYMBOL_VAL: {
      uint16_t value = next_u16();
      push_symbol(value);
      break;
    }
    case SYMBOL_BOUND: {
      push_symbol(bound);
      break;
    }
    case TAKE_ARG: {
      uint8_t arg_index = next();
      push_state(states[arg_index]);
      break;
    }
    case CLONE_ARG: {
      uint8_t arg_index = next();
      push_state(clone_state(&states[arg_index]));
      break;
    }
    case FREE_ARG: {
      uint8_t arg_index = next();
      free_state(&states[arg_index]);
      break;
    }
    case MAKE_STATE: {
      uint8_t args = next();
      uint32_t address = next_u32();

      State state;
      state.address = address;
      state.state_count = args;
      state.symbol_count = symbol_stack_top - symbol_stack;

      if (state.state_count) {
        state_stack_top -= args;
        state.states = malloc(args * sizeof(State));
        memcpy(state.states, state_stack_top, args * sizeof(State));
      }
      if (state.symbol_count) {
        state.symbols = malloc(state.symbol_count * sizeof(uint16_t));
        memcpy(state.symbols, symbol_stack,
               state.symbol_count * sizeof(uint16_t));
        symbol_stack_top = symbol_stack;
      }

      push_state(state);
      break;
    }
    case FINAL_STATE: {
      address = next_u32();
      state_count = state_stack_top - state_stack;
      symbol_count = symbol_stack_top - symbol_stack;

      if (state_count) {
        memcpy(states, state_stack, state_count * sizeof(State));
        state_stack_top = state_stack;
      }
      if (symbol_count) {
        memcpy(symbols, symbol_stack, symbol_count * sizeof(uint16_t));
        symbol_stack_top = symbol_stack;
      }

      go_to(address);
      return CONTINUE;
    }
    case FINAL_ARG: {
      uint8_t arg_index = next();
      State state = states[arg_index];
      address = state.address;
      state_count = state.state_count;
      if (state_count) {
        memcpy(states, &state.states[0], state.state_count * sizeof(State));
        free(state.states);
      }
      symbol_count = state.symbol_count;
      if (symbol_count) {
        memcpy(symbols, &state.symbols[0],
               state.symbol_count * sizeof(uint8_t));
        free(state.symbols);
      }
      go_to(address);
      return CONTINUE;
    }
    }
  }
}

ControlFlow run_move() {
  while (true) {
    switch (next()) {
    case COMPARE_ARG: {
      uint8_t arg_index = next();
      if (read_tape() == symbols[arg_index]) {
        next_u16();
        return run_rhs();
      } else {
        skip(next_u16());
      }
      break;
    }
    case COMPARE_VAL: {
      if (next_u16() == read_tape()) {
        next_u16();
        return run_rhs();
      } else {
        skip(next_u16());
      }
      break;
    }
    case OTHER: {
      bound = read_tape();
      return run_rhs();
    }
    case HALT: {
      return STOP;
    }
    }
  }
}

void run(uint8_t *bytes, size_t max_moves_) {
  bytes_start = bytes;
  ip = bytes;
  max_moves = max_moves_;
  moves = 0;

  next_u16();
  state_count = 0;
  symbol_count = 0;
  address = next_u32();
  go_to(address);

  while (moves < max_moves) {
    if (run_move() == STOP) {
      break;
    }
    moves++;
  }
}

uint32_t get_final_address() { return address; }

uint16_t *get_tape() { return tape; }

size_t get_tape_len() { return tape_end - tape; }

size_t get_tape_head_position() { return tape_head - tape; }

size_t get_move_count() { return moves; }

void cleanup() {
  free(tape);
  for (size_t i = 0; i < state_count; i++) {
    free_state(&states[i]);
  }
}
