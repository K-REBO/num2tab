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

// ==================== 描画ユーティリティ ====================

// 5×7 ビットマップフォント（上位5ビット×7行）
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
                        if px >= 0
                            && py >= 0
                            && px < img.width() as i32
                            && py < img.height() as i32
                        {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
        }
    }
}

/// 複数桁の数字を描画（右端x座標を指定）
fn draw_number_right(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    n: u32,
    right_x: i32,
    y: i32,
    scale: u32,
    color: Rgb<u8>,
) {
    let dw = (5 * scale + 2) as i32; // 1桁の幅（ギャップ含む）
    if n < 10 {
        draw_digit(img, n as u8, right_x - 5 * scale as i32, y, scale, color);
    } else {
        let tens = (n / 10) as u8;
        let units = (n % 10) as u8;
        draw_digit(img, units, right_x - 5 * scale as i32, y, scale, color);
        draw_digit(img, tens, right_x - dw - 5 * scale as i32, y, scale, color);
    }
}

fn draw_open_marker(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    r: i32,
    color: Rgb<u8>,
) {
    let white = Rgb([255u8, 255u8, 255u8]);
    draw_filled_circle_mut(img, (cx, cy), r, color);
    draw_filled_circle_mut(img, (cx, cy), r - 2, white);
}

fn draw_muted_marker(
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    r: i32,
    color: Rgb<u8>,
) {
    let d = r as f32 * 0.7;
    for t in 0..2i32 {
        let o = t as f32 - 0.5;
        draw_line_segment_mut(
            img,
            (cx as f32 - d + o, cy as f32 - d),
            (cx as f32 + d + o, cy as f32 + d),
            color,
        );
        draw_line_segment_mut(
            img,
            (cx as f32 + d + o, cy as f32 - d),
            (cx as f32 - d + o, cy as f32 + d),
            color,
        );
    }
}

fn draw_dot(img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, cx: i32, cy: i32, r: i32, color: Rgb<u8>) {
    draw_filled_circle_mut(img, (cx, cy), r, color);
}

// ==================== 横レイアウト ====================

fn draw_horizontal(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let string_count = frets.len() as u32;
    let max_fret = frets
        .iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(3);
    let fret_count = max_fret.max(3);

    let string_spacing: u32 = 20;
    let fret_spacing: u32 = 36;
    let left_margin: u32 = 28;
    let top_margin: u32 = 14;
    let bottom_margin: u32 = 26;
    let right_margin: u32 = 14;

    let grid_height = (string_count - 1) * string_spacing;
    let grid_width = fret_count * fret_spacing;
    let img_width = left_margin + grid_width + right_margin;
    let img_height = top_margin + grid_height + bottom_margin;

    let white = Rgb([255u8, 255u8, 255u8]);
    let black = Rgb([0u8, 0u8, 0u8]);
    let mut img = ImageBuffer::from_pixel(img_width, img_height, white);

    let x_left = left_margin as f32;
    let x_right = (left_margin + grid_width) as f32;
    let y_top = top_margin as f32;
    let y_bottom = (top_margin + grid_height) as f32;

    // 弦（横線）＋ o/× マーカー
    let marker_cx = (left_margin / 2) as i32;
    let marker_r = 5i32;
    for (s, fret_pos) in frets.iter().enumerate() {
        let y = (top_margin + s as u32 * string_spacing) as f32;
        draw_line_segment_mut(&mut img, (x_left, y), (x_right, y), black);
        if show_ox {
            let cy = (top_margin + s as u32 * string_spacing) as i32;
            match fret_pos {
                FretPos::Open => draw_open_marker(&mut img, marker_cx, cy, marker_r, black),
                FretPos::Muted => draw_muted_marker(&mut img, marker_cx, cy, marker_r, black),
                FretPos::Fret(_) => {}
            }
        }
    }

    // ナット / 開始フレット線
    let nut_thickness = if fret_offset == 0 { 3 } else { 1 };
    for dx in 0..nut_thickness as i32 {
        draw_line_segment_mut(
            &mut img,
            (x_left + dx as f32, y_top),
            (x_left + dx as f32, y_bottom),
            black,
        );
    }

    // フレット線（縦線）
    for f in 1..=fret_count {
        let x = (left_margin + f * fret_spacing) as f32;
        draw_line_segment_mut(&mut img, (x, y_top), (x, y_bottom), black);
    }

    // フレット番号（下部、オフセット適用）
    let digit_scale = 2u32;
    let digit_h = 7 * digit_scale;
    let label_y = (top_margin + grid_height + (bottom_margin - digit_h) / 2) as i32;
    for f in 0..=fret_count {
        let x = (left_margin + f * fret_spacing) as i32;
        let n = f + fret_offset;
        // 数字を中央揃えで描画
        let nw = if n < 10 { 5 * digit_scale as i32 } else { 12 * digit_scale as i32 };
        draw_number_right(&mut img, n, x + nw / 2, label_y, digit_scale, black);
    }

    // 押弦ドット（セル中央）
    let dot_r = 7i32;
    for (s, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = left_margin as i32
                + *n as i32 * fret_spacing as i32
                - fret_spacing as i32 / 2;
            let cy = (top_margin + s as u32 * string_spacing) as i32;
            draw_dot(&mut img, cx, cy, dot_r, black);
        }
    }

    img
}

