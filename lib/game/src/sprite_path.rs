pub use models::enums::weapon::WeaponType;

use crate::data_table::name_table::NameTable;
use crate::entity::{EntityCategory, EntityType};
use models::enums::EnumWithNumberValue;
use models::enums::class::JobName;
use models::enums::skill_enums::SkillEnum;

pub const JT_WARPNPC: u16 = 45;
pub const JT_EFFECTLAUNCHER: u16 = 104;
pub const JT_HIDDEN_NPC: u16 = 111;
pub const JT_HIDDEN_WARP_NPC: u16 = 139;
pub const JT_INVISIBLE: u16 = 32767;

pub const JT_ARCHER_GUARDIAN: u16 = 1285;
pub const JT_KNIGHT_GUARDIAN: u16 = 1286;
pub const JT_SOLDIER_GUARDIAN: u16 = 1287;

pub fn is_guardian(job: u16) -> bool {
    matches!(
        job,
        JT_ARCHER_GUARDIAN | JT_KNIGHT_GUARDIAN | JT_SOLDIER_GUARDIAN
    )
}

pub const SKILL_UNIT_JOB_MIN: u16 = 126;
pub const SKILL_UNIT_JOB_MAX: u16 = 201;

pub const HOMUNCULUS_JOB_MIN: u16 = 6001;
pub const HOMUNCULUS_JOB_MAX: u16 = 6016;
pub const MERCENARY_JOB_MIN: u16 = 6017;
pub const MERCENARY_JOB_MAX: u16 = 6046;

/// Inclusive job-id ranges, first match wins.
const JOB_CATEGORIES: &[(u16, u16, EntityCategory)] = &[
    (0, 44, EntityCategory::Player),
    (JT_WARPNPC, JT_WARPNPC, EntityCategory::WarpPoint),
    (46, 125, EntityCategory::Npc),
    (
        SKILL_UNIT_JOB_MIN,
        SKILL_UNIT_JOB_MAX,
        EntityCategory::Skill,
    ),
    (400, 999, EntityCategory::Npc),
    (1000, 3999, EntityCategory::Monster),
    (4001, 5999, EntityCategory::Player),
    (
        HOMUNCULUS_JOB_MIN,
        HOMUNCULUS_JOB_MAX,
        EntityCategory::Homunculus,
    ),
    (
        MERCENARY_JOB_MIN,
        MERCENARY_JOB_MAX,
        EntityCategory::Mercenary,
    ),
    (10000, 19999, EntityCategory::Npc),
    (JT_INVISIBLE, JT_INVISIBLE, EntityCategory::Invisible),
];

/// Pets are monster jobs, told apart by their head marker — see `Entity::category`.
pub fn entity_category_from_job(job: u16) -> EntityCategory {
    JOB_CATEGORIES
        .iter()
        .find(|(low, high, _)| job >= *low && job <= *high)
        .map_or(EntityCategory::Monster, |(_, _, category)| *category)
}

pub fn entity_type_from_job(job: u16) -> EntityType {
    match entity_category_from_job(job) {
        EntityCategory::Player => EntityType::Player,
        EntityCategory::Npc
        | EntityCategory::WarpPoint
        | EntityCategory::Skill
        | EntityCategory::Cart
        | EntityCategory::Invisible => EntityType::Npc,
        EntityCategory::Monster | EntityCategory::Pet => EntityType::Monster,
        EntityCategory::Homunculus => EntityType::Homunculus,
        EntityCategory::Mercenary => EntityType::Mercenary,
    }
}

pub fn npc_sprite_path(name: &str) -> String {
    ragnarok_resources::sprite::npc::of(name)
}

pub fn monster_sprite_path(name: &str) -> String {
    ragnarok_resources::sprite::monster::of(name)
}

pub fn homunculus_sprite_path(name: &str) -> String {
    ragnarok_resources::sprite::homun::of(name)
}

/// Mercenary bodies live with the human sprites; the name table already carries
/// the sex/type sub-path (backslashes are normalized to `/` by the GRF reader).
pub fn mercenary_sprite_path(name: &str) -> String {
    ragnarok_resources::sprite::player::mercenary_body(name)
}

pub fn mercenary_imf_path(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next()?;
    Some(ragnarok_resources::imf::of(base))
}

/// The mercenary weapon sprite sits under `mercenary`, named `<body>_<weapon>` where
/// the weapon is the text before the first `_` of the base name (`bow_mercenary` ->
/// `bow`, likewise `spear` / `sword`). The sex sub-path present on the body name is
/// dropped here.
pub fn mercenary_weapon_sprite_path(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next()?;
    let weapon = base.split('_').next().filter(|w| !w.is_empty())?;
    Some(ragnarok_resources::sprite::player::mercenary_weapon(
        base, weapon,
    ))
}

/// Trigger actors that are never drawn. The identity table maps all four to a
/// real sprite name — 104 and 111 to a townsfolk body, 139 to a Poring — so the
/// body has to be suppressed by job id.
pub fn is_undrawn_actor(job: u16) -> bool {
    matches!(
        job,
        JT_WARPNPC | JT_EFFECTLAUNCHER | JT_HIDDEN_NPC | JT_HIDDEN_WARP_NPC
    )
}

/// Undrawn actors that take no click either. The warp point answers with its own
/// cursor and the hidden NPC is a clickable trigger, so neither is listed.
pub fn is_inert_actor(job: u16) -> bool {
    matches!(job, JT_EFFECTLAUNCHER | JT_HIDDEN_WARP_NPC)
}

pub fn entity_sprite_base_path(name_table: &NameTable, job: u16) -> Option<String> {
    let name = name_table.get_name(job)?;
    if is_undrawn_actor(job) {
        return None;
    }
    match entity_type_from_job(job) {
        EntityType::Npc => Some(npc_sprite_path(name)),
        EntityType::Monster => Some(monster_sprite_path(name)),
        EntityType::Homunculus => Some(homunculus_sprite_path(name)),
        EntityType::Mercenary => Some(mercenary_sprite_path(name)),
        EntityType::Player => None,
    }
}

/// Homunculus type index used by the AI (1..16), from the job id.
pub fn homunculus_type_index(job: u16) -> u16 {
    job.saturating_sub(HOMUNCULUS_JOB_MIN) + 1
}

/// Mercenary type index used by the AI (1..30), from the job id.
pub fn mercenary_type_index(job: u16) -> u16 {
    job.saturating_sub(MERCENARY_JOB_MIN) + 1
}

