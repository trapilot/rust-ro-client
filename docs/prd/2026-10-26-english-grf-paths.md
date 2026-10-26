# PRD: Convert Korean text and GRF paths in source to English

Date: 2026-10-26 · Workspaces: `rust-ro-client` (main), `rust-ro-server` (comments, tools)

## Goal and background

The client hard-codes kRO GRF paths and names in Korean (`data/texture/유저인터페이스/`,
`data/sprite/인간족/몸통/남/검사_남`, status icons `프로보크.tga`, sounds, built-in
accessory/name tables, ...). The owner wants every Korean string in source converted to
English and a complete listing of what changed, so they can later rename the matching
files (and in-file references) inside `data.grf` with the same mapping.

Decisions already made by the owner:

- **Naming:** English meaning, lowercase ASCII snake_case (`유저인터페이스` -> `user_interface`,
  `몬스터` -> `monster`, `천사날개깃털` -> `angel_wing_feather`, `남`/`여` -> `male`/`female`).
- **Transition:** hard switch. Source uses English only; the client cannot load Korean
  assets until the GRF is converted. No fallback lookup code.
- **Extra scope:** delete Korean lines from the server `client_messages.rs` doc comments
  (English lines already present); translate Korean in `rust-ro-server/tools/stat-calc/*.js`;
  also convert the Hangul glyph tests (`font_atlas.rs`) and the CP949 decode test
  (`network/src/handler.rs`).

## Inventory (measured 2026-10-26)

- Client: ~2,300 lines in ~60 UTF-8 files, **1,648 unique Korean runs** (917 also occur in
  `data.grf` paths, 731 source-only — mostly newer headgear in `builtin_accessory_table.rs`).
- Server: `lib/models/src/enums/client_messages.rs` ~9,000 Korean comment lines;
  `tools/stat-calc/{db.js,old-stats-calc/item.js,old-stats-calc/card.js}` 16 lines.
- `data.grf`: 43,715 files, 36,539 with Korean paths, 5,106 unique Korean path runs.

## Scope

In scope: every UTF-8 text file in both workspaces outside `target/` and `data/`:
`.rs`, `.md`, `.json` (including the untracked local `config.json`), `.js`.

Non-goals:

- Translating `data.grf` (owner does it later using the mapping).
- Binary / EUC-KR fixtures in `rust-ro-client/lib/formats/tests/fixtures/`
  (`*.lub` Lua bytecode, `accname.lua`, `accessoryid.lua`, `idnum2itemresnametable.txt`).
  They are verbatim GRF extracts; they are re-extracted from the converted GRF later. The
  assertions in `lib/formats/tests/lua_table_integration.rs` that check values decoded from
  those fixtures stay Korean, and are listed in the report as "regenerate with GRF".
- Removing EUC-KR decoding (`decode_euc_kr`, CP949 packet decoding). GRF tables and server
  chat may still carry Korean; decoding stays.
- Any fallback/dual-path lookup.

## Requirements and acceptance criteria

1. **Mapping file** `rust-ro-client/docs/i18n/korean-to-english.tsv`, columns
   `korean<TAB>english<TAB>in_grf<TAB>note`, one row per unique Korean run found in source.
   - `english` matches `^[a-z0-9]+(_[a-z0-9]+)*$`.
   - One Korean run -> one English value, used identically everywhere.
   - Phrases with spaces (`오라 블레이드`) are one run -> `aura_blade`.
2. **Collision-free against the GRF:** applying the mapping to every `data.grf` path whose
   Korean runs are all covered must not produce two identical lowercase paths, nor a path
   equal to an existing lowercase GRF path. (Checked by script, see Verification.)
3. **Source converted:** after the change, the only Hangul left in the two workspaces
   (excluding `target/`, `data/`) is in the non-goal fixtures and the fixture assertions in
   `lua_table_integration.rs`.
4. **Korean-dependent logic rewritten** (string substitution alone is wrong here):
   - `lib/game/src/sprite_path.rs` `mercenary_weapon_sprite_path`: weapon was the body name's
     first Hangul char (`활`/`창`/`검`). Mercenary body names translate as
     `<weapon>_mercenary` (`활용병` -> `bow_mercenary`, `창용병` -> `spear_mercenary`,
     `검용병` -> `sword_mercenary`); the weapon is the text before the first `_`, and
     `ragnarok_resources::sprite::player::mercenary_weapon` takes `&str` instead of `char`.
     The weapon words must equal the mapping of `활`/`창`/`검` (`bow`/`spear`/`sword`).
   - `lib/game/src/sprite_loader.rs:547` `name.starts_with('여')` -> `starts_with("female")`.
   - `lib/game/src/sprite_path.rs:429` sex segment `"여"`/`"남"` -> `"female"`/`"male"`.
   - `lib/effects/src/effects/napalmbeat.rs:217` test prefix `폭발` -> its mapping.
   - Every other `starts_with`/`contains`/`ends_with` over a Korean literal (list in Touch
     points) must use the mapped value.
5. **Comments**: Korean prose in comments is rewritten as natural English (not token-by-token);
   Korean path names quoted in comments use the mapped value.
6. **Server `client_messages.rs`**: delete each Korean line inside the `/* ... */` doc blocks,
   keeping the date-range lines and English lines. Where a block has a Korean line with no
   English counterpart, translate it instead of deleting. Enum variants unchanged.
