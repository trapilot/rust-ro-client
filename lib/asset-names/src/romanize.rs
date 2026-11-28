//! Revised Romanization of Hangul syllables, one syllable at a time.
//!
//! No sound-change rules are applied: `각` is `gak` wherever it appears. The
//! output only has to be stable and ASCII, not phonetically exact.

/// Initial consonants (choseong), in Unicode order.
const INITIAL: [&str; 19] = [
    "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p",
    "h",
];

/// Vowels (jungseong), in Unicode order.
const MEDIAL: [&str; 21] = [
    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we",
    "wi", "yu", "eu", "ui", "i",
];

/// Final consonants (jongseong), index 0 meaning none.
const FINAL: [&str; 28] = [
    "", "k", "k", "k", "n", "n", "n", "t", "l", "k", "m", "l", "l", "l", "p", "l", "m", "p", "p",
    "t", "t", "ng", "t", "t", "k", "t", "p", "t",
];

const FIRST_SYLLABLE: u32 = 0xAC00;
const LAST_SYLLABLE: u32 = 0xD7A3;

/// Whether `c` is a precomposed Hangul syllable (U+AC00..U+D7A3).
pub fn is_hangul_syllable(c: char) -> bool {
    (FIRST_SYLLABLE..=LAST_SYLLABLE).contains(&(c as u32))
}

/// The romanization of one syllable, or `None` when `c` is not a Hangul syllable.
pub fn romanize_syllable(c: char) -> Option<String> {
    if !is_hangul_syllable(c) {
        return None;
    }
    let s = c as u32 - FIRST_SYLLABLE;
    let initial = (s / 588) as usize;
    let medial = ((s % 588) / 28) as usize;
    let last = (s % 28) as usize;
    Some(format!(
        "{}{}{}",
        INITIAL[initial], MEDIAL[medial], FINAL[last]
    ))
}

/// Romanizes every syllable of `text` and joins them with no separator.
/// Characters that are not Hangul syllables are skipped.
pub fn romanize(text: &str) -> String {
    text.chars().filter_map(romanize_syllable).collect()
}