pub const OPTION_FALCON: i32 = 0x10;
pub const OPTION_RIDING: i32 = 0x20;
pub const OPTION_WEDDING: i32 = 0x1000;
pub const OPTION_SANTA: i32 = 0x10000;
pub const OPTION_SUMMER: i32 = 0x40000;

/// Costume jobs: the body layer uses a whole-body sprite and the weapon and
/// shield layers are not loaded while a costume bit is set.
pub const JT_MARRIED: u16 = 22;
pub const JT_SANTA: u16 = 26;
pub const JT_SUMMER: u16 = 27;

pub const OPTION_ORCISH: i32 = 0x800;

/// Reverse Orcish (Sage): the head layer is drawn with the orc-face sprite.
pub const ORCFACE_SPRITE_PATH: &str = ragnarok_resources::sprite::effect::ORCFACE;

pub fn is_orcish(effect_state: i32) -> bool {
    (effect_state & OPTION_ORCISH) != 0
}

pub fn is_wedding(effect_state: i32) -> bool {
    (effect_state & OPTION_WEDDING) != 0
}

pub fn costume_job(effect_state: i32) -> Option<u16> {
    if effect_state & OPTION_WEDDING != 0 {
        Some(JT_MARRIED)
    } else if effect_state & OPTION_SANTA != 0 {
        Some(JT_SANTA)
    } else if effect_state & OPTION_SUMMER != 0 {
        Some(JT_SUMMER)
    } else {
        None
    }
}

pub fn is_costume_job(job: u16) -> bool {
    matches!(job, JT_MARRIED | JT_SANTA | JT_SUMMER)
}

pub const EFST_RIDING: i16 = 27;
pub const EFST_FALCON: i16 = 28;
/// Maya Purple: the wearer sees stealthed actors as black silhouettes.
pub const EFST_CLAIRVOYANCE: i16 = 184;

/// OPTION bits the server delivers only as a bitmask (no status packet), paired with the
/// status-bar icon the client synthesizes for the local player when the bit toggles.
pub const OPTION_STATUS_ICONS: &[(i32, i16)] =
    &[(OPTION_FALCON, EFST_FALCON), (OPTION_RIDING, EFST_RIDING)];

pub fn has_falcon(effect_state: i32) -> bool {
    (effect_state & OPTION_FALCON) != 0
}

pub fn falcon_sprite_path(job: u16) -> &'static str {
    match job {
        4012 => ragnarok_resources::sprite::effect::FALCON2,
        _ => ragnarok_resources::sprite::effect::FALCON,
    }
}
pub const OPTION_CART_MASK: i32 = 0x08 | 0x80 | 0x100 | 0x200 | 0x400;
pub const OPTION_REMOVABLE_MASK: i32 = OPTION_FALCON | OPTION_RIDING | OPTION_CART_MASK;

pub const OPTION_SIGHT: i32 = 0x01;
pub const OPTION_RUWACH: i32 = 0x2000;
pub const OPTION_HIDE: i32 = 0x02;
pub const OPTION_CLOAK: i32 = 0x04;
pub const OPTION_CHASEWALK: i32 = 0x4000;
pub const OPTION_HIDDEN_MASK: i32 = OPTION_HIDE | OPTION_CLOAK | OPTION_CHASEWALK;

pub const CLOAK_BODY_ALPHA: f32 = 100.0 / 255.0;

/// Relationship of the viewer to the stealthed actor. Self and party members
/// (and, later, the caster's own pet) still see a cloaked body; everyone else
/// sees nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenViewer {
    Own,
    Ally,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HiddenRender {
    Visible,
    /// Draw only the shadow (Hiding for the local player).
    ShadowOnly,
    Alpha(f32),
    /// Body at full opacity with its colour crushed to black, and no shadow.
    Silhouette,
    Skip,
}

pub fn is_hidden(effect_state: i32) -> bool {
    (effect_state & OPTION_HIDDEN_MASK) != 0
}

/// A hidden local player cannot walk unless Tunnel Drive is known; cloak and
/// chase walk never block movement.
pub fn hide_blocks_move(effect_state: i32, knows_tunnel_drive: bool) -> bool {
    (effect_state & OPTION_HIDE) != 0 && !knows_tunnel_drive
}

/// The few skills usable while Hiding (TF_HIDING to unhide, plus the ambush
/// attacks). Every other action is blocked until the bit clears.
pub fn hide_allows_skill(skill: SkillEnum) -> bool {
    matches!(
        skill,
        SkillEnum::TfHiding | SkillEnum::AsGrimtooth | SkillEnum::RgBackstap | SkillEnum::RgRaid
    )
}

pub fn hidden_render(effect_state: i32, viewer: HiddenViewer, clairvoyant: bool) -> HiddenRender {
    use HiddenViewer::*;
    let render = if effect_state & OPTION_HIDE != 0 {
        match viewer {
            Own => HiddenRender::ShadowOnly,
            _ => HiddenRender::Skip,
        }
    } else if effect_state & (OPTION_CLOAK | OPTION_CHASEWALK) != 0 {
        match viewer {
            Own | Ally => HiddenRender::Alpha(CLOAK_BODY_ALPHA),
            Other => HiddenRender::Skip,
        }
    } else {
        HiddenRender::Visible
    };
    if clairvoyant && render == HiddenRender::Skip {
        HiddenRender::Silhouette
    } else {
        render
    }
}

pub fn mounted_job(job: u16) -> Option<u16> {
    match job {
        7 => Some(13),
        14 => Some(21),
        4008 => Some(4014),
        4015 => Some(4022),
        4030 => Some(4036),
        4037 => Some(4044),
        _ => None,
    }
}

pub fn visual_job(job: u16, effect_state: i32) -> u16 {
    if let Some(costume) = costume_job(effect_state) {
        costume
    } else if (effect_state & OPTION_RIDING) != 0 {
        mounted_job(job).unwrap_or(job)
    } else {
        job
    }
}

pub fn cart_design_from_option(effect_state: i32) -> Option<u8> {
    match effect_state & OPTION_CART_MASK {
        0x08 => Some(1),
        0x80 => Some(2),
        0x100 => Some(3),
        0x200 => Some(4),
        0x400 => Some(5),
        _ => None,
    }
}

pub fn cart_sprite_path(design: u8) -> String {
    ragnarok_resources::sprite::effect::cart(design)
}

