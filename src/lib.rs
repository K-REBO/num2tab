pub mod chord;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// ==================== データ型 ====================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FretPos {
    Open,
    Muted,
    Fret(u8),
}

pub fn parse_fret_string(s: &str) -> Option<Vec<FretPos>> {
    if s.len() != 6 {
        return None;
    }
    s.chars()
        .map(|c| match c {
            'x' | 'X' => Some(FretPos::Muted),
            '0' => Some(FretPos::Open),
            '1'..='9' => Some(FretPos::Fret(c as u8 - b'0')),
            _ => None,
        })
        .collect()
}

// ==================== 音名計算 ====================

pub const STRING_OPEN: [u8; 6] = [4, 9, 2, 7, 11, 4];
pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub fn string_note(string_idx: usize, fret: u8, fret_offset: u32) -> &'static str {
    let semitone = (STRING_OPEN[string_idx] as u32 + fret as u32 + fret_offset) % 12;
    NOTE_NAMES[semitone as usize]
}

pub fn fret_count(frets: &[FretPos]) -> u32 {
    frets.iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(0)
        .max(3)
}

pub fn invert_string(s: usize, string_count: u32) -> u32 {
    string_count - 1 - s as u32
}

// ==================== レイアウトパラメータ ====================

pub struct LayoutParams {
    pub string_spacing: u32,
    pub fret_spacing: u32,
    pub left_margin: u32,
    pub top_margin: u32,
    pub bottom_margin: u32,
    pub right_margin: u32,
}

impl LayoutParams {
    pub fn horizontal() -> Self {
        Self {
            string_spacing: 20,
            fret_spacing: 36,
            left_margin: 28,
            top_margin: 14,
            bottom_margin: 26,
            right_margin: 14,
        }
    }

    pub fn vertical() -> Self {
        Self {
            string_spacing: 20,
            fret_spacing: 30,
            left_margin: 38,
            top_margin: 28,
            bottom_margin: 10,
            right_margin: 10,
        }
    }
}

// ==================== Canvas トレイト ====================

pub trait Canvas {
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: u32);
    fn draw_filled_circle(&mut self, cx: i32, cy: i32, r: i32);
    fn draw_open_circle(&mut self, cx: i32, cy: i32, r: i32);
    fn draw_cross(&mut self, cx: i32, cy: i32, r: i32);
    fn draw_note_label(&mut self, cx: i32, cy: i32, note: &str);
    fn draw_number(&mut self, n: u32, x: i32, y: i32, center_align: bool);
}

// ==================== SvgCanvas ====================

pub struct SvgCanvas {
    body: String,
    width: u32,
    height: u32,
}

impl SvgCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self { body: String::new(), width, height }
    }

    pub fn into_svg(self) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" fill="white"/>
{body}</svg>"#,
            w = self.width,
            h = self.height,
            body = self.body,
        )
    }
}

impl Canvas for SvgCanvas {
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: u32) {
        self.body += &format!(
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="black" stroke-width="{width}"/>
"#
        );
    }

    fn draw_filled_circle(&mut self, cx: i32, cy: i32, r: i32) {
        self.body += &format!(r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="black"/>
"#);
    }

    fn draw_open_circle(&mut self, cx: i32, cy: i32, r: i32) {
        self.body += &format!(
            r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="black" stroke-width="2"/>
"#
        );
    }

    fn draw_cross(&mut self, cx: i32, cy: i32, r: i32) {
        let d = (r as f32 * 0.7) as i32;
        self.body += &format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
"#,
            cx - d, cy - d, cx + d, cy + d,
            cx + d, cy - d, cx - d, cy + d,
        );
    }

    fn draw_note_label(&mut self, cx: i32, cy: i32, note: &str) {
        self.body += &format!(
            r#"<circle cx="{cx}" cy="{cy}" r="8" fill="black"/>
<text x="{cx}" y="{cy}" text-anchor="middle" dominant-baseline="middle" font-size="8" font-family="monospace" font-weight="bold" fill="white">{note}</text>
"#
        );
    }

    fn draw_number(&mut self, n: u32, x: i32, y: i32, center_align: bool) {
        let anchor = if center_align { "middle" } else { "end" };
        self.body += &format!(
            r#"<text x="{x}" y="{y}" text-anchor="{anchor}" dominant-baseline="middle" font-size="10" font-family="monospace">{n}</text>
"#
        );
    }
}

// ==================== 描画ロジック ====================

