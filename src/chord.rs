use crate::FretPos;
use std::collections::HashSet;

// ==================== 弦の開放音（半音値: C=0）====================
// 6弦(低E)→1弦(高e): E A D G B e
const STRING_OPEN: [u8; 6] = [4, 9, 2, 7, 11, 4];

// ==================== データ型 ====================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChordQuality {
    Major,
    Minor,
    Dom7,
    Maj7,
    Min7,
    Dom9,
    Maj9,
    Min9,
    Dom11,
    Maj11,
    Min11,
    Dom13,
    Sus2,
    Sus4,
    Dim,
    Aug,
}

#[derive(Debug, Clone)]
pub struct ChordName {
    pub root: u8,           // 0=C, 1=C#, ..., 11=B
    pub quality: ChordQuality,
}

// ==================== コード名パース ====================

fn note_to_semitone(s: &str) -> Option<u8> {
    match s {
        "C" => Some(0),
        "C#" | "Db" => Some(1),
        "D" => Some(2),
        "D#" | "Eb" => Some(3),
        "E" => Some(4),
        "F" => Some(5),
        "F#" | "Gb" => Some(6),
        "G" => Some(7),
        "G#" | "Ab" => Some(8),
        "A" => Some(9),
        "A#" | "Bb" => Some(10),
        "B" => Some(11),
        _ => None,
    }
}

fn parse_quality(s: &str) -> Option<ChordQuality> {
    // 長い順にマッチ（greedy）
    match s {
        ""     => Some(ChordQuality::Major),
        "m"    => Some(ChordQuality::Minor),
        "7"    => Some(ChordQuality::Dom7),
        "M7"   => Some(ChordQuality::Maj7),
        "m7"   => Some(ChordQuality::Min7),
        "9"    => Some(ChordQuality::Dom9),
        "M9"   => Some(ChordQuality::Maj9),
        "m9"   => Some(ChordQuality::Min9),
        "11"   => Some(ChordQuality::Dom11),
        "M11"  => Some(ChordQuality::Maj11),
        "m11"  => Some(ChordQuality::Min11),
        "13"   => Some(ChordQuality::Dom13),
        "sus2" => Some(ChordQuality::Sus2),
        "sus4" => Some(ChordQuality::Sus4),
        "dim"  => Some(ChordQuality::Dim),
        "aug"  => Some(ChordQuality::Aug),
        _      => None,
    }
}

pub fn parse_chord_name(s: &str) -> Option<ChordName> {
    if s.is_empty() { return None; }
    // ルート音: 先頭の大文字 A-G + オプションの # または b
    let first = s.chars().next()?;
    if !('A'..='G').contains(&first) { return None; }
    let (root_end, rest) = if s.len() > 1 && (s.chars().nth(1) == Some('#') || s.chars().nth(1) == Some('b')) {
        (2, &s[2..])
    } else {
        (1, &s[1..])
    };
    let root = note_to_semitone(&s[..root_end])?;
    let quality = parse_quality(rest)?;
    Some(ChordName { root, quality })
}

// ==================== CAGEDテンプレート ====================

struct CagedTemplate {
    name: char,
    frets: [i8; 6],        // -1=muted, 0=open, >=1=フレット番号
    root_semitone: u8,     // テンプレート根音の半音値
}

// ----- Major -----
const MAJOR_TEMPLATES: [CagedTemplate; 5] = [
    CagedTemplate { name: 'E', frets: [0,2,2,1,0,0],    root_semitone: 4  }, // E shape: 022100
    CagedTemplate { name: 'A', frets: [-1,0,2,2,2,0],   root_semitone: 9  }, // A shape: x02220
    CagedTemplate { name: 'G', frets: [3,2,0,0,0,3],    root_semitone: 7  }, // G shape: 320003
    CagedTemplate { name: 'C', frets: [-1,3,2,0,1,0],   root_semitone: 0  }, // C shape: x32010
    CagedTemplate { name: 'D', frets: [-1,-1,0,2,3,2],  root_semitone: 2  }, // D shape: xx0232
];

