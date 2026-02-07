//! Per-key debounce logic.
//!
//! Each key has a counter that must reach DEBOUNCE_THRESHOLD consecutive
//! consistent readings before the debounced state changes. This prevents
//! false triggers from contact bounce.

use crate::matrix::{COLS, ROWS};

/// Number of consistent scan cycles required to register a state change.
/// At ~1ms scan rate, this gives ~5ms debounce time.
const DEBOUNCE_THRESHOLD: u8 = 5;

pub struct Debouncer {
    /// Debounced key states: false = released, true = pressed.
    state: [[bool; COLS]; ROWS],
    /// Per-key counters tracking consecutive raw readings that differ from debounced state.
    counters: [[u8; COLS]; ROWS],
}

impl Debouncer {
    pub const fn new() -> Self {
        Self {
            state: [[false; COLS]; ROWS],
            counters: [[0; COLS]; ROWS],
        }
    }

    /// Update the debouncer with a new raw matrix scan.
    /// `raw_state[row][col]`: true = not pressed (active low convention from matrix scan).
    /// Returns the debounced state where true = key is pressed.
    pub fn update(&mut self, raw_state: &[[bool; COLS]; ROWS]) -> &[[bool; COLS]; ROWS] {
        for row in 0..ROWS {
            for col in 0..COLS {
                // Convert from active-low (true=released) to logical (true=pressed)
                let pressed = !raw_state[row][col];

                if pressed == self.state[row][col] {
                    // Raw matches debounced state, reset counter
                    self.counters[row][col] = 0;
                } else {
                    // Raw differs from debounced state, increment counter
                    self.counters[row][col] += 1;
                    if self.counters[row][col] >= DEBOUNCE_THRESHOLD {
                        self.state[row][col] = pressed;
                        self.counters[row][col] = 0;
                    }
                }
            }
        }

        &self.state
    }
}
