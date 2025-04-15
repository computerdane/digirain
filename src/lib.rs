use std::{
    cmp::{max, min},
    time::{SystemTime, UNIX_EPOCH},
};

use rand::{rngs::ThreadRng, Rng};

// const SYMBOLS: [char; 75] = [
//     ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
//     'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'ｦ', 'ｧ', 'ｨ', 'ｩ', 'ｪ', 'ｫ', 'ｬ', 'ｭ', 'ｮ', 'ｯ', 'ﾝ',
//     'ｱ', 'ｲ', 'ｳ', 'ｴ', 'ｵ', 'ｶ', 'ｷ', 'ｸ', 'ｹ', 'ｺ', 'ｻ', 'ｼ', 'ﾔ', 'ｽ', 'ｿ', '0', '1', '2', '3',
//     'Ɛ', '4', '5', '6', '7', '8', '9', 'ρ', 'ﾃ', 'ﾊ', 'ﾌ', 'ﾉ', 'ﾎ', 'ﾒ', 'ﾄ', 'ﾁ', 'ﾆ', 'ﾂ',
// ];

pub const SYMBOLS: [char; 73] = [
    '　', 'Ａ', 'Ｂ', 'Ｃ', 'Ｄ', 'Ｅ', 'Ｆ', 'Ｇ', 'Ｈ', 'Ｉ', 'Ｊ', 'Ｋ', 'Ｌ', 'Ｍ', 'Ｎ', 'Ｏ',
    'Ｐ', 'Ｑ', 'Ｒ', 'Ｓ', 'Ｔ', 'Ｕ', 'Ｖ', 'Ｗ', 'Ｘ', 'Ｙ', 'Ｚ', 'ヲ', 'ァ', 'ィ', 'ゥ', 'ェ',
    'ォ', 'ャ', 'ュ', 'ョ', 'ッ', 'ン', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ', 'ク', 'ケ', 'コ',
    'サ', 'シ', 'ヤ', 'ス', 'ソ', '０', '１', '２', '３', '４', '５', '６', '７', '８', '９', 'テ',
    'ハ', 'フ', 'ノ', 'ホ', 'メ', 'ト', 'チ', 'ニ', 'ツ',
];

pub fn random_symbol(rng: &mut ThreadRng) -> char {
    let random_index = rng.random_range(0..SYMBOLS.len());
    SYMBOLS[random_index]
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
