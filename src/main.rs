mod chord;

use chord::{best_caged_voicing, caged_voicing_by_shape, parse_chord_name};
use clap::Parser;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_circle_mut, draw_line_segment_mut};
use std::fs;

// ==================== CLI ====================

#[derive(Parser)]
#[command(name = "num2tab", about = "ギターコードダイアグラム生成")]
struct Args {
    /// 6桁フレット番号 または コード名 (例: 320003, x32010, C, Am, G7)
    input: String,

    /// 縦向き表示（標準コードダイアグラム形式）
    #[arg(short = 'v', long)]
    vertical: bool,

    /// o/× マーカーを表示
    #[arg(long = "enable-ox-marker", alias = "ox")]
    enable_ox_marker: bool,

    /// 表示開始フレット番号（デフォルト: 0）
    #[arg(short = 'f', long = "fret", default_value = "0")]
    fret: u32,

    /// 出力ファイル（拡張子で形式判定: .png .jpg .svg）
    #[arg(short, long, default_value = "out.png")]
    output: String,

    /// CAGED C形状を使用（コード名入力時のみ有効）
    #[arg(short = 'C', long = "caged-c")]
    caged_c: bool,

    /// CAGED A形状を使用（コード名入力時のみ有効）
    #[arg(short = 'A', long = "caged-a")]
    caged_a: bool,

    /// CAGED G形状を使用（コード名入力時のみ有効）
    #[arg(short = 'G', long = "caged-g")]
    caged_g: bool,

    /// CAGED E形状を使用（コード名入力時のみ有効）
    #[arg(short = 'E', long = "caged-e")]
    caged_e: bool,

    /// CAGED D形状を使用（コード名入力時のみ有効）
    #[arg(short = 'D', long = "caged-d")]
    caged_d: bool,

    /// 押弦位置に音名を表示（ドットの代わり）
    #[arg(short = 'n', long = "notes")]
    show_notes: bool,
}

// ==================== データ型 ====================

#[derive(Debug, Clone, Copy, PartialEq)]
enum FretPos {
    Open,
    Muted,
    Fret(u8),
}

fn parse_fret_string(s: &str) -> Option<Vec<FretPos>> {
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

// ==================== フォント ====================

static DIGIT_FONT: [[u8; 7]; 10] = [
    [0b11110, 0b10010, 0b10010, 0b10010, 0b10010, 0b10010, 0b11110], // 0
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110], // 1
    [0b11110, 0b00010, 0b00010, 0b11110, 0b10000, 0b10000, 0b11110], // 2
    [0b11110, 0b00010, 0b00010, 0b11110, 0b00010, 0b00010, 0b11110], // 3
    [0b10010, 0b10010, 0b10010, 0b11110, 0b00010, 0b00010, 0b00010], // 4
    [0b11110, 0b10000, 0b10000, 0b11110, 0b00010, 0b00010, 0b11110], // 5
    [0b11110, 0b10000, 0b10000, 0b11110, 0b10010, 0b10010, 0b11110], // 6
    [0b11110, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000], // 7
    [0b11110, 0b10010, 0b10010, 0b11110, 0b10010, 0b10010, 0b11110], // 8
    [0b11110, 0b10010, 0b10010, 0b11110, 0b00010, 0b00010, 0b11110], // 9
];

