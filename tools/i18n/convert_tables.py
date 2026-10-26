"""Translate Korean resource names inside client data tables with the mapping.

The client builds sprite/texture paths from names stored in these tables, so
after renaming files with `rename_extracted.py` the tables must use the same
English names. Handles EUC-KR text tables and Lua 5.1 bytecode (`.lub`).
Korean runs that the mapping does not cover are left unchanged and counted.

usage:
  convert_tables.py <mapping.tsv> <root> <backup_dir> [--apply]   # dry run unless --apply
  convert_tables.py --selftest <root>   # identity rewrite must reproduce every .lub byte-for-byte
"""
import csv, os, re, shutil, struct, sys

H = re.compile(r'[가-힣]+(?: [가-힣]+)*')

TEXT_TABLES = [
    'idnum2itemresnametable.txt',
    'num2itemresnametable.txt',
    'num2cardillustnametable.txt',
    'resnametable.txt',
]
LUB_TABLES = [
    'luafiles514/lua_files/datainfo/accname.lub',
    'luafiles514/lua_files/datainfo/jobname.lub',
    'luafiles514/lua_files/datainfo/npcidentity.lub',
    'lua_files/datainfo/accname.lub',
    'lua_files/datainfo/jobname.lub',
    'lua_files/datainfo/npcidentity.lub',
]


class Stats:
    def __init__(self):
        self.replaced = 0
        self.unmapped = {}


def translate(text, m, stats):
    def sub(x):
        run = x.group(0)
        if run in m:
            stats.replaced += 1
            return m[run]
        stats.unmapped[run] = stats.unmapped.get(run, 0) + 1
        return run
    return H.sub(sub, text)


def convert_text(data, m, stats):
    return translate(data.decode('cp949', errors='strict'), m, stats).encode('cp949')


class Lub51:
    """Minimal Lua 5.1 bytecode reader/writer that rewrites string constants."""

    def __init__(self, data, fn):
        self.d, self.p, self.out, self.fn = data, 0, bytearray(), fn
        hdr = data[:12]
        if hdr[:4] != b'\x1bLua' or hdr[4] != 0x51 or hdr[5] != 0:
            raise ValueError(f'not Lua 5.1 bytecode: {hdr[:6]!r}')
        self.le = hdr[6] == 1
        self.int_size, self.size_t, self.instr, self.num = hdr[7], hdr[8], hdr[9], hdr[10]
        self.e = '<' if self.le else '>'
        self.out += hdr
        self.p = 12

    def raw(self, n):
        b = self.d[self.p:self.p + n]
        if len(b) != n:
            raise ValueError('truncated bytecode')
        self.p += n
        self.out += b
        return b

    def int_(self):
        fmt = {4: 'i', 8: 'q'}[self.int_size]
        return struct.unpack(self.e + fmt, self.raw(self.int_size))[0]

    def string(self, rewrite):
        fmt = {4: 'I', 8: 'Q'}[self.size_t]
        n = struct.unpack(self.e + fmt, self.d[self.p:self.p + self.size_t])[0]
        self.p += self.size_t
        if n == 0:
            self.out += struct.pack(self.e + fmt, 0)
            return
        s = self.d[self.p:self.p + n - 1]
        assert self.d[self.p + n - 1] == 0, 'string without NUL'
        self.p += n
        if rewrite:
            s = self.fn(s)
        self.out += struct.pack(self.e + fmt, len(s) + 1) + s + b'\0'

    def function(self):
        self.string(False)            # source name
        self.int_(); self.int_()      # line defined, last line defined
        self.raw(4)                   # nups, numparams, is_vararg, maxstacksize
        self.raw(self.int_() * self.instr)
        for _ in range(self.int_()):  # constants
            t = self.raw(1)[0]
            if t == 1:
                self.raw(1)
            elif t == 3:
                self.raw(self.num)
            elif t == 4:
                self.string(True)
            elif t != 0:
                raise ValueError(f'bad constant type {t}')
        for _ in range(self.int_()):  # nested prototypes
            self.function()
        self.raw(self.int_() * self.int_size)    # line info
        for _ in range(self.int_()):  # local variables
            self.string(False); self.int_(); self.int_()
        for _ in range(self.int_()):  # upvalue names
            self.string(False)

    def run(self):
        self.function()
        if self.p != len(self.d):
            raise ValueError(f'{len(self.d) - self.p} trailing bytes')
        return bytes(self.out)


def convert_lub(data, m, stats):
    return Lub51(data, lambda s: convert_text(s, m, stats)).run()


def main():
    if sys.argv[1] == '--selftest':
        root = sys.argv[2]
        for rel in LUB_TABLES:
            p = os.path.join(root, rel)
            if os.path.exists(p):
                data = open(p, 'rb').read()
                assert Lub51(data, lambda s: s).run() == data, rel
                print('identity ok', rel)
        return
    mapping_tsv, root, backup = sys.argv[1:4]
    apply = '--apply' in sys.argv
    with open(mapping_tsv, encoding='utf-8', newline='') as f:
        m = {r['korean']: r['english'] for r in csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)}
    for rel, conv in [(t, convert_text) for t in TEXT_TABLES] + [(t, convert_lub) for t in LUB_TABLES]:
        p = os.path.join(root, rel)
        if not os.path.exists(p):
            continue
        stats = Stats()
        data = open(p, 'rb').read()
        new = conv(data, m, stats)
        left = sum(stats.unmapped.values())
        sample = ' '.join(list(stats.unmapped)[:8])
        print(f'{rel}: replaced={stats.replaced} unmapped={left} ({len(stats.unmapped)} distinct) {sample}')
        if apply and new != data:
            dst = os.path.join(backup, rel)
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            if not os.path.exists(dst):  # keep the first, original copy
                shutil.copy2(p, dst)
            open(p, 'wb').write(new)
    print('applied' if apply else 'dry run')


if __name__ == '__main__':
    main()