pub fn unmounted_job(job: u16) -> Option<u16> {
    match job {
        13 => Some(7),
        21 => Some(14),
        4014 => Some(4008),
        4022 => Some(4015),
        4036 => Some(4030),
        4044 => Some(4037),
        _ => None,
    }
}

pub fn is_baby(job: u16) -> bool {
    (4023..=4045).contains(&job)
}

/// Baby classes reuse their base job's resources, drawn smaller — first-class
/// babies (and the Baby Super Novice) smaller than second-class ones. `1.0` for
/// every other job.
pub fn baby_body_scale(job: u16) -> f32 {
    if !is_baby(job) {
        return 1.0;
    }
    match base_job(job) {
        0..=6 | 23 => 0.75,
        _ => 0.82,
    }
}

/// The adult job a baby class draws as. Identity for non-baby jobs.
pub fn base_job(job: u16) -> u16 {
    match job {
        4045 => 23,
        4023..=4044 => job - 4023,
        _ => job,
    }
}

fn job_name_kr(job_class: u16) -> &'static str {
    match base_job(job_class) {
        0 => "novice",
        1 => "swordman",
        2 => "magician",
        3 => "archer",
        4 => "acolyte",
        5 => "merchant",
        6 => "thief",
        7 => "knight",
        8 => "priest",
        9 => "wizard",
        10 => "blacksmith",
        11 => "hunter",
        12 => "assassin",
        13 => "peco_peco_knight",
        14 => "crusader",
        15 => "monk",
        16 => "sage",
        17 => "rogue",
        18 => "alchemist",
        19 => "bard",
        20 => "dancer",
        21 => "new_peco_crusader",
        22 => "wedding",
        23 => "super_novice",
        24 => "gunslinger",
        25 => "ninja",
        26 => "santa",
        27 => "summer",
        4001 => "novice",
        4002 => "swordman",
        4003 => "magician",
        4004 => "archer",
        4005 => "acolyte",
        4006 => "merchant",
        4007 => "thief",
        4008 => "lord_knight",
        4009 => "high_priest",
        4010 => "high_wizard",
        4011 => "whitesmith",
        4012 => "sniper",
        4013 => "assassin_cross",
        4014 => "lord_peco",
        4015 => "paladin",
        4016 => "champion",
        4017 => "professor",
        4018 => "stalker",
        4019 => "creator",
        4020 => "clown",
        4021 => "gypsy",
        4022 => "peco_paladin",
        4046 => "taekwon_boy",
        4047 => "star_gladiator",
        4048 => "star_gladiator_union",
        4049 => "soul_linker",
        _ => "novice",
    }
}

fn sex_kr(sex: u8) -> &'static str {
    if sex == 0 { "female" } else { "male" }
}

pub fn body_sprite_path(job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::player::body(job, sex_str)
}

pub fn gm_body_sprite_path(sex: u8) -> String {
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::player::gm_body(sex_str)
}

pub fn gm_weapon_sprite_path(sex: u8) -> String {
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::player::gm_weapon(sex_str)
}

pub fn imf_path(job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    ragnarok_resources::imf::for_job(job, sex_str)
}

/// The job whose IMF a class borrows when it ships none of its own: transcendent
/// classes fall back to their pre-trans form, the seasonal costumes to the
/// wedding outfit.
pub fn imf_fallback_job(job_class: u16) -> Option<u16> {
    match base_job(job_class) {
        job @ 4001..=4022 => Some(job - 4001),
        4047 | 4048 => Some(4046),
        26 | 27 => Some(22),
        _ => None,
    }
}

/// The hair style a character wears is an index, not a file number: each sex
/// resolves it through its own table.
const HEAD_NAMES_MALE: [&str; 26] = [
    "2", "2", "1", "7", "5", "4", "3", "6", "8", "9", "10", "12", "11", "13", "14", "15", "16",
    "17", "18", "19", "20", "21", "22", "23", "24", "25",
];

const HEAD_NAMES_FEMALE: [&str; 26] = [
    "2", "2", "4", "7", "1", "5", "3", "6", "12", "10", "9", "11", "8", "13", "14", "15", "16",
    "17", "18", "19", "20", "21", "22", "23", "24", "25",
];

fn clamp_head_id(head_id: u16) -> u16 {
    if head_id as usize >= HEAD_NAMES_MALE.len() {
        13
    } else {
        head_id
    }
}

fn head_name(head_id: u16, sex: u8) -> &'static str {
    let table = if sex == 0 {
        &HEAD_NAMES_FEMALE
    } else {
        &HEAD_NAMES_MALE
    };
    table[head_id as usize]
}

pub fn head_sprite_path(head_id: u16, sex: u8) -> String {
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::player::head(head_name(clamp_head_id(head_id), sex), sex_str)
}

/// Index 0 means "whatever suits the job", and only the palette resolves it;
/// the sprite keeps the table entry for 0.
pub fn head_palette_path(head_id: u16, job_class: u16, sex: u8, palette_id: u16) -> String {
    let head_id = match clamp_head_id(head_id) {
        0 => match job_class {
            1..=6 => job_class + 1,
            _ => 1,
        },
        id => id,
    };
    let sex_str = sex_kr(sex);
    ragnarok_resources::palette::head(head_name(head_id, sex), sex_str, palette_id)
}

pub fn body_palette_path(job_class: u16, sex: u8, palette_id: u16) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    ragnarok_resources::palette::body(job, sex_str, palette_id)
}

fn weapon_suffix(weapon_type: WeaponType) -> &'static str {
    match weapon_type {
        WeaponType::Dagger => "_dagger",
        WeaponType::Sword1H | WeaponType::Sword2H => "_sword",
        WeaponType::Spear1H | WeaponType::Spear2H => "_spear",
        WeaponType::Axe1H | WeaponType::Axe2H => "_axe",
        WeaponType::Mace | WeaponType::Mace2H => "_club",
        WeaponType::Staff | WeaponType::Staff2H => "_rod",
        WeaponType::Bow => "_bow",
        WeaponType::Knuckle => "_knuckle",
        WeaponType::Musical => "_instrument",
        WeaponType::Whip => "_whip",
        WeaponType::Book => "_book",
        WeaponType::Katar => "_katar_katar",
        WeaponType::DoubleDd => "_dagger_dagger",
        WeaponType::DoubleSs => "_sword_sword",
        WeaponType::DoubleAa => "_axe_axe",
        WeaponType::DoubleDs => "_dagger_sword",
        WeaponType::DoubleDa => "_dagger_axe",
        WeaponType::DoubleSa => "_sword_axe",
        WeaponType::Revolver => "_pistol",
        WeaponType::Rifle | WeaponType::Gatling | WeaponType::Shotgun | WeaponType::Grenade => {
            "_machine_gun"
        }
        WeaponType::Huuma => "_shuriken",
        _ => "_sword",
    }
}

