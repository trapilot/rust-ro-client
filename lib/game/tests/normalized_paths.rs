//! Characterization of every resource path `ragnarok-game` assembles from its
//! own tables: sprite paths, status icons and sound names. The lookup key
//! (`normalize`) of each must not change when Korean literals become English.
//!
//! To regenerate the snapshot after an intended change:
//! `BLESS_SNAPSHOT=1 cargo test -p ragnarok-game --test normalized_paths`.

use models::enums::EnumWithNumberValue;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_asset_names::normalize;
use ragnarok_game::sound::tables::ui;
use ragnarok_game::sound::tables::{
    StatusSoundKind, skill_cast_begin_sound, skill_projectile_sound, skill_use_sound, status_sound,
    swing_sound, weapon_hit_sound,
};
use ragnarok_game::sprite_path as sp;
use ragnarok_game::status_icon::status_icon_info;

const SNAPSHOT_FILE: &str = "tests/snapshots/normalized_paths.txt";
const SNAPSHOT: &str = include_str!("snapshots/normalized_paths.txt");

fn weapon_types() -> Vec<WeaponType> {
    (0..64usize)
        .filter_map(|v| WeaponType::try_from_value(v).ok())
        .collect()
}

fn dump() -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |label: String, path: &str| out.push(format!("{label} {}", normalize(path)));

    // Sprite paths: the base, baby, transcendent and costume job ranges plus
    // two unknown ids that fall back to the novice, both sexes.
    let jobs: Vec<u16> = (0u16..=60).chain(4000..=4060).chain([4300, 9999]).collect();
    for &job in &jobs {
        for sex in 0u8..=1 {
            add(format!("body {job} {sex}"), &sp::body_sprite_path(job, sex));
            add(format!("imf {job} {sex}"), &sp::imf_path(job, sex));
            add(
                format!("kick_glow {job} {sex}"),
                &sp::kick_glow_sprite_path(job, sex),
            );
            for pal in [0u16, 3] {
                add(
                    format!("body_pal {job} {sex} {pal}"),
                    &sp::body_palette_path(job, sex, pal),
                );
            }
            for view in 0u16..=5 {
                if let Some(p) = sp::shield_sprite_path(view, job, sex) {
                    add(format!("shield {view} {job} {sex}"), &p);
                }
                add(
                    format!("shield_num {view} {job} {sex}"),
                    &sp::shield_sprite_path_numeric(view, job, sex),
                );
            }
            for item in [1201u16, 1301] {
                add(
                    format!("weapon_item {job} {sex} {item}"),
                    &sp::weapon_item_sprite_path(job, sex, item),
                );
            }
            for wt in weapon_types() {
                add(
                    format!("weapon {job} {sex} {}", wt.value()),
                    &sp::weapon_sprite_path(job, sex, wt),
                );
                // The trail suffix is the same for every job, so a few jobs show it.
                if ![0u16, 1, 4001].contains(&job) {
                    continue;
                }
                if let Some(p) = sp::weapon_trail_sprite_path(job, sex, wt) {
                    add(format!("trail {job} {sex} {}", wt.value()), &p);
                }
                if let Some(p) = sp::weapon_item_trail_sprite_path(job, sex, 1201, wt) {
                    add(format!("item_trail {job} {sex} {}", wt.value()), &p);
                }
            }
        }
    }
    for sex in 0u8..=1 {
        add(format!("gm_body {sex}"), &sp::gm_body_sprite_path(sex));
        add(format!("gm_weapon {sex}"), &sp::gm_weapon_sprite_path(sex));
        for head in 0u16..=40 {
            add(
                format!("head {head} {sex}"),
                &sp::head_sprite_path(head, sex),
            );
            for job in [0u16, 1, 6, 7, 4001] {
                add(
                    format!("head_pal {head} {job} {sex}"),
                    &sp::head_palette_path(head, job, sex, 2),
                );
            }
        }
        for suffix in ["_goggles", "_고글", "_RTC3등투구", ""] {
            add(
                format!("headgear {suffix} {sex}"),
                &sp::headgear_sprite_path(suffix, sex),
            );
        }
    }
    for design in 0u8..=6 {
        add(format!("cart {design}"), &sp::cart_sprite_path(design));
    }
    for job in [0u16, 4012, 4013] {
        add(format!("falcon {job}"), sp::falcon_sprite_path(job));
    }
    for name in [
        "name",
        "활용병",
        "여\\활용병",
        "남\\창용병",
        "남\\검용병",
        "poring",
    ] {
        add(format!("npc {name}"), &sp::npc_sprite_path(name));
        add(format!("monster {name}"), &sp::monster_sprite_path(name));
        add(format!("homun {name}"), &sp::homunculus_sprite_path(name));
        add(
            format!("merc_body {name}"),
            &sp::mercenary_sprite_path(name),
        );
        if let Some(p) = sp::mercenary_imf_path(name) {
            add(format!("merc_imf {name}"), &p);
        }
        if let Some(p) = sp::mercenary_weapon_sprite_path(name) {
            add(format!("merc_weapon {name}"), &p);
        }
    }

    // Status icons: every effect-state id.
    for efst in i16::MIN..=i16::MAX {
        if let Some(info) = status_icon_info(efst) {
            add(format!("status_icon {efst}"), info.icon);
        }
    }

    // Sound tables.
    for id in 0u32..=12000 {
        let Ok(skill) = SkillEnum::try_from_value(id) else {
            continue;
        };
        if let Some((name, pos)) = skill_use_sound(skill) {
            add(format!("skill_use {id} {pos:?}"), name);
        }
        if let Some((name, pos)) = skill_cast_begin_sound(skill) {
            add(format!("skill_cast {id} {pos:?}"), name);
        }
        if let Some(name) = skill_projectile_sound(skill) {
            add(format!("skill_projectile {id}"), name);
        }
    }
    let weapons: Vec<Option<WeaponType>> = std::iter::once(None)
        .chain(weapon_types().into_iter().map(Some))
        .collect();
    for weapon in weapons {
        let label = weapon.map(|w| w.value() as i64).unwrap_or(-1);
        add(format!("swing {label}"), swing_sound(weapon));
        for roll in 0u32..4 {
            for taekwon in [false, true] {
                add(
                    format!("weapon_hit {label} {roll} {taekwon}"),
                    &weapon_hit_sound(weapon, roll, taekwon),
                );
            }
        }
    }
    for kind in [
        StatusSoundKind::FreezeEnter,
        StatusSoundKind::FreezeExit,
        StatusSoundKind::StoneCurseExit,
        StatusSoundKind::StunEnter,
        StatusSoundKind::PoisonSet,
        StatusSoundKind::CurseSet,
        StatusSoundKind::SilenceSet,
        StatusSoundKind::ConfusionSet,
        StatusSoundKind::BlindSet,
        StatusSoundKind::DetectOn,
    ] {
        if let Some(name) = status_sound(kind) {
            add(format!("status_sound {kind:?}"), name);
        }
    }
    add("ui button".to_string(), ui::BUTTON);
    add("ui login".to_string(), ui::LOGIN);
    add("ui repair".to_string(), ui::REPAIR);
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
