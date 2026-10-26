"""Rewrite file paths stored inside RO data files to the English snake_case names.

Each path string is translated with the Korean->English mapping, then every
path segment gets the snake rule from rename_extracted.py (the same rule as
`snake_path` in lib/formats/src/grf.rs). Only fields that hold file paths are
touched; instance names and RSM node names are kept because other data refers
to them by name.

  .rsw  model file name (80), sound file name (80)
  .gnd  texture names (fixed length given in the header)
  .rsm  header texture names (40 bytes; length-prefixed for v2.2; v2.3+ is reported, not changed)
  .str  layer texture names (128)
  .act  sound event names (40, the block at the end of v2.1+ files)
  .lub  string constants that look like file paths (variable length)

A new name that does not fit its fixed field is reported and left unchanged.

usage:
  rewrite_inner_paths.py <mapping.tsv> <root> <backup_dir> [--apply]
"""
import collections, csv, os, re, shutil, struct, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import convert_tables as ct  # noqa: E402
from rename_extracted import snake_segment  # noqa: E402

H = re.compile(r'[가-힣]+(?: [가-힣]+)*')
EXT = re.compile(r'\.(bmp|tga|spr|act|wav|str|rsm|rsm2|gr2|jpg|png|mp3|pal|imf|ebm)$', re.I)


class Rewriter:
    def __init__(self, mapping):
        self.m = mapping
        self.stats = collections.Counter()
        self.overflow = []
        self.unmapped = collections.Counter()

    def path(self, s):
        """Translated + snake form of a path string, keeping its separators."""
        def tr(run):
            if run.group(0) in self.m:
                return self.m[run.group(0)]
            self.unmapped[run.group(0)] += 1
            return run.group(0)
        parts = re.split(r'([\\/])', H.sub(tr, s))
        return ''.join(p if p in ('\\', '/') or not p else snake_segment(p) for p in parts)

    def field(self, buf, off, size, where):
        """Rewrite one fixed-size NUL-padded string field in place."""
        raw = bytes(buf[off:off + size])
        old_b = raw.split(b'\0')[0]
        try:
            old = old_b.decode('cp949')
        except UnicodeDecodeError:
            self.stats['undecodable'] += 1
            return
        if not old:
            return
        new = self.path(old)
        if new == old:
            return
        new_b = new.encode('cp949')
        if len(new_b) >= size:
            self.overflow.append((where, old, new, size))
            return
        buf[off:off + size] = new_b + b'\0' * (size - len(new_b))
        self.stats['fields'] += 1


def u32(b, o):
    return struct.unpack_from('<I', b, o)[0]


def at_least(v, a, b):
    return v >= (a, b)


def do_rsw(rw, buf, where):
    v = (buf[4], buf[5])
    o = 6
    build = None
    if at_least(v, 2, 5):
        build = u32(buf, o); o += 4
    if at_least(v, 2, 2):
        o += 1
    o += 40 * 3
    if at_least(v, 1, 4):
        o += 40
    if not at_least(v, 2, 6):
        o += 4 if at_least(v, 1, 3) else 0
        o += 16 if at_least(v, 1, 8) else 0
        o += 4 if at_least(v, 1, 9) else 0
    if at_least(v, 1, 5):
        o += 4 + 4 + 12 + 12
    if at_least(v, 1, 7):
        o += 4
    if at_least(v, 1, 6):
        o += 16
    count = u32(buf, o); o += 4
    for _ in range(count):
        t = struct.unpack_from('<i', buf, o)[0]; o += 4
        if t == 1:
            if at_least(v, 1, 3):
                o += 40 + 12
            if at_least(v, 2, 6) and build is not None and build >= 186:
                o += 1
            rw.field(buf, o, 80, where + ':model'); o += 80
            o += 80 + 36
        elif t == 2:
            o += 80 + 12 + 12 + 4
        elif t == 3:
            o += 80
            rw.field(buf, o, 80, where + ':sound'); o += 80
            o += 12 + 4 + 4 + 4 + 4 + (4 if at_least(v, 2, 0) else 0)
        elif t == 4:
            o += 80 + 12 + 4 + 4 + 16
        else:
            rw.stats['rsw_unknown_object'] += 1
            break


def do_gnd(rw, buf, where):
    count, size = struct.unpack_from('<ii', buf, 18)
    for i in range(count):
        rw.field(buf, 26 + i * size, size, where + ':texture')


