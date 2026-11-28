//! Characterization of every resource name `ragnarok-effects` declares: the
//! lookup key (`normalize`) of each must not change when Korean literals
//! become English.
//!
//! To regenerate the snapshot after an intended change:
//! `BLESS_SNAPSHOT=1 cargo test -p ragnarok-effects --test normalized_paths`.

use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;
use ragnarok_asset_names::normalize;
use ragnarok_effects::sfx::{WaveChoice, effect_sound};
use ragnarok_effects::skill_units::{
    skill_unit_entry_sound, skill_unit_sprite_paths, trap_model_name,
};
use ragnarok_effects::spec::EffectSpec;
use ragnarok_effects::status_buff::reaction_for_efst;
use ragnarok_effects::{
    effect_spec, effect_texture_paths, str_aliases, str_variant, texture_wraps,
};

const SNAPSHOT_FILE: &str = "tests/snapshots/normalized_paths.txt";
const SNAPSHOT: &str = include_str!("snapshots/normalized_paths.txt");

fn dump() -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |label: String, path: &str| out.push(format!("{label} {}", normalize(path)));

    for value in 0..3000usize {
        let Ok(id) = EffectId::try_from_value(value) else {
            continue;
        };
        match effect_spec(id) {
            Some(EffectSpec::Str { file, .. }) => add(format!("str_file {value}"), file),
            Some(EffectSpec::Spr { sprite, .. }) => add(format!("spr {value}"), sprite),
            Some(EffectSpec::SprBurst { sprite, .. }) => add(format!("spr_burst {value}"), sprite),
            _ => {}
        }
        for (i, alias) in str_aliases(id).iter().enumerate() {
            add(format!("str_alias {value} {i}"), alias);
        }
        for roll in 0u64..4 {
            add(format!("str_variant {value} {roll}"), str_variant(id, roll));
        }
        for (i, cue) in effect_sound(id).unwrap_or(&[]).iter().enumerate() {
            match cue.wave {
                WaveChoice::Fixed(name) => add(format!("sfx {value} {i}"), name),
                WaveChoice::Randomized { pattern, count } => {
                    add(format!("sfx_random {value} {i} {count}"), pattern)
                }
            }
        }
    }

    for (i, path) in effect_texture_paths().iter().enumerate() {
        add(format!("texture[{i}] wraps={}", texture_wraps(path)), path);
    }
    for (i, path) in skill_unit_sprite_paths().into_iter().enumerate() {
        add(format!("unit_sprite[{i}]"), path);
    }
    for unit in 0u8..=255 {
        if let Some(sound) = skill_unit_entry_sound(unit) {
            add(format!("unit_sound {unit} {}", sound.one_in), sound.wave);
        }
        if let Some(model) = trap_model_name(unit) {
            add(format!("trap_model {unit}"), model);
        }
    }
    for efst in -1i16..=1500 {
        if let Some(sound) = reaction_for_efst(efst).and_then(|r| r.on_activate_sound) {
            add(format!("status_sound {efst} {:?}", sound.pos), sound.wave);
        }
    }
    out
}

#[test]
fn normalized_keys_match_the_snapshot() {
    let actual = dump();
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
