"""Rename Korean file/folder names under an extracted GRF tree via the mapping.

usage:
  rename_extracted.py <mapping.tsv> <root> <log.tsv> [--apply]   # dry run unless --apply
  rename_extracted.py --snake <root> <log.tsv> [--apply]          # snake_case names, dry run unless --apply
  rename_extracted.py --undo <root> <log.tsv>                     # reverse a logged run

Everything is checked before the first rename: every Korean run must be mapped,
and no two final paths may collide case-insensitively (Windows/NTFS) with each
other or with an existing untouched path. The log lists `new_rel<TAB>old_rel`
for each renamed entry in execution order (deepest first).

`--snake` applies the rule from docs/prd/2026-10-26-snake-case-paths.md (must
match `snake_path` in lib/formats/src/grf.rs). Siblings whose snake names
would collide are all left unrenamed.
"""
import csv, os, re, sys

H = re.compile(r'[가-힣]+(?: [가-힣]+)*')
ANY = re.compile(r'[가-힣ᄀ-ᇿ㄰-㆏]')


def snake_segment(seg):
    seg = seg.lower()
    stem, dot, ext = seg.rpartition('.')
    if not dot or not stem:
        stem, dot, ext = seg, '', ''
    keep = lambda c: not c.isascii() or c.isalnum() or c == '_'
    core = stem
    while core and not keep(core[0]):
        core = core[1:]
    while core and not keep(core[-1]):
        core = core[:-1]
    out, in_run = [], False
    for c in core:
        if keep(c):
            out.append(c)
            in_run = False
        elif not in_run:
            out.append('_')
            in_run = True
    return (''.join(out) or stem) + dot + ext


SNAKE_VECTORS = {
    '1@cata.gat': '1_cata.gat', 'lua files': 'lua_files', 'cru_grand cross.wav': 'cru_grand_cross.wav',
    'acidus_.spr': 'acidus_.spr', 'acidus__attack.wav': 'acidus__attack.wav',
    '_attack_bow.wav': '_attack_bow.wav', 'blast_mine##clock.bmp': 'blast_mine_clock.bmp',
    'c-big(side).bmp': 'c_big_side.bmp', 'mo-castle-w3+.bmp': 'mo_castle_w3.bmp',
    'moc_wall08-.bmp': 'moc_wall08.bmp', 'wl_soulexpansion .bmp': 'wl_soulexpansion.bmp',
    'skilldesctable.txt.bak': 'skilldesctable_txt.bak', '##.bmp': '##.bmp', '.hidden': 'hidden',
    '여-낙엽.act': '여_낙엽.act', 'pvp_n_2-2': 'pvp_n_2_2',
}


def snake(root, log, apply):
    for k, v in SNAKE_VECTORS.items():
        assert snake_segment(k) == v, (k, snake_segment(k), v)
    todo, kept = [], []
    for dp, dn, fn in os.walk(root):
        groups = {}
        for name in dn + fn:
            groups.setdefault(snake_segment(name), []).append(name)
        for target, names in groups.items():
            changed = [n for n in names if n != target]
            if not changed:
                continue
            rel = lambda n: os.path.relpath(os.path.join(dp, n), root)
            if len(names) > 1:
                kept.extend(rel(n) for n in names)
                continue
            todo.append((rel(names[0]), target))
    for r in kept:
        print('KEPT (snake name collides)', r)
    print(f'to_rename={len(todo)} kept={len(kept)}')
    if not apply:
        return
    todo.sort(key=lambda t: t[0].count(os.sep), reverse=True)
    with open(log, 'w', encoding='utf-8', newline='\n') as out:
        for rel, target in todo:
            parent = os.path.dirname(rel)
            os.rename(os.path.join(root, rel), os.path.join(root, parent, target))
            out.write(f'{os.path.join(parent, target)}\t{rel}\n')
            out.flush()
    print(f'renamed={len(todo)} log={log}')


def undo(root, log):
    with open(log, encoding='utf-8') as f:
        rows = [l.rstrip('\n').split('\t') for l in f if l.strip()]
    for new, old in reversed(rows):
        parent = os.path.dirname(os.path.join(root, old))
        os.rename(os.path.join(parent, os.path.basename(new)), os.path.join(root, old))
    print(f'undone={len(rows)}')


def main():
    if sys.argv[1] == '--undo':
        return undo(sys.argv[2], sys.argv[3])
    if sys.argv[1] == '--snake':
        return snake(sys.argv[2], sys.argv[3], '--apply' in sys.argv)
    mapping_tsv, root, log = sys.argv[1:4]
    apply = '--apply' in sys.argv
    with open(mapping_tsv, encoding='utf-8', newline='') as f:
        m = {r['korean']: r['english'] for r in csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)}

    def tr(name):
        return H.sub(lambda x: m[x.group(0)], name)

    unmapped, entries = set(), []
    for dp, dn, fn in os.walk(root):
        for name in dn + fn:
            rel = os.path.relpath(os.path.join(dp, name), root)
            entries.append(rel)
            unmapped.update(r for r in H.findall(name) if r not in m)
    if unmapped:
        print('UNMAPPED:', len(unmapped), ' '.join(sorted(unmapped)[:50]))
        sys.exit(1)

    # Final path of every entry, compared case-insensitively.
    final = {}
    for rel in entries:
        new = os.sep.join(tr(p) for p in rel.split(os.sep))
        final.setdefault(new.lower(), []).append(rel)
    clashes = {k: v for k, v in final.items() if len(v) > 1}
    for k, v in list(clashes.items())[:50]:
        print('COLLISION', k, '<=', ' | '.join(v))
    leftover = [r for r in entries if ANY.search(tr(os.path.basename(r)))]
    for r in leftover[:20]:
        print('JAMO LEFT', r)
    todo = [r for r in entries if H.search(os.path.basename(r))]
    print(f'entries={len(entries)} to_rename={len(todo)} collisions={len(clashes)} leftover={len(leftover)}')
    if clashes or leftover or not apply:
        sys.exit(1 if clashes or leftover else 0)

    # Deepest first, so parents are renamed after their children.
    todo.sort(key=lambda r: r.count(os.sep), reverse=True)
    with open(log, 'w', encoding='utf-8', newline='\n') as out:
        for rel in todo:
            parent, name = os.path.split(rel)
            src = os.path.join(root, rel)
            dst = os.path.join(root, parent, tr(name))
            os.rename(src, dst)
            out.write(f'{os.path.join(parent, tr(name))}\t{rel}\n')
            out.flush()
    print(f'renamed={len(todo)} log={log}')


if __name__ == '__main__':
    main()
