//! Shared, globally accessible IO state.
//!
//! Both the USB command handler and the input watcher tasks need to read or
//! update a common snapshot of the current output/input state (e.g. to
//! answer a `GET` request with the full status). This is done with plain
//! atomics rather than a mutex since single-bit read/modify/write is all
//! that's needed.

use portable_atomic::{AtomicU8, Ordering};

use crate::io::{NUM_INPUTS, NUM_OUTPUTS};

/// Bitmask mirror of the current output levels, bit `n` == `Out{n+1}`.
static OUTPUT_STATE: AtomicU8 = AtomicU8::new(0);
/// Bitmask mirror of the current (logical) input levels, bit `n` == `In{n+2}`.
static INPUT_STATE: AtomicU8 = AtomicU8::new(0);

fn set_bit(state: &AtomicU8, bit: u8, value: bool) {
    if value {
        state.fetch_or(1 << bit, Ordering::Relaxed);
    } else {
        state.fetch_and(!(1 << bit), Ordering::Relaxed);
    }
}

/// Record output `output` (board label number, `1..=8`) as being set to `value`.
pub fn set_output(output: u8, value: bool) {
    let bit = output - 1;
    debug_assert!((bit as usize) < NUM_OUTPUTS);
    set_bit(&OUTPUT_STATE, bit, value);
}

/// Record input `input` (board label number, `2..=8`) as being at logical level `value`.
pub fn set_input(input: u8, value: bool) {
    let bit = input - promatrix_protocol::FIRST_INPUT;
    debug_assert!((bit as usize) < NUM_INPUTS);
    set_bit(&INPUT_STATE, bit, value);
}

/// Current output bitmask, bit `n` == `Out{n+1}`.
pub fn outputs() -> u8 {
    OUTPUT_STATE.load(Ordering::Relaxed)
}

/// Current input bitmask, bit `n` == `In{n+2}`.
pub fn inputs() -> u8 {
    INPUT_STATE.load(Ordering::Relaxed)
}
