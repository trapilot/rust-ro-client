"""Extract / apply translations of Korean display text inside data files.

A unit is one line of a text file (.txt .lua .scp .xml) or one string constant
of a Lua 5.1 `.lub` that contains Hangul. `pettalktable.xml` is ciphered on disk
(ASCII letters Atbash, bytes 0xB0-0xBF mirrored; the cipher is its own inverse)
and is handled deciphered. Path-like strings are left to the path rewrite;
`manner.txt` (bad-word filter) and the resource-name tables (handled by
convert_tables.py) are skipped. In the TSV files tabs inside a unit are written
as the two characters `\\t`.

usage:
  text_units.py extract <root> <units.tsv>       # unit<TAB>count<TAB>first file
  text_units.py apply <root> <backup_dir> <translations.tsv>... [--apply]
"""
import collections, os, re, shutil, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import convert_tables as ct  # noqa: E402

H = re.compile(r'[가-힣ᄀ-ᇿ㄰-㆏]')
PATHY = re.compile(r'\.(bmp|tga|spr|act|wav|str|rsw|gnd|gat|rsm|pal|imf|lub|lua|jpg|png|mp3|gr2|ebm)\b', re.I)
TEXT_EXT = {'.txt', '.lua', '.xml', '.scp'}
SKIP = {'manner.txt', 'idnum2itemresnametable.txt', 'num2itemresnametable.txt',
        'num2cardillustnametable.txt', 'resnametable.txt'}
CIPHERED = {'pettalktable.xml'}


def pet_cipher(data):
    return bytes(155 - c if 65 <= c <= 90 else 219 - c if 97 <= c <= 122
                 else 0x16F - c if 0xB0 <= c <= 0xBF else c for c in data)


def esc(s):
    return s.replace('\t', '\\t')


def unesc(s):
    return s.replace('\\t', '\t')


def is_unit(s):
    return bool(H.search(s)) and not PATHY.search(s)


def data_files(root):
    for dp, dn, fn in os.walk(root):
        for f in fn:
            ext = os.path.splitext(f)[1].lower()
            if f.lower() not in SKIP and (ext in TEXT_EXT or ext == '.lub'):
                yield os.path.join(dp, f), f.lower(), ext


def rewrite(path, name, ext, fn):
    """Return (old_bytes, new_bytes, units_changed) with fn applied to each unit."""
    raw = open(path, 'rb').read()
    changed = 0
    if ext == '.lub':
        if raw[:5] != b'\x1bLuaQ':
            return raw, raw, 0

        def sub(s):
            nonlocal changed
            t = s.decode('cp949', 'replace')
            if not is_unit(t):
                return s
            new = fn(t)
            changed += new != t
            return new.encode('cp949')
        return raw, ct.Lub51(raw, sub).run(), changed
    data = pet_cipher(raw) if name in CIPHERED else raw
    # Decode per line: some tables mix CP949 and UTF-8 lines. Lines that are
    # neither are kept byte for byte.
    out_lines = []
    for bline in data.split(b'\n'):
        for enc in ('cp949', 'utf-8'):
            try:
                line = bline.decode(enc)
                break
            except UnicodeDecodeError:
                continue
        else:
            out_lines.append(bline)
            continue
        core = line.strip()
        if is_unit(core):
            new = fn(core)
            if new != core:
                start = line.index(core)
                line = line[:start] + new + line[start + len(core):]
                changed += 1
        out_lines.append(line.encode(enc))
    out = b'\n'.join(out_lines)
    return raw, pet_cipher(out) if name in CIPHERED else out, changed


def main():
    cmd = sys.argv[1]
    if cmd == 'extract':
        root, out = sys.argv[2:4]
        count, first = collections.Counter(), {}
        for p, name, ext in data_files(root):
            def grab(u, p=p):
                count[u] += 1
                first.setdefault(u, os.path.relpath(p, root))
                return u
            rewrite(p, name, ext, grab)
        with open(out, 'w', encoding='utf-8', newline='\n') as o:
            o.write('unit\tcount\tfile\n')
            for u in sorted(count, key=lambda u: (first[u], u)):
                o.write(f'{esc(u)}\t{count[u]}\t{first[u]}\n')
        print(f'units={len(count)} occurrences={sum(count.values())}')
    elif cmd == 'apply':
        do = '--apply' in sys.argv
        args = [a for a in sys.argv[2:] if a != '--apply']
        root, backup, tsvs = args[0], args[1], args[2:]
        tr = {}
        for t in tsvs:
            with open(t, encoding='utf-8') as f:
                next(f)
                for line in f:
                    u, e = line.rstrip('\n').split('\t')[:2]
                    tr[unesc(u)] = unesc(e)
        missing, files_changed, units = collections.Counter(), 0, 0

        def fn(u):
            if u in tr:
                return tr[u]
            missing[u] += 1
            return u
        for p, name, ext in data_files(root):
            raw, new, n = rewrite(p, name, ext, fn)
            if new != raw:
                files_changed += 1
                units += n
                if do:
                    dst = os.path.join(backup, os.path.relpath(p, root))
                    os.makedirs(os.path.dirname(dst), exist_ok=True)
                    if not os.path.exists(dst):
                        shutil.copy2(p, dst)
                    open(p, 'wb').write(new)
        print(f'files_changed={files_changed} units_changed={units} untranslated={len(missing)} apply={do}')
        for u in list(missing)[:10]:
            print('  MISSING', u[:100])


if __name__ == '__main__':
    main()