static NOTE_CHAR_FONT: [(char, [u8; 7]); 8] = [
    ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
    ('C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
    ('D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
    ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
    ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('G', [0b01110, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111]),
    ('#', [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010]),
];

fn draw_digit(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    digit: u8,
    x: i32,
    y: i32,
    scale: u32,
    color: Rgb<u8>,
) {
    if digit > 9 {
        return;
    }
    let pattern = &DIGIT_FONT[digit as usize];
    for (row, &bits) in pattern.iter().enumerate() {
        for col in 0..5u32 {
            if (bits >> (4 - col)) & 1 == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + (col * scale) as i32 + dx as i32;
                        let py = y + (row as u32 * scale) as i32 + dy as i32;
                        if px >= 0 && py >= 0 && px < img.width() as i32 && py < img.height() as i32 {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
        }
    }
}

fn draw_number_right(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    n: u32,
    right_x: i32,
    y: i32,
    scale: u32,
    color: Rgb<u8>,
) {
    let dw = (5 * scale + 2) as i32;
    if n < 10 {
        draw_digit(img, n as u8, right_x - 5 * scale as i32, y, scale, color);
    } else {
        let tens = (n / 10) as u8;
        let units = (n % 10) as u8;
        draw_digit(img, units, right_x - 5 * scale as i32, y, scale, color);
        draw_digit(img, tens, right_x - dw - 5 * scale as i32, y, scale, color);
    }
}

fn draw_char(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    c: char,
    x: i32,
    y: i32,
    scale: u32,
    color: Rgb<u8>,
) {
    let pattern: Option<&[u8; 7]> = if c.is_ascii_digit() {
        Some(&DIGIT_FONT[(c as u8 - b'0') as usize])
    } else {
        NOTE_CHAR_FONT.iter().find(|(ch, _)| *ch == c).map(|(_, p)| p)
    };
    let Some(pattern) = pattern else { return };
    for (row, &bits) in pattern.iter().enumerate() {
        for col in 0..5u32 {
            if (bits >> (4 - col)) & 1 == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + (col * scale) as i32 + dx as i32;
                        let py = y + (row as u32 * scale) as i32 + dy as i32;
                        if px >= 0 && py >= 0 && px < img.width() as i32 && py < img.height() as i32 {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
        }
    }
}

// ==================== 音名計算 ====================

const STRING_OPEN: [u8; 6] = [4, 9, 2, 7, 11, 4];
const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

fn string_note(string_idx: usize, fret: u8, fret_offset: u32) -> &'static str {
    let semitone = (STRING_OPEN[string_idx] as u32 + fret as u32 + fret_offset) % 12;
    NOTE_NAMES[semitone as usize]
}

fn fret_count(frets: &[FretPos]) -> u32 {
    frets.iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(0)
        .max(3)
}

fn invert_string(s: usize, string_count: u32) -> u32 {
    string_count - 1 - s as u32
}

// ==================== レイアウトパラメータ ====================

struct LayoutParams {
    string_spacing: u32,
    fret_spacing: u32,
    left_margin: u32,
    top_margin: u32,
    bottom_margin: u32,
    right_margin: u32,
}

impl LayoutParams {
    fn horizontal() -> Self {
        Self {
            string_spacing: 20,
            fret_spacing: 36,
            left_margin: 28,
            top_margin: 14,
            bottom_margin: 26,
            right_margin: 14,
        }
    }

    fn vertical() -> Self {
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

trait Canvas {
    /// 直線を描画（width=1 通常、width=3 ナット）
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: u32);
    /// 黒塗り円（押弦ドット）
    fn draw_filled_circle(&mut self, cx: i32, cy: i32, r: i32);
    /// 開放弦マーカー ○
    fn draw_open_circle(&mut self, cx: i32, cy: i32, r: i32);
    /// ミュートマーカー ×
    fn draw_cross(&mut self, cx: i32, cy: i32, r: i32);
    /// 音名ラベル（黒円＋白文字）
    fn draw_note_label(&mut self, cx: i32, cy: i32, note: &str);
    /// フレット番号（y はテキストの垂直中心）
    /// center_align=true: x は中心座標（横向き用）
    /// center_align=false: x は右端座標（縦向き用）
    fn draw_number(&mut self, n: u32, x: i32, y: i32, center_align: bool);
}

// ==================== PngCanvas ====================

struct PngCanvas {
    img: ImageBuffer<Rgb<u8>, Vec<u8>>,
}

impl PngCanvas {
    fn new(width: u32, height: u32) -> Self {
        let white = Rgb([255u8, 255u8, 255u8]);
        Self { img: ImageBuffer::from_pixel(width, height, white) }
    }

    fn into_image(self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        self.img
    }
}

impl Canvas for PngCanvas {
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: u32) {
        let black = Rgb([0u8, 0u8, 0u8]);
        if width <= 1 {
            draw_line_segment_mut(&mut self.img, (x1, y1), (x2, y2), black);
        } else if y1 == y2 {
            // 横線: y方向にオフセット
            for d in 0..width as i32 {
                draw_line_segment_mut(&mut self.img, (x1, y1 + d as f32), (x2, y2 + d as f32), black);
            }
        } else {
            // 縦線: x方向にオフセット
            for d in 0..width as i32 {
                draw_line_segment_mut(&mut self.img, (x1 + d as f32, y1), (x2 + d as f32, y2), black);
            }
        }
    }

    fn draw_filled_circle(&mut self, cx: i32, cy: i32, r: i32) {
        let black = Rgb([0u8, 0u8, 0u8]);
        draw_filled_circle_mut(&mut self.img, (cx, cy), r, black);
    }

    fn draw_open_circle(&mut self, cx: i32, cy: i32, r: i32) {
        let black = Rgb([0u8, 0u8, 0u8]);
        let white = Rgb([255u8, 255u8, 255u8]);
        draw_filled_circle_mut(&mut self.img, (cx, cy), r, black);
        draw_filled_circle_mut(&mut self.img, (cx, cy), r - 2, white);
    }

    fn draw_cross(&mut self, cx: i32, cy: i32, r: i32) {
        let black = Rgb([0u8, 0u8, 0u8]);
        let d = r as f32 * 0.7;
        for t in 0..2i32 {
            let o = t as f32 - 0.5;
            draw_line_segment_mut(
                &mut self.img,
                (cx as f32 - d + o, cy as f32 - d),
                (cx as f32 + d + o, cy as f32 + d),
                black,
            );
            draw_line_segment_mut(
                &mut self.img,
                (cx as f32 + d + o, cy as f32 - d),
                (cx as f32 - d + o, cy as f32 + d),
                black,
            );
        }
    }

    fn draw_note_label(&mut self, cx: i32, cy: i32, note: &str) {
        let black = Rgb([0u8, 0u8, 0u8]);
        let white = Rgb([255u8, 255u8, 255u8]);
        draw_filled_circle_mut(&mut self.img, (cx, cy), 8, black);
        let n = note.len() as i32;
        let total_w = n * 5 + (n - 1);
        let x0 = cx - total_w / 2;
        let y0 = cy - 3;
        for (i, c) in note.chars().enumerate() {
            draw_char(&mut self.img, c, x0 + i as i32 * 6, y0, 1, white);
        }
    }

    fn draw_number(&mut self, n: u32, x: i32, y: i32, center_align: bool) {
        let black = Rgb([0u8, 0u8, 0u8]);
        let scale = 2u32;
        let digit_h = (7 * scale) as i32;
        let top_y = y - digit_h / 2;
        let right_x = if center_align {
            let nw = if n < 10 { 5 * scale as i32 } else { 12 * scale as i32 };
            x + nw / 2
        } else {
            x
        };
        draw_number_right(&mut self.img, n, right_x, top_y, scale, black);
    }
}

// ==================== SvgCanvas ====================

struct SvgCanvas {
    body: String,
    width: u32,
    height: u32,
}

impl SvgCanvas {
    fn new(width: u32, height: u32) -> Self {
        Self { body: String::new(), width, height }
    }

    fn into_svg(self) -> String {
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

fn render_horizontal<C: Canvas>(
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

    // 弦（横線）と o/× マーカー
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

    // ナット線・フレット線（縦線）
    let nut_w = if fret_offset == 0 { 3 } else { 1 };
    canvas.draw_line(x_left, y_top, x_left, y_bottom, nut_w);
    for f in 1..=fc {
        let x = (lp.left_margin + f * lp.fret_spacing) as f32;
        canvas.draw_line(x, y_top, x, y_bottom, 1);
    }

    // フレット番号（下部、中央揃え）
    let label_cy = (lp.top_margin + grid_height + lp.bottom_margin / 2) as i32;
    for f in 0..=fc {
        let x = (lp.left_margin + f * lp.fret_spacing) as i32;
        canvas.draw_number(f + fret_offset, x, label_cy, true);
    }

    // 押弦ドット or 音名
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

fn render_vertical<C: Canvas>(
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

    // 弦（縦線）と o/× マーカー
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

    // ナット線・フレット線（横線）
    let nut_w = if fret_offset == 0 { 3 } else { 1 };
    canvas.draw_line(x_left, y_top, x_right, y_top, nut_w);
    for f in 1..=fc {
        let y = (lp.top_margin + f * lp.fret_spacing) as f32;
        canvas.draw_line(x_left, y, x_right, y, 1);
    }

    // フレット番号（左側、右揃え）
    let num_right_x = lp.left_margin as i32 - 4;
    for f in 1..=fc {
        let cy = (lp.top_margin + f * lp.fret_spacing - lp.fret_spacing / 2) as i32;
        canvas.draw_number(f + fret_offset, num_right_x, cy, false);
    }

    // 押弦ドット or 音名
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

// ==================== 出力関数 ====================

fn draw_horizontal(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let lp = LayoutParams::horizontal();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + fc * lp.fret_spacing + lp.right_margin;
    let h = lp.top_margin + (sc - 1) * lp.string_spacing + lp.bottom_margin;
    let mut canvas = PngCanvas::new(w, h);
    render_horizontal(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_image()
}

fn draw_vertical(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let lp = LayoutParams::vertical();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + (sc - 1) * lp.string_spacing + lp.right_margin;
    let h = lp.top_margin + fc * lp.fret_spacing + lp.bottom_margin;
    let mut canvas = PngCanvas::new(w, h);
    render_vertical(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_image()
}

fn render_svg_horizontal(frets: &[FretPos], show_ox: bool, fret_offset: u32, show_notes: bool) -> String {
    let lp = LayoutParams::horizontal();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + fc * lp.fret_spacing + lp.right_margin;
    let h = lp.top_margin + (sc - 1) * lp.string_spacing + lp.bottom_margin;
    let mut canvas = SvgCanvas::new(w, h);
    render_horizontal(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_svg()
}

fn render_svg_vertical(frets: &[FretPos], show_ox: bool, fret_offset: u32, show_notes: bool) -> String {
    let lp = LayoutParams::vertical();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + (sc - 1) * lp.string_spacing + lp.right_margin;
    let h = lp.top_margin + fc * lp.fret_spacing + lp.bottom_margin;
    let mut canvas = SvgCanvas::new(w, h);
    render_vertical(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes);
    canvas.into_svg()
}

fn save_svg(frets: &[FretPos], show_ox: bool, vertical: bool, fret_offset: u32, show_notes: bool, path: &str) {
    let svg = if vertical {
        render_svg_vertical(frets, show_ox, fret_offset, show_notes)
    } else {
        render_svg_horizontal(frets, show_ox, fret_offset, show_notes)
    };
    fs::write(path, svg).expect("SVG保存失敗");
}

// ==================== main ====================

fn main() {
    let args = Args::parse();

    let caged_shape: Option<char> = if args.caged_c { Some('C') }
        else if args.caged_a { Some('A') }
        else if args.caged_g { Some('G') }
        else if args.caged_e { Some('E') }
        else if args.caged_d { Some('D') }
        else { None };

    let (frets, fret_offset) = if let Some(f) = parse_fret_string(&args.input) {
        (f, args.fret)
    } else if let Some(chord) = parse_chord_name(&args.input) {
        let voicing = if let Some(shape) = caged_shape {
            caged_voicing_by_shape(&chord, shape)
        } else {
            best_caged_voicing(&chord)
        };
        match voicing {
            Some((f, fo)) => (f, if args.fret > 0 { args.fret } else { fo }),
            None => {
                eprintln!("エラー: '{}' のボイシングが見つかりませんでした", args.input);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("エラー: 入力を解析できません。6桁フレット番号またはコード名を指定してください (例: 320003, C, Am, G7)");
        std::process::exit(1);
    };

    let ext = std::path::Path::new(&args.output)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    if ext == "svg" {
        save_svg(&frets, args.enable_ox_marker, args.vertical, fret_offset, args.show_notes, &args.output);
    } else {
        let img = if args.vertical {
            draw_vertical(&frets, args.enable_ox_marker, fret_offset, args.show_notes)
        } else {
            draw_horizontal(&frets, args.enable_ox_marker, fret_offset, args.show_notes)
        };
        img.save(&args.output).expect("画像保存失敗");
    }

    println!("{} を生成しました", args.output);
}
