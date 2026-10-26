use models::enums::class::JobName;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_effects::merc_skill_base;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillSoundPos {
    /// Full volume, no coordinates.
    NonPositional,
    /// Non-positional but with the original's dy volume knob (negative value).
    Depth(f32),
    /// Positional at the skill target.
    TargetPositional,
}

pub fn skill_use_sound(skill: SkillEnum) -> Option<(&'static str, SkillSoundPos)> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::BsGreed => ("effect\\ef_entry.wav", NonPositional),
        S::AllCatcry => ("effect\\2008cat.wav", NonPositional),
        S::MgStonecurse => ("_stonecurse.wav", TargetPositional),
        S::MoExplosionspirits => ("effect\\mon_qi_burst.wav", NonPositional),
        S::MoSteelbody => ("effect\\mon_fierce_dragon.wav", NonPositional),
        S::LkAurablade => ("effect\\aura_blade.wav", NonPositional),
        S::LkBerserk | S::LkFury => ("effect\\berserk.wav", NonPositional),
        S::NpcPowerup => ("effect\\mon_qi_burst.wav", NonPositional),
        S::NpcFirebreath => ("effect\\EF_FireWall.wav", NonPositional),
        S::NpcWidestone => ("_stonecurse.wav", NonPositional),
        S::NpcWidesleep => ("effect\\widesleep.wav", NonPositional),
        S::NpcWideconfuse => ("amon_ra_die01.wav", NonPositional),
        S::DcWinkcharm => ("effect\\vallentine.wav", NonPositional),
        S::BaPangvoice => ("amon_ra_die01.wav", NonPositional),
        S::BaAssassincross => ("effect\\sunset_assassin.wav", NonPositional),
        S::BaPoembragi => ("effect\\bragis_poem.wav", NonPositional),
        S::BaAppleidun => ("effect\\idun_apple.wav", NonPositional),
        S::DcHumming => ("effect\\humming.wav", NonPositional),
        S::DcDontforgetme => ("effect\\forget_me_not.wav", NonPositional),
        S::DcServiceforyou => ("effect\\service_for_you.wav", NonPositional),
        S::BdLullaby => ("effect\\lullaby.wav", NonPositional),
        S::CgHermode => ("effect\\hermod_staff.wav", NonPositional),
        S::BdEternalchaos => ("effect\\eternal_chaos.wav", NonPositional),
        S::BdDrumbattlefield => ("effect\\of_the_battlefield.wav", NonPositional),
        S::BdRingnibelungen => ("effect\\ring_of_nibelungen.wav", NonPositional),
        S::BdIntoabyss => ("effect\\into_the_abyss.wav", NonPositional),
        S::BdSiegfried => ("effect\\immortal.wav", NonPositional),
        S::BdRichmankim => ("effect\\kim_seobang_money.wav", NonPositional),
        S::BdRokisweil => ("effect\\loki.wav", NonPositional),
        S::DcFortunekiss => ("effect\\of_luck.wav", NonPositional),
        S::CgTarotcard => ("effect\\priest_slowpoison.wav", NonPositional),
        S::CgLongingfreedom => ("effect\\ac_concentration.wav", NonPositional),
        S::HwMagicpower => ("effect\\magic_power_amplification.wav", NonPositional),
        S::PfDoublecasting => ("effect\\magic_power_amplification.wav", NonPositional),
        S::CgMoonlit => ("effect\\moonlight.wav", NonPositional),
        S::TkDodge => ("effect\\t_break_fall.wav", Depth(-150.0)),
        S::TkSevenwind => ("effect\\t_wind_release.wav", NonPositional),
        S::TkMission => ("effect\\t_piring.wav", Depth(-100.0)),
        S::SgFusion => ("effect\\t_transform.wav", NonPositional),
        S::SgHate => ("effect\\t_registration.wav", NonPositional),
        S::SlSwoo => ("effect\\t_whoosh.wav", Depth(-50.0)),
        S::SlSke => ("effect\\t_attack_power.wav", NonPositional),
        S::SlSka => ("effect\\t_defensive_type.wav", NonPositional),
        S::SlKaizel => ("effect\\priest_resurrection.wav", NonPositional),
        S::SlKaahi => ("effect\\t_support_magic.wav", NonPositional),
        S::SlKaupe => ("effect\\t_chiing.wav", NonPositional),
        S::SlKaite => ("effect\\t_magic_reflection.wav", NonPositional),
        S::HamiDefence => ("effect\\h_defence.wav", NonPositional),
        S::HamiCastle => ("effect\\h_castling.wav", NonPositional),
        S::NjTatamigaeshi => ("effect\\tatami_flip.wav", NonPositional),
        S::GsCracker => ("effect\\cracker.wav", NonPositional),
        S::GsGlittering => ("effect\\flip.wav", NonPositional),
        S::PaGospel => ("effect\\gospel.wav", NonPositional),
        S::HpBasilica => ("effect\\basilica.wav", NonPositional),
        S::CrAutoguard | S::MlAutoguard | S::LkParrying | S::MsParrying => {
            ("effect\\kyrie_guard.wav", NonPositional)
        }
        _ => return None,
    })
}

