use num2tab::chord::{best_caged_voicing, best_voicing_for_tuning, caged_voicing_by_shape, parse_chord_name};
use num2tab::{
    fret_count, get_string_open, parse_fret_string, render_horizontal, render_vertical,
    render_svg_horizontal, render_svg_vertical, Canvas, FretPos, LayoutParams,
};
use clap::{CommandFactory, FromArgMatches, Parser};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_filled_circle_mut, draw_line_segment_mut};
use std::fs;

// ==================== CLI ====================

#[derive(Parser)]
#[command(name = "num2tab", about = "Guitar chord diagram generator")]
struct Args {
    /// Fret number string or chord name (e.g. 320003, x32010, C, Am, G7)
    input: String,

    /// Vertical layout (standard chord diagram format)
    #[arg(short = 'v', long)]
    vertical: bool,

    /// Show o/x markers
    #[arg(long = "enable-ox-marker", alias = "ox")]
    enable_ox_marker: bool,

    /// Starting fret number (default: 0)
    #[arg(short = 'f', long = "fret", default_value = "0")]
    fret: u32,

    /// Output file (format determined by extension: .png .jpg .svg)
    /// Defaults to <input>.png (e.g. G -> G.png, 320003 -> 320003.png)
    #[arg(short, long)]
    output: Option<String>,

    /// Use CAGED C shape (only valid with chord name input on 6-string)
    #[arg(short = 'C', long = "caged-c")]
    caged_c: bool,

    /// Use CAGED A shape (only valid with chord name input on 6-string)
    #[arg(short = 'A', long = "caged-a")]
    caged_a: bool,

    /// Use CAGED G shape (only valid with chord name input on 6-string)
    #[arg(short = 'G', long = "caged-g")]
    caged_g: bool,

    /// Use CAGED E shape (only valid with chord name input on 6-string)
    #[arg(short = 'E', long = "caged-e")]
    caged_e: bool,

    /// Use CAGED D shape (only valid with chord name input on 6-string)
    #[arg(short = 'D', long = "caged-d")]
    caged_d: bool,

    /// Show note names at fretted positions (instead of dots)
    #[arg(short = 'n', long = "notes")]
    show_notes: bool,

    /// Number of strings: 4 (bass), 6 (standard guitar, default), 7 (seven-string guitar)
    #[arg(long = "strings", default_value = "6")]
    strings: u32,
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
            for d in 0..width as i32 {
                draw_line_segment_mut(&mut self.img, (x1, y1 + d as f32), (x2, y2 + d as f32), black);
            }
        } else {
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

// ==================== PNG出力 ====================

fn draw_horizontal(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
    string_open: &[u8],
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let lp = LayoutParams::horizontal();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + fc * lp.fret_spacing + lp.right_margin;
    let h = lp.top_margin + (sc - 1) * lp.string_spacing + lp.bottom_margin;
    let mut canvas = PngCanvas::new(w, h);
    render_horizontal(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes, string_open);
    canvas.into_image()
}

fn draw_vertical(
    frets: &[FretPos],
    show_ox: bool,
    fret_offset: u32,
    show_notes: bool,
    string_open: &[u8],
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let lp = LayoutParams::vertical();
    let sc = frets.len() as u32;
    let fc = fret_count(frets);
    let w = lp.left_margin + (sc - 1) * lp.string_spacing + lp.right_margin;
    let h = lp.top_margin + fc * lp.fret_spacing + lp.bottom_margin;
    let mut canvas = PngCanvas::new(w, h);
    render_vertical(&mut canvas, frets, &lp, show_ox, fret_offset, show_notes, string_open);
    canvas.into_image()
}

fn save_svg(frets: &[FretPos], show_ox: bool, vertical: bool, fret_offset: u32, show_notes: bool, string_open: &[u8], path: &str) {
    let svg = if vertical {
        render_svg_vertical(frets, show_ox, fret_offset, show_notes, string_open)
    } else {
        render_svg_horizontal(frets, show_ox, fret_offset, show_notes, string_open)
    };
    fs::write(path, svg).expect("SVG保存失敗");
}

// ==================== main ====================

fn auto_output_name(input: &str) -> String {
    let safe: String = input.chars().map(|c| {
        if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c }
    }).collect();
    format!("{}.png", safe)
}

