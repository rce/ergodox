//! Shared keymap definitions and layer management for the ErgoDox.
//!
//! This crate is `no_std`-compatible so it can be used by both the AVR
//! firmware and the native CLI tool. Meow!

#![no_std]
#![allow(dead_code)]

/// Number of rows in the matrix.
pub const ROWS: usize = 6;
/// Number of columns per half.
pub const COLS_PER_HALF: usize = 7;
/// Total number of columns.
pub const COLS: usize = COLS_PER_HALF * 2;

/// Maps Nordic ISO key labels to their HID keycodes.
///
/// HID keycodes are layout-agnostic — the OS interprets them based on the
/// active input language. These aliases let you write keymaps using the
/// labels printed on a Nordic keyboard instead of the US-centric HID names.
pub mod layout {
    pub mod nordic {
        use super::super::Keycode;

        /// `+` (unshifted) / `?` (shifted) — key right of 0
        pub const PLUS_QUESTION: Keycode = Keycode::Minus;
        /// `´` (unshifted) / `` ` `` (shifted) — key right of +
        pub const ACUTE_GRAVE: Keycode = Keycode::Equal;
        /// `å`
        pub const A_RING: Keycode = Keycode::LBracket;
        /// `¨` (unshifted) / `^` (shifted)
        pub const DIAERESIS_CARET: Keycode = Keycode::RBracket;
        /// `'` (unshifted) / `*` (shifted)
        pub const APOSTROPHE_STAR: Keycode = Keycode::Backslash;
        /// `ö`
        pub const O_DIAERESIS: Keycode = Keycode::Semicolon;
        /// `ä`
        pub const A_DIAERESIS: Keycode = Keycode::Quote;
        /// `§` (unshifted) / `½` (shifted) — top-left key
        pub const SECTION_HALF: Keycode = Keycode::Grave;
        /// `<` (unshifted) / `>` (shifted) — ISO key left of Z
        pub const ANGLE_BRACKETS: Keycode = Keycode::NonUsBackslash;
        /// `-` (unshifted) / `_` (shifted) — key right of `.`
        pub const MINUS_UNDERSCORE: Keycode = Keycode::Slash;
    }
}

/// USB HID keycodes.
/// See USB HID Usage Tables, Section 10 (Keyboard/Keypad Page 0x07).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    /// Display name for use in layout visualizations.
    pub fn display_name(self) -> &'static str {
        match self {
            Keycode::Trans => "",
            Keycode::None => "ERR",
            Keycode::A => "A",
            Keycode::B => "B",
            Keycode::C => "C",
            Keycode::D => "D",
            Keycode::E => "E",
            Keycode::F => "F",
            Keycode::G => "G",
            Keycode::H => "H",
            Keycode::I => "I",
            Keycode::J => "J",
            Keycode::K => "K",
            Keycode::L => "L",
            Keycode::M => "M",
            Keycode::N => "N",
            Keycode::O => "O",
            Keycode::P => "P",
            Keycode::Q => "Q",
            Keycode::R => "R",
            Keycode::S => "S",
            Keycode::T => "T",
            Keycode::U => "U",
            Keycode::V => "V",
            Keycode::W => "W",
            Keycode::X => "X",
            Keycode::Y => "Y",
            Keycode::Z => "Z",
            Keycode::N1 => "1",
            Keycode::N2 => "2",
            Keycode::N3 => "3",
            Keycode::N4 => "4",
            Keycode::N5 => "5",
            Keycode::N6 => "6",
            Keycode::N7 => "7",
            Keycode::N8 => "8",
            Keycode::N9 => "9",
            Keycode::N0 => "0",
            Keycode::Enter => "Ent",
            Keycode::Escape => "Esc",
            Keycode::Backspace => "Bksp",
            Keycode::Tab => "Tab",
            Keycode::Space => "Spc",
            Keycode::Minus => "+?",
            Keycode::Equal => "\u{b4}`",
            Keycode::LBracket => "\u{e5}",
            Keycode::RBracket => "\u{a8}^",
            Keycode::Backslash => "'*",
            Keycode::Semicolon => "\u{f6}",
            Keycode::Quote => "\u{e4}",
            Keycode::Grave => "\u{a7}\u{bd}",
            Keycode::Comma => ",",
            Keycode::Dot => ".",
            Keycode::Slash => "-_",
            Keycode::CapsLock => "Caps",
            Keycode::NonUsBackslash => "<>",
            Keycode::F1 => "F1",
            Keycode::F2 => "F2",
            Keycode::F3 => "F3",
            Keycode::F4 => "F4",
            Keycode::F5 => "F5",
            Keycode::F6 => "F6",
            Keycode::F7 => "F7",
            Keycode::F8 => "F8",
            Keycode::F9 => "F9",
            Keycode::F10 => "F10",
            Keycode::F11 => "F11",
            Keycode::F12 => "F12",
            Keycode::PrintScreen => "PScr",
            Keycode::ScrollLock => "ScrL",
            Keycode::Pause => "Paus",
            Keycode::Insert => "Ins",
            Keycode::Home => "Home",
            Keycode::PageUp => "PgUp",
            Keycode::Delete => "Del",
            Keycode::End => "End",
            Keycode::PageDown => "PgDn",
            Keycode::Right => "\u{2192}",
            Keycode::Left => "\u{2190}",
            Keycode::Down => "\u{2193}",
            Keycode::Up => "\u{2191}",
            Keycode::LCtrl => "Ctrl",
            Keycode::LShift => "Shft",
            Keycode::LAlt => "Alt",
            Keycode::LGui => "Gui",
            Keycode::RCtrl => "RCtl",
            Keycode::RShift => "RSft",
            Keycode::RAlt => "RAlt",
            Keycode::RGui => "RGui",
            Keycode::Layer1 => "Ly1",
        }
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
const PGUP: Keycode = Keycode::PageUp;
const PGDN: Keycode = Keycode::PageDown;
const LY1: Keycode = Keycode::Layer1;