// ----- Minor -----
const MINOR_TEMPLATES: [CagedTemplate; 5] = [
    CagedTemplate { name: 'E', frets: [0,2,2,0,0,0],    root_semitone: 4  }, // Em: 022000
    CagedTemplate { name: 'A', frets: [-1,0,2,2,1,0],   root_semitone: 9  }, // Am: x02210
    CagedTemplate { name: 'D', frets: [-1,-1,0,2,3,1],  root_semitone: 2  }, // Dm: xx0231
    CagedTemplate { name: 'G', frets: [3,5,5,3,3,3],    root_semitone: 7  }, // Gm: 355333
    CagedTemplate { name: 'C', frets: [-1,3,5,5,4,3],   root_semitone: 0  }, // Cm: x35543
];

// ----- Dominant 7th -----
const DOM7_TEMPLATES: [CagedTemplate; 5] = [
    CagedTemplate { name: 'E', frets: [0,2,0,1,0,0],    root_semitone: 4  }, // E7: 020100
    CagedTemplate { name: 'A', frets: [-1,0,2,0,2,0],   root_semitone: 9  }, // A7: x02020
    CagedTemplate { name: 'G', frets: [3,2,0,0,0,1],    root_semitone: 7  }, // G7: 320001
    CagedTemplate { name: 'C', frets: [-1,3,2,3,1,0],   root_semitone: 0  }, // C7: x32310
    CagedTemplate { name: 'D', frets: [-1,-1,0,2,1,2],  root_semitone: 2  }, // D7: xx0212
];

// ----- Major 7th -----
const MAJ7_TEMPLATES: [CagedTemplate; 5] = [
    CagedTemplate { name: 'E', frets: [0,2,1,1,0,0],    root_semitone: 4  }, // EMaj7: 021100
    CagedTemplate { name: 'A', frets: [-1,0,2,1,2,0],   root_semitone: 9  }, // AMaj7: x02120
    CagedTemplate { name: 'G', frets: [3,-1,0,0,0,2],   root_semitone: 7  }, // GMaj7: 3x0002
    CagedTemplate { name: 'C', frets: [-1,3,2,0,0,0],   root_semitone: 0  }, // CMaj7: x32000
    CagedTemplate { name: 'D', frets: [-1,-1,0,2,2,2],  root_semitone: 2  }, // DMaj7: xx0222
];

// ----- Minor 7th -----
const MIN7_TEMPLATES: [CagedTemplate; 5] = [
    CagedTemplate { name: 'E', frets: [0,2,0,0,0,0],    root_semitone: 4  }, // Em7: 020000
    CagedTemplate { name: 'A', frets: [-1,0,2,0,1,0],   root_semitone: 9  }, // Am7: x02010
    CagedTemplate { name: 'D', frets: [-1,-1,0,2,1,1],  root_semitone: 2  }, // Dm7: xx0211
    CagedTemplate { name: 'G', frets: [3,5,3,3,3,3],    root_semitone: 7  }, // Gm7: 353333
    CagedTemplate { name: 'C', frets: [-1,3,5,3,4,3],   root_semitone: 0  }, // Cm7: x35343
];

fn get_templates(quality: ChordQuality) -> &'static [CagedTemplate] {
    match quality {
        ChordQuality::Major => &MAJOR_TEMPLATES,
        ChordQuality::Minor => &MINOR_TEMPLATES,
        ChordQuality::Dom7  => &DOM7_TEMPLATES,
        ChordQuality::Maj7  => &MAJ7_TEMPLATES,
        ChordQuality::Min7  => &MIN7_TEMPLATES,
        // テンションコードは別途生成
        _ => &[],
    }
}

// ==================== テンプレート転置 ====================