def do_rsm(rw, buf, where):
    v = (buf[4], buf[5])
    if at_least(v, 2, 3):
        rw.stats['rsm_v23_skipped'] += 1
        return bytes(buf)
    o = 6 + 8 + (1 if at_least(v, 1, 4) else 0)
    if not at_least(v, 2, 2):
        o += 16
        count = u32(buf, o); o += 4
        for i in range(count):
            rw.field(buf, o + i * 40, 40, where + ':texture')
        return bytes(buf)
    o += 4  # fps
    count = u32(buf, o); o += 4
    out = bytearray(buf[:o])
    for _ in range(count):
        n = u32(buf, o); s = bytes(buf[o + 4:o + 4 + n]); o += 4 + n
        text, nul = s.split(b'\0')[0], b'\0' if b'\0' in s else b''
        new = rw.path(text.decode('cp949')).encode('cp949')
        if new != text:
            rw.stats['fields'] += 1
        out += struct.pack('<I', len(new) + len(nul)) + new + nul
    out += buf[o:]
    return bytes(out)


def do_str(rw, buf, where):
    layers = u32(buf, 16)
    o = 36
    for _ in range(layers):
        tex = struct.unpack_from('<i', buf, o)[0]; o += 4
        for i in range(tex):
            rw.field(buf, o, 128, where + ':texture'); o += 128
        frames = struct.unpack_from('<i', buf, o)[0]; o += 4 + frames * 124


def do_act(rw, buf, where):
    v = (buf[3], buf[2])
    if not at_least(v, 2, 1):
        return
    actions = struct.unpack_from('<H', buf, 4)[0]
    tail = actions * 4 if at_least(v, 2, 2) else 0
    end = len(buf) - tail
    for n in range(0, 4096):
        start = end - n * 40 - 4
        if start < 16:
            break
        if u32(buf, start) == n:
            for i in range(n):
                rw.field(buf, start + 4 + i * 40, 40, where + ':event')
            return
    rw.stats['act_events_not_found'] += 1


def do_lub(rw, raw, where):
    def sub(s):
        try:
            t = s.decode('cp949')
        except UnicodeDecodeError:
            return s
        if not EXT.search(t):
            return s
        new = rw.path(t)
        if new != t:
            rw.stats['fields'] += 1
        return new.encode('cp949')
    return ct.Lub51(raw, sub).run()


HANDLERS = {'.rsw': (b'GRSW', do_rsw), '.gnd': (b'GRGN', do_gnd), '.rsm': (b'GRSM', do_rsm),
            '.str': (b'STRM', do_str), '.act': (b'AC', do_act)}


def main():
    mapping_tsv, root, backup = sys.argv[1:4]
    apply = '--apply' in sys.argv
    with open(mapping_tsv, encoding='utf-8', newline='') as f:
        m = {r['korean']: r['english'] for r in csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)}
    rw = Rewriter(m)
    changed = collections.Counter()
    for dp, dn, fn in os.walk(root):
        for name in fn:
            p = os.path.join(dp, name)
            ext = os.path.splitext(name)[1].lower()
            where = os.path.relpath(p, root)
            raw = open(p, 'rb').read()
            before = rw.stats['fields']
            try:
                if ext == '.lub' and raw[:5] == b'\x1bLuaQ':
                    new = do_lub(rw, raw, where)
                elif ext in HANDLERS and raw.startswith(HANDLERS[ext][0]):
                    buf = bytearray(raw)
                    out = HANDLERS[ext][1](rw, buf, where)
                    new = out if out is not None else bytes(buf)
                else:
                    continue
            except (struct.error, IndexError, ValueError, UnicodeDecodeError) as e:
                rw.stats[f'parse_error{ext}'] += 1
                print('PARSE ERROR', where, e)
                continue
            if new != raw:
                changed[ext] += 1
                if apply:
                    dst = os.path.join(backup, where)
                    os.makedirs(os.path.dirname(dst), exist_ok=True)
                    if not os.path.exists(dst):
                        shutil.copy2(p, dst)
                    open(p, 'wb').write(new)
            elif rw.stats['fields'] != before:
                rw.stats['noop_mismatch'] += 1
    print('files changed by type:', dict(changed))
    print('stats:', dict(rw.stats))
    print(f'overflow={len(rw.overflow)} unmapped_runs={len(rw.unmapped)}')
    for w, old, new, size in rw.overflow[:15]:
        print(f'  OVERFLOW {w}: {old} -> {new} ({len(new.encode("cp949"))} >= {size})')
    for r, n in rw.unmapped.most_common(10):
        print(f'  UNMAPPED {r} x{n}')
    print('applied' if apply else 'dry run')


if __name__ == '__main__':
    main()
