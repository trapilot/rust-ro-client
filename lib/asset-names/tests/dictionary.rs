use ragnarok_asset_names::Dictionary;

fn err(text: &str) -> String {
    match Dictionary::parse(text) {
        Ok(_) => panic!("expected an error for:\n{text}"),
        Err(e) => e,
    }
}

#[test]
fn builtin_dictionary_parses() {
    // Guards the data shipped by RO Design: a bad edit to dictionary.toml fails here.
    let _ = Dictionary::builtin();
}

#[test]
fn parses_comments_blank_lines_and_words() {
    let dict = Dictionary::parse("# note\n\n[words]\n\"검사\" = \"swordman\"\n\"남\" = \"male\"\n");
    assert!(dict.is_ok(), "{:?}", dict.err());
}

#[test]
fn empty_dictionary_is_valid() {
    assert!(Dictionary::parse("").is_ok());
    assert!(Dictionary::parse("[words]\n").is_ok());
}

#[test]
fn rejects_key_that_is_not_pure_hangul() {
    for key in ["검사_남", "abc", "검사1", "검 사", "ㄱ", "漢字"] {
        let e = err(&format!("[words]\n\"{key}\" = \"x\"\n"));
        assert!(e.contains("key") && e.contains(key), "{key}: {e}");
    }
}

#[test]
fn rejects_empty_key() {
    let e = err("[words]\n\"\" = \"x\"\n");
    assert!(e.contains("key"), "{e}");
}

#[test]
fn rejects_empty_value() {
    let e = err("[words]\n\"검사\" = \"\"\n");
    assert!(e.contains("value") && e.contains("empty"), "{e}");
}

#[test]
fn rejects_value_with_uppercase() {
    let e = err("[words]\n\"검사\" = \"Swordman\"\n");
    assert!(e.contains("value") && e.contains("Swordman"), "{e}");
}

#[test]
fn rejects_value_with_double_underscore() {
    let e = err("[words]\n\"검사\" = \"sword__man\"\n");
    assert!(e.contains("value") && e.contains("sword__man"), "{e}");
}

#[test]
fn rejects_value_with_leading_or_trailing_underscore() {
    for value in ["_sword", "sword_"] {
        let e = err(&format!("[words]\n\"검사\" = \"{value}\"\n"));
        assert!(e.contains("value") && e.contains(value), "{value}: {e}");
    }
}

#[test]
fn rejects_value_with_other_characters() {
    for value in ["sword man", "sword-man", "검사", "sword.man"] {
        let e = err(&format!("[words]\n\"검사\" = \"{value}\"\n"));
        assert!(e.contains("value") && e.contains(value), "{value}: {e}");
    }
}

#[test]
fn rejects_duplicate_values() {
    let e = err("[words]\n\"검사\" = \"sword\"\n\"칼\" = \"sword\"\n");
    assert!(e.contains("duplicate") && e.contains("sword"), "{e}");
    assert!(e.contains("검사") && e.contains("칼"), "{e}");
}

#[test]
fn rejects_duplicate_keys() {
    let e = err("[words]\n\"검사\" = \"a\"\n\"검사\" = \"b\"\n");
    assert!(e.contains("duplicate") && e.contains("검사"), "{e}");
}

#[test]
fn rejects_malformed_lines_with_a_line_number() {
    for text in [
        "[words]\n검사 = \"x\"\n",
        "[words]\n\"검사\" \"x\"\n",
        "[words]\n\"검사\" = x\n",
        "[words]\n\"검사\" = \"x\n",
        "[words]\n\"검사\" = \"x\" junk\n",
        "[words]\n\"검사 = \"x\"\n",
    ] {
        let e = err(text);
        assert!(e.contains("line 2"), "{text:?}: {e}");
    }
}

#[test]
fn rejects_entries_outside_words_table() {
    let e = err("\"검사\" = \"x\"\n");
    assert!(e.contains("line 1") && e.contains("[words]"), "{e}");
    let e = err("[other]\n\"검사\" = \"x\"\n");
    assert!(e.contains("line 1") && e.contains("[other]"), "{e}");
}