pub fn weapon_view_id_to_type(id: u16) -> Option<WeaponType> {
    match id {
        0 => None,
        1 => Some(WeaponType::Dagger),
        2 => Some(WeaponType::Sword1H),
        3 => Some(WeaponType::Sword2H),
        4 => Some(WeaponType::Spear1H),
        5 => Some(WeaponType::Spear2H),
        6 => Some(WeaponType::Axe1H),
        7 => Some(WeaponType::Axe2H),
        8 => Some(WeaponType::Mace),
        9 => Some(WeaponType::Mace2H),
        10 => Some(WeaponType::Staff),
        11 => Some(WeaponType::Bow),
        12 => Some(WeaponType::Knuckle),
        13 => Some(WeaponType::Musical),
        14 => Some(WeaponType::Whip),
        15 => Some(WeaponType::Book),
        16 => Some(WeaponType::Katar),
        17 => Some(WeaponType::Revolver),
        18 => Some(WeaponType::Rifle),
        19 => Some(WeaponType::Gatling),
        20 => Some(WeaponType::Shotgun),
        21 => Some(WeaponType::Grenade),
        22 => Some(WeaponType::Huuma),
        23 => Some(WeaponType::Staff2H),
        25 => Some(WeaponType::DoubleDd),
        26 => Some(WeaponType::DoubleSs),
        27 => Some(WeaponType::DoubleAa),
        28 => Some(WeaponType::DoubleDs),
        29 => Some(WeaponType::DoubleDa),
        30 => Some(WeaponType::DoubleSa),
        _ => weapon_type_from_item_id(id),
    }
}

fn weapon_type_from_item_id(id: u16) -> Option<WeaponType> {
    if id < 1100 {
        return None;
    }
    if (1116..=1118).contains(&id) {
        return Some(WeaponType::Sword2H);
    }
    if (1314..=1315).contains(&id) {
        return Some(WeaponType::Axe2H);
    }
    if (1410..=1412).contains(&id) {
        return Some(WeaponType::Spear2H);
    }
    if (1472..=1473).contains(&id) {
        return Some(WeaponType::Staff);
    }
    if id == 1599 {
        return Some(WeaponType::Mace);
    }
    match id {
        1100..1150 => Some(WeaponType::Sword1H),
        1150..1200 => Some(WeaponType::Sword2H),
        1200..1250 => Some(WeaponType::Dagger),
        1250..1300 => Some(WeaponType::Katar),
        1300..1350 => Some(WeaponType::Axe1H),
        1350..1400 => Some(WeaponType::Axe2H),
        1400..1450 => Some(WeaponType::Spear1H),
        1450..1500 => Some(WeaponType::Spear2H),
        1500..1550 => Some(WeaponType::Mace),
        1550..1600 => Some(WeaponType::Book),
        1600..1700 => Some(WeaponType::Staff),
        1700..1750 => Some(WeaponType::Bow),
        1800..1900 => Some(WeaponType::Knuckle),
        1900..1950 => Some(WeaponType::Musical),
        1950..2000 => Some(WeaponType::Whip),
        2000..2100 => Some(WeaponType::Staff2H),
        13000..13100 => Some(WeaponType::Dagger),
        13100..13150 => Some(WeaponType::Revolver),
        13150..13200 => Some(WeaponType::Rifle),
        13300..13400 => Some(WeaponType::Huuma),
        13400..13500 => Some(WeaponType::Sword1H),
        _ => None,
    }
}

pub fn dual_wield_type(right: WeaponType, left: WeaponType) -> Option<WeaponType> {
    match (right, left) {
        (WeaponType::Dagger, WeaponType::Dagger) => Some(WeaponType::DoubleDd),
        (WeaponType::Sword1H, WeaponType::Sword1H) => Some(WeaponType::DoubleSs),
        (WeaponType::Axe1H, WeaponType::Axe1H) => Some(WeaponType::DoubleAa),
        (WeaponType::Dagger, WeaponType::Sword1H) | (WeaponType::Sword1H, WeaponType::Dagger) => {
            Some(WeaponType::DoubleDs)
        }
        (WeaponType::Dagger, WeaponType::Axe1H) | (WeaponType::Axe1H, WeaponType::Dagger) => {
            Some(WeaponType::DoubleDa)
        }
        (WeaponType::Sword1H, WeaponType::Axe1H) | (WeaponType::Axe1H, WeaponType::Sword1H) => {
            Some(WeaponType::DoubleSa)
        }
        _ => None,
    }
}

pub fn headgear_sprite_path(suffix: &str, sex: u8) -> String {
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::accessory::of(sex_str, suffix)
}

fn shield_name_kr(view_id: u16) -> Option<&'static str> {
    match view_id {
        1 => Some("guard"),
        2 => Some("buckler"),
        3 => Some("shield"),
        4 => Some("mirror_shield"),
        _ => None,
    }
}

fn shield_view_from_item_id(id: u16) -> Option<u16> {
    match id {
        2101 | 2102 | 2112 | 2116..=2120 => Some(1),
        2103 | 2104 | 2114 | 2126 => Some(2),
        2105 | 2106 | 2113 => Some(3),
        2107 | 2108 | 2110 | 2111 | 2115 | 2127 | 2128 => Some(4),
        _ => None,
    }
}

pub fn resolve_shield_view_id(id: u16) -> u16 {
    if (1..=4).contains(&id) {
        return id;
    }
    shield_view_from_item_id(id).unwrap_or(id)
}

pub fn shield_sprite_path(view_id: u16, job_class: u16, sex: u8) -> Option<String> {
    let resolved = resolve_shield_view_id(view_id);
    let shield = shield_name_kr(resolved)?;
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    Some(ragnarok_resources::sprite::shield::of(job, sex_str, shield))
}

pub fn shield_sprite_path_numeric(view_id: u16, job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::shield::of(job, sex_str, view_id)
}

