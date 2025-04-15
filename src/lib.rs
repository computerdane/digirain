use std::{
    cmp::{max, min},
    time::{SystemTime, UNIX_EPOCH},
};

use crossterm::style::Color;
use rand::{rngs::ThreadRng, Rng};

pub const SYMBOLS_HALF: [char; 75] = [
    ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ', 'ﾝ',
    'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ﾔ', 'ｽ', 'ｿ', '0', '1', '2', '3',
    'Ɛ', '4', '5', '6', '7', '8', '9', 'ρ', 'ﾃ', 'ﾊ', 'ﾌ', 'ﾉ', 'ﾎ', 'ﾒ', 'ﾄ', 'ﾁ', 'ﾆ', 'ﾂ',
];

pub const SYMBOLS: [char; 73] = [
    '　', 'Ａ', 'Ｂ', 'Ｃ', 'Ｄ', 'Ｅ', 'Ｆ', 'Ｇ', 'Ｈ', 'Ｉ', 'Ｊ', 'Ｋ', 'Ｌ', 'Ｍ', 'Ｎ', 'Ｏ',
    'Ｐ', 'Ｑ', 'Ｒ', 'Ｓ', 'Ｔ', 'Ｕ', 'Ｖ', 'Ｗ', 'Ｘ', 'Ｙ', 'Ｚ', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ン', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ',
    'サ', 'シ', 'ヤ', 'ス', 'ソ', '０', '１', '２', '３', '４', '５', '６', '７', '８', '９', 'テ',
    'ハ', 'フ', 'ノ', 'ホ', 'メ', 'ト', 'チ', 'ニ', 'ツ',
];

pub const COLOR_BLACK: Color = Color::Rgb { r: 0, g: 0, b: 0 };
pub const COLOR_WHITE: Color = Color::Rgb {
    r: 0xff,
    g: 0xff,
    b: 0xff,
};

pub fn random_item<T: Copy>(a: &[T], rng: &mut ThreadRng) -> T {
    let random_index = rng.random_range(0..a.len());
    a[random_index]
}

pub fn current_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn clamp<T: Ord>(value: T, min_value: T, max_value: T) -> T {
    max(min(value, max_value), min_value)
}

pub fn clamp_min_zero<T: Ord + Default>(value: T, len: T) -> T {
    clamp(value, T::default(), len)
}

pub fn interp(a: Color, b: Color, x: f64) -> Color {
    if x < 0.0 || x > 1.0 {
        panic!("interp x out of range");
    }
    if let Color::Rgb {
        r: r_a,
        g: g_a,
        b: b_a,
    } = a
    {
        if let Color::Rgb {
            r: r_b,
            g: g_b,
            b: b_b,
        } = b
        {
            return Color::Rgb {
                r: (x * r_a as f64 + (1.0 - x) * r_b as f64) as u8,
                g: (x * g_a as f64 + (1.0 - x) * g_b as f64) as u8,
                b: (x * b_a as f64 + (1.0 - x) * b_b as f64) as u8,
            };
        }
    }
    panic!("colors passed to interp must be Color::Rgb");
}
