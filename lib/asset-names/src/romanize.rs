//! Revised Romanization of Hangul syllables, one syllable at a time.

/// Whether `c` is a precomposed Hangul syllable (U+AC00..U+D7A3).
pub fn is_hangul_syllable(_c: char) -> bool {
    todo!()
}

/// The romanization of one syllable, or `None` when `c` is not a Hangul syllable.
pub fn romanize_syllable(_c: char) -> Option<String> {
    todo!()
}

/// Romanizes every syllable of `text` and joins them with no separator.
pub fn romanize(_text: &str) -> String {
    todo!()
}