fn transpose_template(tmpl: &CagedTemplate, target_root: u8) -> (Vec<FretPos>, u32) {
    let offset = ((target_root as i16 - tmpl.root_semitone as i16 + 12) % 12) as u8;

    // 絶対フレット位置に変換
    let raw: Vec<FretPos> = tmpl.frets.iter().map(|&f| match f {
        -1 => FretPos::Muted,
        0 if offset == 0 => FretPos::Open,
        n if n >= 0 => FretPos::Fret((n as u8) + offset),
        _ => FretPos::Muted,
    }).collect();

    // 最小押弦フレットを求めて正規化
    let min_fret = raw.iter().filter_map(|f| {
        if let FretPos::Fret(n) = f { Some(*n) } else { None }
    }).min().unwrap_or(0);

    let fret_offset = if min_fret > 1 { (min_fret - 1) as u32 } else { 0 };

    let normalized: Vec<FretPos> = raw.iter().map(|f| match f {
        FretPos::Fret(n) => FretPos::Fret(n - fret_offset as u8),
        other => *other,
    }).collect();

    (normalized, fret_offset)
}

// ==================== テンションコード全弦探索 ====================

/// コードトーンの半音セット (mandatory, optional) を返す
fn chord_tone_sets(root: u8, quality: ChordQuality) -> (Vec<u8>, Vec<u8>) {
    let (mand, opt): (Vec<u8>, Vec<u8>) = match quality {
        ChordQuality::Major  => (vec![0,4,7], vec![]),
        ChordQuality::Minor  => (vec![0,3,7], vec![]),
        ChordQuality::Dom7   => (vec![0,4,10], vec![7]),
        ChordQuality::Maj7   => (vec![0,4,11], vec![7]),
        ChordQuality::Min7   => (vec![0,3,10], vec![7]),
        ChordQuality::Dom9   => (vec![0,4,10,2], vec![7]),    // 9th=2
        ChordQuality::Maj9   => (vec![0,4,11,2], vec![7]),
        ChordQuality::Min9   => (vec![0,3,10,2], vec![7]),
        ChordQuality::Dom11  => (vec![0,10,2,5], vec![4,7]), // 11th=5
        ChordQuality::Maj11  => (vec![0,11,2,5], vec![4,7]),
        ChordQuality::Min11  => (vec![0,3,10,5], vec![7,2]),
        ChordQuality::Dom13  => (vec![0,4,10,9], vec![7,2,5]), // 13th=9
        ChordQuality::Sus2   => (vec![0,2,7], vec![]),
        ChordQuality::Sus4   => (vec![0,5,7], vec![]),
        ChordQuality::Dim    => (vec![0,3,6], vec![]),
        ChordQuality::Aug    => (vec![0,4,8], vec![]),
    };
    let to_semitone = |i: u8| ((root as u16 + i as u16) % 12) as u8;
    (
        mand.into_iter().map(to_semitone).collect(),
        opt.into_iter().map(to_semitone).collect(),
    )
}

/// 指定フレットウィンドウでの各弦候補を返す
/// (fret_pos, note_semitone)
fn string_candidates(string_idx: usize, win_min: u8, win_max: u8,
                     all_tones: &[u8]) -> Vec<(FretPos, u8)> {
    let mut result = Vec::new();
    let open_note = STRING_OPEN[string_idx];

    // 開放弦（ウィンドウが0から始まる場合）
    if win_min == 0 && all_tones.contains(&open_note) {
        result.push((FretPos::Open, open_note));
    }

    // フレット音
    for fret in win_min.max(1)..=win_max {
        let note = (open_note as u16 + fret as u16) as u8 % 12;
        if all_tones.contains(&note) {
            result.push((FretPos::Fret(fret), note));
        }
    }

    // ミュート（常に候補）
    result.push((FretPos::Muted, 255)); // 255=muted sentinel

    result
}

