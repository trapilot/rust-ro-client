//! Characterization of the resource registry: the lookup key of every path it
//! declares, and of every builder it offers, must not change when the Korean
//! literals are replaced by their English names.
//!
//! The snapshot holds `normalize(path)`, which is what the GRF reader indexes
//! and looks up. To regenerate it after an intended change:
//! `BLESS_SNAPSHOT=1 cargo test -p ragnarok-resources --test normalized_paths`.

use ragnarok_asset_names::{normalize, rename};
use ragnarok_resources as r;

const SNAPSHOT_FILE: &str = "tests/snapshots/normalized_paths.txt";
const SNAPSHOT: &str = include_str!("snapshots/normalized_paths.txt");

/// Names a builder is called with: Korean, English and awkward ones.
const NAMES: &[&str] = &["name", "검사", "poring_01", "a b"];
const SEXES: &[&str] = &["남", "여", "male"];

/// `(label, path)` for everything the registry builds, as written in source.
fn dump() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut add = |label: &str, path: String| {
        out.push((label.to_string(), path));
    };

    for (i, path) in r::all_static_paths().into_iter().enumerate() {
        add(&format!("static[{i}]"), path.to_string());
    }

    for n in NAMES {
        add("imf::of", r::imf::of(n));
        add("map::rsw", r::map::rsw(n));
        add("map::gnd", r::map::gnd(n));
        add("map::gat", r::map::gat(n));
        add("map::file", r::map::file(n));
        add("model::mob", r::model::mob(n));
        add("sound::bgm", r::sound::bgm(n));
        add("sound::sfx", r::sound::sfx(n));
        add("table::book", r::table::book(n));
        add("sprite::homun", r::sprite::homun::of(n));
        add("sprite::item::of", r::sprite::item::of(n));
        add("sprite::monster::of", r::sprite::monster::of(n));
        add("sprite::npc::of", r::sprite::npc::of(n));
        add(
            "sprite::player::mercenary_body",
            r::sprite::player::mercenary_body(n),
        );
        add("texture::named", r::texture::named(n));
        add("texture::effect::named", r::texture::effect::named(n));
        add("texture::effect::str_file", r::texture::effect::str_file(n));
        add("ui::cardbmp", r::ui::cardbmp::named(n));
        add("ui::collection", r::ui::collection::named(n));
        add("ui::illust", r::ui::illust::named(n));
        add("ui::item::icon", r::ui::item::icon(n));
        add("ui::minimap::of", r::ui::minimap::of(n));
        for c in ["bow", "spear", "a"] {
            add(
                "sprite::player::mercenary_weapon",
                r::sprite::player::mercenary_weapon(n, c),
            );
        }
        for s in SEXES {
            add("imf::for_job", r::imf::for_job(n, s));
            add("sprite::accessory::of", r::sprite::accessory::of(s, n));
            add("sprite::player::body", r::sprite::player::body(n, s));
            add("sprite::player::head", r::sprite::player::head(n, s));
            add(
                "sprite::player::weapon",
                r::sprite::player::weapon(n, s, "_검"),
            );
            add(
                "sprite::player::weapon_by_item",
                r::sprite::player::weapon_by_item(n, s, 1201),
            );
            add("sprite::shield::of", r::sprite::shield::of(n, s, 3));
            // Head names are numeric ids from a table; Korean text here would
            // merge with the Hangul run in the literal and is not a real input.
            if n.is_ascii() {
                add("palette::head", r::palette::head(n, s, 7));
            }
            add("palette::body", r::palette::body(n, s, 7));
        }
    }
    for s in SEXES {
        add("sprite::player::gm_body", r::sprite::player::gm_body(s));
        add("sprite::player::gm_weapon", r::sprite::player::gm_weapon(s));
    }
    for design in [0u8, 1, 2, 5] {
        add("sprite::effect::cart", r::sprite::effect::cart(design));
    }
    for (kind, frame) in [(1, 0), (2, 31), (3, 7)] {
        add("texture::water", r::texture::water(kind, frame));
    }
    out
}

/// The lookup key of every entry, one line each.
fn keys() -> Vec<String> {
    dump()
        .into_iter()
        .map(|(label, path)| format!("{label} {}", normalize(&path)))
        .collect()
}

/// Acceptance for the rename: every path the registry states outright is
/// already ASCII and in its final form, so `rename` leaves it alone.
#[test]
fn all_static_paths_are_normalized() {
    for path in r::all_static_paths() {
        assert!(path.is_ascii(), "{path:?} is not ASCII");
        assert_eq!(rename(path), path, "{path:?} is not in normalized form");
    }
}

#[test]
fn normalized_keys_match_the_snapshot() {
    let actual = keys();
    if std::env::var_os("BLESS_SNAPSHOT").is_some() {
        let path = format!("{}/{SNAPSHOT_FILE}", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(path, actual.join("\n") + "\n").expect("write snapshot");
        return;
    }
    let expected: Vec<&str> = SNAPSHOT.lines().collect();
    assert_eq!(actual.len(), expected.len(), "number of snapshot lines");
    for (a, e) in actual.iter().zip(&expected) {
        assert_eq!(a, e);
    }
}
