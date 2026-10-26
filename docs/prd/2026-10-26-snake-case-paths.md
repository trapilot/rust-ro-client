# PRD: snake_case file names in `data/extracted` and source paths

Date: 2026-10-26 · Follows [`2026-10-26-english-grf-paths.md`](2026-10-26-english-grf-paths.md)

## Goal and background

After the Korean -> English conversion, 2,602 names under `data/extracted` are still not
snake_case: spaces (`lua files`, `pe_r_01 copy.bmp`, `cru_grand cross.wav`), hyphens
(`force_1-1.gat`, `china-abandoned_house01.rsm`, `arw-agi0.bmp`), `@` (`1@cata.gat`), brackets
(`hanbok_angel(main_body).spr`, `chemical_protection[weapon].tga`) and `&`, `#`, `+`. The owner
wants files, folders and source paths in snake_case.

Names reach the client from places we do not control: server map names (`1@cata`, `pvp_n_2-2`),
table contents, paths stored inside `.rsw`/`.gnd`/`.rsm`/`.str`, and the unconverted `data.grf`.
So renaming files alone breaks loading; lookups must resolve a non-snake request to the snake file.

## The snake rule (single source of truth; Python and Rust must match exactly)

Applied per path segment, after lowercasing and `\` -> `/`:

1. Split the segment at its last `.` into `stem` + `.ext` when the dot is not the first character;
   otherwise the whole segment is the stem and there is no extension.
2. A character is *kept* if it is ASCII alphanumeric, `_`, or any non-ASCII character (Hangul in
   unconverted names stays as is).
3. Drop non-kept characters at the start and end of the stem.
4. Replace every inner run of non-kept characters with a single `_`.
5. Existing underscores are never changed, collapsed or trimmed (`acidus_.spr`,
   `acidus__attack.wav` and `_attack_bow.wav` are distinct RO assets).
6. If the result is empty, keep the original stem. The extension is unchanged.

Test vectors:

| in | out |
| --- | --- |
| `1@cata.gat` | `1_cata.gat` |
| `lua files` | `lua_files` |
| `cru_grand cross.wav` | `cru_grand_cross.wav` |
| `acidus_.spr` | `acidus_.spr` |
| `acidus__attack.wav` | `acidus__attack.wav` |
| `_attack_bow.wav` | `_attack_bow.wav` |
| `blast_mine##clock.bmp` | `blast_mine_clock.bmp` |
| `c-big(side).bmp` | `c_big_side.bmp` |
| `mo-castle-w3+.bmp` | `mo_castle_w3.bmp` |
| `moc_wall08-.bmp` | `moc_wall08.bmp` |
| `wl_soulexpansion .bmp` | `wl_soulexpansion.bmp` |
| `skilldesctable.txt.bak` | `skilldesctable_txt.bak` |
| `##.bmp` | `##.bmp` |
| `.hidden` | `hidden` |
| `여-낙엽.act` | `여_낙엽.act` |
| `pvp_n_2-2` | `pvp_n_2_2` |

## Requirements

1. **Rename `data/extracted`** (Opus, `tools/i18n/rename_extracted.py --snake`): every file and folder
   whose snake name differs is renamed, deepest first, with an undo log
   (`data/extracted-snake-log.tsv`). An entry is left as is when its snake name collides
   (case-insensitively) with another entry; there are 18 such files, exact duplicates that differ
   only by punctuation (`st-tree01.bmp` / `st_tree01.bmp`, `nc_b-sideslide.*`, `aspersio_cup .bmp`).
2. **Lookup fallback in `lib/formats/src/grf.rs`** (Sonnet): one `pub fn snake_path(name: &str) -> String`
   implementing the rule on a normalized name (lowercase, `/`). For both the archive entries and the
   `DataDirIndex`:
   - lookup order: exact normalized key (today's behavior) -> `snake_path(key)` as an exact key ->
     a snake index `snake_path(original) -> original` built once at open time;
   - the snake index only holds keys where `snake_path(k) != k`, and omits ambiguous entries
     (two originals with the same snake form, or a snake form that is itself an existing key);
   - every place that today does `to_lowercase().replace('\\', "/")` for a lookup goes through the
     same helper, so `read_file`, existence checks and size queries behave the same; `file_names()`
     still lists the real names.
   - Unit tests: every vector above; a GRF with `lua files/a.lub` found via `lua_files/a.lub` and the
     reverse; data_dir file `1_cata.rsw` found via `data/1@cata.rsw`; ambiguous pair not resolved via
     the snake index but each still found by its exact name.
3. **Source paths** (Sonnet): every literal GRF path / asset file name in client source that is not
   snake under the rule is rewritten to its snake form: `lib/resources/src/lib.rs` (lua files dirs,
   `hanbok_angel(...)`, `status-*`, `arw-*`, `name-edit`), `lib/effects/src/sfx.rs` (wav names with
   spaces), `effects/cloud_projectile.rs` (`emp shock.tga`), `effects/bottom_landprotector.rs`
   (`aaa copy.bmp`), `effects/tarot_card.rs` (`blast_mine##clock.bmp`), `game/src/status_icon.rs`
   (`chemical_protection[...]`), `tools/grf-editor/src/preview.rs`, docs, and any others found.
   Not changed: map names and table/server data used as *inputs* (`pvp_n_2-2`, `1@halo`,
   `new_1-1` in res-name, map-name, map-position and connection tests), log/usage messages.
4. Build and tests as before; the only allowed failure remains the real-GRF test listed in the
   first PRD.

## Risks

- Python and Rust rules drift -> shared vectors in both test suites.
- A snake request resolving to the wrong variant -> ambiguous snake forms are never indexed.
- Tools that list or write GRFs (`grf-editor`, `grf-merge`) see real names; unchanged.

## Verification

```bash
cd rust-ro-client
python tools/i18n/rename_extracted.py --snake data/extracted data/extracted-snake-log.tsv   # dry run
cargo build --workspace --all-targets --locked
cargo test --workspace --locked --no-fail-fast
cargo test -p ragnarok-formats snake
```