pub fn transcendent_to_base_class(job_class: u16) -> Option<JobName> {
    let job = JobName::try_from_value(job_class as usize).ok()?;
    match job {
        JobName::LordKnight => Some(JobName::Knight),
        JobName::HighPriest => Some(JobName::Priest),
        JobName::HighWizard => Some(JobName::Wizard),
        JobName::Whitesmith => Some(JobName::Blacksmith),
        JobName::Sniper => Some(JobName::Hunter),
        JobName::AssassinCross => Some(JobName::Assassin),
        JobName::Paladin => Some(JobName::Crusader),
        JobName::Champion => Some(JobName::Monk),
        JobName::Professor => Some(JobName::Sage),
        JobName::Stalker => Some(JobName::Rogue),
        JobName::Creator => Some(JobName::Alchemist),
        JobName::Clown => Some(JobName::Bard),
        JobName::Gypsy => Some(JobName::Dancer),
        _ => None,
    }
}

pub fn weapon_sprite_path(job_class: u16, sex: u8, weapon_type: WeaponType) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    let suffix = weapon_suffix(weapon_type);
    ragnarok_resources::sprite::player::weapon(job, sex_str, suffix)
}

/// Weapon art named after the item id rather than the weapon type. The original
/// game reaches for this first and only falls back to [`weapon_sprite_path`]
/// when the archive has no such file.
pub fn weapon_item_sprite_path(job_class: u16, sex: u8, item_id: u16) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    ragnarok_resources::sprite::player::weapon_by_item(job, sex_str, item_id)
}

pub fn weapon_item_trail_sprite_path(
    job_class: u16,
    sex: u8,
    item_id: u16,
    weapon_type: WeaponType,
) -> Option<String> {
    weapon_has_trail(weapon_type).then(|| {
        format!(
            "{}_sword_flash",
            weapon_item_sprite_path(job_class, sex, item_id)
        )
    })
}

pub fn weapon_has_trail(weapon_type: WeaponType) -> bool {
    matches!(
        weapon_type,
        WeaponType::Dagger
            | WeaponType::Sword1H
            | WeaponType::Sword2H
            | WeaponType::Spear1H
            | WeaponType::Spear2H
            | WeaponType::Axe1H
            | WeaponType::Axe2H
            | WeaponType::Katar
            | WeaponType::DoubleDd
            | WeaponType::DoubleSs
            | WeaponType::DoubleAa
            | WeaponType::DoubleDs
            | WeaponType::DoubleDa
            | WeaponType::DoubleSa
            | WeaponType::Revolver
            | WeaponType::Rifle
            | WeaponType::Gatling
            | WeaponType::Shotgun
            | WeaponType::Grenade
    )
}

pub fn weapon_trail_sprite_path(
    job_class: u16,
    sex: u8,
    weapon_type: WeaponType,
) -> Option<String> {
    weapon_has_trail(weapon_type).then(|| {
        format!(
            "{}_sword_flash",
            weapon_sprite_path(job_class, sex, weapon_type)
        )
    })
}