7. **Hangul tests** (`font_atlas.rs` `'가'`, `handler.rs` `"프론테라"`): replace with English
   (`'A'`/ `"Prontera"`), keeping the tests meaningful (the CP949 test still feeds bytes
   through the CP949 decoder; ASCII is valid CP949).
8. **Listing** `rust-ro-client/docs/i18n/conversion-report.md`: summary by file and category,
   the GRF action items, and a pointer to `conversion-occurrences.tsv`
   (`file<TAB>line<TAB>before<TAB>after`, every changed line in both workspaces).
9. Build and tests pass (see Verification), except tests that open the real
   `data/data.grf` by Korean-converted path; those are expected to fail until the GRF is
   converted and are listed by name in the report.

## Design / approach

1. **Phase A — mapping (Sonnet):** translate the worklist
   `<scratchpad>/tokens.tsv` (Korean run, counts, source context, example GRF path) into
   the mapping TSV. Rules: official iRO English names for jobs, skills, items, monsters,
   maps; RO folder conventions (`인간족` human, `몸통` body, `머리통` head, `악세사리`
   accessory, `방패` shield, `몬스터` monster, `이팩트` effect, `유저인터페이스`
   user_interface, `아이템` item, `용병` mercenary); sex `남` male / `여` female.
2. **Phase B — validate (Opus):** script checks regex, duplicates, GRF collisions; fix rows.
3. **Phase C — apply (Sonnet):** a script replaces each Korean run in in-scope files via the
   mapping (regex of maximal runs, exact lookup; unmapped run = hard error), writes the
   occurrences TSV, then manual edits for requirement 4–7 and the comment prose.
4. **Phase D — verify (Opus):** Hangul grep, build, tests, diff review, report.

## Touch points

Client (`rust-ro-client/`): `lib/resources/src/lib.rs` (path constants and `format!`
builders), `lib/game/src/{sprite_path.rs,sprite_loader.rs,status_icon.rs,sound/tables.rs,
inventory.rs,npc_shop.rs,gr2_model.rs,data_table/*.rs}`, `lib/formats/src/{builtin_accessory_table.rs,
builtin_name_table.rs,lua_table.rs,res_name_table.rs,act.rs}`, `lib/formats/tests/{grf_integration.rs,
lua_table_integration.rs}`, `lib/effects/src/{lib.rs,sfx.rs,table.rs,str_aliases.rs,
skill_units.rs,status_buff.rs,effects/*.rs}`, `lib/renderer/src/{font_atlas.rs,sprite.rs,
effect/holder.rs}`, `lib/network/src/handler.rs`, `lib/ui-component/src/game/item_info_window.rs`,
`client/src/{game_state.rs,scene/mod.rs}`, `tools/src/{bin/check_peco_sprites.rs,
bin/list_effect_sprites.rs,grf_audit/roots.rs,grf_merge/tables.rs,sprite_viewer/browser.rs}`,
`tools/ui-component-hot/src/lib.rs`, `docs/configuration.md`, `docs/internal/ui-system.md`,
`config.json`.

Korean-matching sites: `napalmbeat.rs:217`, `effects/src/lib.rs:328,349`,
`sprite_loader.rs:547`, `item_info_window.rs:1093`, `check_peco_sprites.rs:43`,
`grf_audit/roots.rs:89`, `sprite_path.rs:106-116,429`.

Server (`rust-ro-server/`): `lib/models/src/enums/client_messages.rs`,
`tools/stat-calc/db.js`, `tools/stat-calc/old-stats-calc/{item.js,card.js}`.

## Risks and edge cases

- **The GRF must use the identical mapping** for folders and file names, *and* for paths
  stored inside files: `.rsw` (model paths), `.gnd` (texture paths), `.rsm` (textures),
  `.str` (effect textures), `.act`/`.spr` siblings, Lua/TXT tables
  (`idnum2itemresnametable.txt`, `accname`, `jobname`/`npcidentity`, ...). Runtime names
  read from those tables (item resnames, NPC identity names, `여\활용병`) are composed into
  paths by the code, so tables and files must be translated together.
- The mapping covers only runs present in source (1,648); the GRF has 5,106. The GRF-only
  runs need their own translations following the same rules (possible follow-up).
- Compound tokens: same run must always map the same way; GRF collision check guards it.
- GRF lookup is case-insensitive in practice; all values are lowercase.
- Hard switch: the client is unusable with the current Korean `data.grf` until conversion.
- Tests reading the real `data.grf` will fail locally after the switch (CI has no GRF and
  skips them).

## Verification

```bash
# Hangul left (expect only non-goal fixtures + their asserts in lua_table_integration.rs)
rg -n --glob '!**/target/**' --glob '!**/data/**' '[\x{AC00}-\x{D7A3}]' rust-ro-client rust-ro-server
# client (stable toolchain)
cd rust-ro-client && cargo build --workspace --all-targets --locked && cargo test --workspace --locked
# server (nightly toolchain)
cd rust-ro-server && cargo build --workspace && cargo test --features unit_tests
# after the owner converts data.grf:
cd rust-ro-client && cargo run --release -p ragnarok-tools --bin grf-verify -- data/data.grf
```

Test baseline before the change is recorded so new failures can be attributed to the
real-GRF tests only. The `grf-verify` baseline on the current Korean GRF is saved for comparison
after GRF conversion.
