//! Keymap definitions and layer management for the ErgoDox.
//!
//! The ErgoDox has a 6×14 matrix (6 rows, 14 columns: 7 left + 7 right).
//! Multiple layers can be defined, with transparent keys falling through
//! to lower layers.

use crate::matrix::{COLS, ROWS};

/// USB HID keycodes.
/// See USB HID Usage Tables, Section 10 (Keyboard/Keypad Page 0x07).
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Keycode {
    /// No key / transparent (fall through to lower layer)
    Trans = 0x00,
    /// Error rollover
    None = 0x01,

    // Letters
    A = 0x04,
    B = 0x05,
    C = 0x06,
    D = 0x07,
    E = 0x08,
    F = 0x09,
    G = 0x0A,
    H = 0x0B,
    I = 0x0C,
    J = 0x0D,
    K = 0x0E,
    L = 0x0F,
    M = 0x10,
    N = 0x11,
    O = 0x12,
    P = 0x13,
    Q = 0x14,
    R = 0x15,
    S = 0x16,
    T = 0x17,
    U = 0x18,
    V = 0x19,
    W = 0x1A,
    X = 0x1B,
    Y = 0x1C,
    Z = 0x1D,

    // Numbers
    N1 = 0x1E,
    N2 = 0x1F,
    N3 = 0x20,
    N4 = 0x21,
    N5 = 0x22,
    N6 = 0x23,
    N7 = 0x24,
    N8 = 0x25,
    N9 = 0x26,
    N0 = 0x27,

    // Control keys
    Enter = 0x28,
    Escape = 0x29,
    Backspace = 0x2A,
    Tab = 0x2B,
    Space = 0x2C,
    Minus = 0x2D,
    Equal = 0x2E,
    LBracket = 0x2F,
    RBracket = 0x30,
    Backslash = 0x31,
    Semicolon = 0x33,
    Quote = 0x34,
    Grave = 0x35,
    Comma = 0x36,
    Dot = 0x37,
    Slash = 0x38,
    CapsLock = 0x39,
    /// Non-US \ and | (ISO key left of Z — produces < > on Nordic layouts)
    NonUsBackslash = 0x64,

    // Function keys
    F1 = 0x3A,
    F2 = 0x3B,
    F3 = 0x3C,
    F4 = 0x3D,
    F5 = 0x3E,
    F6 = 0x3F,
    F7 = 0x40,
    F8 = 0x41,
    F9 = 0x42,
    F10 = 0x43,
    F11 = 0x44,
    F12 = 0x45,

    // Navigation
    PrintScreen = 0x46,
    ScrollLock = 0x47,
    Pause = 0x48,
    Insert = 0x49,
    Home = 0x4A,
    PageUp = 0x4B,
    Delete = 0x4C,
    End = 0x4D,
    PageDown = 0x4E,
    Right = 0x4F,
    Left = 0x50,
    Down = 0x51,
    Up = 0x52,

    // Modifiers (used in the modifier byte, not in keycode array)
    LCtrl = 0xE0,
    LShift = 0xE1,
    LAlt = 0xE2,
    LGui = 0xE3,
    RCtrl = 0xE4,
    RShift = 0xE5,
    RAlt = 0xE6,
    RGui = 0xE7,

    // Special: layer momentary hold (not a real HID keycode)
    // Encoded as 0xF0 + layer number
    Layer1 = 0xF1,
}

impl Keycode {
    /// Check if this keycode is a modifier (LCtrl..RGui).
    pub fn is_modifier(self) -> bool {
        let v = self as u8;
        (0xE0..=0xE7).contains(&v)
    }

    /// Get the modifier bit mask (bit 0 = LCtrl, bit 7 = RGui).
    pub fn modifier_bit(self) -> u8 {
        if self.is_modifier() {
            1 << (self as u8 - 0xE0)
        } else {
            0
        }
    }

    /// Check if this is a layer switch key.
    pub fn is_layer(self) -> bool {
        let v = self as u8;
        (0xF0..=0xFF).contains(&v)
    }

    /// Get the target layer number for a layer key.
    pub fn layer_number(self) -> usize {
        (self as u8 - 0xF0) as usize
    }

    /// Check if this is a transparent key.
    pub fn is_transparent(self) -> bool {
        self as u8 == 0x00
    }
}

/// Number of layers.
pub const NUM_LAYERS: usize = 2;

/// Key is unused in the matrix position.
const ___: Keycode = Keycode::Trans;

/// Shorthand aliases for readability.
const ENT: Keycode = Keycode::Enter;
const ESC: Keycode = Keycode::Escape;
const BSP: Keycode = Keycode::Backspace;
const TAB: Keycode = Keycode::Tab;
const SPC: Keycode = Keycode::Space;
const DEL: Keycode = Keycode::Delete;
const LCTL: Keycode = Keycode::LCtrl;
const LSFT: Keycode = Keycode::LShift;
const LALT: Keycode = Keycode::LAlt;
const LGUI: Keycode = Keycode::LGui;
const RSFT: Keycode = Keycode::RShift;
const RALT: Keycode = Keycode::RAlt;
const NUBS: Keycode = Keycode::NonUsBackslash;
const LY1: Keycode = Keycode::Layer1;