/// テンションコードのボイシングを全弦探索で生成
pub fn generate_tension_voicings(chord: &ChordName) -> Vec<(Vec<FretPos>, u32)> {
    let (mandatory, optional) = chord_tone_sets(chord.root, chord.quality);
    let all_tones: Vec<u8> = mandatory.iter().chain(optional.iter()).copied().collect();

    let mut results = Vec::new();

    // フレットウィンドウを複数試す: 0-4, 1-5, ..., 9-13
    for win_start in 0u8..=9 {
        let win_end = win_start + 4;

        // 各弦の候補を収集
        let per_string: Vec<Vec<(FretPos, u8)>> = (0..6).map(|s| {
            string_candidates(s, win_start, win_end, &all_tones)
        }).collect();

        // 全組み合わせを再帰的に列挙
        let mut combo: Vec<(FretPos, u8)> = Vec::with_capacity(6);
        enumerate_voicings(&per_string, 0, &mut combo, &mandatory, &mut results);
    }

    // ウィンドウ重複で生成された同一ボイシングを除去
    let mut seen = HashSet::new();
    results.retain(|(frets, fo)| seen.insert((frets.clone(), *fo)));

    results
}

fn enumerate_voicings(
    per_string: &[Vec<(FretPos, u8)>],
    string_idx: usize,
    combo: &mut Vec<(FretPos, u8)>,
    mandatory: &[u8],
    results: &mut Vec<(Vec<FretPos>, u32)>,
) {
    if string_idx == 6 {
        // 必須音が全て含まれているか確認
        let notes: Vec<u8> = combo.iter()
            .filter(|(_, n)| *n != 255)
            .map(|(_, n)| *n)
            .collect();
        let all_present = mandatory.iter().all(|m| notes.contains(m));
        if all_present {
            let frets: Vec<FretPos> = combo.iter().map(|(f, _)| *f).collect();
            let fret_offset = calc_fret_offset(&frets);
            let normalized = normalize_frets(&frets, fret_offset);
            results.push((normalized, fret_offset));
        }
        return;
    }

    for candidate in &per_string[string_idx] {
        combo.push(*candidate);
        enumerate_voicings(per_string, string_idx + 1, combo, mandatory, results);
        combo.pop();
    }
}

fn calc_fret_offset(frets: &[FretPos]) -> u32 {
    let min_fret = frets.iter().filter_map(|f| {
        if let FretPos::Fret(n) = f { Some(*n) } else { None }
    }).min().unwrap_or(0);
    if min_fret > 1 { (min_fret - 1) as u32 } else { 0 }
}

fn normalize_frets(frets: &[FretPos], fret_offset: u32) -> Vec<FretPos> {
    frets.iter().map(|f| match f {
        FretPos::Fret(n) => FretPos::Fret(n - fret_offset as u8),
        other => *other,
    }).collect()
}

// ==================== プレイアビリティスコア ====================

