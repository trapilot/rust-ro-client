# Wuxia Conversion Plan

How to turn this Ragnarok Online client and server into an original Asian
martial-arts (wuxia) MMORPG, with gameplay and mood inspired by games like JX1.

Status: draft, nothing implemented yet. Covers `rust-ro-client` and `rust-ro-server`.

## Goal and ground rules

**Goal:** keep the engine (networking, map runtime, combat loop, UI framework) and
replace the content and presentation: characters, classes, skills, world, UI and
audio.

**Inspired by, not a copy of.** The game borrows genre ideas that nobody owns:
wuxia setting, martial sects, five-element combat, 2.5D world. Everything else is
ours:

- No JX1 characters, sect names, maps, art, music or story, and no art extracted
  from its client, even as a placeholder.
- Ragnarok assets (GRF data) are also someone else's property. They are a temporary
  placeholder and must all be gone by Phase 8.

## Where the code stands today

| Area | Today | Why it matters |
| --- | --- | --- |
| Player sprite | Composited from body, head, weapon, headgear and shield (`PlayerSpriteData` in `lib/game/src/sprite_loader.rs`) | There is no face or eye layer to add art to |
| Head | One sprite per style, IDs 1-25 (`HEAD_MIN`/`HEAD_MAX` in `char_create_window.rs`), with hair and face drawn together | Hair, face and eyes must be split apart |
| Colors | Palette swap on the head and body (`head_palette_path`, `body_palette_path`) | Reusable for hair, skin and eye colors |
| Draw order | `ImfLayerOrder` stores one bool per frame: does the body cover the head? | Too simple for more than two layers |
| Body art | Chosen by job and sex through hardcoded Korean paths (`lib/game/src/sprite_path.rs`) | Ties appearance to RO jobs |
| Create-character packet | `PacketChMakeChar`, `2` and `3`, handled in `server/src/server/request_handler/char.rs`; they carry only `head` and `head_pal` (plus sex in `3`) | No room for new appearance fields |
| Spawn packets | `PacketZcNotify{Stand,New,Move}entry*`, read in `lib/network/src/handler.rs` | Other players cannot learn a new look through them |
| Database | `char` has `hair`, `hair_color`, `clothes_color` | Needs new columns |
| Asset loading | `data_dir` overrides every GRF | New art can ship as loose files right away, no archive needed |
| Shared code | The client uses the server's `packets`, `models` and `movement` crates by path | One packet change updates both sides, so they must be released together |

## Roadmap

Each phase should leave a game that runs. Size is relative effort, not a time estimate.

| Phase | Outcome | Depends on | Size |
| --- | --- | --- | --- |
| 0. Foundations | Decisions made, art style proven on one test sprite | None | S |
| **1. Character creation** | **Create a character with hair, eyes, face and body, seen by everyone** | 0 | **L** |
| 2. Outfits and weapons | Wuxia clothes and weapons on the new body | 1 | L |
| 3. Sects and classes | Sects replace the RO job tree | 1 | M |
| 4. Stats, skills, combat | Five-element combat and new martial skills | 3 | XL |
| 5. World | One original starter town and field | 0 | XL |
| 6. Content | Monsters, items, NPCs and quests | 4, 5 | XL |
| 7. UI, text, audio | Wuxia interface, Vietnamese text, music and effects | 1 | L |
| 8. Cutover | RO assets and leftovers removed, final name chosen | All | M |

Phases 3, 5 and 7 can run in parallel once Phase 1 is done.

## Phase 0: Foundations