pub fn render_horizontal<C: Canvas>(
    canvas: &mut C,
    frets: &[FretPos],
    lp: &LayoutParams,
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
) {
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let grid_height = (sc - 1) * lp.string_spacing;
    let grid_width = fc * lp.fret_spacing;
    let x_left = lp.left_margin as f32;
    let x_right = (lp.left_margin + grid_width) as f32;
    let y_top = lp.top_margin as f32;
    let y_bottom = (lp.top_margin + grid_height) as f32;
    let marker_cx = (lp.left_margin / 2) as i32;
    let marker_r = 5i32;

    for (s, fret_pos) in frets.iter().enumerate() {
        let y = (lp.top_margin + invert_string(s, sc) * lp.string_spacing) as f32;
        canvas.draw_line(x_left, y, x_right, y, 1);
        let cy = y as i32;
        match fret_pos {
            FretPos::Open if show_notes => canvas.draw_note_label(marker_cx, cy, string_note(s, 0, 0)),
            FretPos::Open if show_ox => canvas.draw_open_circle(marker_cx, cy, marker_r),
            FretPos::Muted if show_ox => canvas.draw_cross(marker_cx, cy, marker_r),
            _ => {}
        }
    }

    let nut_w = if fret_offset == 0 { 3 } else { 1 };
    canvas.draw_line(x_left, y_top, x_left, y_bottom, nut_w);
    for f in 1..=fc {
        let x = (lp.left_margin + f * lp.fret_spacing) as f32;
        canvas.draw_line(x, y_top, x, y_bottom, 1);
    }

    let label_cy = (lp.top_margin + grid_height + lp.bottom_margin / 2) as i32;
    for f in 0..=fc {
        let x = (lp.left_margin + f * lp.fret_spacing) as i32;
        canvas.draw_number(f + fret_offset, x, label_cy, true);
    }

    for (s, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = lp.left_margin as i32 + *n as i32 * lp.fret_spacing as i32 - lp.fret_spacing as i32 / 2;
            let cy = (lp.top_margin + invert_string(s, sc) * lp.string_spacing) as i32;
            if show_notes {
                canvas.draw_note_label(cx, cy, string_note(s, *n, fret_offset));
            } else {
                canvas.draw_filled_circle(cx, cy, 7);
            }
        }
    }
}

pub fn render_vertical<C: Canvas>(
    canvas: &mut C,
    frets: &[FretPos],
    lp: &LayoutParams,
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
) {
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let grid_width = (sc - 1) * lp.string_spacing;
    let grid_height = fc * lp.fret_spacing;
    let x_left = lp.left_margin as f32;
    let x_right = (lp.left_margin + grid_width) as f32;
    let y_top = lp.top_margin as f32;
    let y_bottom = (lp.top_margin + grid_height) as f32;
    let marker_cy = (lp.top_margin / 2) as i32;
    let marker_r = 5i32;

    for (s, fret_pos) in frets.iter().enumerate() {
        let x = (lp.left_margin + s as u32 * lp.string_spacing) as f32;
        canvas.draw_line(x, y_top, x, y_bottom, 1);
        let cx = x as i32;
        match fret_pos {
            FretPos::Open if show_notes => canvas.draw_note_label(cx, marker_cy, string_note(s, 0, 0)),
            FretPos::Open if show_ox => canvas.draw_open_circle(cx, marker_cy, marker_r),
            FretPos::Muted if show_ox => canvas.draw_cross(cx, marker_cy, marker_r),
            _ => {}
        }
    }

    let nut_w = if fret_offset == 0 { 3 } else { 1 };
    canvas.draw_line(x_left, y_top, x_right, y_top, nut_w);
    for f in 1..=fc {
        let y = (lp.top_margin + f * lp.fret_spacing) as f32;
        canvas.draw_line(x_left, y, x_right, y, 1);
    }

    let num_right_x = lp.left_margin as i32 - 4;
    for f in 1..=fc {
        let cy = (lp.top_margin + f * lp.fret_spacing - lp.fret_spacing / 2) as i32;
        canvas.draw_number(f + fret_offset, num_right_x, cy, false);
    }

    for (s, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = (lp.left_margin + s as u32 * lp.string_spacing) as i32;
            let cy = lp.top_margin as i32 + *n as i32 * lp.fret_spacing as i32 - lp.fret_spacing as i32 / 2;
            if show_notes {
                canvas.draw_note_label(cx, cy, string_note(s, *n, fret_offset));
            } else {
                canvas.draw_filled_circle(cx, cy, 7);
            }
        }
    }
}

// ==================== SVG出力 ====================

pub fn render_svg_horizontal(frets: &[FretPos], show_ox: bool, fret_offset: u32, show_notes: bool) -> String {
    let lp = LayoutParams::horizontal();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + fc * lp.fret_spacing + lp.right_margin;
    let h = lp.top_margin + (sc - 1) * lp.string_spacing + lp.bottom_margin;
    let mut canvas = SvgCanvas::new(w, h);
    render_horizontal(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_svg()
}

pub fn render_svg_vertical(frets: &[FretPos], show_ox: bool, fret_offset: u32, show_notes: bool) -> String {
    let lp = LayoutParams::vertical();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + (sc - 1) * lp.string_spacing + lp.right_margin;
    let h = lp.top_margin + fc * lp.fret_spacing + lp.bottom_margin;
    let mut canvas = SvgCanvas::new(w, h);
    render_vertical(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_svg()
}

// ==================== WASM バインディング ====================

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn chord_to_svg(
    input: &str,
    vertical: bool,
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
) -> String {
    use chord::{best_caged_voicing, parse_chord_name};

    let frets = if let Some(f) = parse_fret_string(input) {
        f
    } else if let Some(chord_name) = parse_chord_name(input) {
        match best_caged_voicing(&chord_name) {
            Some((f, _fo)) => f,
            None => return format!("<!-- Error: No voicing found for '{}' -->", input),
        }
    } else {
        return format!("<!-- Error: Cannot parse input '{}' -->", input);
    };

    if vertical {
        render_svg_vertical(&frets, show_ox, fret_offset, show_notes)
    } else {
        render_svg_horizontal(&frets, show_ox, fret_offset, show_notes)
    }
}