fn is_japanese_locale() -> bool {
    std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .map(|v| v.starts_with("ja"))
        .unwrap_or(false)
}

fn main() {
    let args = if is_japanese_locale() {
        let matches = Args::command()
            .about("ギターコードダイアグラム生成")
            .mut_arg("input", |a| a.help("フレット番号文字列 または コード名 (例: 320003, x32010, C, Am, G7)"))
            .mut_arg("vertical", |a| a.help("縦向き表示（標準コードダイアグラム形式）"))
            .mut_arg("enable_ox_marker", |a| a.help("o/× マーカーを表示"))
            .mut_arg("fret", |a| a.help("表示開始フレット番号（デフォルト: 0）"))
            .mut_arg("output", |a| a.help("出力ファイル（省略時は入力名.png、拡張子で形式判定: .png .jpg .svg）"))
            .mut_arg("caged_c", |a| a.help("CAGED C形状を使用（6弦コード名入力時のみ有効）"))
            .mut_arg("caged_a", |a| a.help("CAGED A形状を使用（6弦コード名入力時のみ有効）"))
            .mut_arg("caged_g", |a| a.help("CAGED G形状を使用（6弦コード名入力時のみ有効）"))
            .mut_arg("caged_e", |a| a.help("CAGED E形状を使用（6弦コード名入力時のみ有効）"))
            .mut_arg("caged_d", |a| a.help("CAGED D形状を使用（6弦コード名入力時のみ有効）"))
            .mut_arg("show_notes", |a| a.help("押弦位置に音名を表示（ドットの代わり）"))
            .mut_arg("strings", |a| a.help("弦数: 4（ベース）, 6（標準ギター、デフォルト）, 7（7弦ギター）"))
            .get_matches();
        Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
    } else {
        Args::parse()
    };
    let ja = is_japanese_locale();

    let output = args.output.unwrap_or_else(|| auto_output_name(&args.input));
    let string_open = get_string_open(args.strings);

    let caged_shape: Option<char> = if args.caged_c { Some('C') }
        else if args.caged_a { Some('A') }
        else if args.caged_g { Some('G') }
        else if args.caged_e { Some('E') }
        else if args.caged_d { Some('D') }
        else { None };

    let (frets, fret_offset) = if let Some(f) = parse_fret_string(&args.input, args.strings as usize) {
        (f, args.fret)
    } else if let Some(chord) = parse_chord_name(&args.input) {
        let is_standard = args.strings == 6;
        let voicing = if is_standard {
            if let Some(shape) = caged_shape {
                caged_voicing_by_shape(&chord, shape)
            } else {
                best_caged_voicing(&chord)
            }
        } else {
            if caged_shape.is_some() {
                if ja {
                    eprintln!("警告: CAGED形状指定は標準6弦チューニングのみ有効です。最良ボイシングを使用します。");
                } else {
                    eprintln!("Warning: CAGED shape selection is only valid for standard 6-string tuning. Using best voicing.");
                }
            }
            best_voicing_for_tuning(&chord, string_open)
        };
        match voicing {
            Some((f, fo)) => (f, if args.fret > 0 { args.fret } else { fo }),
            None => {
                if ja {
                    eprintln!("エラー: '{}' のボイシングが見つかりませんでした", args.input);
                } else {
                    eprintln!("Error: No voicing found for '{}'", args.input);
                }
                std::process::exit(1);
            }
        }
    } else {
        if ja {
            eprintln!("エラー: 入力を解析できません。{}桁フレット番号またはコード名を指定してください (例: 320003, C, Am, G7)",
                args.strings);
        } else {
            eprintln!("Error: Cannot parse input. Provide a {}-digit fret number or chord name (e.g. 320003, C, Am, G7)",
                args.strings);
        }
        std::process::exit(1);
    };

    let ext = std::path::Path::new(&output)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    if ext == "svg" {
        save_svg(&frets, args.enable_ox_marker, args.vertical, fret_offset, args.show_notes, string_open, &output);
    } else {
        let img = if args.vertical {
            draw_vertical(&frets, args.enable_ox_marker, fret_offset, args.show_notes, string_open)
        } else {
            draw_horizontal(&frets, args.enable_ox_marker, fret_offset, args.show_notes, string_open)
        };
        img.save(&output).expect("Failed to save image");
    }

    if ja {
        println!("{} を生成しました", output);
    } else {
        println!("Generated {}", output);
    }
}