// ==================== 縦レイアウト ====================

fn draw_vertical(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let string_count = frets.len() as u32;
    let max_fret = frets
        .iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(3);
    let fret_count = max_fret.max(3);

    let string_spacing: u32 = 20;
    let fret_spacing: u32 = 30;
    let left_margin: u32 = 38; // フレット番号用（左側）
    let top_margin: u32 = 28;  // o/× マーカー用
    let right_margin: u32 = 10;
    let bottom_margin: u32 = 10;

    let grid_width = (string_count - 1) * string_spacing;
    let grid_height = fret_count * fret_spacing;
    let img_width = left_margin + grid_width + right_margin;
    let img_height = top_margin + grid_height + bottom_margin;

    let white = Rgb([255u8, 255u8, 255u8]);
    let black = Rgb([0u8, 0u8, 0u8]);
    let mut img = ImageBuffer::from_pixel(img_width, img_height, white);

    let x_left = left_margin as f32;
    let x_right = (left_margin + grid_width) as f32;
    let y_top = top_margin as f32;
    let y_bottom = (top_margin + grid_height) as f32;

    // 弦（縦線）＋ o/× マーカー（ナット上部）
    let marker_cy = (top_margin / 2) as i32;
    let marker_r = 5i32;
    for (s, fret_pos) in frets.iter().enumerate() {
        let x = (left_margin + s as u32 * string_spacing) as f32;
        draw_line_segment_mut(&mut img, (x, y_top), (x, y_bottom), black);
        if show_ox {
            let cx = (left_margin + s as u32 * string_spacing) as i32;
            match fret_pos {
                FretPos::Open => draw_open_marker(&mut img, cx, marker_cy, marker_r, black),
                FretPos::Muted => draw_muted_marker(&mut img, cx, marker_cy, marker_r, black),
                FretPos::Fret(_) => {}
            }
        }
    }

    // ナット / 開始フレット線
    let nut_thickness = if fret_offset == 0 { 3 } else { 1 };
    for dy in 0..nut_thickness as i32 {
        draw_line_segment_mut(
            &mut img,
            (x_left, y_top + dy as f32),
            (x_right, y_top + dy as f32),
            black,
        );
    }

    // フレット線（横線）
    for f in 1..=fret_count {
        let y = (top_margin + f * fret_spacing) as f32;
        draw_line_segment_mut(&mut img, (x_left, y), (x_right, y), black);
    }

    // フレット番号（左側、オフセット適用）
    let digit_scale = 2u32;
    let digit_h = 7 * digit_scale;
    let num_right_x = (left_margin - 10) as i32; // 数字の右端（ドットと重ならないよう余裕を持たせる）
    for f in 1..=fret_count {
        let cy = top_margin as i32
            + f as i32 * fret_spacing as i32
            - fret_spacing as i32 / 2
            - digit_h as i32 / 2;
        let n = f + fret_offset;
        draw_number_right(&mut img, n, num_right_x, cy, digit_scale, black);
    }

    // 押弦ドット（セル中央）
    let dot_r = 7i32;
    for (s, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = (left_margin + s as u32 * string_spacing) as i32;
            let cy = top_margin as i32
                + *n as i32 * fret_spacing as i32
                - fret_spacing as i32 / 2;
            draw_dot(&mut img, cx, cy, dot_r, black);
        }
    }

    img
}

