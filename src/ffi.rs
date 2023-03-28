use crate::vm::Simulated;

extern "C" {
    fn init_tape(tape: *const u16, len: usize);
    fn run(bytes: *const u8, max_moves: usize);
    fn get_final_address() -> u32;
    fn get_tape() -> *const u16;
    fn get_tape_len() -> usize;
    fn get_tape_head_position() -> usize;
    fn get_move_count() -> usize;
    fn cleanup();
}

pub fn simulate(bytes: &[u8], tape: &[u16], max_moves: usize) -> Simulated {
    unsafe {
        init_tape(tape.as_ptr(), tape.len());
        run(bytes.as_ptr(), max_moves);

        let mut tape = std::slice::from_raw_parts(get_tape(), get_tape_len()).to_vec();
        while let Some(0) = tape.last() {
            tape.pop();
        }

        cleanup();

        Simulated {
            tape,
            head_position: get_tape_head_position(),
            final_address: get_final_address(),
            moves: get_move_count(),
        }
    }
}