pub fn skill_cast_begin_sound(skill: SkillEnum) -> Option<(&'static str, SkillSoundPos)> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::MoCombofinish => ("effect\\mon_fierce_dragon.wav", NonPositional),
        S::ChPalmstrike => ("effect\\fierce_tiger_mountain_crusher.wav", NonPositional),
        S::CrSlimpitcher => ("assulter_attack.wav", Depth(-150.0)),
        S::HwGanbantein => ("effect\\EF_FireWall.wav", NonPositional),
        S::HwGravitation => ("effect\\wizard_earthspike.wav", NonPositional),
        S::NjSuiton => ("effect\\water_escape.wav", NonPositional),
        S::GsGrounddrift => ("effect\\ground.wav", NonPositional),
        _ => return None,
    })
}

pub fn skill_projectile_sound(skill: SkillEnum) -> Option<&'static str> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    Some(match skill {
        S::NjSyuriken | S::NjKunai | S::NjHuuma | S::NjZenynage => "effect\\ninja_throw.wav",
        S::MoExtremityfist => "effect\\mon_asura_strike.wav",
        _ => return None,
    })
}

pub fn swing_sound(weapon: Option<WeaponType>) -> &'static str {
    use WeaponType as W;
    match weapon {
        Some(W::Sword1H | W::Sword2H) => "_attack_sword.wav",
        Some(W::Bow) => "_attack_bow.wav",
        Some(W::Spear1H | W::Spear2H) => "_attack_spear.wav",
        Some(W::Axe1H | W::Axe2H) => "_attack_axe.wav",
        Some(W::Staff | W::Staff2H) => "_attack_rod.wav",
        _ => "_attack_mace.wav",
    }
}

pub fn weapon_hit_sound(weapon: Option<WeaponType>, roll: u32, is_taekwon: bool) -> String {
    use WeaponType as W;
    let one_of = |base: &str, n: u32| format!("{base}{}.wav", 1 + (roll % n));
    match weapon {
        None | Some(W::Fist) => {
            if is_taekwon {
                "_hit_mace.wav".to_string()
            } else {
                one_of("_hit_fist", 4)
            }
        }
        Some(W::Sword1H | W::Sword2H) => "_hit_sword.wav".to_string(),
        Some(W::Bow) => "_hit_arrow.wav".to_string(),
        Some(W::Spear1H | W::Spear2H) => "_hit_spear.wav".to_string(),
        Some(W::Axe1H | W::Axe2H) => "_hit_axe.wav".to_string(),
        Some(W::Mace | W::Mace2H) => "_hit_mace.wav".to_string(),
        Some(W::Staff | W::Staff2H) => "_hit_rod.wav".to_string(),
        Some(W::Book) => "_hit_mace.wav".to_string(),
        Some(W::Revolver) => "_hit_pistol.wav".to_string(),
        Some(W::Gatling) => "_hit_gatling_single_shot.wav".to_string(),
        Some(W::Shotgun) => "_hit_shotgun.wav".to_string(),
        Some(W::Grenade) => "_hit_grenade_launcher.wav".to_string(),
        Some(W::Rifle) => "_hit_rifle.wav".to_string(),
        _ => "_hit_mace.wav".to_string(),
    }
}

