use ragnarok_asset_names::{Dictionary, rename_with};

fn dict() -> Dictionary {
    Dictionary::parse("[words]\n\"검사\" = \"swordman\"\n\"남\" = \"male\"\n").unwrap()
}

#[test]
fn translates_known_runs_and_romanizes_unknown_ones() {
    assert_eq!(
        rename_with(&dict(), "data/sprite/인간족/몸통/검사/검사_남.spr"),
        "data/sprite/inganjok/momtong/swordman/swordman_male.spr"
    );
}

#[test]
fn backslashes_become_slashes_and_ascii_is_lowercased() {
    assert_eq!(
        rename_with(&dict(), "DATA\\Texture\\유저인터페이스\\X.BMP"),
        "data/texture/yujeointeopeiseu/x.bmp"
    );
}

#[test]
fn run_is_split_on_the_longest_known_key() {
    assert_eq!(rename_with(&dict(), "검사남"), "swordman_male");
    assert_eq!(rename_with(&dict(), "검사철"), "swordman_cheol");
    assert_eq!(rename_with(&dict(), "철검사"), "cheol_swordman");
    assert_eq!(rename_with(&dict(), "철검사철"), "cheol_swordman_cheol");
}

#[test]
fn longest_key_wins_over_shorter_prefix() {
    let d = Dictionary::parse("[words]\n\"검\" = \"sword\"\n\"검사\" = \"swordman\"\n").unwrap();
    assert_eq!(rename_with(&d, "검사"), "swordman");
    assert_eq!(rename_with(&d, "검검사"), "sword_swordman");
}

#[test]
fn whole_run_key_beats_splitting() {
    let d = Dictionary::parse(
        "[words]\n\"검사남\" = \"sword_guy\"\n\"검사\" = \"swordman\"\n\"남\" = \"male\"\n",
    )
    .unwrap();
    assert_eq!(rename_with(&d, "검사남"), "sword_guy");
}

#[test]
fn no_underscore_is_inserted_next_to_a_run() {
    let d = Dictionary::parse("[words]\n\"고글\" = \"goggles\"\n").unwrap();
    assert_eq!(rename_with(&d, "_고글"), "_goggles");
    assert_eq!(rename_with(&dict(), "검사_남"), "swordman_male");
    assert_eq!(rename_with(&dict(), "무게50이상"), "muge50isang");
    assert_eq!(rename_with(&dict(), "a검사b"), "aswordmanb");
}

#[test]
fn edge_cases() {
    let d = dict();
    assert_eq!(rename_with(&d, ""), "");
    assert_eq!(
        rename_with(&d, "data/Sprite/NPC/Aaa.SPR"),
        "data/sprite/npc/aaa.spr"
    );
    assert_eq!(rename_with(&d, "a b c"), "a_b_c");
    assert_eq!(rename_with(&d, "ㄱ"), "u3131");
    assert_eq!(rename_with(&d, "漢"), "u6f22");
    assert_eq!(rename_with(&d, "Ａ"), "uff21");
    assert_eq!(rename_with(&d, "😀"), "u1f600");
    assert_eq!(rename_with(&d, "/"), "/");
    assert_eq!(rename_with(&d, "\\"), "/");
}

#[test]
fn empty_dictionary_romanizes_everything() {
    let d = Dictionary::parse("").unwrap();
    assert_eq!(rename_with(&d, "한글/글.txt"), "hangeul/geul.txt");
}

#[test]
fn result_is_idempotent_for_a_mixed_path() {
    let d = dict();
    let once = rename_with(&d, "Data\\스프라이트 1/검사_ㄱ漢.SPR");
    assert!(once.is_ascii(), "{once}");
    assert_eq!(rename_with(&d, &once), once);
}