/// Keymap layers.
/// Layout follows the ErgoDox physical matrix:
///   Row 0-5, Columns 0-6 = left half, Columns 7-13 = right half.
///
/// Layer 0: Default QWERTY
/// Layer 1: Function/Symbol layer
pub static LAYERS: [[[Keycode; COLS]; ROWS]; NUM_LAYERS] = [
    // Layer 0: QWERTY
    [
        // Row 0: number row
        //  Left: =, 1, 2, 3, 4, 5, Esc       Right: -, 6, 7, 8, 9, 0, _unused
        [Keycode::Equal, Keycode::N1, Keycode::N2, Keycode::N3, Keycode::N4, Keycode::N5, ESC,
         Keycode::Minus, Keycode::N6, Keycode::N7, Keycode::N8, Keycode::N9, Keycode::N0, ___],

        // Row 1: top letter row
        //  Left: Tab, Q, W, E, R, T, [         Right: ], Y, U, I, O, P, \
        [TAB, Keycode::Q, Keycode::W, Keycode::E, Keycode::R, Keycode::T, Keycode::LBracket,
         Keycode::RBracket, Keycode::Y, Keycode::U, Keycode::I, Keycode::O, Keycode::P, Keycode::Backslash],

        // Row 2: home row
        //  Left: LCtrl, A, S, D, F, G, _unused  Right: _unused, H, J, K, L, ;, '
        [LCTL, Keycode::A, Keycode::S, Keycode::D, Keycode::F, Keycode::G, ___,
         ___, Keycode::H, Keycode::J, Keycode::K, Keycode::L, Keycode::Semicolon, Keycode::Quote],

        // Row 3: bottom row
        //  Left: <>, Z, X, C, V, B, LY1    Right: LY1, N, M, ,, ., /, RShift
        [NUBS, Keycode::Z, Keycode::X, Keycode::C, Keycode::V, Keycode::B, LY1,
         LY1, Keycode::N, Keycode::M, Keycode::Comma, Keycode::Dot, Keycode::Slash, RSFT],

        // Row 4: thumb cluster top
        //  Left: `, LAlt, LGui, _, _unused, _unused, _unused
        //  Right: _unused, _unused, _unused, _, RAlt, _, _unused
        [Keycode::Grave, LALT, LGUI, ___, ___, ___, ___,
         ___, ___, ___, ___, RALT, ___, ___],

        // Row 5: thumb cluster bottom
        //  Left: _unused, _unused, Space, Enter, _unused, _unused, _unused
        //  Right: _unused, _unused, _unused, RShift, Bksp, _unused, _unused
        [___, ___, ENT, SPC, ___, ___, ___,
         ___, ___, ___, RSFT, BSP, ___, ___],
    ],

    // Layer 1: Function/Symbol
    [
        // Row 0
        [___, Keycode::F1, Keycode::F2, Keycode::F3, Keycode::F4, Keycode::F5, ___,
         ___, Keycode::F6, Keycode::F7, Keycode::F8, Keycode::F9, Keycode::F10, ___],

        // Row 1
        [___, ___, ___, ___, ___, ___, Keycode::F11,
         Keycode::F12, ___, ___, ___, ___, ___, ___],

        // Row 2
        [___, ___, ___, ___, ___, ___, ___,
         ___, Keycode::Left, Keycode::Down, Keycode::Up, Keycode::Right, ___, ___],

        // Row 3
        [___, ___, ___, ___, ___, ___, ___,
         ___, ___, ___, ___, ___, ___, ___],

        // Row 4
        [___, ___, ___, ___, ___, ___, ___,
         ___, ___, ___, ___, ___, ___, ___],

        // Row 5
        [___, ___, ___, ___, ___, ___, ___,
         ___, ___, ___, ___, ___, ___, ___],
    ],
];

/// Resolve which layer is active based on currently pressed keys.
/// Layer keys are momentary: holding the key activates the layer.
pub fn resolve_layer(keys: &[[bool; COLS]; ROWS]) -> usize {
    // Check all keys for layer holds, highest layer wins
    let mut active_layer = 0usize;

    for row in 0..ROWS {
        for col in 0..COLS {
            if keys[row][col] {
                let kc = LAYERS[0][row][col]; // Layer keys are always on layer 0
                if kc.is_layer() {
                    let layer = kc.layer_number();
                    if layer > active_layer && layer < NUM_LAYERS {
                        active_layer = layer;
                    }
                }
            }
        }
    }

    active_layer
}

/// Look up the keycode for a matrix position, resolving transparent keys
/// through the layer stack.
pub fn lookup(layer: usize, row: usize, col: usize) -> Keycode {
    // Start at the active layer and fall through on Trans
    let mut l = layer;
    loop {
        let kc = LAYERS[l][row][col];
        if !kc.is_transparent() || l == 0 {
            return kc;
        }
        l -= 1;
    }
}