pub fn playability_score(frets: &[FretPos]) -> i32 {
    let fretted: Vec<u8> = frets.iter().filter_map(|f| {
        if let FretPos::Fret(n) = f { Some(*n) } else { None }
    }).collect();

    if fretted.is_empty() { return 0; }

    let min_fret = *fretted.iter().min().unwrap();
    let max_fret = *fretted.iter().max().unwrap();
    let fret_span = max_fret - min_fret;

    let open_count = frets.iter().filter(|&&f| f == FretPos::Open).count() as i32;

    // バレー: 最低フレットに2弦以上
    let barre_count = fretted.iter().filter(|&&f| f == min_fret).count();
    let has_barre = barre_count >= 2;

    // 実効指数: ユニークフレット数（バレーは1本指）
    let mut unique_frets: Vec<u8> = fretted.clone();
    unique_frets.sort();
    unique_frets.dedup();
    let finger_count = if has_barre {
        unique_frets.len()
    } else {
        unique_frets.len()
    };

    // 鳴らす弦の外側境界内のミュート数
    let played_strings: Vec<usize> = frets.iter().enumerate()
        .filter(|(_, f)| **f != FretPos::Muted)
        .map(|(i, _)| i)
        .collect();
    let muted_mid = if played_strings.len() >= 2 {
        let lo = *played_strings.first().unwrap();
        let hi = *played_strings.last().unwrap();
        frets[lo..=hi].iter().filter(|&&f| f == FretPos::Muted).count() as i32
    } else {
        0
    };

    // ネックポジションペナルティ（高いフレットは弾きにくい）
    let neck_penalty = 0_i32.max(min_fret as i32 - 3) * 3;

    // フレットスパン段階的ペナルティ（物理制約: スパンが大きいほど急増）
    let span_penalty: i32 = match fret_span {
        0..=2 => 0,
        3     => 15,
        4     => 30,
        5     => 50,
        _     => 80,
    };

    // 鳴らす弦数ボーナス（3弦超で +3/弦: 豊かなボイシングを優先）
    let sounding_count = frets.iter().filter(|&&f| f != FretPos::Muted).count() as i32;
    let sounding_bonus = (sounding_count - 3).max(0) * 3;

    100
        - span_penalty
        - finger_count as i32 * 5
        - muted_mid * 8
        + open_count * 5
        - neck_penalty
        + sounding_bonus
}

// ==================== ルートオンベースボーナス ====================

/// 最低音弦がルート音を鳴らしている場合 +15 を返す（voice leading: 自然な低音配置）
fn root_on_bass_bonus(
    frets: &[FretPos],
    fret_offset: u32,
    string_open: &[u8],
    root: u8,
) -> i32 {
    for (idx, fret_pos) in frets.iter().enumerate() {
        let note = match fret_pos {
            FretPos::Muted => continue,
            FretPos::Open => string_open[idx] % 12,
            FretPos::Fret(f) => {
                ((string_open[idx] as u32 + *f as u32 + fret_offset) % 12) as u8
            }
        };
        return if note == root { 15 } else { 0 };
    }
    0
}

// ==================== 任意チューニング対応ボイシング探索 ====================

/// 任意チューニングの1弦分の候補を返す（string_candidatesの汎用版）
fn string_candidates_tuned(
    string_open: &[u8],
    string_idx: usize,
    win_min: u8,
    win_max: u8,
    all_tones: &[u8],
) -> Vec<(FretPos, u8)> {
    let mut result = Vec::new();
    let open_note = string_open[string_idx];

    if win_min == 0 && all_tones.contains(&open_note) {
        result.push((FretPos::Open, open_note));
    }
    for fret in win_min.max(1)..=win_max {
        let note = (open_note as u16 + fret as u16) as u8 % 12;
        if all_tones.contains(&note) {
            result.push((FretPos::Fret(fret), note));
        }
    }
    result.push((FretPos::Muted, 255));
    result
}

/// 弦数を引数で受け取る enumerate_voicings
fn enumerate_voicings_n(
    per_string: &[Vec<(FretPos, u8)>],
    string_idx: usize,
    combo: &mut Vec<(FretPos, u8)>,
    mandatory: &[u8],
    results: &mut Vec<(Vec<FretPos>, u32)>,
    n_strings: usize,
) {
    if string_idx == n_strings {
        let notes: Vec<u8> = combo.iter()
            .filter(|(_, n)| *n != 255)
            .map(|(_, n)| *n)
            .collect();
        if mandatory.iter().all(|m| notes.contains(m)) {
            let frets: Vec<FretPos> = combo.iter().map(|(f, _)| *f).collect();
            let fret_offset = calc_fret_offset(&frets);
            let normalized = normalize_frets(&frets, fret_offset);
            results.push((normalized, fret_offset));
        }
        return;
    }
    for candidate in &per_string[string_idx] {
        combo.push(*candidate);
        enumerate_voicings_n(per_string, string_idx + 1, combo, mandatory, results, n_strings);
        combo.pop();
    }
}