// Nordic layout shorthand aliases
use layout::nordic as Nordic;
const PLSQ: Keycode = Nordic::PLUS_QUESTION;
const ACGR: Keycode = Nordic::ACUTE_GRAVE;
const ARING: Keycode = Nordic::A_RING;
const DIAC: Keycode = Nordic::DIAERESIS_CARET;
const APST: Keycode = Nordic::APOSTROPHE_STAR;
const ODIA: Keycode = Nordic::O_DIAERESIS;
const ADIA: Keycode = Nordic::A_DIAERESIS;
const SECT: Keycode = Nordic::SECTION_HALF;
const ANGB: Keycode = Nordic::ANGLE_BRACKETS;
const MINU: Keycode = Nordic::MINUS_UNDERSCORE;

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
        //  Left: §½, 1, 2, 3, 4, 5, ___       Right: +?, 6, 7, 8, 9, 0, +?
        [
            SECT,
            Keycode::N1,
            Keycode::N2,
            Keycode::N3,
            Keycode::N4,
            Keycode::N5,
            ___,
            ___,
            Keycode::N6,
            Keycode::N7,
            Keycode::N8,
            Keycode::N9,
            Keycode::N0,
            PLSQ,
        ],
        // Row 1: top letter row
        //  Left: Tab, Q, W, E, R, T, PgUp      Right: ¨^, Y, U, I, O, P, '*
        [
            TAB,
            Keycode::Q,
            Keycode::W,
            Keycode::E,
            Keycode::R,
            Keycode::T,
            PGUP,
            ___,
            Keycode::Y,
            Keycode::U,
            Keycode::I,
            Keycode::O,
            Keycode::P,
            ___,
        ],
        // Row 2: home row
        //  Left: LCtrl, A, S, D, F, G, LY1     Right: _unused, H, J, K, L, ö, ä
        [
            LCTL,
            Keycode::A,
            Keycode::S,
            Keycode::D,
            Keycode::F,
            Keycode::G,
            LY1, // ???
            ___, // ???
            Keycode::H,
            Keycode::J,
            Keycode::K,
            Keycode::L,
            ODIA,
            ADIA,
        ],
        // Row 3: bottom row
        //  Left: <>, Z, X, C, V, B, PgDn   Right: ___, N, M, ,, ., -_, '*
        [
            ANGB,
            Keycode::Z,
            Keycode::X,
            Keycode::C,
            Keycode::V,
            Keycode::B,
            PGDN,
            ___,
            Keycode::N,
            Keycode::M,
            Keycode::Comma,
            Keycode::Dot,
            MINU,
            APST,
        ],
        // Row 4: thumb cluster top
        //  Left: LY1, LAlt, LGui, LAlt, LGui, _unused, _unused
        //  Right: _unused, _unused, Left, Down, Up, Right, LY1
        [
            LY1,
            ___,
            ___,
            LALT,
            LGUI, // Cmd/Win
            ___, // ??
            ___, // ??
            ___, // ??
            ___, // ??
            Keycode::Left,
            Keycode::Down,
            Keycode::Up,
            Keycode::Right,
            ___,
        ],
        // Row 5: thumb cluster bottom
        //  Left: Esc, _unused, Space, Enter, Home, End, _unused
        //  Right: _unused, _unused, _unused, RShift, Bksp, _unused, _unused
        [
            Keycode::A,
            ESC, // Esc
            ENT, // Enter
            SPC, // Space
            ___, // Endin alla
            Keycode::Home, // Home
            Keycode::End, // End
            ___, // oikeen puolen 'home'
            DEL, // oikeen puolen 'end'
            ___, // ylempi pieni
            RSFT, // Shift
            BSP, // Backspace
            ___, // alempi pieni
            Keycode::F,
        ],
    ],
    // Layer 1: Function/Symbol
    [
        // Row 0
        [
            ___,
            Keycode::F1,
            Keycode::F2,
            Keycode::F3,
            Keycode::F4,
            Keycode::F5,
            ___,
            ___,
            Keycode::F6,
            Keycode::F7,
            Keycode::F8,
            Keycode::F9,
            Keycode::F10,
            ___,
        ],
        // Row 1
        [
            ___,
            ___,
            ___,
            ___,
            ___,
            ___,
            Keycode::F11,
            Keycode::F12,
            ___,
            ___,
            ___,
            ___,
            ___,
            ___,
        ],
        // Row 2
        [
            ___,
            ___,
            ___,
            ___,
            ___,
            ___,
            ___,
            ___,
            Keycode::Left,
            Keycode::Down,
            Keycode::Up,
            Keycode::Right,
            ___,
            ___,
        ],
        // Row 3
        [
            ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___,
        ],
        // Row 4
        [
            ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___,
        ],
        // Row 5
        [
            ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___, ___,
        ],
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

// =============================================================================
// Tests — literate contracts for the ErgoDox keymap
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Matrix dimensions
    // =========================================================================
    //
    // The ErgoDox has a 6×14 key matrix split across two halves connected by
    // a TRRS cable. Each half contributes 7 columns: left (cols 0–6) and
    // right (cols 7–13). These constants must match the physical PCB wiring
    // — if they drift, the firmware will scan the wrong pins.

    #[test]
    fn matrix_is_six_rows() {
        // The PCB has 6 row traces (rows 0–5). Row 5 is the thumb cluster.
        assert_eq!(ROWS, 6);
    }

    #[test]
    fn matrix_is_fourteen_columns() {
        // 7 columns per half × 2 halves = 14 total columns.
        assert_eq!(COLS, 14);
        assert_eq!(COLS_PER_HALF, 7);
        assert_eq!(COLS, COLS_PER_HALF * 2);
    }

    #[test]
    fn layer_table_matches_matrix_dimensions() {
        // Every layer must be exactly ROWS × COLS. A mismatch would cause
        // out-of-bounds access during matrix scanning.
        assert_eq!(LAYERS.len(), NUM_LAYERS);
        for (i, layer) in LAYERS.iter().enumerate() {
            assert_eq!(layer.len(), ROWS, "layer {i} row count");
            for (r, row) in layer.iter().enumerate() {
                assert_eq!(row.len(), COLS, "layer {i} row {r} col count");
            }
        }
    }

    // =========================================================================
    // Modifier encoding — USB HID modifier byte
    // =========================================================================
    //
    // USB HID boot-protocol keyboards report modifiers in a single byte
    // (byte 0 of the 8-byte report). Each modifier occupies one bit:
    //
    //   bit 0 = Left Ctrl   (0xE0)
    //   bit 1 = Left Shift  (0xE1)
    //   bit 2 = Left Alt    (0xE2)
    //   bit 3 = Left GUI    (0xE3)
    //   bit 4 = Right Ctrl  (0xE4)
    //   bit 5 = Right Shift (0xE5)
    //   bit 6 = Right Alt   (0xE6)
    //   bit 7 = Right GUI   (0xE7)
    //
    // The modifier_bit() method converts a keycode in 0xE0–0xE7 to the
    // corresponding bitmask by computing 1 << (keycode - 0xE0).

    #[test]
    fn modifiers_span_0xe0_through_0xe7() {
        // The USB HID spec (Usage Tables §10) assigns keycodes 0xE0–0xE7
        // to the eight modifier keys. All eight must be recognized.
        let mods = [
            Keycode::LCtrl,
            Keycode::LShift,
            Keycode::LAlt,
            Keycode::LGui,
            Keycode::RCtrl,
            Keycode::RShift,
            Keycode::RAlt,
            Keycode::RGui,
        ];
        for (i, &kc) in mods.iter().enumerate() {
            assert!(kc.is_modifier(), "0x{:02X} should be a modifier", kc as u8);
            assert_eq!(kc as u8, 0xE0 + i as u8);
        }
    }

    #[test]
    fn modifier_bit_maps_to_correct_position() {
        // Each modifier must map to exactly one bit. LCtrl = bit 0 (0x01),
        // RGui = bit 7 (0x80). The firmware ORs these together to build
        // the modifier byte in the HID report.
        assert_eq!(Keycode::LCtrl.modifier_bit(), 0x01); // bit 0
        assert_eq!(Keycode::LShift.modifier_bit(), 0x02); // bit 1
        assert_eq!(Keycode::LAlt.modifier_bit(), 0x04); // bit 2
        assert_eq!(Keycode::LGui.modifier_bit(), 0x08); // bit 3
        assert_eq!(Keycode::RCtrl.modifier_bit(), 0x10); // bit 4
        assert_eq!(Keycode::RShift.modifier_bit(), 0x20); // bit 5
        assert_eq!(Keycode::RAlt.modifier_bit(), 0x40); // bit 6
        assert_eq!(Keycode::RGui.modifier_bit(), 0x80); // bit 7
    }

    #[test]
    fn non_modifier_has_zero_bit() {
        // Regular keys must return 0 — they go in the keycode array, not
        // the modifier byte. A nonzero result here would cause phantom
        // modifier presses.
        assert_eq!(Keycode::A.modifier_bit(), 0);
        assert_eq!(Keycode::Space.modifier_bit(), 0);
        assert_eq!(Keycode::Layer1.modifier_bit(), 0);
    }

    // =========================================================================
    // Layer key encoding
    // =========================================================================
    //
    // Layer keys use keycodes 0xF0+N (a range well above real HID keycodes).
    // The firmware interprets these during matrix scanning: when a layer key
    // is held, it activates layer N. These are momentary — releasing the key
    // drops back to layer 0.
    //
    // Trans (0x00) is the "transparent" sentinel. In HID, 0x00 means
    // "no event" — the host ignores it. We reuse it to mean "look at the
    // layer below" during keycode resolution.

    #[test]
    fn layer1_encodes_as_0xf1() {
        // Layer keys are 0xF0 + layer number. Layer1 = 0xF1.
        assert_eq!(Keycode::Layer1 as u8, 0xF1);
        assert!(Keycode::Layer1.is_layer());
        assert_eq!(Keycode::Layer1.layer_number(), 1);
    }

    #[test]
    fn trans_is_zero_and_transparent() {
        // 0x00 = "no event" in HID. We use it as "fall through to lower layer."
        // This works because the host already ignores 0x00 in key reports,
        // so if it somehow leaks through, no spurious keypress occurs.
        assert_eq!(Keycode::Trans as u8, 0x00);
        assert!(Keycode::Trans.is_transparent());
    }

    #[test]
    fn trans_is_not_a_modifier_or_layer() {
        // Trans must not be mistaken for a modifier or layer key — it's
        // the absence of a binding, not an action.
        assert!(!Keycode::Trans.is_modifier());
        assert!(!Keycode::Trans.is_layer());
    }

    // =========================================================================
    // Layer resolution
    // =========================================================================
    //
    // resolve_layer() scans the pressed-key matrix and returns the highest
    // active layer. Layer keys are always read from layer 0 (so you can't
    // accidentally remap your layer keys on a higher layer).
    //
    // lookup() resolves a keycode at a position: if the active layer has
    // Trans, it falls through to layer 0. This is the "transparent" concept
    // — higher layers only override keys they explicitly define.

    #[test]
    fn no_layer_keys_pressed_gives_layer_zero() {
        // With nothing pressed, the active layer is 0.
        let keys = [[false; COLS]; ROWS];
        assert_eq!(resolve_layer(&keys), 0);
    }

    #[test]
    fn pressing_layer1_key_activates_layer_one() {
        // Layer1 keys exist at several positions on layer 0 (e.g., row 2 col 6).
        // Holding any of them should activate layer 1.
        let mut keys = [[false; COLS]; ROWS];

        // Find a Layer1 key position on layer 0
        let (ly_row, ly_col) = find_layer_key_position();
        keys[ly_row][ly_col] = true;

        assert_eq!(resolve_layer(&keys), 1);
    }

    #[test]
    fn lookup_returns_layer0_key_on_base_layer() {
        // On layer 0, lookup returns exactly what's in the LAYERS table.
        // Row 1, col 1 = Q on the default QWERTY layout.
        assert_eq!(lookup(0, 1, 1), Keycode::Q);
    }

    #[test]
    fn lookup_falls_through_transparent_keys() {
        // On layer 1, most keys are Trans (0x00). lookup() should fall
        // through to layer 0 and return the base-layer binding.
        //
        // Row 1, col 1 = Trans on layer 1, Q on layer 0 → returns Q.
        assert_eq!(LAYERS[1][1][1], Keycode::Trans);
        assert_eq!(lookup(1, 1, 1), Keycode::Q);
    }

    #[test]
    fn lookup_returns_override_when_not_transparent() {
        // Layer 1 overrides some keys — e.g., row 0 col 1 is F1.
        // lookup() should return the override, not the base-layer key.
        assert_eq!(LAYERS[1][0][1], Keycode::F1);
        assert_eq!(lookup(1, 0, 1), Keycode::F1);
    }

    // =========================================================================
    // Nordic aliases — layout-agnostic keycodes
    // =========================================================================
    //
    // HID keycodes are layout-agnostic: they describe a physical key position,
    // not the character it produces. The character depends on the OS keyboard
    // layout. A Nordic keyboard has different legends than a US one, but the
    // HID keycodes are the same physical keys.
    //
    // These aliases let us write the keymap using Nordic labels (å, ö, ä, etc.)
    // while emitting the correct US-centric HID keycodes. The OS, set to a
    // Nordic layout, translates them to the right characters.

    #[test]
    fn nordic_aliases_map_to_us_keycodes() {
        use layout::nordic::*;

        // Each Nordic key occupies the same physical position as a US key.
        // The alias documents what the Nordic legend says; the value is the
        // US keycode at that physical position.
        assert_eq!(PLUS_QUESTION, Keycode::Minus, "+? is US Minus");
        assert_eq!(ACUTE_GRAVE, Keycode::Equal, "´` is US Equal");
        assert_eq!(A_RING, Keycode::LBracket, "å is US [");
        assert_eq!(DIAERESIS_CARET, Keycode::RBracket, "¨^ is US ]");
        assert_eq!(APOSTROPHE_STAR, Keycode::Backslash, "'* is US \\");
        assert_eq!(O_DIAERESIS, Keycode::Semicolon, "ö is US ;");
        assert_eq!(A_DIAERESIS, Keycode::Quote, "ä is US '");
        assert_eq!(SECTION_HALF, Keycode::Grave, "§½ is US `");
        assert_eq!(
            ANGLE_BRACKETS,
            Keycode::NonUsBackslash,
            "<> is ISO extra key"
        );
        assert_eq!(MINUS_UNDERSCORE, Keycode::Slash, "-_ is US /");
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Find any Layer1 key position on layer 0.
    fn find_layer_key_position() -> (usize, usize) {
        for row in 0..ROWS {
            for col in 0..COLS {
                if LAYERS[0][row][col] == Keycode::Layer1 {
                    return (row, col);
                }
            }
        }
        panic!("no Layer1 key found on layer 0");
    }
}
