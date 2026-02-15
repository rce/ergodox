//! Generate an HTML/SVG visualization of the ErgoDox keymap.
//! Each key is a purr-fectly positioned rectangle with its label. :3

use ergodox_keymap::{Keycode, LAYERS, NUM_LAYERS};

/// Physical key position and size for SVG rendering.
struct Key {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    row: usize,
    col: usize,
}

/// Key unit size in SVG pixels.
const U: f64 = 54.0;
/// Gap between keys.
const GAP: f64 = 4.0;
/// Step: key + gap.
const S: f64 = U + GAP;
/// Key corner radius.
const R: f64 = 4.0;
/// Spacing between left and right halves.
const HALF_GAP: f64 = 60.0;
/// Margin around the SVG content.
const MARGIN: f64 = 20.0;

/// Column stagger for the left half (y offset in units of S).
/// Index 0 = outermost (pinky extra), index 6 = innermost.
const STAGGER: [f64; 7] = [0.50, 0.25, 0.00, -0.15, 0.10, 0.40, 0.65];

/// Build all physical key positions for both halves.
fn build_keys() -> Vec<Key> {
    let mut keys = Vec::new();

    // Left half at origin
    build_half(&mut keys, true, 0.0, 0.0);

    // Right half offset to the right
    let right_x = 7.0 * S + HALF_GAP;
    build_half(&mut keys, false, right_x, 0.0);

    keys
}

/// Build key positions for one half of the ErgoDox.
///
/// Left half: local col 0 = outer (pinky), local col 6 = inner.
/// Right half: local col 0 = inner, local col 6 = outer (mirrored).
fn build_half(keys: &mut Vec<Key>, is_left: bool, bx: f64, by: f64) {
    let col_offset: usize = if is_left { 0 } else { 7 };

    // Stagger: left uses as-is, right reverses (inner col is on the left side)
    let stagger: [f64; 7] = if is_left {
        STAGGER
    } else {
        let mut s = STAGGER;
        s.reverse();
        s
    };

    // Which local column is the inner extra column (1.5u tall keys, no row 2)?
    let inner_lc: usize = if is_left { 6 } else { 0 };

    // --- Main section: rows 0-3, all columns except inner ---
    for lc in 0..7 {
        if lc == inner_lc {
            continue;
        }
        for row in 0..4 {
            keys.push(Key {
                x: bx + lc as f64 * S,
                y: by + (row as f64 + stagger[lc]) * S,
                w: U,
                h: U,
                row,
                col: col_offset + lc,
            });
        }
    }

    // --- Inner column: rows 0 (1u), 1 (1.5u), 3 (1.5u) ---
    // Align top with the adjacent column so it looks natural.
    let inner_x = bx + inner_lc as f64 * S;
    let adj_lc = if is_left { 5 } else { 1 };
    let inner_top = stagger[adj_lc]; // start at same y as adjacent column
    let h15u = 1.5 * U + 0.5 * GAP; // 1.5u key height

    // Row 0: 1u
    keys.push(Key {
        x: inner_x,
        y: by + inner_top * S,
        w: U,
        h: U,
        row: 0,
        col: col_offset + inner_lc,
    });
    // Row 1: 1.5u tall
    keys.push(Key {
        x: inner_x,
        y: by + (inner_top + 1.0) * S,
        w: U,
        h: h15u,
        row: 1,
        col: col_offset + inner_lc,
    });
    // Row 3: 1.5u tall
    keys.push(Key {
        x: inner_x,
        y: by + (inner_top + 2.5) * S,
        w: U,
        h: h15u,
        row: 3,
        col: col_offset + inner_lc,
    });

    // --- Bottom row: row 4, 5 keys ---
    // Left: local cols 0-4 (matrix 0-4), Right: local cols 2-6 (matrix 9-13)
    let bottom_start: usize = if is_left { 0 } else { 2 };
    let bottom_end: usize = bottom_start + 5;
    for lc in bottom_start..bottom_end {
        keys.push(Key {
            x: bx + lc as f64 * S,
            y: by + (4.0 + stagger[lc]) * S,
            w: U,
            h: U,
            row: 4,
            col: col_offset + lc,
        });
    }

    // --- Thumb cluster: row 5, 6 keys ---
    build_thumb(keys, is_left, bx, by);
}

