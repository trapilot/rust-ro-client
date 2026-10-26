# Korean -> English conversion report

Date: 2026-10-26 · PRD: [`docs/prd/2026-10-26-english-grf-paths.md`](../prd/2026-10-26-english-grf-paths.md)

## Files in this folder

| File | What it is |
| --- | --- |
| `korean-to-english.tsv` | **The mapping contract.** `korean<TAB>english<TAB>in_grf<TAB>note`, 5,835 rows. Every Korean run (a maximal run of Hangul syllables, single spaces included) maps to one lowercase snake_case English value. Use it unchanged when converting `data.grf`. |
| `conversion-occurrences.tsv` | Every changed source line in both repos: `file<TAB>line<TAB>before<TAB>after` (11,450 rows). |
| `conversion-report.md` | This file. |

Mapping row kinds (see the `note` column):

- 1,646 rows used by source code (no `extracted-only` note). Changing one of these needs a source change too.
- 4,189 rows marked `extracted-only`: names that occur only in GRF paths.
- 36 rows with `alt:`: the GRF contains two spellings of the same thing as separate files
  (`어새신`/`어세신`, `커텐`/`커튼`, ...), so the second one gets an `_alt` value to avoid a clash.
- 194 rows marked `unsure`: best-effort translations (mostly obscure headgear, card and item
  names). Rename freely, but only together with the files that use them.

Checks run against the mapping: every value matches `^[a-z0-9]+(_[a-z0-9]+)*$`; applying it to
all 36,539 Korean paths of the current `data.grf` gives **0 collisions** (case-insensitive) with
each other or with existing English paths.

## Source changes

### rust-ro-client

All Korean path/name strings now use the mapping (folders `data/sprite/human/`,
`data/sprite/monster/`, `data/texture/user_interface/`, `data/sprite/effect/`, job/sex sprite names
such as `human/body/male/swordman_male`, status icons, sound files, built-in accessory and name
tables, docs, `config.json`). Changes per file are in `conversion-occurrences.tsv`.

Logic that depended on Korean text (not just string swaps):

- `lib/game/src/sprite_path.rs` `mercenary_weapon_sprite_path`: the weapon is now the text before the
  first `_` of the body name (`bow_mercenary` -> `mercenary/bow_mercenary_bow`), and
  `ragnarok_resources::sprite::player::mercenary_weapon` takes `&str`. New test added.
- `lib/game/src/sprite_loader.rs`: mercenary sex is `name.starts_with("female")`.
- `lib/network/src/handler.rs`: the CP949 decode test feeds ASCII `prontera` + NUL + junk.
- `lib/game/src/sound/tables.rs`: `ui::BUTTON` was Hangul written as `\u{..}` escapes; now `button_sound.wav`.
- `tools/src/sprite_viewer/browser.rs`: test expectation re-ordered (English paths sort differently).
- `tools/src/bin/check_peco_sprites.rs`: name filter simplified to `peco` / `crusader`.

### rust-ro-server

- `lib/models/src/enums/client_messages.rs`: 7,136 Korean comment lines that already had an English
  line were deleted; 1,858 Korean-only lines (older wording for earlier client dates) were translated
  in place. Enum variants are unchanged.

### Deliberately not converted

- `lib/formats/tests/fixtures/*` (`*.lub` Lua bytecode, `accname.lua`, `accessoryid.lua`,
  `idnum2itemresnametable.txt`): verbatim EUC-KR GRF extracts. Regenerate them from the converted
  GRF; then update the 11 assertions in `lib/formats/tests/lua_table_integration.rs`.
- `lib/renderer/src/font_atlas.rs`: the tests use `'\u{ac00}'` (Hangul) on purpose. They check that
  the EUC-KR glyph range used for Korean chat text is in the atlas; ASCII would not test that.
- `rust-ro-server/tools/stat-calc/*.js`: not Korean. It is corrupted Japanese text in which a few
  broken byte pairs happen to form Hangul syllables.
- EUC-KR/CP949 decoding is kept: GRF tables and server chat can still contain Korean.

## `data/extracted` (the client's `data_dir`)

All 24,537 Korean file and folder names were renamed with the mapping (43,715 files before and
after, 0 Korean names left). The undo log is `data/extracted-rename-log.tsv`
(`new<TAB>old`, deepest first). The script is `tools/i18n/rename_extracted.py`:

```bash
# dry run (checks mapping coverage and collisions, renames nothing)
python tools/i18n/rename_extracted.py docs/i18n/korean-to-english.tsv data/extracted data/extracted-rename-log.tsv
# undo the rename
python tools/i18n/rename_extracted.py --undo data/extracted data/extracted-rename-log.tsv
```

### Data tables in `data/extracted` (done)

`tools/i18n/convert_tables.py` translated the resource names inside the tables the client builds
paths from: `idnum2itemresnametable.txt`, `num2itemresnametable.txt`,
`num2cardillustnametable.txt`, `resnametable.txt` (EUC-KR text) and
`luafiles514/lua_files/datainfo/{accname,jobname,npcidentity}.lub` (Lua 5.1 bytecode, string
constants rewritten; an identity rewrite reproduces each file byte for byte). The originals are in
`data/extracted-tables-backup/`. Names the mapping does not cover stay Korean. Their files do not
exist in the GRF, so they were missing before too (about 3,800 occurrences: item and card names
without a sprite or illustration). Display-name and description tables
(`*displaynametable.txt`, `*desctable.txt`, `skillnametable.txt`, `msgstringtable.txt`,
`questid2display.txt`) are UI text and were not touched.

```bash
python tools/i18n/convert_tables.py --selftest data/extracted
python tools/i18n/convert_tables.py docs/i18n/korean-to-english.tsv data/extracted data/extracted-tables-backup [--apply]
```

## What you still have to do in the GRF / extracted data

Renaming paths is not enough: the client reads names *from inside files* and builds paths from them.
These contents still hold Korean and must be translated with the same mapping:

1. Text and Lua tables: done for `data/extracted` (see above); repeat with
   `convert_tables.py` for the GRF copies when you convert `data.grf`.
2. Map files: `.rsw` (model paths), `.gnd` (texture paths), `.rsm` models (texture paths).
3. Effects: `.str` texture names; check `.act` sound references too.
4. Fixed-length string fields: an English name can be longer than the CP949 bytes it replaces.
   Check the field limits (for example RSW/GND/STR name fields) before writing.

Until that is done, item sprites, NPC/job identity sprites and map models that are named in
those tables will not load.

## Verification

- `cargo build --workspace --all-targets --locked` (client): passes.
- `cargo test --workspace --locked --no-fail-fast` (client): all pass except
  `aliased_map_resolves_through_res_name_table` (`lib/formats/tests/grf_integration.rs:130`). It reads
  `data/texture/user_interface/map/pvp_n_2-2.bmp` from the real `data/data.grf`, which still has
  Korean names. It passes once the GRF is converted. Before this change every test passed.
- `cargo build -p models` (server): passes.
- `grf-verify` on the unconverted GRF before this change: 4,152 missing resources. Run it again after
  converting the GRF and compare:
  `cargo run --release -p ragnarok-tools --bin grf-verify -- data/data.grf`
