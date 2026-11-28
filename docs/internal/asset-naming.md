# Asset naming: Korean resource paths to English

The original `data.grf` names its resources in Korean (`data/sprite/인간족/몸통/남/검사_남.spr`).
The client reads only English ASCII names (`data/sprite/human/body/male/swordman_male.spr`). The
directory tree stays the same as in RO; only the names change. `data.grf` is rebuilt by the
`grf-build` tool, which renames every entry with `ragnarok-asset-names::rename()`.

The translations live in one file: `lib/asset-names/dictionary.toml`.

## How `rename()` works

1. `\` becomes `/`, and ASCII letters become lowercase.
2. A **Hangul run** is a maximal sequence of Hangul syllables (U+AC00..U+D7A3). Each run is
   translated on its own:
   - If the whole run is a key in the dictionary, its value is used.
   - Otherwise the run is split greedily, left to right, on the longest key that matches. A
     stretch of syllables that matches no key is romanized (Revised Romanization). The pieces
     are joined with `_`.
3. ASCII characters stay as they are, except a space, which becomes `_`.
4. Any other non-ASCII character becomes `u` plus its lowercase hex code point (`u3131`).
5. No `_` is inserted between a run and the characters next to it: `_고글` becomes `_goggles`,
   `검사_남` becomes `swordman_male`, `무게50이상` becomes `weight50over`.

Examples:

| Korean path | English path |
|---|---|
| `data\sprite\인간족\몸통\남\검사_남.spr` | `data/sprite/human/body/male/swordman_male.spr` |
| `data/texture/유저인터페이스/item/빨간포션.bmp` | `data/texture/user_interface/item/red_potion.bmp` |
| `effect\니벨룽겐의 반지.wav` | `effect/nibelungen_ring.wav` |
| `data/sprite/악세사리/남/남_고글.spr` | `data/sprite/accessory/male/male_goggles.spr` |

## Dictionary format

```toml
[words]
"몬스터" = "monster"
```

- Key: Hangul syllables only (U+AC00..U+D7A3). No spaces, `_`, digits or Latin letters. A key
  is exactly one run: `"검사"`, never `"검사_남"`.
- Value: matches `^[a-z0-9]+(_[a-z0-9]+)*$`. Not empty, no leading or trailing `_`.
- Every value is unique in the file. When two keys mean the same thing, translate one more
  precisely or add a suffix: `hat` and `hat_2`.
- Keys are sorted in Unicode order.

## Translation rules

- Translate what the asset **is**, using the standard English RO names: class names as in
  iRO (`novice`, `swordman`, `knight`, `assassin_cross`), skill and status names as in iRO
  (`kyrie_eleison`, `venom_bleed`), monsters as in iRO (`poring`, `deviruchi`).
- Do not use JX1 names here. Renaming game systems to JX1 is separate work.
- Short, singular, no articles (`a`, `an`, `the`).
- Possessive runs translate to the bare noun: `브라기의` is `bragi`, so `브라기의 시` becomes
  `bragi_poem`.
- Headgear (`builtin_accessory_table.rs`, the `_고글` style names) uses the iRO item name from
  the rAthena item database (`db/pre-re/item_db_equip.yml`, then `db/re/item_db_equip.yml`),
  matched by the `View` id. When a Korean name also has digits or Latin letters outside the
  run (`_RTC3등투구`), the value covers only the run (`place_helm`), so the result reads
  `_rtc3place_helm`.

## Adding or changing a word

1. Add a line under `[words]` in its sorted place.
2. Check the rules above. This script checks the file:

   ```bash
   python3 - <<'EOF'
   import re, tomllib
   from collections import Counter
   d = tomllib.load(open('lib/asset-names/dictionary.toml', 'rb'))['words']
   key = re.compile(r'^[가-힣]+$')
   val = re.compile(r'^[a-z0-9]+(_[a-z0-9]+)*$')
   assert all(key.match(k) for k in d), [k for k in d if not key.match(k)]
   assert all(val.match(v) for v in d.values()), [v for v in d.values() if not val.match(v)]
   dup = [v for v, n in Counter(d.values()).items() if n > 1]
   assert not dup, dup
   assert list(d) == sorted(d), 'keys not sorted'
   print(len(d), 'entries OK')
   EOF
   ```

3. Every Hangul run in client code must have a key (except the list below). The runs are
   listed by:

   ```bash
   export LC_ALL=C.UTF-8
   grep -rhoP '[\x{AC00}-\x{D7A3}]+' --include=*.rs lib client tools \
     --exclude=builtin_accessory_table.rs --exclude=font_atlas.rs | sort -u
   ```

4. Changing an existing value renames files. Rebuild `data.grf` with `grf-build` and update
   any literal path in code that used the old name.

### Single-syllable keys

During the greedy split, a one-syllable key matches inside any longer run that has no key of
its own. For example, an unknown run `시계` would become `poem_gye` because `시` = `poem`. Avoid
new one-syllable keys. The ones in the file are needed by paths in the code: `검 꽃 남 독 돌
떡 매 몸 별 시 여 창 책 캡 펫 햇 활`. To fix a bad split, add the longer run as its own key.

## Runs deliberately left out

These runs have no key. `rename()` romanizes them.

| Run | Where | Reason |
|---|---|---|
| `갂` | `tools/src/grf_merge/tables.rs`, doc comment | A CP949 encoding test character (`81 41`), not part of any asset name. |
| `금`, `은`, `동` | `_용투구_금/은/동` | Clearly gold, silver and bronze there, but as one-syllable keys they would break the greedy split of many other words. |
| `호` | `_고블린가면1호` .. `4호` | Counter word ("No."); one syllable, too generic. |
| `위` | `_RWC13영광의왕관1위` .. `3위` | "place" (1st place); one syllable, means "above" elsewhere. |
| `강` | `_RWC승리의왕관_8강` | "8강" = quarter-final; one syllable, means "river" or "strong" elsewhere. |
| `세` | `_랄프폰지게666세` | "the 666th"; one syllable, too generic. |
| `랄프폰지게` | `_랄프폰지게666세` | A proper name with no item in the rAthena database; spelling unknown. |