/// 任意のチューニング・弦数でボイシングを全弦探索して返す
pub fn generate_voicings_for_tuning(
    chord: &ChordName,
    string_open: &[u8],
) -> Vec<(Vec<FretPos>, u32)> {
    let (mandatory, optional) = chord_tone_sets(chord.root, chord.quality);
    let all_tones: Vec<u8> = mandatory.iter().chain(optional.iter()).copied().collect();
    let n_strings = string_open.len();
    let mut results = Vec::new();

    for win_start in 0u8..=9 {
        let win_end = win_start + 4;
        let per_string: Vec<Vec<(FretPos, u8)>> = (0..n_strings)
            .map(|s| string_candidates_tuned(string_open, s, win_start, win_end, &all_tones))
            .collect();
        let mut combo = Vec::with_capacity(n_strings);
        enumerate_voicings_n(&per_string, 0, &mut combo, &mandatory, &mut results, n_strings);
    }

    // ウィンドウ重複で生成された同一ボイシングを除去
    let mut seen = HashSet::new();
    results.retain(|(frets, fo)| seen.insert((frets.clone(), *fo)));

    results
}

/// 任意のチューニング・弦数で最良ボイシングを返す
/// 標準6弦チューニングの場合はCAGEDテンプレートを優先使用
pub fn best_voicing_for_tuning(
    chord: &ChordName,
    string_open: &[u8],
) -> Option<(Vec<FretPos>, u32)> {
    // 標準6弦チューニングならCAGEDを使う
    if string_open == [4u8, 9, 2, 7, 11, 4] {
        return best_caged_voicing(chord);
    }

    generate_voicings_for_tuning(chord, string_open)
        .into_iter()
        .max_by_key(|(frets, fo)| {
            playability_score(frets)
            - *fo as i32 * 3
            + root_on_bass_bonus(frets, *fo, string_open, chord.root)
        })
}

// ==================== パブリックAPI ====================

/// コード名から最良ボイシング（スコア最高）を返す
pub fn best_caged_voicing(chord: &ChordName) -> Option<(Vec<FretPos>, u32)> {
    let templates = get_templates(chord.quality);

    if templates.is_empty() {
        // テンションコード: 全弦探索
        let voicings = generate_tension_voicings(chord);
        return voicings.into_iter()
            .max_by_key(|(frets, fo)| {
                playability_score(frets)
                - *fo as i32 * 3
                + root_on_bass_bonus(frets, *fo, &STRING_OPEN, chord.root)
            })
            .map(|(frets, fo)| (frets, fo));
    }

    templates.iter()
        .map(|t| transpose_template(t, chord.root))
        .max_by_key(|(frets, fo)| {
            playability_score(frets)
            - *fo as i32 * 3
            + root_on_bass_bonus(frets, *fo, &STRING_OPEN, chord.root)
        })
}

/// コード名 + CAGED形状指定 ('C','A','G','E','D') でボイシングを返す
pub fn caged_voicing_by_shape(chord: &ChordName, shape: char) -> Option<(Vec<FretPos>, u32)> {
    // テンションコードはCAGED形状非対応: 基底品質（7th相当）に落として使用
    let effective_quality = match chord.quality {
        ChordQuality::Dom9 | ChordQuality::Dom11 | ChordQuality::Dom13 => ChordQuality::Dom7,
        ChordQuality::Maj9 | ChordQuality::Maj11 => ChordQuality::Maj7,
        ChordQuality::Min9 | ChordQuality::Min11 => ChordQuality::Min7,
        q => q,
    };
    let effective = ChordName { root: chord.root, quality: effective_quality };
    let templates = get_templates(effective.quality);

    templates.iter()
        .find(|t| t.name == shape)
        .map(|t| transpose_template(t, effective.root))
}