// ==================== SVG 出力 ====================

fn save_svg(frets: &[FretPos], show_ox: bool, vertical: bool, fret_offset: u32, path: &str) {
    let svg = if vertical {
        render_svg_vertical(frets, show_ox, fret_offset)
    } else {
        render_svg_horizontal(frets, show_ox, fret_offset)
    };
    fs::write(path, svg).expect("SVG保存失敗");
}

fn render_svg_horizontal(frets: &[FretPos], show_ox: bool, fret_offset: u32) -> String {
    let string_count = frets.len() as u32;
    let max_fret = frets
        .iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(3);
    let fret_count = max_fret.max(3);

    let ss: u32 = 20;
    let fs: u32 = 36;
    let lm: u32 = 28;
    let tm: u32 = 14;
    let bm: u32 = 26;
    let rm: u32 = 14;

    let gh = (string_count - 1) * ss;
    let gw = fret_count * fs;
    let w = lm + gw + rm;
    let h = tm + gh + bm;

    let nut_w = if fret_offset == 0 { 3 } else { 1 };

    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" fill="white"/>
"#
    );

    for i in 0..string_count {
        let y = tm + i * ss;
        s += &format!(
            r#"<line x1="{lm}" y1="{y}" x2="{}" y2="{y}" stroke="black" stroke-width="1"/>
"#,
            lm + gw
        );
    }

    s += &format!(
        r#"<line x1="{lm}" y1="{tm}" x2="{lm}" y2="{}" stroke="black" stroke-width="{nut_w}"/>
"#,
        tm + gh
    );

    for f in 1..=fret_count {
        let x = lm + f * fs;
        s += &format!(
            r#"<line x1="{x}" y1="{tm}" x2="{x}" y2="{}" stroke="black" stroke-width="1"/>
"#,
            tm + gh
        );
    }

    for f in 0..=fret_count {
        let x = lm + f * fs;
        let n = f + fret_offset;
        s += &format!(
            r#"<text x="{x}" y="{}" text-anchor="middle" font-size="10" font-family="monospace">{n}</text>
"#,
            tm + gh + 18
        );
    }

    if show_ox {
        let mx = lm / 2;
        for (i, fret_pos) in frets.iter().enumerate() {
            let cy = tm + i as u32 * ss;
            match fret_pos {
                FretPos::Open => {
                    s += &format!(
                        r#"<circle cx="{mx}" cy="{cy}" r="5" fill="none" stroke="black" stroke-width="2"/>
"#
                    );
                }
                FretPos::Muted => {
                    let d = 4u32;
                    s += &format!(
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
"#,
                        mx - d, cy - d, mx + d, cy + d,
                        mx + d, cy - d, mx - d, cy + d
                    );
                }
                FretPos::Fret(_) => {}
            }
        }
    }

    for (i, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = lm + *n as u32 * fs - fs / 2;
            let cy = tm + i as u32 * ss;
            s += &format!(r#"<circle cx="{cx}" cy="{cy}" r="7" fill="black"/>
"#);
        }
    }

    s += "</svg>";
    s
}