/// The kick flash a TaeKwon-line actor wears in place of a weapon.
pub fn kick_glow_sprite_path(job_class: u16, sex: u8) -> String {
    ragnarok_resources::sprite::player::weapon(job_name_kr(job_class), sex_kr(sex), "_glow")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercenary_weapon_path_uses_text_before_first_underscore() {
        assert_eq!(
            mercenary_weapon_sprite_path("female\\bow_mercenary").as_deref(),
            Some("data/sprite/human/mercenary/bow_mercenary_bow")
        );
        assert_eq!(
            mercenary_weapon_sprite_path("male\\spear_mercenary").as_deref(),
            Some("data/sprite/human/mercenary/spear_mercenary_spear")
        );
        assert_eq!(
            mercenary_weapon_sprite_path("male/sword_mercenary").as_deref(),
            Some("data/sprite/human/mercenary/sword_mercenary_sword")
        );
        assert_eq!(mercenary_weapon_sprite_path("_mercenary"), None);
    }

    #[test]
    fn novice_male_path() {
        assert_eq!(
            body_sprite_path(0, 1),
            "data/sprite/human/body/male/novice_male"
        );
    }

    #[test]
    fn novice_female_path() {
        assert_eq!(
            body_sprite_path(0, 0),
            "data/sprite/human/body/female/novice_female"
        );
    }

    #[test]
    fn knight_male_path() {
        assert_eq!(
            body_sprite_path(7, 1),
            "data/sprite/human/body/male/knight_male"
        );
    }

    #[test]
    fn lord_knight_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4008, 1),
            "data/sprite/human/body/male/lord_knight_male"
        );
    }

    #[test]
    fn clown_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4020, 1),
            "data/sprite/human/body/male/clown_male"
        );
    }

    #[test]
    fn high_priest_path() {
        assert_eq!(
            body_sprite_path(4009, 1),
            "data/sprite/human/body/male/high_priest_male"
        );
    }

    #[test]
    fn unknown_class_falls_back_to_novice() {
        assert_eq!(
            body_sprite_path(9999, 1),
            "data/sprite/human/body/male/novice_male"
        );
    }

    #[test]
    fn hidden_detects_hide_cloak_and_chasewalk() {
        assert!(is_hidden(OPTION_HIDE));
        assert!(is_hidden(OPTION_CLOAK));
        assert!(is_hidden(OPTION_CHASEWALK));
        assert!(is_hidden(OPTION_RIDING | OPTION_CLOAK));
        assert!(!is_hidden(0));
        assert!(!is_hidden(OPTION_RIDING));
    }

    #[test]
    fn hidden_render_is_per_state_and_viewer_aware() {
        use HiddenRender::*;
        use HiddenViewer::*;
        assert_eq!(hidden_render(0, Own, false), Visible);
        assert_eq!(hidden_render(OPTION_RIDING, Other, false), Visible);

        assert_eq!(
            hidden_render(OPTION_CLOAK, Own, false),
            Alpha(CLOAK_BODY_ALPHA)
        );
        assert_eq!(
            hidden_render(OPTION_CLOAK, Ally, false),
            Alpha(CLOAK_BODY_ALPHA)
        );
        assert_eq!(hidden_render(OPTION_CLOAK, Other, false), Skip);

        assert_eq!(hidden_render(OPTION_HIDE, Own, false), ShadowOnly);
        assert_eq!(hidden_render(OPTION_HIDE, Ally, false), Skip);
        assert_eq!(hidden_render(OPTION_HIDE, Other, false), Skip);

        // Hide beats cloak when both are set.
        assert_eq!(
            hidden_render(OPTION_HIDE | OPTION_CLOAK, Own, false),
            ShadowOnly
        );
        assert_eq!(hidden_render(OPTION_HIDE | OPTION_CLOAK, Ally, false), Skip);

        let chasewalk = OPTION_CHASEWALK | OPTION_CLOAK;
        assert_eq!(
            hidden_render(chasewalk, Own, false),
            Alpha(CLOAK_BODY_ALPHA)
        );
        assert_eq!(
            hidden_render(chasewalk, Ally, false),
            Alpha(CLOAK_BODY_ALPHA)
        );
        assert_eq!(hidden_render(chasewalk, Other, false), Skip);
    }

    #[test]
    fn maya_purple_turns_an_unseen_body_into_a_silhouette() {
        use HiddenRender::*;
        use HiddenViewer::*;
        assert_eq!(hidden_render(OPTION_CLOAK, Other, true), Silhouette);
        assert_eq!(hidden_render(OPTION_HIDE, Other, true), Silhouette);
        assert_eq!(hidden_render(OPTION_HIDE, Ally, true), Silhouette);

        assert_eq!(hidden_render(0, Other, true), Visible);
        assert_eq!(hidden_render(OPTION_HIDE, Own, true), ShadowOnly);
        assert_eq!(
            hidden_render(OPTION_CLOAK, Ally, true),
            Alpha(CLOAK_BODY_ALPHA)
        );
    }

    #[test]
    fn hide_blocks_move_only_without_tunnel_drive() {
        use crate::skill::SkillList;

        let mut list = SkillList::new();
        list.set_skills(vec![crate::skill::SkillData {
            skill: SkillEnum::SmBash,
            level: 5,
            selected_level: 5,
            sp_cost: 10,
            attack_range: 1,
            upgradable: true,
            skill_target_type: crate::skill::SkillTargetType::Target,
        }]);
        let knows = |l: &SkillList| l.get_skill(SkillEnum::RgTunneldrive).is_some();

        assert!(hide_blocks_move(OPTION_HIDE, knows(&list)));
        assert!(!hide_blocks_move(OPTION_CLOAK, knows(&list)));
        assert!(!hide_blocks_move(
            OPTION_CLOAK | OPTION_CHASEWALK,
            knows(&list)
        ));

        list.add_skill(crate::skill::SkillData {
            skill: SkillEnum::RgTunneldrive,
            level: 1,
            selected_level: 1,
            sp_cost: 0,
            attack_range: 0,
            upgradable: false,
            skill_target_type: crate::skill::SkillTargetType::Passive,
        });
        assert!(!hide_blocks_move(OPTION_HIDE, knows(&list)));
    }

    #[test]
    fn imf_path_falls_back_to_the_base_class() {
        assert_eq!(imf_path(6, 1), "data/imf/thief_male.imf");
        assert_eq!(imf_fallback_job(6), None);
        assert_eq!(imf_path(4030, 1), "data/imf/knight_male.imf");
        assert_eq!(
            imf_fallback_job(4013).map(|j| imf_path(j, 1)),
            Some("data/imf/assassin_male.imf".to_string())
        );
        assert_eq!(
            imf_fallback_job(26).map(|j| imf_path(j, 0)),
            Some("data/imf/wedding_female.imf".to_string())
        );
    }

    #[test]
    fn head_index_maps_per_sex() {
        assert_eq!(head_sprite_path(1, 1), "data/sprite/human/head/male/2_male");
        assert_eq!(head_sprite_path(2, 1), "data/sprite/human/head/male/1_male");
        assert_eq!(
            head_sprite_path(2, 0),
            "data/sprite/human/head/female/4_female"
        );
        assert_eq!(
            head_sprite_path(8, 0),
            "data/sprite/human/head/female/12_female"
        );
        assert_eq!(
            head_sprite_path(20, 0),
            "data/sprite/human/head/female/20_female"
        );
        assert_eq!(
            head_sprite_path(99, 1),
            "data/sprite/human/head/male/13_male"
        );
    }

    #[test]
    fn knight_dagger_weapon_path() {
        assert_eq!(
            weapon_sprite_path(7, 1, WeaponType::Dagger),
            "data/sprite/human/knight/knight_male_dagger"
        );
    }

    #[test]
    fn novice_sword_weapon_path() {
        assert_eq!(
            weapon_sprite_path(0, 1, WeaponType::Sword1H),
            "data/sprite/human/novice/novice_male_sword"
        );
    }

    #[test]
    fn staff_weapon_path() {
        assert_eq!(
            weapon_sprite_path(2, 1, WeaponType::Staff),
            "data/sprite/human/magician/magician_male_rod"
        );
    }

    #[test]
    fn female_weapon_path() {
        assert_eq!(
            weapon_sprite_path(7, 0, WeaponType::Spear1H),
            "data/sprite/human/knight/knight_female_spear"
        );
    }

    #[test]
    fn only_bladed_weapons_have_a_trail() {
        let with_trail = [
            WeaponType::Dagger,
            WeaponType::Sword1H,
            WeaponType::Sword2H,
            WeaponType::Spear1H,
            WeaponType::Spear2H,
            WeaponType::Axe1H,
            WeaponType::Axe2H,
            WeaponType::Katar,
            WeaponType::DoubleDd,
            WeaponType::DoubleSs,
            WeaponType::DoubleAa,
            WeaponType::DoubleDs,
            WeaponType::DoubleDa,
            WeaponType::DoubleSa,
            WeaponType::Revolver,
            WeaponType::Rifle,
            WeaponType::Gatling,
            WeaponType::Shotgun,
            WeaponType::Grenade,
        ];
        let without_trail = [
            WeaponType::Fist,
            WeaponType::Mace,
            WeaponType::Mace2H,
            WeaponType::Staff,
            WeaponType::Staff2H,
            WeaponType::Bow,
            WeaponType::Knuckle,
            WeaponType::Musical,
            WeaponType::Whip,
            WeaponType::Book,
            WeaponType::Huuma,
            WeaponType::Shuriken,
        ];
        for wt in with_trail {
            assert!(weapon_has_trail(wt), "{wt:?} should have a trail");
        }
        for wt in without_trail {
            assert!(!weapon_has_trail(wt), "{wt:?} should have no trail");
            assert!(weapon_trail_sprite_path(7, 1, wt).is_none());
        }
        assert_eq!(
            weapon_trail_sprite_path(7, 1, WeaponType::Sword2H).as_deref(),
            Some("data/sprite/human/knight/knight_male_sword_sword_flash")
        );
    }

    #[test]
    fn taekwon_kick_glow_is_named_after_the_job() {
        assert_eq!(
            kick_glow_sprite_path(4046, 1),
            "data/sprite/human/taekwon_boy/taekwon_boy_male_glow"
        );
        assert_eq!(
            kick_glow_sprite_path(4046, 0),
            "data/sprite/human/taekwon_boy/taekwon_boy_female_glow"
        );
    }

    #[test]
    fn weapon_view_id_zero_is_none() {
        assert!(weapon_view_id_to_type(0).is_none());
    }

    #[test]
    fn weapon_view_id_maps_correctly() {
        assert_eq!(weapon_view_id_to_type(1), Some(WeaponType::Dagger));
        assert_eq!(weapon_view_id_to_type(2), Some(WeaponType::Sword1H));
        assert_eq!(weapon_view_id_to_type(11), Some(WeaponType::Bow));
        assert_eq!(weapon_view_id_to_type(16), Some(WeaponType::Katar));
        assert_eq!(weapon_view_id_to_type(25), Some(WeaponType::DoubleDd));
        assert_eq!(weapon_view_id_to_type(30), Some(WeaponType::DoubleSa));
    }

    #[test]
    fn weapon_item_id_fallback() {
        assert_eq!(weapon_view_id_to_type(1101), Some(WeaponType::Sword1H));
        assert_eq!(weapon_view_id_to_type(1201), Some(WeaponType::Dagger));
        assert_eq!(weapon_view_id_to_type(1701), Some(WeaponType::Bow));
        assert_eq!(weapon_view_id_to_type(1250), Some(WeaponType::Katar));
        assert_eq!(weapon_view_id_to_type(1450), Some(WeaponType::Spear2H));
        assert_eq!(weapon_view_id_to_type(1116), Some(WeaponType::Sword2H));
        assert!(weapon_view_id_to_type(999).is_none());
    }

    #[test]
    fn gunslinger_guns_resolve_to_their_art() {
        const GUNSLINGER: u16 = 24;

        // Revolvers share one sprite, everything else shares the gatling one.
        assert_eq!(weapon_view_id_to_type(13100), Some(WeaponType::Revolver));
        assert_eq!(
            weapon_sprite_path(GUNSLINGER, 1, WeaponType::Revolver),
            "data/sprite/human/gunslinger/gunslinger_male_pistol"
        );
        assert_eq!(weapon_view_id_to_type(13154), Some(WeaponType::Rifle));
        assert_eq!(
            weapon_sprite_path(GUNSLINGER, 1, WeaponType::Rifle),
            "data/sprite/human/gunslinger/gunslinger_male_machine_gun"
        );

        // Individual guns carry their own art, keyed by item id.
        assert_eq!(
            weapon_item_sprite_path(GUNSLINGER, 1, 13154),
            "data/sprite/human/gunslinger/gunslinger_male_13154"
        );
        assert_eq!(
            weapon_item_trail_sprite_path(GUNSLINGER, 0, 13154, WeaponType::Rifle).as_deref(),
            Some("data/sprite/human/gunslinger/gunslinger_female_13154_sword_flash")
        );

        // A weapon look the archive cannot name a file after.
        assert!(weapon_view_id_to_type(13200).is_none()); // bullets are not a look
        assert_eq!(weapon_view_id_to_type(17), Some(WeaponType::Revolver));
    }

    #[test]
    fn entity_type_from_job_boundaries() {
        use crate::entity::EntityType;
        assert_eq!(entity_type_from_job(0), EntityType::Player);
        assert_eq!(entity_type_from_job(44), EntityType::Player);
        assert_eq!(entity_type_from_job(45), EntityType::Npc);
        assert_eq!(entity_type_from_job(46), EntityType::Npc);
        assert_eq!(entity_type_from_job(999), EntityType::Npc);
        assert_eq!(entity_type_from_job(1000), EntityType::Monster);
        assert_eq!(entity_type_from_job(1002), EntityType::Monster);
        assert_eq!(entity_type_from_job(3999), EntityType::Monster);
        assert_eq!(entity_type_from_job(4001), EntityType::Player);
        assert_eq!(entity_type_from_job(5999), EntityType::Player);
        assert_eq!(entity_type_from_job(10000), EntityType::Npc);
    }

    #[test]
    fn entity_category_from_job_boundaries() {
        let cases = [
            (JT_WARPNPC, EntityCategory::WarpPoint),
            (46, EntityCategory::Npc),
            (JT_EFFECTLAUNCHER, EntityCategory::Npc),
            (JT_HIDDEN_NPC, EntityCategory::Npc),
            (125, EntityCategory::Npc),
            (SKILL_UNIT_JOB_MIN, EntityCategory::Skill),
            (JT_HIDDEN_WARP_NPC, EntityCategory::Skill),
            (SKILL_UNIT_JOB_MAX, EntityCategory::Skill),
            (400, EntityCategory::Npc),
            (1000, EntityCategory::Monster),
            (10000, EntityCategory::Npc),
            (19999, EntityCategory::Npc),
            (HOMUNCULUS_JOB_MIN, EntityCategory::Homunculus),
            (MERCENARY_JOB_MAX, EntityCategory::Mercenary),
            (JT_INVISIBLE, EntityCategory::Invisible),
        ];
        for (job, expected) in cases {
            assert_eq!(entity_category_from_job(job), expected, "job {job}");
        }
    }

    #[test]
    fn only_castle_guardians_are_guardians() {
        assert!(is_guardian(JT_ARCHER_GUARDIAN));
        assert!(is_guardian(JT_KNIGHT_GUARDIAN));
        assert!(is_guardian(JT_SOLDIER_GUARDIAN));
        // Emperium and the castle flag.
        assert!(!is_guardian(1288));
        assert!(!is_guardian(722));
    }

    #[test]
    fn body_palette_path_formats_correctly() {
        assert_eq!(
            body_palette_path(1, 1, 3),
            "data/palette/body/swordman_male_3.pal"
        );
        assert_eq!(
            body_palette_path(0, 0, 1),
            "data/palette/body/novice_female_1.pal"
        );
    }

    #[test]
    fn head_palette_path_formats_correctly() {
        assert_eq!(
            head_palette_path(1, 0, 1, 3),
            "data/palette/hair/hair2_male_3.pal"
        );
        assert_eq!(
            head_palette_path(5, 0, 0, 7),
            "data/palette/hair/hair5_female_7.pal"
        );
    }

    #[test]
    fn head_zero_palette_follows_the_job() {
        assert_eq!(
            head_palette_path(0, 1, 1, 2),
            "data/palette/hair/hair1_male_2.pal"
        );
        assert_eq!(
            head_palette_path(0, 0, 1, 2),
            "data/palette/hair/hair2_male_2.pal"
        );
        assert_eq!(head_sprite_path(0, 1), "data/sprite/human/head/male/2_male");
    }

    #[test]
    fn npc_and_monster_sprite_paths() {
        assert_eq!(npc_sprite_path("1_ETC_01"), "data/sprite/npc/1_ETC_01");
        assert_eq!(monster_sprite_path("Poring"), "data/sprite/monster/Poring");
    }

    #[test]
    fn shield_view_id_resolution() {
        assert_eq!(resolve_shield_view_id(1), 1);
        assert_eq!(resolve_shield_view_id(2), 2);
        assert_eq!(resolve_shield_view_id(4), 4);
        assert_eq!(resolve_shield_view_id(2101), 1);
        assert_eq!(resolve_shield_view_id(2103), 2);
        assert_eq!(resolve_shield_view_id(2105), 3);
        assert_eq!(resolve_shield_view_id(2107), 4);
    }

    #[test]
    fn shield_sprite_path_with_item_id() {
        let path = shield_sprite_path(2103, 12, 1);
        assert_eq!(
            path.unwrap(),
            "data/sprite/shield/assassin/assassin_male_buckler"
        );
    }

    #[test]
    fn visual_job_with_riding() {
        assert_eq!(visual_job(7, OPTION_RIDING), 13);
        assert_eq!(visual_job(14, OPTION_RIDING), 21);
        assert_eq!(visual_job(4008, OPTION_RIDING), 4014);
        assert_eq!(visual_job(4015, OPTION_RIDING), 4022);
        assert_eq!(visual_job(0, OPTION_RIDING), 0);
        assert_eq!(visual_job(12, OPTION_RIDING), 12);
        assert_eq!(visual_job(7, 0), 7);
        assert_eq!(visual_job(14, 0), 14);
        assert_eq!(visual_job(7, 0x01), 7);
    }

    #[test]
    fn baby_class_resolves_to_base_job_sprite_and_rides() {
        assert!(is_baby(4024));
        assert!(!is_baby(1));
        assert_eq!(
            body_sprite_path(4024, 1),
            "data/sprite/human/body/male/swordman_male"
        );
        assert_eq!(base_job(4045), 23);
        assert_eq!(visual_job(4030, OPTION_RIDING), 4036);
        assert_eq!(unmounted_job(4036), Some(4030));
    }

    #[test]
    fn baby_scale_splits_by_class_tier() {
        for first in [4023u16, 4024, 4029, 4045] {
            assert_eq!(baby_body_scale(first), 0.75, "job {first}");
        }
        for second in [4030u16, 4036, 4039, 4044] {
            assert_eq!(baby_body_scale(second), 0.82, "job {second}");
        }
        assert_eq!(baby_body_scale(7), 1.0);
        assert_eq!(baby_body_scale(4008), 1.0);
    }

    #[test]
    fn costume_bits_swap_body_and_win_over_riding() {
        assert_eq!(visual_job(7, OPTION_WEDDING), JT_MARRIED);
        assert_eq!(visual_job(7, OPTION_SANTA), JT_SANTA);
        assert_eq!(visual_job(7, OPTION_SUMMER), JT_SUMMER);
        assert_eq!(visual_job(7, OPTION_WEDDING | OPTION_RIDING), JT_MARRIED);
        assert_eq!(visual_job(4008, OPTION_SANTA | OPTION_RIDING), JT_SANTA);
        assert_eq!(visual_job(7, 0), 7);
        assert_eq!(
            body_sprite_path(JT_MARRIED, 1),
            "data/sprite/human/body/male/wedding_male"
        );
        assert_eq!(
            body_sprite_path(JT_SANTA, 0),
            "data/sprite/human/body/female/santa_female"
        );
        assert_eq!(
            body_sprite_path(JT_SUMMER, 1),
            "data/sprite/human/body/male/summer_male"
        );
    }

    #[test]
    fn gm_sprite_paths_per_sex() {
        assert_eq!(
            gm_body_sprite_path(1),
            "data/sprite/human/body/male/game_master_male"
        );
        assert_eq!(
            gm_body_sprite_path(0),
            "data/sprite/human/body/female/game_master_female"
        );
        assert_eq!(
            gm_weapon_sprite_path(1),
            "data/sprite/human/game_master/game_master_male_sword"
        );
        assert_eq!(
            gm_weapon_sprite_path(0),
            "data/sprite/human/game_master/game_master_female_sword"
        );
    }

    #[test]
    fn cart_design_from_option_maps_each_bit() {
        assert_eq!(cart_design_from_option(0), None);
        assert_eq!(cart_design_from_option(OPTION_RIDING), None);
        assert_eq!(cart_design_from_option(0x08), Some(1));
        assert_eq!(cart_design_from_option(0x80), Some(2));
        assert_eq!(cart_design_from_option(0x100), Some(3));
        assert_eq!(cart_design_from_option(0x200), Some(4));
        assert_eq!(cart_design_from_option(0x400), Some(5));
        assert_eq!(cart_design_from_option(OPTION_RIDING | 0x100), Some(3));
    }

    #[test]
    fn falcon_bit_and_sprite_path() {
        assert!(has_falcon(OPTION_FALCON));
        assert!(has_falcon(OPTION_FALCON | OPTION_RIDING));
        assert!(!has_falcon(0));
        assert!(!has_falcon(OPTION_RIDING));
        assert_eq!(falcon_sprite_path(11), "data/sprite/effect/hawk");
        assert_eq!(falcon_sprite_path(4012), "data/sprite/effect/hawk2");
    }

    #[test]
    fn mounted_job_sprite_paths() {
        assert_eq!(
            body_sprite_path(13, 1),
            "data/sprite/human/body/male/peco_peco_knight_male"
        );
        assert_eq!(
            body_sprite_path(21, 0),
            "data/sprite/human/body/female/new_peco_crusader_female"
        );
        assert_eq!(
            body_sprite_path(4014, 1),
            "data/sprite/human/body/male/lord_peco_male"
        );
        assert_eq!(
            body_sprite_path(4022, 1),
            "data/sprite/human/body/male/peco_paladin_male"
        );
    }
}
