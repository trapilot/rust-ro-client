use ragnarok_asset_names::romanize::{is_hangul_syllable, romanize, romanize_syllable};

#[test]
fn romanizes_the_reference_syllables() {
    let cases = [
        ('가', "ga"),
        ('각', "gak"),
        ('한', "han"),
        ('글', "geul"),
        ('닭', "dak"),
        ('값', "gap"),
        ('왕', "wang"),
    ];
    for (syllable, expected) in cases {
        assert_eq!(
            romanize_syllable(syllable).as_deref(),
            Some(expected),
            "syllable {syllable}"
        );
    }
}

#[test]
fn run_is_joined_without_separator() {
    assert_eq!(romanize("고글"), "gogeul");
}

#[test]
fn first_and_last_syllable_of_the_block_are_covered() {
    assert_eq!(romanize_syllable('\u{AC00}').as_deref(), Some("ga"));
    // U+D7A3 is 힣: initial h, medial i, final h (-> t).
    assert_eq!(romanize_syllable('\u{D7A3}').as_deref(), Some("hit"));
}

#[test]
fn every_syllable_is_non_empty_ascii_lowercase() {
    for code in 0xAC00u32..=0xD7A3 {
        let c = char::from_u32(code).unwrap();
        let r = romanize_syllable(c).unwrap_or_else(|| panic!("no romanization for U+{code:X}"));
        assert!(!r.is_empty(), "U+{code:X}");
        assert!(
            r.chars().all(|ch| ch.is_ascii_lowercase()),
            "U+{code:X}: {r}"
        );
    }
}

#[test]
fn non_syllables_are_rejected() {
    for c in ['a', '/', 'ㄱ', '漢', '\u{ABFF}', '\u{D7A4}'] {
        assert!(!is_hangul_syllable(c), "{c}");
        assert_eq!(romanize_syllable(c), None, "{c}");
    }
    assert_eq!(romanize(""), "");
    assert_eq!(romanize("a/ㄱ漢"), "");
}
