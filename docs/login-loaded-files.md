# Files loaded when a character logs in

Debug reference: every resource the client reads from the moment a character is picked on the
character select screen until it stands in game, and whether each one comes from
`data/extracted` (`data_dir`) or from `data/data.grf`.

Snapshot from 2026-11-10, generated against the local `data/data.grf` + `data/extracted`.
The full per-file list for **prontera** is in [login-loaded-files-prontera.tsv](login-loaded-files-prontera.tsv)
(1,430 files, columns `phase`, `source`, `path`).

## Known issue: Korean-named files in `data/extracted` are never used

36,539 of the 43,715 files in `data/extracted` have Korean names. They are stored on disk in
Unicode **NFD** (decomposed Hangul). The client builds and looks up names in **NFC** (composed),
both in code (`lib/resources/src/lib.rs`) and in the names it reads from the data tables.

`DataDirIndex` (`lib/formats/src/grf.rs:50-88`) indexes the raw names returned by `read_dir`
and looks them up with `data_dir_key`. That only lowercases the name and turns `\` into `/`;
it never normalizes Unicode. So every lookup of a Korean path misses the index and silently
falls back to the GRF. Only ASCII-named files (7,176) actually override the archive.

Effect at login on prontera: 697 files are served from `data/extracted`, while **725 files sit in
`data/extracted` but are read from the GRF instead** (`EXTRACTED_NFD` in the TSV). This
includes all 214 UI textures, all map textures and models under Korean folders, and all
`sprite/이팩트/` effect sprites.

To fix it, either normalize both sides to NFC in `index_dir` and `data_dir_key`, or
re-extract or rename the folder so its names are NFC on disk.

Source values in the TSV:

| `source` | Meaning |
| --- | --- |
| `EXTRACTED` | Read from `data/extracted` (overrides the GRF). |
| `EXTRACTED_NFD` | Present in `data/extracted`, but the client misses it (NFD name) and reads the GRF copy. |
| `grf` | Only in the GRF. |
| `MISSING` | Requested but found nowhere. The client logs a warning and continues. |

## Login flow and what each step reads

| Step | Code | Reads |
| --- | --- | --- |
| 0. Client start (before login) | `client/src/main.rs:752-835` | Fonts, login windows, cursor, emotion, status overlay and damage sprites, all data tables (see below). |
| 1. Login accepted | `client/src/events/login.rs:8` | Server list window textures. |
| 2. Character list received | `client/src/events/connection.rs:20` | Char select window textures, plus one full player sprite set per character slot (see "Per character"). |
| 3. Character picked, zone info received | `connection.rs:95` | Nothing. It only connects to the map server. |
| 4. `ZC_ACCEPT_ENTER`, i.e. `handle_map_entered` | `connection.rs:201` | **Map** (`load_map`, `client/src/main.rs:266`), **all effects**, **the player's sprites**, **all in-game windows**, minimap, BGM. |
| 5. After entering the map | other handlers | Other players, monsters, NPCs and items. These are loaded on demand when the server spawns them, so they are not part of this list. |

### Step 0: tables loaded at client start (shared by every login)

All of these are at the root of `data/extracted` with ASCII names, so they **are** read from `data/extracted`:

`resnametable.txt` (path aliases, read first),
`idnum2itemdisplaynametable.txt`, `num2itemdisplaynametable.txt`,
`idnum2itemresnametable.txt`, `num2itemresnametable.txt`,
`idnum2itemdesctable.txt`, `num2itemdesctable.txt`, `itemslotcounttable.txt`,
`cardprefixnametable.txt`, `cardpostfixnametable.txt`, `num2cardillustnametable.txt`,
`skillnametable.txt`, `skilldesctable.txt`, `skilltreeview.txt`, `leveluseskillspamount.txt`,
`questid2display.txt`, `msgstringtable.txt`, `mappostable.txt`, `mapnametable.txt`,
`pettalktable.xml`, `mp3nametable.txt`.

Lua tables: `luafiles514/lua files/datainfo/accessoryid.lub` and `accname.lub`. The loader tries the
`luafiles514/` path first and falls back to a builtin table. These paths are ASCII, so they are read from `data/extracted`.

Fonts: `data/Font/NanumBarunGothic*.ttf`. Sprites: `sprite/cursors`, `sprite/shadow`, and the emotion, number and msg sprites under `sprite/이팩트/`.

### Step 4a: map (example: prontera, 324 files)

| What | Path | Source |
| --- | --- | --- |
| Map triple | `prontera.rsw`, `prontera.gnd`, `prontera.gat` | `EXTRACTED` |
| Map tables | `fogparametertable.txt`, `indoorrswtable.txt` | `EXTRACTED` |
| Click grid | `texture/grid.tga` | `EXTRACTED` |
| Ground and model textures | `texture/프론테라/` (167), `texture/워터/` (32 water frames), `texture/필드바닥/`, `texture/내부소품/`, `texture/외부소품/`, ... | `EXTRACTED_NFD` (from the GRF) |
| ASCII textures | `texture/pron-ch1.bmp` … `pron-ch9.bmp` | `EXTRACTED` |
| RSM models | `model/프론테라/` (43), `model/외부소품/`, `model/내부소품/`, `model/나무잡초꽃/` (62 in total) | `EXTRACTED_NFD` |
| Ambient sounds | `wav/se_prtbird_01.wav`, `se_prtbird_05.wav`, `se_prtchbell_03.wav`, `se_prtthewaterofabrook.wav` | `EXTRACTED` |
| Ambient effects placed in the RSW | 2 files under `sprite/이팩트/` | `EXTRACTED_NFD` |
| Minimap | `texture/유저인터페이스/map/prontera.bmp` | `EXTRACTED_NFD` |
| BGM | Track from `mp3nametable.txt`, played from `data/BGM/` (not the GRF) | n/a |

Another map gives a different list. Rerun the generator (see the end of this document) with that map name.

### Step 4b: effects preloaded on every map load (884 files, the same for every map)

`load_map` preloads **every** effect the client knows, not only the ones the map uses:

- `texture/effect/*.str` (146 STR effects) and the `.bmp`/`.tga` textures they reference (`texture/effect/`, 712 files in total). ASCII names, so they are `EXTRACTED`.
- Effect sprites: `sprite/이팩트/*.spr/.act` (134), plus a few in `sprite/아이템/`, `sprite/몬스터/` and the `sprite/` root. Korean folder, so they are `EXTRACTED_NFD`.
- Item icons used by effects: `texture/유저인터페이스/item/` (18), `EXTRACTED_NFD`.
- **Missing everywhere** (warnings expected in the log): `texture/effect/ring4.str`, `ro2year.str`,
  `ring_green.tga`, `ring_red2.tga`, `wind.tga`, `texture/유저인터페이스/item/풍마_뇌우.bmp`.

### Step 4c: in-game windows (214 files, the same for every map)

`handle_map_entered` preloads the textures of about 50 windows (chat, inventory, equipment, skill tree,
status, guild, party, shops, mail, quest, options, homunculus, mercenary, pet, ...). All of them live under
`texture/유저인터페이스/` (`basic_interface/`, `map/`, `item/`, and the root), so all of them are `EXTRACTED_NFD`.
`@emo/0.spr/.act` shows up as MISSING. It is a synthetic texture key the emotion window registers itself, not a file.

### Per character (body, head, gear)

Loaded once for each slot on the char select screen, and again for the selected character in step 4
(`client/src/sprite/mod.rs:85`, `lib/game/src/sprite_loader.rs:443`). `<sex>` is `남` (male) or `여` (female),
and `<job>` is the Korean job name.

| Part | Path pattern |
| --- | --- |
| Body | `sprite/인간족/몸통/<sex>/<job>_<sex>.spr/.act` (GM: `운영자_<sex>`) |
| Body palette (cloth colour > 0) | `palette/몸/<job>_<sex>_<n>.pal` |
| Head | `sprite/인간족/머리통/<sex>/<head>_<sex>.spr/.act` |
| Hair palette (hair colour > 0) | `palette/머리/머리<head>_<sex>_<n>.pal` |
| Weapon | `sprite/인간족/<job>/<job>_<sex><weapon suffix>.spr/.act`, or `<job>_<sex>_<item id>` for item-specific weapons. Mounted and transcendent jobs fall back to the base job. |
| Weapon trail | Same path with `_검광` appended, when it exists |
| Headgear (top, mid, bottom) | `sprite/악세사리/<sex>/<sex><accname suffix>.spr/.act`, with the suffix from `accname.lub` |
| Shield | `sprite/방패/<job>/<job>_<sex>_<shield>.spr/.act` |
| Layer order | `imf/<job>_<sex>.imf` |
| Shadow | `sprite/shadow.spr/.act` |
| Optional | Falcon, cart, and level-99 aura effects, depending on the character's state |

All of these are Korean paths, so with the NFD issue above they are read from the GRF.

## Regenerating the list

The list was produced by a throwaway harness (not in the repo). It calls the same path functions as
`handle_map_entered` (`ambient_effect_assets`, `effect_texture_paths`, `effect_spr_paths`,
`custom_effect_sprite_paths`, `skill_unit_sprite_paths`, `effect_str_names`, and each window's
`grf_texture_paths`). It then expands them through file references with
`tools/src/grf_audit/closure.rs::walk` (rsw → gnd/rsm/textures/sounds, str → textures, spr → act).
Per-character sprites are not included because they depend on the character.