pub fn skill_hit_sound(roll: u32) -> String {
    format!("_enemy_hit_normal{}.wav", 1 + (roll % 4))
}

/// PC-victim body-material hit wave (overrides the weapon table for PC targets).
pub fn job_hit_sound(job: JobName) -> &'static str {
    use JobName as J;
    match job {
        J::Archer
        | J::ArcherHigh
        | J::BabyArcher
        | J::Thief
        | J::ThiefHigh
        | J::BabyThief
        | J::Hunter
        | J::Sniper
        | J::BabyHunter
        | J::Assassin
        | J::AssassinCross
        | J::BabyAssassin
        | J::Bard
        | J::Clown
        | J::BabyBard
        | J::Dancer
        | J::Gypsy
        | J::BabyDancer
        | J::Rogue
        | J::Stalker
        | J::BabyRogue
        | J::Gunslinger
        | J::Ninja
        | J::Taekwon => "player_wooden_male.wav",

        J::Swordsman
        | J::SwordsmanHigh
        | J::BabySwordsman
        | J::Knight
        | J::LordKnight
        | J::BabyKnight
        | J::Crusader
        | J::Paladin
        | J::BabyCrusader
        | J::Monk
        | J::Champion
        | J::BabyMonk
        | J::StarGladiator => "player_metal.wav",

        _ => "player_clothes.wav",
    }
}

/// A status/ailment transition sound. `enter` distinguishes onset from clear.
pub fn status_sound(kind: StatusSoundKind) -> Option<&'static str> {
    use StatusSoundKind as S;
    Some(match kind {
        S::FreezeEnter => "_stonecurse.wav",
        S::FreezeExit => "_frozen_explosion.wav",
        S::StoneCurseExit => "_stone_explosion.wav",
        S::StunEnter => "_stun.wav",
        S::PoisonSet => "_poison.wav",
        S::CurseSet => "_curse.wav",
        S::SilenceSet => "_silence.wav",
        S::ConfusionSet => "_confusion.wav",
        S::BlindSet => "_blind.wav",
        S::DetectOn => "effect\\EF_Sight.wav",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusSoundKind {
    FreezeEnter,
    FreezeExit,
    StoneCurseExit,
    StunEnter,
    PoisonSet,
    CurseSet,
    SilenceSet,
    ConfusionSet,
    BlindSet,
    /// Sight or Ruwach turning on.
    DetectOn,
}

pub mod ui {
    pub const LOGIN: &str = "login.wav";
    pub const BUTTON: &str = "button_sound.wav";
    pub const REPAIR: &str = "effect\\black_weapon_repair.wav";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_and_job_tables_pick_expected_waves() {
        assert_eq!(
            weapon_hit_sound(Some(WeaponType::Bow), 0, false),
            "_hit_arrow.wav"
        );
        assert_eq!(
            weapon_hit_sound(Some(WeaponType::Sword2H), 0, false),
            "_hit_sword.wav"
        );
        assert_eq!(weapon_hit_sound(None, 0, false), "_hit_fist1.wav");
        assert_eq!(weapon_hit_sound(None, 2, false), "_hit_fist3.wav");
        assert_eq!(weapon_hit_sound(None, 0, true), "_hit_mace.wav");
        assert_eq!(job_hit_sound(JobName::Novice), "player_clothes.wav");
        assert_eq!(job_hit_sound(JobName::Archer), "player_wooden_male.wav");
        assert_eq!(job_hit_sound(JobName::Knight), "player_metal.wav");
        assert_eq!(swing_sound(Some(WeaponType::Axe2H)), "_attack_axe.wav");
        assert_eq!(swing_sound(None), "_attack_mace.wav");
        assert_eq!(
            skill_use_sound(SkillEnum::NpcWidesleep),
            Some(("effect\\widesleep.wav", SkillSoundPos::NonPositional))
        );
    }
}
