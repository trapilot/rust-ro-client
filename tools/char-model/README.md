# char-model: original wuxia player sprites

Builds an original humanoid (primitives only, no third-party art) in Blender, renders it into
sprite frames and packs them into the SPR/ACT/PAL/IMF files the client loads, replacing the Novice
body and every player head in `data/extracted` (the client's `data_dir` override).

## Requirements
- Blender (developed against 5.3; the scripts use only stable `bpy` calls), Workbench engine, headless.
- System `python3` with Pillow (pack/preview/synthetic). `pack.py --help` works without Pillow.

## Run
    tools/char-model/run.sh                 # real render, both sexes, writes into data/extracted
    tools/char-model/run.sh --synthetic     # stand-in frames, no Blender
    BLENDER=/path/to/Blender tools/char-model/run.sh --data /some/copy --sex m

Steps (all under `tools/char-model/out/`, git-ignored):
1. `pack.py layout` reads the original Novice body ACT (from the backup if present): motion counts,
   event ids, events, delays and the foot line -> `layout_{sex}.json`.
2. `build_model.py` -> `model_{sex}.blend`; `render.py` -> `{sex}/{layer}/aNNN_mMM[_mask].png` + `frames.json`.
   Layers: `body`, `head_<style>`. Masks mark the recolor material (ROBE on the body, HAIR on heads).
3. `pack.py preview` -> `{sex}/preview_idle_walk.png` (idle + walk contact sheet).
4. `pack.py pack` backs up every file it overwrites to `data/extracted-backup/<same path>` (never
   overwriting an existing backup), then writes ACT 2.5 / SPR 2.1 / PAL / IMF. Originals are always
   read from the backup, so re-running is idempotent. Palette indices 1..32 are a recolor ramp
   (tinted per hair color / robe color in the `.pal` files), 33..255 the quantized image colors.
5. `pack.py verify` re-parses the written files with a Python reader that mirrors the Rust parsers.

## Restore the originals
    python3 tools/char-model/pack.py restore --data data/extracted

Hair styles are assigned to head numbers cyclically: head N uses style `(N-1) % len`. Colors, hair
style names and render settings live in `styles.json`. Direction d is the root yaw -45*d degrees about Z.
