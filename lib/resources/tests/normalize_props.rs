//! Properties of `ragnarok_asset_names::rename` over every path literal the
//! client declares. They live here so `ragnarok-asset-names` does not depend
//! on the resource registry.

use ragnarok_asset_names::{normalize, rename};

const HAND_WRITTEN: &[&str] = &[
    "",
    "data/sprite/인간족/몸통/남/검사_남.spr",
    "data\\sprite\\인간족\\몸통\\남\\검사_남.spr",
    "data/texture/유저인터페이스/item/빨간포션.bmp",
    "effect\\니벨룽겐의 반지.wav",
    "data/sprite/악세사리/남/남_고글.spr",
    "DATA\\Texture\\유저인터페이스\\X.BMP",
    "data/sprite/몬스터/poring.act",
    "data/model/프론테라/건물.rsm",
    "data/wav/_effect.wav",
    "data/idnum2itemdisplaynametable.txt",
    "data/texture/effect/ring_blue.tga",
    "data/palette/몸/남/body_1.pal",
    "a b/c d.txt",
    "ㄱ/漢/Ａ.bmp",
    "무게50이상.bmp",
    "_고글_",
    "한글한글한글.spr",
    "///",
    "a",
];

fn samples() -> Vec<String> {
    ragnarok_resources::all_static_paths()
        .into_iter()
        .chain(HAND_WRITTEN.iter().copied())
        .map(str::to_string)
        .collect()
}

#[test]
fn registry_is_not_empty() {
    assert!(ragnarok_resources::all_static_paths().len() > 100);
}

#[test]
fn result_is_always_ascii() {
    for path in samples() {
        let renamed = rename(&path);
        assert!(renamed.is_ascii(), "{path:?} -> {renamed:?}");
    }
}

#[test]
fn result_has_no_backslash_space_or_uppercase() {
    for path in samples() {
        let renamed = rename(&path);
        assert!(
            !renamed.contains(['\\', ' ']) && !renamed.chars().any(|c| c.is_ascii_uppercase()),
            "{path:?} -> {renamed:?}"
        );
    }
}

#[test]
fn rename_is_idempotent() {
    for path in samples() {
        let once = rename(&path);
        assert_eq!(rename(&once), once, "{path:?}");
    }
}

#[test]
fn normalize_is_rename() {
    for path in samples() {
        assert_eq!(normalize(&path), rename(&path), "{path:?}");
    }
}

#[test]
fn rename_composes_at_path_separators() {
    for path in samples() {
        for (at, c) in path.char_indices() {
            if !matches!(c, '/' | '\\' | '_' | '.') {
                continue;
            }
            // Cut on both sides of the separator; neither cut is inside a Hangul run.
            for cut in [at, at + c.len_utf8()] {
                let (a, b) = path.split_at(cut);
                assert_eq!(
                    format!("{}{}", rename(a), rename(b)),
                    rename(&path),
                    "{path:?} cut at {cut}"
                );
            }
        }
    }
}

#[test]
fn separators_and_case_do_not_change_the_result() {
    for path in samples() {
        let other = path.replace('/', "\\").to_ascii_uppercase();
        // Uppercasing Hangul is a no-op, so only ASCII letters differ.
        assert_eq!(rename(&other), rename(&path), "{path:?}");
    }
}

#[test]
fn documented_examples_hold_with_the_builtin_dictionary() {
    assert_eq!(
        rename("data\\sprite\\인간족\\몸통\\남\\검사_남.spr"),
        "data/sprite/human/body/male/swordman_male.spr"
    );
    assert_eq!(
        rename("data/texture/유저인터페이스/item/빨간포션.bmp"),
        "data/texture/user_interface/item/red_potion.bmp"
    );
    assert_eq!(
        rename("effect\\니벨룽겐의 반지.wav"),
        "effect/nibelungen_ring.wav"
    );
    assert_eq!(
        rename("data/sprite/악세사리/남/남_고글.spr"),
        "data/sprite/accessory/male/male_goggles.spr"
    );
}