- [ ] Answer the [open decisions](#open-decisions).
- [ ] Finish or cancel the paused `ragnarok` -> `ro` crate rename. It touches about 3,150
      lines across the client, so do it **before** feature work to avoid merge conflicts.
- [ ] Fix stale references to `../rust-ragnarok-server` in docs; the folder is `rust-ro-server`.
- [ ] Choose a working title that doesn't use "JX1" or any other game's name.
- [ ] Draw one test character in one direction at target size. Check it in the game
      next to an RO map before committing to the style.

**Done when:** the decisions are recorded here and the test sprite is approved.

## Phase 1: Character creation

**Result:** a player opens the create screen, picks body, face, eyes and hair, and
enters the game. The look survives a relog, and other players see the same look.

### Appearance model

| Field | Options to start | How it's drawn |
| --- | --- | --- |
| `sex` | 2 | Selects the art set (exists today) |
| `body_type` | 1, room for more | Body sprite |
| `skin_tone` | 4 | Palette on face and body |
| `face` | 4 per sex | Face sprite |
| `eye_style` | 4 per sex | Eye sprite |
| `eye_color` | 4 | Palette on eyes |
| `hair` | 6 per sex | Hair sprites, back and front |
| `hair_color` | 9 | Palette on hair (exists today) |

Start small: art cost grows quickly with each extra option, because every option is
drawn for every direction and animation.

### Layers

Draw order, back to front, when facing the camera:

```text
hair_back -> body -> face -> eyes -> hair_front -> headgear
```

- **Hair has two layers** so long hair can fall behind the body and bangs can sit in
  front of the face.
- **Facing away:** skip face and eyes, and draw the hair over the body.
- **Shared frame grid:** face, eyes and both hair layers use exactly the same
  action, direction and frame layout as each other, so they all attach at the head
  anchor without per-part offset tables.
- **New proportions:** RO characters are short and big-headed. Taller wuxia bodies mean
  new anchor values, and headgear and weapon offsets will have to be redone in Phase 2.

### Tasks

**Step 1: Tooling and placeholder art**

- [ ] Write down the art contract: canvas size, palette rules, frame grid, file names.
- [ ] Build a PNG -> SPR/ACT tool in `tools/`, so artists work in normal image editors.
- [ ] Make placeholder art for one sex: 8 directions, idle and walk.
- [ ] Add new ASCII paths under `data_dir`, for example `data/sprite/character/hair/...`.
      Don't extend the Korean path tables.

**Step 2: Client rendering**

- [ ] Add a `CharacterAppearance` struct in `lib/game` to replace loose
      `head` and `hair_color` arguments in `load_player_sprite` and its callers.
- [ ] Add the new layers to `PlayerSpriteData`, and load them with the palette logic
      already in `load_head_sprite`.
- [ ] Replace the `ImfLayerOrder` bool with an ordered layer list per direction.
- [ ] Keep the old path working for characters without new appearance data, until
      Phase 8 removes it.

**Step 3: Server and persistence**

- [ ] Add `db/add_character_appearance.sql`, adding `body_type`, `skin_tone`, `face`,
      `eye_style` and `eye_color` to `char` as `smallint NOT NULL DEFAULT 0`.
- [ ] Update `CharInsertModel`, `char_model.rs`, `Look` in `lib/models/src/status.rs`, and
      the save query in `character_repository.rs`.
- [ ] Add a new create packet instead of changing the existing RO layouts.
- [ ] **Validate every field on the server.** A modified client can send any value, so
      reject anything out of range.

**Step 4: Showing the look to other players**

- [ ] Add a new `ZC_APPEARANCE` packet (GID + appearance), sent right after spawn
      packets and to anyone a player comes into view of.
- [ ] Include appearance in the character list for the select screen.
- [ ] Handle arrival order: draw a default look until the appearance arrives, then
      rebuild the sprite through `reload_player_sprite`.

**Step 5: Create screen**

- [ ] Rebuild `lib/ui-component/src/account/char_create_window.rs` with a selector per
      field, a live preview, a rotate control and a randomize button.
- [ ] Remove RO's stat allocation (`STAT_POS`, `STAT_PARTNER`) from the screen.
- [ ] Test Vietnamese names. `name_to_char24` limits names to 23 characters, and how
      accented letters survive the trip to the database has not been checked.

### Done when

- [ ] Each field can be changed, and the preview updates at once in all 8 directions.
- [ ] Relogging shows the identical look.
- [ ] A second client sees the correct look, including when walking into view.
- [ ] No layer order glitches when walking in a circle.
- [ ] Existing characters still load, using the default values.
- [ ] A hand-crafted out-of-range create packet is rejected.

### Risks

| Risk | Mitigation |
| --- | --- |
| Art volume is the real bottleneck: options x directions x animations | Start with idle and walk only, and small option counts |
| Eyes unreadable at sprite size | Test in the game in Phase 0, and enlarge the face or use a separate portrait if needed |
| Each character adds more textures | Measure with a crowded map, and combine layers into one texture per character if needed |

## Phase 2: Outfits and weapons

- Separate body art from job: `outfit` becomes its own layer, chosen by equipment.
- Replace RO weapon types in `weapon_view_id_to_type` with a wuxia set, such as sword,
  saber, spear, staff, fist and thrown weapons.
- Redraw attack, cast, hit, sit and death animations for each weapon type.
- Redo headgear and weapon anchors for the new proportions.

## Phase 3: Sects and classes

- Design original sects with their own names, identities and roles.
- Replace RO jobs in `rust-ro-server/config/job.json`, `lib/game/src/job_class.rs` and
  the skill tree data. Decide when a player joins a sect, such as at creation or by quest.
- Remove RO job-specific rendering such as carts, falcons and mounts in `sprite_path.rs`.

## Phase 4: Stats, skills and combat

- Replace or rename the six RO stats, and rewrite the damage, hit and defense formulas.
- Add five-element generation and counter rules (metal, wood, water, fire, earth) to
  combat and character data.
- Write new skills in `config/skill.json`, and regenerate structs with `skills-struct-generator`.
- Rebalance everything against level curves in `config/exp.json`.

## Phase 5: World

- Choose the map approach (see [open decisions](#open-decisions)) and build tooling for it.
- Build one original town and one field, including collision and warps.
- Add a look suited to wuxia: lighting, fog, day and night.
- Replace the RO maps in `config/maps` and map cache only when the new ones are playable.

## Phase 6: Content

- Monsters: new designs and sprites replacing `config/mobs.json`.
- Items: new `config/items.json` and `db/pg.sql` item data. Start with a small set.
- NPCs and quests: original scripts replacing `config/npc`, through the existing script VM.
- Original story and setting bible.

## Phase 7: UI, text and audio

- New wuxia UI skin, replacing RO textures and layouts in `lib/ui-component`.
- Vietnamese text. Move strings into translation files, and check the font covers all
  Vietnamese characters.
- Replace `client_messages.rs` text, which still carries RO URLs and wording.
- New music, sound effects and skill effects.

## Phase 8: Cutover

- [ ] Delete all RO assets and the fallback rendering paths.
- [ ] Remove GRF-only requirements if no longer needed, and package our own assets.
- [ ] Rename leftovers: database schema and user, window title, docs and identifiers.
- [ ] Review licenses of all third-party code and assets in both repos.
- [ ] Have the final name and artwork checked for conflicts before any public release.

## Open decisions

| # | Decision | Recommendation | Why |
| --- | --- | --- | --- |
| 1 | Character rendering: layered 2D sprites or 3D models? | Layered 2D sprites | Fits the existing engine and the 2.5D look. 3D makes customization easier, but needs a new pipeline |
| 2 | Art source: artist, generated, or pre-rendered 3D? | Pre-render 3D models to sprites | Consistent across 8 directions, and new options cost less to add |
| 3 | Maps: keep 3D RSW/GND format or move to 2D? | Keep the format at first | Lets the world change without a rewrite of the renderer |
| 4 | Keep the RO protocol, or replace it? | Keep it, add custom packets | Far less work; there's no need for official RO client compatibility |
| 5 | How deep is face customization? | Fixed options as in Phase 1 | Sliders are impractical with sprites |
| 6 | Target language(s) | Vietnamese first, English second | To be confirmed |