/// Build the 6-key thumb cluster for one half.
///
/// Arrangement (left half, from left to right):
/// ```text
///                  [s_top] [s1]
/// [tall1        ] [tall2 ] [s2]
/// [             ] [      ] [s3]
/// ```
/// - Column A: one 2u tall key
/// - Column B: one 1u small key on top, one 2u tall key below
/// - Column C: three 1u keys stacked
///
/// Right half is mirrored.
fn build_thumb(keys: &mut Vec<Key>, is_left: bool, bx: f64, by: f64) {
    let ty = by + 5.5 * S;
    let h2u = 2.0 * U + GAP; // height of a 2u key

    // (matrix_col, x, y, h)
    let positions: [(usize, f64, f64, f64); 6] = if is_left {
        // Left thumb cluster: tall keys on left, stacked smalls on right
        let tx = bx + 4.0 * S;
        [
            (3, tx,           ty + S,       h2u), // col A: tall1 (2u)
            (5, tx + S,       ty,           U),   // col B top: small above tall2
            (2, tx + S,       ty + S,       h2u), // col B bot: tall2 (2u)
            (4, tx + 2.0 * S, ty,           U),   // col C: small 1 (top)
            (1, tx + 2.0 * S, ty + S,       U),   // col C: small 2 (mid)
            (0, tx + 2.0 * S, ty + 2.0 * S, U),  // col C: small 3 (bot)
        ]
    } else {
        // Right thumb cluster: mirrored — stacked smalls on left, tall keys on right
        let tx = bx + GAP;
        [
            (9,  tx,           ty,           U),   // col C: small 1 (top)
            (12, tx,           ty + S,       U),   // col C: small 2 (mid)
            (13, tx,           ty + 2.0 * S, U),   // col C: small 3 (bot)
            (8,  tx + S,       ty,           U),   // col B top: small above tall2
            (11, tx + S,       ty + S,       h2u), // col B bot: tall2 (2u)
            (10, tx + 2.0 * S, ty + S,       h2u), // col A: tall1 (2u)
        ]
    };

    for (col, x, y, h) in positions {
        keys.push(Key { x, y, w: U, h, row: 5, col });
    }
}

/// Compute the bounding box of all keys: (max_x + w, max_y + h).
fn bbox(keys: &[Key]) -> (f64, f64) {
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for k in keys {
        max_x = max_x.max(k.x + k.w);
        max_y = max_y.max(k.y + k.h);
    }
    (max_x, max_y)
}

/// Render a single layer as an SVG group.
fn render_layer(keys: &[Key], layer_idx: usize, y_offset: f64) -> String {
    let mut svg = String::new();

    svg.push_str(&format!(
        r#"<g transform="translate({MARGIN}, {y_offset})">"#
    ));

    // Layer title
    svg.push_str(&format!(
        r#"<text x="0" y="-10" class="layer-title">Layer {layer_idx}{}</text>"#,
        if layer_idx == 0 {
            " (Default)"
        } else {
            " (Fn)"
        }
    ));

    for key in keys {
        let kc = LAYERS[layer_idx][key.row][key.col];

        // For non-base layers, show the resolved key (fall-through)
        let display_kc = if layer_idx > 0 && kc.is_transparent() {
            ergodox_keymap::lookup(layer_idx, key.row, key.col)
        } else {
            kc
        };

        let label = display_kc.display_name();
        let is_transparent = layer_idx > 0 && kc.is_transparent();

        let key_class = if kc == Keycode::Trans && layer_idx == 0 {
            "key unused"
        } else if is_transparent {
            "key transparent"
        } else if kc.is_layer() {
            "key layer"
        } else if kc.is_modifier() {
            "key modifier"
        } else {
            "key"
        };

        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{R}" class="{key_class}"/>"#,
            key.x, key.y, key.w, key.h,
        ));

        if !label.is_empty() {
            let font_class = if label.len() > 3 { " small" } else { "" };
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="label{font_class}">{}</text>"#,
                key.x + key.w / 2.0,
                key.y + key.h / 2.0 + 1.0,
                html_escape(label),
            ));
        }
    }

    svg.push_str("</g>");
    svg
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Generate the complete HTML document with inline SVG.
pub fn generate_html() -> String {
    let keys = build_keys();
    let (content_w, content_h) = bbox(&keys);
    let layer_height = content_h + 60.0;
    let total_width = content_w + 2.0 * MARGIN;
    let total_height = NUM_LAYERS as f64 * layer_height + 2.0 * MARGIN;

    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>ErgoDox Layout</title>
<style>
  body {{
    background: #1a1a2e;
    color: #eee;
    font-family: system-ui, -apple-system, sans-serif;
    display: flex;
    justify-content: center;
    padding: 2em;
  }}
  svg {{
    filter: drop-shadow(0 2px 8px rgba(0,0,0,0.3));
  }}
  .key {{
    fill: #16213e;
    stroke: #0f3460;
    stroke-width: 1.5;
  }}
  .key:hover {{
    fill: #1a1a5e;
    stroke: #e94560;
  }}
  .key.unused {{
    fill: #0d1117;
    stroke: #21262d;
    stroke-dasharray: 3 3;
  }}
  .key.transparent {{
    fill: #1a1a2e;
    stroke: #30365e;
    stroke-dasharray: 2 2;
  }}
  .key.layer {{
    fill: #2d1b4e;
    stroke: #e94560;
    stroke-width: 2;
  }}
  .key.modifier {{
    fill: #1b2e4e;
    stroke: #53a8b6;
    stroke-width: 1.5;
  }}
  .label {{
    fill: #eee;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 13px;
    text-anchor: middle;
    dominant-baseline: middle;
    pointer-events: none;
  }}
  .label.small {{
    font-size: 10px;
  }}
  .layer-title {{
    fill: #e94560;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 16px;
    font-weight: bold;
  }}
</style>
</head>
<body>
<svg width="{total_width}" height="{total_height}" xmlns="http://www.w3.org/2000/svg">
"#
    );

    for layer_idx in 0..NUM_LAYERS {
        let y_offset = MARGIN + layer_idx as f64 * layer_height + 30.0;
        html.push_str(&render_layer(&keys, layer_idx, y_offset));
        html.push('\n');
    }

    html.push_str("</svg>\n</body>\n</html>\n");
    html
}
