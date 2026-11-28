//! RO QA adversarial test for PL-23: the built-in (English) name table is the
//! default runtime input of mercenary_weapon_sprite_path, but the snapshot
//! test only pins Korean names (it filters `!name.is_ascii()`).

use ragnarok_asset_names::normalize;
use ragnarok_game::sprite_path as sp;

#[test]
fn qa_english_mercenary_names_hit_the_rebuilt_grf_weapon_sprite() {
    // (built-in English name, original Korean GRF weapon sprite)
    let cases = [
        ("female/bow_mercenary", "data/sprite/인간족/용병/활용병_활"),
        ("male/spear_mercenary", "data/sprite/인간족/용병/창용병_창"),
        ("male/sword_mercenary", "data/sprite/인간족/용병/검용병_검"),
        ("bow_mercenary", "data/sprite/인간족/용병/활용병_활"),
    ];
    for (name, grf_original) in cases {
        let path = sp::mercenary_weapon_sprite_path(name).unwrap();
        assert_eq!(normalize(&path), normalize(grf_original), "{name}");
    }
}

#[test]
fn qa_korean_and_english_mercenary_names_agree() {
    for (kr, en) in [
        ("여\\활용병", "female/bow_mercenary"),
        ("남\\창용병", "male/spear_mercenary"),
        ("남\\검용병", "male/sword_mercenary"),
    ] {
        assert_eq!(
            normalize(&sp::mercenary_weapon_sprite_path(kr).unwrap()),
            normalize(&sp::mercenary_weapon_sprite_path(en).unwrap()),
        );
    }
}