fn render_svg_vertical(frets: &[FretPos], show_ox: bool, fret_offset: u32) -> String {
    let string_count = frets.len() as u32;
    let max_fret = frets
        .iter()
        .filter_map(|f| if let FretPos::Fret(n) = f { Some(*n as u32) } else { None })
        .max()
        .unwrap_or(3);
    let fret_count = max_fret.max(3);

    let ss: u32 = 20;
    let fs: u32 = 30;
    let lm: u32 = 28; // フレット番号用（左側）
    let tm: u32 = 28;
    let rm: u32 = 10;
    let bm: u32 = 10;

    let gw = (string_count - 1) * ss;
    let gh = fret_count * fs;
    let w = lm + gw + rm;
    let h = tm + gh + bm;

    let nut_w = if fret_offset == 0 { 3 } else { 1 };

    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" fill="white"/>
"#
    );

    for i in 0..string_count {
        let x = lm + i * ss;
        s += &format!(
            r#"<line x1="{x}" y1="{tm}" x2="{x}" y2="{}" stroke="black" stroke-width="1"/>
"#,
            tm + gh
        );
    }

    s += &format!(
        r#"<line x1="{lm}" y1="{tm}" x2="{}" y2="{tm}" stroke="black" stroke-width="{nut_w}"/>
"#,
        lm + gw
    );

    for f in 1..=fret_count {
        let y = tm + f * fs;
        s += &format!(
            r#"<line x1="{lm}" y1="{y}" x2="{}" y2="{y}" stroke="black" stroke-width="1"/>
"#,
            lm + gw
        );
    }

    // フレット番号（左側）
    for f in 1..=fret_count {
        let y = tm + f * fs - fs / 2;
        let n = f + fret_offset;
        s += &format!(
            r#"<text x="{}" y="{y}" text-anchor="end" dominant-baseline="middle" font-size="10" font-family="monospace">{n}</text>
"#,
            lm - 4
        );
    }

    if show_ox {
        let my = tm / 2;
        for (i, fret_pos) in frets.iter().enumerate() {
            let cx = lm + i as u32 * ss;
            match fret_pos {
                FretPos::Open => {
                    s += &format!(
                        r#"<circle cx="{cx}" cy="{my}" r="5" fill="none" stroke="black" stroke-width="2"/>
"#
                    );
                }
                FretPos::Muted => {
                    let d = 4u32;
                    s += &format!(
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="2"/>
"#,
                        cx - d, my - d, cx + d, my + d,
                        cx + d, my - d, cx - d, my + d
                    );
                }
                FretPos::Fret(_) => {}
            }
        }
    }

    for (i, fret_pos) in frets.iter().enumerate() {
        if let FretPos::Fret(n) = fret_pos {
            let cx = lm + i as u32 * ss;
            let cy = tm + *n as u32 * fs - fs / 2;
            s += &format!(r#"<circle cx="{cx}" cy="{cy}" r="7" fill="black"/>
"#);
        }
    }

    s += "</svg>";
    s
}

// ==================== main ====================

fn main() {
    let args = Args::parse();

    // CAGED形状フラグを解析
    let caged_shape: Option<char> = if args.caged_c { Some('C') }
        else if args.caged_a { Some('A') }
        else if args.caged_g { Some('G') }
        else if args.caged_e { Some('E') }
        else if args.caged_d { Some('D') }
        else { None };

    // 入力を解析: まず6桁形式、次にコード名
    let (frets, fret_offset) = if let Some(f) = parse_fret_string(&args.input) {
        // 6桁モード: CAGEDフラグは無視
        (f, args.fret)
    } else if let Some(chord) = parse_chord_name(&args.input) {
        // コード名モード
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
        save_svg(&frets, args.enable_ox_marker, args.vertical, fret_offset, &args.output);
    } else {
        let img = if args.vertical {
            draw_vertical(&frets, args.enable_ox_marker, fret_offset)
        } else {
            draw_horizontal(&frets, args.enable_ox_marker, fret_offset)
        };
        img.save(&args.output).expect("画像保存失敗");
    }

    println!("{} を生成しました", args.output);
}
