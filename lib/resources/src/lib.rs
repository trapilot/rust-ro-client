//! Every GRF resource path the client reads, in one place.
//!
//! Values are GRF entry names: forward slashes, folder and file names spelled
//! the way the archive stores them (lowercase English). Paths built at runtime are functions here too,
//! so no `data/` literal needs to live anywhere else.

/// Declares path constants and the module's `ALL` slice from one list, so the
/// two cannot drift.
#[macro_export]
macro_rules! paths {
    ($($(#[$attr:meta])* $name:ident = $value:literal;)*) => {
        $($(#[$attr])* pub const $name: &str = $value;)*

        /// Every path declared in this module.
        pub const ALL: &[&str] = &[$($value),*];
    };
}

/// Every path the registry states outright. Paths assembled at runtime are not
/// here — those come from the builder functions.
pub fn all_static_paths() -> Vec<&'static str> {
    const MODULES: &[&[&str]] = &[
        dir::ALL,
        font::ALL,
        grf::ALL,
        lua::ALL,
        sprite::ALL,
        sprite::effect::ALL,
        sprite::item::ALL,
        sprite::monster::ALL,
        sprite::pet_accessory::ALL,
        sprite::player::ALL,
        table::ALL,
        texture::ALL,
        texture::effect::ALL,
        ui::ALL,
        ui::basic::ALL,
        ui::basic::seekparty::ALL,
        ui::illust::ALL,
        ui::item::ALL,
        ui::login::ALL,
        ui::minimap::ALL,
    ];
    MODULES.iter().flat_map(|m| m.iter().copied()).collect()
}

/// Directory prefixes, for listing or stripping — not resources themselves.
pub mod dir {
    crate::paths! {
        DATA = "data/";
        SPRITE_ACCESSORY = "data/sprite/accessory/";
        SPRITE_MONSTER = "data/sprite/monster/";
        SPRITE_NPC = "data/sprite/npc/";
        SPRITE_PLAYER = "data/sprite/human/";
        SPRITE_SHIELD = "data/sprite/shield/";
        STR_EFFECT = "data/texture/effect/";
        TEXTURE = "data/texture/";
        UI_TEXTURE = "data/texture/user_interface/";
        UI_TEXTURE_EN = "data/texture/userinterface/";
    }
}

/// TrueType fonts shipped in the archive.
pub mod font {
    crate::paths! {
        NANUM_BARUN_GOTHIC = "data/Font/NanumBarunGothic.ttf";
        NANUM_BARUN_GOTHIC_BOLD = "data/Font/NanumBarunGothicBold.ttf";
    }
}

/// GRF archives.
pub mod grf {
    crate::paths! {
        DEFAULT_ARCHIVE = "data/data.grf";
    }
}

/// Head-attachment offset tables, one per job.
pub mod imf {
    pub fn of(base: &str) -> String {
        format!("data/imf/{base}.imf")
    }

    pub fn for_job(job: &str, sex: &str) -> String {
        format!("data/imf/{job}_{sex}.imf")
    }
}

/// Lua/Lub identity tables. `.lua` and `.lub` are alternate encodings of
/// the same table; readers try one then the other.
pub mod lua {
    crate::paths! {
        ACCESSORY_ID_514_LUB = "data/luafiles514/lua_files/datainfo/accessoryid.lub";
        ACCESSORY_ID_LUA = "data/lua_files/datainfo/accessoryid.lua";
        ACCESSORY_ID_LUB = "data/lua_files/datainfo/accessoryid.lub";
        ACCESSORY_NAME_514_LUB = "data/luafiles514/lua_files/datainfo/accname.lub";
        ACCESSORY_NAME_LUA = "data/lua_files/datainfo/accname.lua";
        ACCESSORY_NAME_LUB = "data/lua_files/datainfo/accname.lub";
        JOB_IDENTITY_514_LUB = "data/luafiles514/lua_files/datainfo/jobidentity.lub";
        JOB_NAME_514_LUB = "data/luafiles514/lua_files/datainfo/jobname.lub";
        JOB_NAME_LUB = "data/lua_files/datainfo/jobname.lub";
        JOB_IDENTITY_LUA = "data/lua_files/datainfo/jobidentity.lua";
        JOB_IDENTITY_LUB = "data/lua_files/datainfo/jobidentity.lub";
        NPC_IDENTITY_514_LUB = "data/luafiles514/lua_files/datainfo/npcidentity.lub";
        NPC_IDENTITY_LUA = "data/lua_files/datainfo/npcidentity.lua";
        NPC_IDENTITY_LUB = "data/lua_files/datainfo/npcidentity.lub";
    }
}

/// Map geometry: one `.rsw`/`.gnd`/`.gat` triple per map.
pub mod map {
    pub fn rsw(map_name: &str) -> String {
        format!("data/{map_name}.rsw")
    }

    pub fn gnd(map_name: &str) -> String {
        format!("data/{map_name}.gnd")
    }

    pub fn gat(map_name: &str) -> String {
        format!("data/{map_name}.gat")
    }

    /// For names already carrying their extension, as RSW headers store them.
    pub fn file(file_name: &str) -> String {
        format!("data/{file_name}")
    }
}

/// 3D monster models and their animation clips.
pub mod model {
    pub fn mob(file_name: &str) -> String {
        format!("data/model/3dmob/{file_name}")
    }

    pub fn mob_animation(bone_type: u32, suffix: &str) -> String {
        format!("data/model/3dmob_bone/{bone_type}_{suffix}.gr2")
    }
}

/// Sprite recolour palettes.
pub mod palette {
    pub fn head(head_name: &str, sex: &str, palette_id: u16) -> String {
        format!("data/palette/hair/hair{head_name}_{sex}_{palette_id}.pal")
    }

    pub fn body(job: &str, sex: &str, palette_id: u16) -> String {
        format!("data/palette/body/{job}_{sex}_{palette_id}.pal")
    }
}

/// BGM tracks and sound effects.
pub mod sound {
    pub fn bgm(track: &str) -> String {
        format!("data/wav/bgm/{track}")
    }

    pub fn sfx(name: &str) -> String {
        format!("data/wav/{name}")
    }
}

/// Sprites outside the actor trees.
pub mod sprite {
    crate::paths! {
        CURSORS_ACT = "data/sprite/cursors.act";
        CURSORS_SPR = "data/sprite/cursors.spr";
        SHADOW = "data/sprite/shadow";
        SHADOW_ACT = "data/sprite/shadow.act";
        SHADOW_SPR = "data/sprite/shadow.spr";
        SLOTMACHINE_ACT = "data/sprite/slotmachine.act";
        SLOTMACHINE_SPR = "data/sprite/slotmachine.spr";
    }

    /// Headgear sprites, drawn per sex.
    pub mod accessory {
        pub fn of(sex: &str, suffix: &str) -> String {
            format!("data/sprite/accessory/{sex}/{sex}{suffix}")
        }
    }

    /// The `effect` tree — visual-effect sprites.
    pub mod effect {
        crate::paths! {
            ANGEL = "data/sprite/effect/angel";
            ANGEL_FEATHER = "data/sprite/effect/angel_wing_feather";
            ANGEL_WINGS = "data/sprite/effect/angel_wings";
            AQUA_BENEDICTA = "data/sprite/effect/scoop_holy_water";
            BAT = "data/sprite/effect/bat";
            BLESSING = "data/sprite/effect/benediction";
            BLOODLUST = "data/sprite/effect/blood_lust";
            CASTLING = "data/sprite/effect/castling";
            CCONFINE = "data/sprite/effect/cconfine";
            CHIMNEY_SMOKE = "data/sprite/effect/chimney_smoke";
            CHRISTMAS = "data/sprite/effect/christmas";
            DARKBREATH = "data/sprite/effect/darkbreath";
            DEMONSTRATION = "data/sprite/effect/demonstration";
            DESPERADO = "data/sprite/effect/desperado";
            EARTHQUAKE = "data/sprite/effect/earthquake";
            EF_SNOW = "data/sprite/effect/ef_snow";
            EMOTION_ACT = "data/sprite/effect/emotion.act";
            EMOTION_SPR = "data/sprite/effect/emotion.spr";
            FALCON = "data/sprite/effect/hawk";
            FALCON2 = "data/sprite/effect/hawk2";
            FAST = "data/sprite/effect/fast";
            FIREBALL = "data/sprite/effect/fireball";
            FIREWORK_BIRTHDAY = "data/sprite/effect/firecracker_birthday";
            FIREWORK_CHRISTMAS = "data/sprite/effect/firecracker_christmas";
            FIREWORK_LOVE = "data/sprite/effect/firecracker_love";
            FIREWORK_VALENTINE = "data/sprite/effect/firecracker_valentine";
            FIREWORK_WHITE_DAY = "data/sprite/effect/firecracker_white_day";
            FVOICE = "data/sprite/effect/fvoice";
            GHOST = "data/sprite/effect/ghost";
            HANBOK_ANGEL_BODY = "data/sprite/effect/hanbok_angel_main_body";
            HANBOK_ANGEL_WINGS = "data/sprite/effect/hanbok_angel_wings";
            ICE_BLOCK = "data/sprite/effect/ice_block";
            ISSEN = "data/sprite/effect/issen";
            ITEM_CLOUD = "data/sprite/effect/item_cloud";
            ITEM_CURSE = "data/sprite/effect/item_curse";
            ITEM_RAIN = "data/sprite/effect/item_rain";
            ITEM_THUNDER = "data/sprite/effect/item_thunder";
            ITEM_ZZZ = "data/sprite/effect/item_zzz";
            KAEN = "data/sprite/effect/flame_array";
            KASUMIKIRI = "data/sprite/effect/mist_slash";
            KIRIKAGE = "data/sprite/effect/shadow_slash";
            MAGICAL_BULLET = "data/sprite/effect/magical_bullet";
            MAPLE_LEAF = "data/sprite/effect/maple_leaf";
            MSG_ACT = "data/sprite/effect/msg.act";
            MSG_SPR = "data/sprite/effect/msg.spr";
            M_EF01 = "data/sprite/effect/m_ef01";
            M_EF02 = "data/sprite/effect/m_ef02";
            M_EF03 = "data/sprite/effect/m_ef03";
            M_EF04 = "data/sprite/effect/m_ef04";
            M_EF05 = "data/sprite/effect/m_ef05";
            M_EF06 = "data/sprite/effect/m_ef06";
            M_EF07 = "data/sprite/effect/m_ef07";
            NUMBER_ACT = "data/sprite/effect/numbers.act";
            NUMBER_SPR = "data/sprite/effect/numbers.spr";
            ORCFACE = "data/sprite/effect/orcface";
            PARTICLE1 = "data/sprite/effect/particle1";
            PARTICLE2 = "data/sprite/effect/particle2";
            PARTICLE3 = "data/sprite/effect/particle3";
            PARTICLE4 = "data/sprite/effect/particle4";
            PARTICLE5 = "data/sprite/effect/particle5";
            PARTICLE6 = "data/sprite/effect/particle6";
            PARTICLE7 = "data/sprite/effect/particle7";
            POISONHIT = "data/sprite/effect/poisonhit";
            RANKFONT_ACT = "data/sprite/effect/rankfont.act";
            RANKFONT_SPR = "data/sprite/effect/rankfont.spr";
            RAPID_SHOWER = "data/sprite/effect/rapid_shower";
            SAKURA01 = "data/sprite/effect/sakura01";
            SIGHT = "data/sprite/effect/sight";
            SMOKE = "data/sprite/effect/smoke";
            SPEAR = "data/sprite/effect/spear";
            SPREAD_ATTACK = "data/sprite/effect/spread";
            STATUS_CURSE = "data/sprite/effect/status_curse";
            STATUS_SLEEP = "data/sprite/effect/status_sleep";
            STATUS_STUN = "data/sprite/effect/status_stun";
            STOP = "data/sprite/effect/stop";
            TATAMI_FLIP = "data/sprite/effect/tatami_turn_over";
            TIMEFONT_ACT = "data/sprite/effect/timefont.act";
            TIMEFONT_SPR = "data/sprite/effect/timefont.spr";
            TORCH_01 = "data/sprite/effect/torch_01";
            TRACKING = "data/sprite/effect/tracking";
            TRIPLE_ACTION = "data/sprite/effect/triple_action";
            VALLENTINE = "data/sprite/effect/vallentine";
            WATERBALL = "data/sprite/effect/waterball";
            WINK = "data/sprite/effect/wink";
        }

        /// Design 0 is the Merchant's own cart, 1.. the purchasable ones.
        pub fn cart(design: u8) -> String {
            match design {
                0 => "data/sprite/effect/shuno_handcart".to_string(),
                1 => "data/sprite/effect/handcart".to_string(),
                n => format!("data/sprite/effect/handcart{}", n - 1),
            }
        }
    }

    /// Homunculus sprites.
    pub mod homun {
        pub fn of(name: &str) -> String {
            format!("data/sprite/homun/{name}")
        }
    }

    /// Dropped-item sprites.
    pub mod item {
        crate::paths! {
            BLIND_SPEAR = "data/sprite/item/blind_sphere";
            BLUE_GEMSTONE = "data/sprite/item/blue_gemstone";
            FLARE_SPEAR = "data/sprite/item/flare_sphere";
            FREEZING_SPEAR = "data/sprite/item/freezing_sphere";
            LIGHTNING_SPEAR = "data/sprite/item/lightning_sphere";
            POISON_SPEAR = "data/sprite/item/poison_sphere";
            RED_GEMSTONE = "data/sprite/item/red_gemstone";
            YELLOW_GEMSTONE = "data/sprite/item/yellow_gemstone";
        }

        pub fn of(name: &str) -> String {
            format!("data/sprite/item/{name}")
        }
    }

    /// Monster sprites.
    pub mod monster {
        crate::paths! {
            SKEL_ARCHER_ARROW = "data/sprite/monster/skel_archer_arrow";
        }

        pub fn of(name: &str) -> String {
            format!("data/sprite/monster/{name}")
        }
    }

    /// NPC sprites.
    pub mod npc {
        pub fn of(name: &str) -> String {
            format!("data/sprite/npc/{name}")
        }
    }

    /// Pet headgear animations, keyed by the pet mob.
    pub mod pet_accessory {
        crate::paths! {
            BACSOJIN = "data/sprite/monster/BACSOJIN_round_head_ornament.act";
            BAPHOMET = "data/sprite/monster/BAPHOMET_bone_hat.act";
            BON_GUN = "data/sprite/monster/bon_gun_taoist_spirit_sword.act";
            CHOCHO = "data/sprite/monster/chocho_respirator.act";
            CIVIL_SERVANT = "data/sprite/monster/CIVIL_SERVANT_golden_earring.act";
            DESERT_WOLF_B = "data/sprite/monster/DESERT_WOLF_B_spacesuit_head.act";
            DEVIRUCHI = "data/sprite/monster/DEVIRUCHI_pacifier.act";
            DOKEBI = "data/sprite/monster/DOKEBI_afro_head.act";
            DULLAHAN = "data/sprite/monster/DULLAHAN_death_ring.act";
            GOBLIN_LEADER = "data/sprite/monster/GOBLIN_LEADER_cool_emblem.act";
            GOLEM = "data/sprite/monster/GOLEM_mainspring.act";
            IMP = "data/sprite/monster/IMP_horn_guard.act";
            INCUBUS = "data/sprite/monster/INCUBUS_masquerade_ball_mask.act";
            ISIS = "data/sprite/monster/isis_cleopatra_headband.act";
            LEAF_CAT = "data/sprite/monster/LEAF_CAT_green_lucky_bag.act";
            LOLI_RURI = "data/sprite/monster/LOLI_RURI_fashion_glasses.act";
            LUNATIC = "data/sprite/monster/lunatic_ribbon.act";
            MARIONETTE = "data/sprite/monster/MARIONETTE_star_shaped_headband.act";
            MEDUSA = "data/sprite/monster/MEDUSA_queen_coronet.act";
            MIYABI_NINGYO = "data/sprite/monster/MIYABI_NINGYO_summer_fan.act";
            MUNAK = "data/sprite/monster/munak_magic_wand.act";
            NIGHTMARE_TERROR = "data/sprite/monster/NIGHTMARE_TERROR_hell_horn.act";
            ORK_WARRIOR = "data/sprite/monster/ork_warrior_flower.act";
            PECOPECO = "data/sprite/monster/pecopeco_pot.act";
            PETIT = "data/sprite/monster/PETIT_star.act";
            PICKY = "data/sprite/monster/picky_egg_shell.act";
            PORING = "data/sprite/monster/poring_school_bag.act";
            ROCKER = "data/sprite/monster/rocker_grasshopper_glasses.act";
            SAVAGE_BABE = "data/sprite/monster/savage_babe_lace.act";
            SHINOBI = "data/sprite/monster/SHINOBI_durumagi_dragon_art.act";
            SMOKIE = "data/sprite/monster/smokie_muffler.act";
            SOHEE = "data/sprite/monster/SOHEE_bell.act";
            SPORE = "data/sprite/monster/spore_native_skirt.act";
            STONE_SHOOTER = "data/sprite/monster/STONE_SHOOTER_afro_hair.act";
            SUCCUBUS = "data/sprite/monster/SUCCUBUS_black_butterfly_mask.act";
            WHISPER = "data/sprite/monster/WHISPER_soul_ring_.act";
            WICKED_NYMPH = "data/sprite/monster/WICKED_NYMPH_jade_ornament.act";
            YOYO = "data/sprite/monster/yoyo_headband.act";
        }
    }

    /// Player body, head and weapon sprites.
    pub mod player {
        crate::paths! {
            GRAND_PECO_CRUSADER_FEMALE = "data/sprite/human/body/female/new_peco_crusader_female";
            GRAND_PECO_CRUSADER_MALE = "data/sprite/human/body/male/new_peco_crusader_male";
            LORD_PECO_MALE = "data/sprite/human/body/male/lord_peco_male";
            PECO_KNIGHT_MALE = "data/sprite/human/body/male/peco_peco_knight_male";
            PECO_PALADIN_MALE = "data/sprite/human/body/male/peco_paladin_male";
        }

        pub fn body(job: &str, sex: &str) -> String {
            format!("data/sprite/human/body/{sex}/{job}_{sex}")
        }

        pub fn head(head_name: &str, sex: &str) -> String {
            format!("data/sprite/human/head/{sex}/{head_name}_{sex}")
        }

        pub fn weapon(job: &str, sex: &str, suffix: &str) -> String {
            format!("data/sprite/human/{job}/{job}_{sex}{suffix}")
        }

        /// Weapon art keyed by item id, preferred over [`weapon`] when present.
        pub fn weapon_by_item(job: &str, sex: &str, item_id: u16) -> String {
            format!("data/sprite/human/{job}/{job}_{sex}_{item_id}")
        }

        pub fn gm_body(sex: &str) -> String {
            format!("data/sprite/human/body/{sex}/game_master_{sex}")
        }

        pub fn gm_weapon(sex: &str) -> String {
            format!("data/sprite/human/game_master/game_master_{sex}_sword")
        }

        /// The name table already carries the sex/type sub-path.
        pub fn mercenary_body(name: &str) -> String {
            format!("data/sprite/human/body/{name}")
        }

        pub fn mercenary_weapon(base: &str, weapon: &str) -> String {
            format!("data/sprite/human/mercenary/{base}_{weapon}")
        }
    }

    /// Shield sprites, drawn per job and sex.
    pub mod shield {
        pub fn of(job: &str, sex: &str, shield: impl std::fmt::Display) -> String {
            format!("data/sprite/shield/{job}/{job}_{sex}_{shield}")
        }
    }
}

/// Flat `data/*.txt` lookup tables.
pub mod table {
    crate::paths! {
        CARD_ILLUSTRATION_NAME = "data/num2cardillustnametable.txt";
        CARD_POSTFIX_NAME = "data/cardpostfixnametable.txt";
        CARD_PREFIX_NAME = "data/cardprefixnametable.txt";
        FOG_PARAMETER = "data/fogparametertable.txt";
        IDENTIFIED_ITEM_DESC = "data/idnum2itemdesctable.txt";
        IDENTIFIED_ITEM_NAME = "data/idnum2itemdisplaynametable.txt";
        IDENTIFIED_ITEM_RESOURCE = "data/idnum2itemresnametable.txt";
        INDOOR_RSW = "data/indoorrswtable.txt";
        ITEM_SLOT_COUNT = "data/itemslotcounttable.txt";
        MAP_NAME = "data/mapnametable.txt";
        MAP_POSITION = "data/mappostable.txt";
        MP3_NAME = "data/mp3nametable.txt";
        MSG_STRING = "data/msgstringtable.txt";
        PET_TALK = "data/pettalktable.xml";
        QUEST_DISPLAY = "data/questid2display.txt";
        RES_NAME = "data/resnametable.txt";
        SKILL_DESC = "data/skilldesctable.txt";
        SKILL_NAME = "data/skillnametable.txt";
        SKILL_SP_AMOUNT = "data/leveluseskillspamount.txt";
        SKILL_TREE = "data/skilltreeview.txt";
        UNIDENTIFIED_ITEM_DESC = "data/num2itemdesctable.txt";
        UNIDENTIFIED_ITEM_NAME = "data/num2itemdisplaynametable.txt";
        UNIDENTIFIED_ITEM_RESOURCE = "data/num2itemresnametable.txt";
    }

    pub fn book(item_id: impl std::fmt::Display) -> String {
        format!("data/book/{item_id}.txt")
    }
}

/// Textures outside the UI tree.
pub mod texture {
    crate::paths! {
        GRID = "data/texture/grid.tga";
    }

    pub fn named(name: &str) -> String {
        format!("data/texture/{name}")
    }

    pub fn water(water_type: i32, frame: usize) -> String {
        format!("data/texture/water/water{water_type}{frame:02}.jpg")
    }

    /// Effect textures and `.str` animation scripts.
    pub mod effect {
        crate::paths! {
            ALPHABET = "data/texture/effect/alpabet.bmp";
        }

        pub fn named(name: &str) -> String {
            format!("data/texture/effect/{name}")
        }

        pub fn str_file(name: &str) -> String {
            format!("data/texture/effect/{name}.str")
        }
    }
}

/// The `user_interface` tree — every window, button and icon texture.
pub mod ui {
    crate::paths! {
        BTN_ADD = "data/texture/user_interface/btn_add.bmp";
        BTN_ADD_A = "data/texture/user_interface/btn_add_a.bmp";
        BTN_ADD_B = "data/texture/user_interface/btn_add_b.bmp";
        BTN_APPLY = "data/texture/user_interface/btn_apply.bmp";
        BTN_APPLY_A = "data/texture/user_interface/btn_apply_a.bmp";
        BTN_APPLY_B = "data/texture/user_interface/btn_apply_b.bmp";
        BTN_BACK = "data/texture/user_interface/btn_back.bmp";
        BTN_BACK_A = "data/texture/user_interface/btn_back_a.bmp";
        BTN_BACK_B = "data/texture/user_interface/btn_back_b.bmp";
        BTN_BUY = "data/texture/user_interface/btn_buy.bmp";
        BTN_BUY_A = "data/texture/user_interface/btn_buy_a.bmp";
        BTN_BUY_B = "data/texture/user_interface/btn_buy_b.bmp";
        BTN_CANCEL = "data/texture/user_interface/btn_cancel.bmp";
        BTN_CANCEL_A = "data/texture/user_interface/btn_cancel_a.bmp";
        BTN_CANCEL_B = "data/texture/user_interface/btn_cancel_b.bmp";
        BTN_CLOSE = "data/texture/user_interface/btn_close.bmp";
        BTN_CLOSE_A = "data/texture/user_interface/btn_close_a.bmp";
        BTN_CLOSE_B = "data/texture/user_interface/btn_close_b.bmp";
        BTN_DEL = "data/texture/user_interface/btn_del.bmp";
        BTN_DEL_A = "data/texture/user_interface/btn_del_a.bmp";
        BTN_DEL_B = "data/texture/user_interface/btn_del_b.bmp";
        BTN_EDIT = "data/texture/user_interface/btn_edit.bmp";
        BTN_EDIT_A = "data/texture/user_interface/btn_edit_a.bmp";
        BTN_EDIT_B = "data/texture/user_interface/btn_edit_b.bmp";
        BTN_EXCHANGE = "data/texture/user_interface/btn_exchange.bmp";
        BTN_EXCHANGE_A = "data/texture/user_interface/btn_exchange_a.bmp";
        BTN_EXCHANGE_B = "data/texture/user_interface/btn_exchange_b.bmp";
        BTN_EXCHANGE_DIS = "data/texture/user_interface/btn_exchange_dis.bmp";
        BTN_FEED = "data/texture/user_interface/btn_feed.bmp";
        BTN_FEED_A = "data/texture/user_interface/btn_feed_a.bmp";
        BTN_FEED_B = "data/texture/user_interface/btn_feed_b.bmp";
        BTN_FIRED = "data/texture/user_interface/btn_fired.bmp";
        BTN_FIRED_A = "data/texture/user_interface/btn_fired_a.bmp";
        BTN_FIRED_B = "data/texture/user_interface/btn_fired_b.bmp";
        BTN_MAKE = "data/texture/user_interface/btn_make.bmp";
        BTN_MAKE_A = "data/texture/user_interface/btn_make_a.bmp";
        BTN_MAKE_B = "data/texture/user_interface/btn_make_b.bmp";
        BTN_NEXT = "data/texture/user_interface/btn_next.bmp";
        BTN_NEXT_A = "data/texture/user_interface/btn_next_a.bmp";
        BTN_NEXT_B = "data/texture/user_interface/btn_next_b.bmp";
        BTN_OK = "data/texture/user_interface/btn_ok.bmp";
        BTN_OK_A = "data/texture/user_interface/btn_ok_a.bmp";
        BTN_OK_B = "data/texture/user_interface/btn_ok_b.bmp";
        BTN_OK_DIS = "data/texture/user_interface/btn_ok_dis.bmp";
        BTN_RESET = "data/texture/user_interface/btn_reset.bmp";
        BTN_RESET_A = "data/texture/user_interface/btn_reset_a.bmp";
        BTN_RESET_B = "data/texture/user_interface/btn_reset_b.bmp";
        BTN_RESIZE = "data/texture/user_interface/btn_resize.bmp";
        BTN_REWRITE = "data/texture/user_interface/btn_rewrite.bmp";
        BTN_REWRITE_A = "data/texture/user_interface/btn_rewrite_a.bmp";
        BTN_REWRITE_B = "data/texture/user_interface/btn_rewrite_b.bmp";
        BTN_SELL = "data/texture/user_interface/btn_sell.bmp";
        BTN_SELL_A = "data/texture/user_interface/btn_sell_a.bmp";
        BTN_SELL_B = "data/texture/user_interface/btn_sell_b.bmp";
        BTN_SKILL = "data/texture/user_interface/btn_skill.bmp";
        BTN_SKILL_A = "data/texture/user_interface/btn_skill_a.bmp";
        BTN_SKILL_B = "data/texture/user_interface/btn_skill_b.bmp";
        BTN_USE = "data/texture/user_interface/btn_use.bmp";
        BTN_USE_A = "data/texture/user_interface/btn_use_a.bmp";
        BTN_USE_B = "data/texture/user_interface/btn_use_b.bmp";
        BTN_VIEW = "data/texture/user_interface/btn_view.bmp";
        BTN_VIEW_A = "data/texture/user_interface/btn_view_a.bmp";
        BTN_VIEW_B = "data/texture/user_interface/btn_view_b.bmp";
        CHAT_CLOSE = "data/texture/user_interface/chat_close.bmp";
        CHAT_OPEN = "data/texture/user_interface/chat_open.bmp";
        CHECKBOX_0 = "data/texture/user_interface/checkbox_0.bmp";
        CHECKBOX_1 = "data/texture/user_interface/checkbox_1.bmp";
        EMPTY_CARD_SLOT = "data/texture/user_interface/empty_card_slot.bmp";
        ESC_01A = "data/texture/user_interface/esc_01a.bmp";
        ESC_01B = "data/texture/user_interface/esc_01b.bmp";
        ESC_01C = "data/texture/user_interface/esc_01c.bmp";
        ESC_02A = "data/texture/user_interface/esc_02a.bmp";
        ESC_02B = "data/texture/user_interface/esc_02b.bmp";
        ESC_02C = "data/texture/user_interface/esc_02c.bmp";
        ESC_03A = "data/texture/user_interface/esc_03a.bmp";
        ESC_03B = "data/texture/user_interface/esc_03b.bmp";
        ESC_03C = "data/texture/user_interface/esc_03c.bmp";
        ESC_04A = "data/texture/user_interface/esc_04a.bmp";
        ESC_04B = "data/texture/user_interface/esc_04b.bmp";
        ESC_04C = "data/texture/user_interface/esc_04c.bmp";
        ESC_05A = "data/texture/user_interface/esc_05a.bmp";
        ESC_05B = "data/texture/user_interface/esc_05b.bmp";
        ESC_05C = "data/texture/user_interface/esc_05c.bmp";
        ESC_06A = "data/texture/user_interface/esc_06a.bmp";
        ESC_06B = "data/texture/user_interface/esc_06b.bmp";
        ESC_06C = "data/texture/user_interface/esc_06c.bmp";
        ESC_07A = "data/texture/user_interface/esc_07a.bmp";
        ESC_07B = "data/texture/user_interface/esc_07b.bmp";
        ESC_07C = "data/texture/user_interface/esc_07c.bmp";
        ESC_08A = "data/texture/user_interface/esc_08a.bmp";
        ESC_08B = "data/texture/user_interface/esc_08b.bmp";
        ESC_08C = "data/texture/user_interface/esc_08c.bmp";
        RADIOBTN_OFF = "data/texture/user_interface/radiobtn_off.bmp";
        RADIOBTN_ON = "data/texture/user_interface/radiobtn_on.bmp";
        RAG_TITLE = "data/texture/user_interface/rag_title.bmp";
        RAG_TITLE2 = "data/texture/user_interface/rag_title2.bmp";
        RAG_TITLE3 = "data/texture/user_interface/rag_title3.bmp";
        SCROLL0BAR_DOWN = "data/texture/user_interface/scroll0bar_down.bmp";
        SCROLL0BAR_MID = "data/texture/user_interface/scroll0bar_mid.bmp";
        SCROLL0BAR_UP = "data/texture/user_interface/scroll0bar_up.bmp";
        SCROLL0DOWN = "data/texture/user_interface/scroll0down.bmp";
        SCROLL0MID = "data/texture/user_interface/scroll0mid.bmp";
        SCROLL0UP = "data/texture/user_interface/scroll0up.bmp";
        SCROLL1LEFT = "data/texture/user_interface/scroll1left.bmp";
        SCROLL1RIGHT = "data/texture/user_interface/scroll1right.bmp";
        SHOP = "data/texture/user_interface/shop.bmp";
        SYSBOX_BG = "data/texture/user_interface/sysbox_bg.bmp";
        SYSBOX_LD = "data/texture/user_interface/sysbox_ld.bmp";
        SYSBOX_LM = "data/texture/user_interface/sysbox_lm.bmp";
        SYSBOX_LU = "data/texture/user_interface/sysbox_lu.bmp";
        SYSBOX_MD = "data/texture/user_interface/sysbox_md.bmp";
        SYSBOX_MU = "data/texture/user_interface/sysbox_mu.bmp";
        SYSBOX_RD = "data/texture/user_interface/sysbox_rd.bmp";
        SYSBOX_RM = "data/texture/user_interface/sysbox_rm.bmp";
        SYSBOX_RU = "data/texture/user_interface/sysbox_ru.bmp";
        WIN_MSGBOX = "data/texture/user_interface/win_msgbox.bmp";
        WORLDMAP = "data/texture/user_interface/worldmap.bmp";
    }

    /// Shared window chrome and the in-game HUD.
    pub mod basic {
        crate::paths! {
            ARW_LEFT = "data/texture/user_interface/basic_interface/arw_left.bmp";
            ARW_RIGHT = "data/texture/user_interface/basic_interface/arw_right.bmp";
            ARW_RIGHT_ON = "data/texture/user_interface/basic_interface/arw_right_on.bmp";
            BASEWIN_BG = "data/texture/user_interface/basic_interface/basewin_bg.bmp";
            BASEWIN_MINI = "data/texture/user_interface/basic_interface/basewin_mini.bmp";
            BTNBAR_MID2 = "data/texture/user_interface/basic_interface/btnbar_mid2.bmp";
            BTN_CANCEL2 = "data/texture/user_interface/basic_interface/cancel2.bmp";
            BTN_CANCEL2_A = "data/texture/user_interface/basic_interface/cancel2_a.bmp";
            BTN_CLOSE = "data/texture/user_interface/basic_interface/btn_close.bmp";
            BTN_CLOSE2 = "data/texture/user_interface/basic_interface/close2.bmp";
            BTN_CLOSE2_A = "data/texture/user_interface/basic_interface/close2_a.bmp";
            BTN_CLOSE_A = "data/texture/user_interface/basic_interface/btn_close_a.bmp";
            BTN_CLOSE_B = "data/texture/user_interface/basic_interface/btn_close_b.bmp";
            BTN_DEL = "data/texture/user_interface/basic_interface/del.bmp";
            BTN_DEL_A = "data/texture/user_interface/basic_interface/del_a.bmp";
            BTN_DIALOG_OFF = "data/texture/user_interface/basic_interface/btn_dialog_off.bmp";
            BTN_DIALOG_ON = "data/texture/user_interface/basic_interface/btn_dialog_on.bmp";
            BTN_EQUIP_OFF = "data/texture/user_interface/basic_interface/btn_equip_off.bmp";
            BTN_EQUIP_ON = "data/texture/user_interface/basic_interface/btn_equip_on.bmp";
            BTN_FRIEND_OFF = "data/texture/user_interface/basic_interface/btn_friend_off.bmp";
            BTN_FRIEND_ON = "data/texture/user_interface/basic_interface/btn_friend_on.bmp";
            BTN_ITEMS_OFF = "data/texture/user_interface/basic_interface/btn_items_off.bmp";
            BTN_ITEMS_ON = "data/texture/user_interface/basic_interface/btn_items_on.bmp";
            BTN_MAP_OFF = "data/texture/user_interface/basic_interface/btn_map_off.bmp";
            BTN_MAP_ON = "data/texture/user_interface/basic_interface/btn_map_on.bmp";
            BTN_OFF = "data/texture/user_interface/basic_interface/btn_off.bmp";
            BTN_OPTION_OFF = "data/texture/user_interface/basic_interface/btn_option_off.bmp";
            BTN_OPTION_ON = "data/texture/user_interface/basic_interface/btn_option_on.bmp";
            BTN_REMAIL = "data/texture/user_interface/basic_interface/remail.bmp";
            BTN_REMAIL_A = "data/texture/user_interface/basic_interface/remail_a.bmp";
            BTN_RETURN = "data/texture/user_interface/basic_interface/return.bmp";
            BTN_RETURN_A = "data/texture/user_interface/basic_interface/return_a.bmp";
            BTN_SEND = "data/texture/user_interface/basic_interface/send.bmp";
            BTN_SEND_A = "data/texture/user_interface/basic_interface/send_a.bmp";
            BTN_SKILL_OFF = "data/texture/user_interface/basic_interface/btn_skill_off.bmp";
            BTN_SKILL_ON = "data/texture/user_interface/basic_interface/btn_skill_on.bmp";
            BTN_STATUS_OFF = "data/texture/user_interface/basic_interface/btn_status_off.bmp";
            BTN_STATUS_ON = "data/texture/user_interface/basic_interface/btn_status_on.bmp";
            COLLECTION_BG = "data/texture/user_interface/basic_interface/collection_bg.bmp";
            COPARISON_DISABLE_CARD_SLOT = "data/texture/user_interface/basic_interface/coparison_disable_card_slot.bmp";
            DIALOG_BG = "data/texture/user_interface/basic_interface/dialog_bg.bmp";
            DIALOG_BTN0 = "data/texture/user_interface/basic_interface/dialog_btn0.bmp";
            DIALOG_BTN1 = "data/texture/user_interface/basic_interface/dialog_btn1.bmp";
            DIALOG_BTN2 = "data/texture/user_interface/basic_interface/dialog_btn2.bmp";
            DIALSCR_DOWN = "data/texture/user_interface/basic_interface/dialscr_down.bmp";
            DIALSCR_UP = "data/texture/user_interface/basic_interface/dialscr_up.bmp";
            ENVELOP = "data/texture/user_interface/basic_interface/envelop.bmp";
            EQUIPWIN_BG = "data/texture/user_interface/basic_interface/equipwin_bg.bmp";
            EXCHANGE_BG2 = "data/texture/user_interface/basic_interface/exchange_bg2.bmp";
            GRP_LEADER = "data/texture/user_interface/basic_interface/grp_leader.bmp";
            GRP_ONLINE = "data/texture/user_interface/basic_interface/grp_online.bmp";
            GZEBLUE_LEFT = "data/texture/user_interface/basic_interface/gzeblue_left.bmp";
            GZEBLUE_MID = "data/texture/user_interface/basic_interface/gzeblue_mid.bmp";
            GZEBLUE_RIGHT = "data/texture/user_interface/basic_interface/gzeblue_right.bmp";
            GZERED_LEFT = "data/texture/user_interface/basic_interface/gzered_left.bmp";
            GZERED_MID = "data/texture/user_interface/basic_interface/gzered_mid.bmp";
            GZERED_RIGHT = "data/texture/user_interface/basic_interface/gzered_right.bmp";
            HOMUNINFO_BG = "data/texture/user_interface/basic_interface/homuninfo_bg.bmp";
            ITEMWIN_MID = "data/texture/user_interface/basic_interface/itemwin_mid.bmp";
            ITEM_INVERT = "data/texture/user_interface/basic_interface/item_invert.bmp";
            LV_UP_OFF = "data/texture/user_interface/basic_interface/lv_up_off.bmp";
            LV_UP_ON = "data/texture/user_interface/basic_interface/lv_up_on.bmp";
            MAILLIST1_BG = "data/texture/user_interface/basic_interface/maillist1_bg.bmp";
            MAILLIST2_BG = "data/texture/user_interface/basic_interface/maillist2_bg.bmp";
            MAILLIST3_BG = "data/texture/user_interface/basic_interface/maillist3_bg.bmp";
            MESBTN_010 = "data/texture/user_interface/basic_interface/mesbtn_010.bmp";
            MESBTN_02 = "data/texture/user_interface/basic_interface/mesbtn_02.bmp";
            MESBTN_04 = "data/texture/user_interface/basic_interface/mesbtn_04.bmp";
            MESBTN_05 = "data/texture/user_interface/basic_interface/mesbtn_05.bmp";
            MESBTN_08 = "data/texture/user_interface/basic_interface/mesbtn_08.bmp";
            MESBTN_09 = "data/texture/user_interface/basic_interface/mesbtn_09.bmp";
            QUEST_WINDOW = "data/texture/user_interface/basic_interface/quest_window.bmp";
            SHORTITEM_BG = "data/texture/user_interface/basic_interface/shortitem_bg.bmp";
            SKILL_UP_A = "data/texture/user_interface/basic_interface/skill_up_a.bmp";
            SKILL_UP_B = "data/texture/user_interface/basic_interface/skill_up_b.bmp";
            SKILL_UP_C = "data/texture/user_interface/basic_interface/skill_up_c.bmp";
            STATWIN0_BG = "data/texture/user_interface/basic_interface/statwin0_bg.bmp";
            SYS_BASE_OFF = "data/texture/user_interface/basic_interface/sys_base_off.bmp";
            SYS_BASE_ON = "data/texture/user_interface/basic_interface/sys_base_on.bmp";
            SYS_CLOSE_OFF = "data/texture/user_interface/basic_interface/sys_close_off.bmp";
            SYS_CLOSE_ON = "data/texture/user_interface/basic_interface/sys_close_on.bmp";
            SYS_MINI_OFF = "data/texture/user_interface/basic_interface/sys_mini_off.bmp";
            SYS_MINI_ON = "data/texture/user_interface/basic_interface/sys_mini_on.bmp";
            TAB_ITM_01 = "data/texture/user_interface/basic_interface/tab_itm_01.bmp";
            TAB_ITM_02 = "data/texture/user_interface/basic_interface/tab_itm_02.bmp";
            TAB_ITM_03 = "data/texture/user_interface/basic_interface/tab_itm_03.bmp";
            TAB_QUE_01 = "data/texture/user_interface/basic_interface/tab_que_01.bmp";
            TAB_QUE_02 = "data/texture/user_interface/basic_interface/tab_que_02.bmp";
            TAB_QUE_03 = "data/texture/user_interface/basic_interface/tab_que_03.bmp";
            TITLEBAR_FIX = "data/texture/user_interface/basic_interface/titlebar_fix.bmp";
            TITLEBAR_MID = "data/texture/user_interface/basic_interface/titlebar_mid.bmp";
            TXTBOX_BTN_A = "data/texture/user_interface/basic_interface/txtbox_btn_a.bmp";
            TXTBOX_BTN_B = "data/texture/user_interface/basic_interface/txtbox_btn_b.bmp";
            TXTBOX_BTN_C = "data/texture/user_interface/basic_interface/txtbox_btn_c.bmp";
        }

        /// Party-recruitment board buttons.
        pub mod seekparty {
            crate::paths! {
                BTN_CLEAR_A = "data/texture/user_interface/basic_interface/seekparty/btn_clear_a.bmp";
                BTN_CLEAR_B = "data/texture/user_interface/basic_interface/seekparty/btn_clear_b.bmp";
                BTN_CLEAR_C = "data/texture/user_interface/basic_interface/seekparty/btn_clear_c.bmp";
            }
        }
    }

    /// Card illustrations.
    pub mod cardbmp {
        pub fn named(name: &str) -> String {
            format!("data/texture/user_interface/cardbmp/{name}.bmp")
        }
    }

    /// Item collection illustrations.
    pub mod collection {
        pub fn named(name: &str) -> String {
            format!("data/texture/user_interface/collection/{name}.bmp")
        }
    }

    /// Pet illustrations, keyed by the pet mob.
    pub mod illust {
        crate::paths! {
            PET_ALICE = "data/texture/user_interface/illust/pet_ALICE.bmp";
            PET_BABY_DESERT_WOLF = "data/texture/user_interface/illust/pet_desert_wolf_baby.bmp";
            PET_BACSOJIN = "data/texture/user_interface/illust/pet_BACSOJIN.bmp";
            PET_BAPHOMET_JR = "data/texture/user_interface/illust/pet_baphomet.bmp";
            PET_BONGUN = "data/texture/user_interface/illust/pet_bongun.bmp";
            PET_CHONCHON = "data/texture/user_interface/illust/pet_chonchon.bmp";
            PET_CHRISTMAS_SNOW_RABBIT = "data/texture/user_interface/illust/pet_christmas_snow_rabbit.bmp";
            PET_CHUNG_E = "data/texture/user_interface/illust/pet_chung_e.bmp";
            PET_CIVIL_SERVANT = "data/texture/user_interface/illust/pet_CIVIL_SERVANT.bmp";
            PET_DELETER = "data/texture/user_interface/illust/pet_deleter_ground.bmp";
            PET_DEVIRUCHI = "data/texture/user_interface/illust/pet_deviruchi.bmp";
            PET_DIABOLIC = "data/texture/user_interface/illust/pet_diabolic.bmp";
            PET_DOKEBI = "data/texture/user_interface/illust/pet_dokkaebi.bmp";
            PET_DROPS = "data/texture/user_interface/illust/pet_drops.bmp";
            PET_DULLAHAN = "data/texture/user_interface/illust/pet_DULLAHAN.bmp";
            PET_GOBLIN_DAGGER = "data/texture/user_interface/illust/pet_goblin_dagger.bmp";
            PET_GOBLIN_EVENT = "data/texture/user_interface/illust/pet_goblin_event.bmp";
            PET_GOBLIN_FLAIL = "data/texture/user_interface/illust/pet_goblin_flail.bmp";
            PET_GOBLIN_HAMMER = "data/texture/user_interface/illust/pet_goblin_hammer.bmp";
            PET_GOBLIN_LEADER = "data/texture/user_interface/illust/pet_GOBLIN_LEADER.bmp";
            PET_GOLEM = "data/texture/user_interface/illust/pet_GOLEM.bmp";
            PET_HUNTER_FLY = "data/texture/user_interface/illust/pet_hunter_fly.bmp";
            PET_IMP = "data/texture/user_interface/illust/pet_IMP.bmp";
            PET_INCUBUS = "data/texture/user_interface/illust/pet_INCUBUS.bmp";
            PET_ISIS = "data/texture/user_interface/illust/pet_isis.bmp";
            PET_J_TAINI = "data/texture/user_interface/illust/pet_j_taini.bmp";
            PET_LEAF_CAT = "data/texture/user_interface/illust/pet_LEAF_CAT.bmp";
            PET_LOLI_RURI = "data/texture/user_interface/illust/pet_LOLI_RURI.BMP";
            PET_LUNATIC = "data/texture/user_interface/illust/pet_lunatic.bmp";
            PET_MARIONETTE = "data/texture/user_interface/illust/pet_MARIONETTE.bmp";
            PET_MEDUSA = "data/texture/user_interface/illust/pet_MEDUSA.bmp";
            PET_MIYABI_NINGYO = "data/texture/user_interface/illust/pet_MIYABI_NINGYO.bmp";
            PET_MUNAK = "data/texture/user_interface/illust/pet_munak.bmp";
            PET_NIGHTMARE_TERROR = "data/texture/user_interface/illust/pet_NIGHTMARE_TERROR.bmp";
            PET_ORC_WARRIOR = "data/texture/user_interface/illust/pet_orc_warrior.bmp";
            PET_PECOPECO = "data/texture/user_interface/illust/pet_peco_peco.bmp";
            PET_PETITE = "data/texture/user_interface/illust/pet_petit.bmp";
            PET_PICKY = "data/texture/user_interface/illust/pet_picky.bmp";
            PET_POISON_SPORE = "data/texture/user_interface/illust/pet_poison_spore.bmp";
            PET_POPORING = "data/texture/user_interface/illust/pet_popoling.bmp";
            PET_PORING = "data/texture/user_interface/illust/pet_poring.bmp";
            PET_RICE_CAKE = "data/texture/user_interface/illust/pet_rice_cake_event.bmp";
            PET_ROCKER = "data/texture/user_interface/illust/pet_rocker.bmp";
            PET_SAVAGE_BABE = "data/texture/user_interface/illust/pet_savage_babe.bmp";
            PET_SHINOBI = "data/texture/user_interface/illust/pet_SHINOBI.bmp";
            PET_SMOKIE = "data/texture/user_interface/illust/pet_smokie.bmp";
            PET_SOHEE = "data/texture/user_interface/illust/pet_sohee.bmp";
            PET_SPORE = "data/texture/user_interface/illust/pet_spore.bmp";
            PET_STEEL_CHONCHON = "data/texture/user_interface/illust/pet_steel_chonchon.bmp";
            PET_STONE_SHOOTER = "data/texture/user_interface/illust/pet_STONE_SHOOTER.bmp";
            PET_SUCCUBUS = "data/texture/user_interface/illust/pet_SUCCUBUS.bmp";
            PET_WANDERER = "data/texture/user_interface/illust/pet_wanderer.bmp";
            PET_WHISPER = "data/texture/user_interface/illust/pet_WHISPER.BMP";
            PET_WICKED_NYMPH = "data/texture/user_interface/illust/pet_WICKED_NYMPH.bmp";
            PET_YOYO = "data/texture/user_interface/illust/pet_yoyo.bmp";
            PET_ZHERLTHSH = "data/texture/user_interface/illust/pet_ZHERLTHSH.bmp";
        }

        pub fn named(name: &str) -> String {
            format!("data/texture/user_interface/illust/{name}.bmp")
        }
    }

    /// Inventory item icons.
    pub mod item {
        crate::paths! {
            CAT_PAW_HAIRPIN = "data/texture/user_interface/item/cat_paw_hairpin.bmp";
        }

        pub fn icon(name: &str) -> String {
            format!("data/texture/user_interface/item/{name}.bmp")
        }
    }

    /// Login, server-select and character-create screens.
    pub mod login {
        crate::paths! {
            ARW_AGI0 = "data/texture/user_interface/login_interface/arw_agi0.bmp";
            ARW_AGI1 = "data/texture/user_interface/login_interface/arw_agi1.bmp";
            ARW_DEX0 = "data/texture/user_interface/login_interface/arw_dex0.bmp";
            ARW_DEX1 = "data/texture/user_interface/login_interface/arw_dex1.bmp";
            ARW_INT0 = "data/texture/user_interface/login_interface/arw_int0.bmp";
            ARW_INT1 = "data/texture/user_interface/login_interface/arw_int1.bmp";
            ARW_LUK0 = "data/texture/user_interface/login_interface/arw_luk0.bmp";
            ARW_LUK1 = "data/texture/user_interface/login_interface/arw_luk1.bmp";
            ARW_STR0 = "data/texture/user_interface/login_interface/arw_str0.bmp";
            ARW_STR1 = "data/texture/user_interface/login_interface/arw_str1.bmp";
            ARW_VIT0 = "data/texture/user_interface/login_interface/arw_vit0.bmp";
            ARW_VIT1 = "data/texture/user_interface/login_interface/arw_vit1.bmp";
            BOX_SELECT = "data/texture/user_interface/login_interface/box_select.bmp";
            BTN_CONNECT = "data/texture/user_interface/login_interface/btn_connect.bmp";
            BTN_CONNECT_A = "data/texture/user_interface/login_interface/btn_connect_a.bmp";
            BTN_CONNECT_B = "data/texture/user_interface/login_interface/btn_connect_b.bmp";
            BTN_EXIT = "data/texture/user_interface/login_interface/btn_exit.bmp";
            BTN_EXIT_A = "data/texture/user_interface/login_interface/btn_exit_a.bmp";
            BTN_EXIT_B = "data/texture/user_interface/login_interface/btn_exit_b.bmp";
            CHK_SAVEOFF = "data/texture/user_interface/login_interface/chk_saveoff.bmp";
            CHK_SAVEON = "data/texture/user_interface/login_interface/chk_saveon.bmp";
            NAME_EDIT = "data/texture/user_interface/login_interface/name_edit.bmp";
            WIN_LOGIN = "data/texture/user_interface/login_interface/win_login.bmp";
            WIN_MAKE = "data/texture/user_interface/login_interface/win_make.bmp";
            WIN_MAKE2 = "data/texture/user_interface/login_interface/win_make2.bmp";
            WIN_SELECT = "data/texture/user_interface/login_interface/win_select.bmp";
            WIN_SERVICE = "data/texture/user_interface/login_interface/win_service.bmp";
        }
    }

    /// Minimap frame and per-map minimap images.
    pub mod minimap {
        crate::paths! {
            MAP_ARROW = "data/texture/user_interface/map/map_arrow.bmp";
            MAP_MINUS0 = "data/texture/user_interface/map/map_minus0.bmp";
            MAP_MINUS1 = "data/texture/user_interface/map/map_minus1.bmp";
            MAP_PLUS0 = "data/texture/user_interface/map/map_plus0.bmp";
            MAP_PLUS1 = "data/texture/user_interface/map/map_plus1.bmp";
            PRONTERA = "data/texture/user_interface/map/prontera.bmp";
        }

        pub fn of(map_name: &str) -> String {
            format!("data/texture/user_interface/map/{map_name}.bmp")
        }
    }
}

#[cfg(test)]
mod tests {
    /// `all_static_paths` lists modules by hand, so a new module can be missed.
    /// Each declared path is one assignment line inside a `paths!` block.
    #[test]
    fn every_declared_path_is_enumerated() {
        let declared = include_str!("lib.rs")
            .lines()
            .filter(|l| l.contains(" = \"data/"))
            .count();
        let enumerated = super::all_static_paths();
        assert_eq!(
            declared,
            enumerated.len(),
            "a paths! module is missing from all_static_paths()"
        );

        let mut sorted = enumerated.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), enumerated.len(), "duplicate path in registry");
    }
}
