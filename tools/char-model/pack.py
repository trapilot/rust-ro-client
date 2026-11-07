#!/usr/bin/env python3
"""Pack rendered character frames into Ragnarok Online SPR / ACT / PAL / IMF files.

Subcommands: layout, synthetic, pack, verify, restore (see README.md).
Pillow is imported lazily so that `--help` works without it.
"""
from __future__ import annotations

import argparse
import json
import math
import re
import shutil
import struct
import sys
import unicodedata
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
MAGENTA_MIN_RB = 254
MAGENTA_MAX_G = 1
RAMP_FIRST = 1
RAMP_LEN = 32
QUANT_FIRST = RAMP_FIRST + RAMP_LEN  # 33
QUANT_COUNT = 256 - QUANT_FIRST  # 223
MAX_RLE = 65535
GROUPS = 13
DIRECTIONS = 8


def nfc(text):
    return unicodedata.normalize("NFC", text)


SEX_CHAR = {"m": nfc("남"), "f": nfc("여")}
NOVICE = nfc("초보자")
RACE_DIR = nfc("인간족")
BODY_DIR = nfc("몸통")
HEAD_DIR = nfc("머리통")
HEAD_PAL_DIR = nfc("머리")
BODY_PAL_DIR = nfc("몸")


class FormatError(Exception):
    pass


def _pil():
    try:
        from PIL import Image, ImageDraw, ImageFilter
    except ImportError as exc:  # pragma: no cover
        raise SystemExit("Pillow is required for this subcommand: %s" % exc)
    return Image, ImageDraw, ImageFilter


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

def backup_root(data):
    return data.parent / (data.name + "-backup")


def body_base_rel(sex):
    s = SEX_CHAR[sex]
    return Path("sprite") / RACE_DIR / BODY_DIR / s / ("%s_%s" % (NOVICE, s))


def head_dir_rel(sex):
    return Path("sprite") / RACE_DIR / HEAD_DIR / SEX_CHAR[sex]


def imf_rel(sex):
    return Path("imf") / ("%s_%s.imf" % (NOVICE, SEX_CHAR[sex]))


def with_suffix(rel, ext):
    return rel.parent / (rel.name + ext)


def orig_path(data, rel):
    """Original file: the backup copy when it exists, else the live file."""
    b = backup_root(data) / rel
    if b.exists():
        return b
    return data / rel


def list_dir_nfc(path):
    """List (nfc_name, real_name) of a directory; empty when missing."""
    if not path.is_dir():
        return []
    return sorted((nfc(p.name), p.name) for p in path.iterdir())


def find_heads(data, sex):
    """Head numbers with both .spr and .act present, with their real file stems."""
    s = SEX_CHAR[sex]
    d = orig_dir(data, head_dir_rel(sex))
    pat = re.compile(r"^(\d+)_%s\.(spr|act)$" % re.escape(s))
    found = {}
    for name, real in list_dir_nfc(d):
        m = pat.match(name)
        if m:
            found.setdefault(int(m.group(1)), {})[m.group(2)] = real
    heads = {}
    for n, kinds in sorted(found.items()):
        if "spr" in kinds and "act" in kinds:
            heads[n] = "%d_%s" % (n, s)
        else:
            print("warning: head %d (%s) lacks an spr/act pair, skipped" % (n, s), file=sys.stderr)
    return heads


def orig_dir(data, rel):
    """Directory to list: union is not needed, the backup mirrors the live dir."""
    b = backup_root(data) / rel
    return b if b.is_dir() else data / rel


def find_head_pals(data, sex, head_numbers):
    s = SEX_CHAR[sex]
    d = data / "palette" / HEAD_PAL_DIR
    pat = re.compile(r"^%s(\d+)_%s_(\d+)\.pal$" % (re.escape(nfc("머리")), re.escape(s)))
    out = []
    for name, real in list_dir_nfc(d):
        m = pat.match(name)
        if m and int(m.group(1)) in head_numbers:
            out.append((int(m.group(1)), int(m.group(2)), Path("palette") / HEAD_PAL_DIR / real))
    return out


def find_body_pals(data, sex):
    s = SEX_CHAR[sex]
    d = data / "palette" / BODY_PAL_DIR
    pat = re.compile(r"^%s_%s_(\d+)\.pal$" % (re.escape(NOVICE), re.escape(s)))
    out = []
    for name, real in list_dir_nfc(d):
        m = pat.match(name)
        if m:
            out.append((int(m.group(1)), Path("palette") / BODY_PAL_DIR / real))
    return out


# ---------------------------------------------------------------------------
# Binary readers (mirror lib/formats/src/{act,spr,imf,pal}.rs)
# ---------------------------------------------------------------------------

class Reader:
    def __init__(self, data):
        self.b = data
        self.p = 0

    def take(self, n):
        if self.p + n > len(self.b):
            raise FormatError("unexpected end of data")
        s = self.b[self.p:self.p + n]
        self.p += n
        return s

    def u8(self):
        return self.take(1)[0]

    def u16(self):
        return struct.unpack("<H", self.take(2))[0]

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def i32(self):
        return struct.unpack("<i", self.take(4))[0]

    def f32(self):
        return struct.unpack("<f", self.take(4))[0]


def version_at_least(version, major, minor):
    return version[0] > major or (version[0] == major and version[1] >= minor)


def parse_act(data):
    r = Reader(data)
    if r.take(2) != b"AC":
        raise FormatError("invalid ACT magic")
    minor = r.u8()
    major = r.u8()
    version = (major, minor)
    action_count = r.u16()
    r.take(10)
    actions = []
    for _ in range(action_count):
        motion_count = r.u32()
        motions = []
        for _ in range(motion_count):
            range1 = [r.i32() for _ in range(4)]
            range2 = [r.i32() for _ in range(4)]
            clip_count = r.u32()
            clips = []
            for _ in range(clip_count):
                clip = {"x": r.i32(), "y": r.i32(), "sprite_index": r.i32(), "mirror": r.u32()}
                if version_at_least(version, 2, 0):
                    clip["color"] = r.u32()
                    if version_at_least(version, 2, 4):
                        clip["zoom_x"] = r.f32()
                        clip["zoom_y"] = r.f32()
                    else:
                        zoom = r.f32()
                        clip["zoom_x"] = zoom
                        clip["zoom_y"] = zoom
                    clip["angle"] = r.i32()
                    clip["sprite_type"] = r.u32()
                else:
                    clip.update(color=0xFFFFFFFF, zoom_x=1.0, zoom_y=1.0, angle=0, sprite_type=0)
                if version_at_least(version, 2, 5):
                    clip["width"] = r.u32()
                    clip["height"] = r.u32()
                else:
                    clip["width"] = None
                    clip["height"] = None
                clips.append(clip)
            event_id = r.i32() if version_at_least(version, 2, 0) else -1
            attach = []
            if version_at_least(version, 2, 3):
                for _ in range(r.u32()):
                    attach.append({"ignored": r.u32(), "x": r.i32(), "y": r.i32(), "attribute": r.u32()})
            motions.append({"range1": range1, "range2": range2, "clips": clips,
                            "event_id": event_id, "attach": attach})
        actions.append(motions)
    events = []
    if version_at_least(version, 2, 1):
        for _ in range(r.u32()):
            raw = r.take(40).split(b"\0")[0]
            events.append(raw.decode("euc_kr"))
    delays = []
    if version_at_least(version, 2, 2):
        delays = [r.f32() for _ in range(action_count)]
    return {"version": version, "actions": actions, "events": events, "delays": delays}


def write_act(actions, events, delays):
    """Write ACT version 2.5. `actions` = list of motion lists; see build_act."""
    out = bytearray(b"AC")
    out += bytes([5, 2])
    out += struct.pack("<H", len(actions))
    out += b"\0" * 10
    for motions in actions:
        out += struct.pack("<I", len(motions))
        for mo in motions:
            out += struct.pack("<8i", *([0] * 8))
            out += struct.pack("<I", len(mo["clips"]))
            for c in mo["clips"]:
                out += struct.pack("<iiiI", c["x"], c["y"], c["sprite_index"], 0)
                out += struct.pack("<I", 0xFFFFFFFF)
                out += struct.pack("<ff", 1.0, 1.0)
                out += struct.pack("<iI", 0, 0)
                out += struct.pack("<II", c["width"], c["height"])
            out += struct.pack("<i", mo["event_id"])
            out += struct.pack("<I", len(mo["attach"]))
            for ax, ay in mo["attach"]:
                out += struct.pack("<IiiI", 0, ax, ay, 0)
    out += struct.pack("<I", len(events))
    for ev in events:
        raw = ev.encode("euc_kr")
        if len(raw) > 40:
            raise FormatError("event name too long: %r" % ev)
        out += raw + b"\0" * (40 - len(raw))
    if len(delays) != len(actions):
        raise FormatError("delay count %d != action count %d" % (len(delays), len(actions)))
    for dl in delays:
        out += struct.pack("<f", dl)
    return bytes(out)


def decompress_indexed(data, pixel_count, strict=False):
    """Mirror of decompress_indexed in spr.rs; strict mode validates exactness."""
    out = bytearray(pixel_count)
    i = 0
    nxt = 0
    while i < len(data) and nxt < pixel_count:
        if data[i] == 0:
            i += 1
            count = max(data[i], 1) if i < len(data) else 1
            nxt += count
        else:
            out[nxt] = data[i]
            nxt += 1
        i += 1
    if strict and (i != len(data) or nxt != pixel_count):
        raise FormatError("RLE does not decode to exactly %d pixels (got %d, consumed %d/%d)"
                          % (pixel_count, nxt, i, len(data)))
    return bytes(out)


def parse_spr(data, strict=False):
    r = Reader(data)
    if r.take(2) != b"SP":
        raise FormatError("invalid SPR magic")
    minor = r.u8()
    major = r.u8()
    version = (major, minor)
    indexed_count = r.u16()
    rgba_count = r.u16() if version_at_least(version, 1, 2) else 0
    indexed = []
    for _ in range(indexed_count):
        w = r.u16()
        h = r.u16()
        n = w * h
        if n == 0:
            px = b""
        elif version_at_least(version, 2, 1):
            size = r.u16()
            px = decompress_indexed(r.take(size), n, strict)
        else:
            px = r.take(n)
        indexed.append({"w": w, "h": h, "data": px})
    rgba = []
    for _ in range(rgba_count):
        w = r.u16()
        h = r.u16()
        rgba.append({"w": w, "h": h, "data": r.take(w * h * 4)})
    palette = None
    if version_at_least(version, 1, 1):
        palette = [tuple(r.take(4)) for _ in range(256)]
    return {"version": version, "indexed": indexed, "rgba": rgba, "palette": palette}


def rle_encode(px):
    out = bytearray()
    i = 0
    n = len(px)
    while i < n:
        if px[i] == 0:
            j = i
            while j < n and px[j] == 0:
                j += 1
            run = j - i
            while run > 0:
                c = min(run, 255)
                out += bytes([0, c])
                run -= c
            i = j
        else:
            out.append(px[i])
            i += 1
    return bytes(out)


def write_spr(images, palette):
    """Write SPR version 2.1. images = [(w, h, index_bytes)], palette = 256 RGB tuples."""
    out = bytearray(b"SP")
    out += bytes([1, 2])
    out += struct.pack("<HH", len(images), 0)
    for w, h, px in images:
        enc = rle_encode(px)
        if len(enc) > MAX_RLE:
            raise FormatError("image %dx%d RLE is %d bytes, exceeds u16" % (w, h, len(enc)))
        out += struct.pack("<HHH", w, h, len(enc)) + enc
    for r, g, b in palette:
        out += bytes([r, g, b, 0])
    return bytes(out)


def parse_pal(data):
    if len(data) < 1024:
        raise FormatError("palette shorter than 1024 bytes")
    return [tuple(data[i * 4:i * 4 + 4]) for i in range(256)]


def write_pal(palette):
    out = bytearray()
    for r, g, b in palette:
        out += bytes([r, g, b, 0])
    return bytes(out)


def parse_imf(data):
    r = Reader(data)
    version = r.f32()
    checksum = r.i32()
    max_layer = r.i32()
    layers = []
    for _ in range(max_layer + 1):
        actions = []
        for _ in range(r.i32()):
            motions = []
            for _ in range(r.i32()):
                motions.append([r.i32(), r.i32(), r.i32()])
            actions.append(motions)
        layers.append(actions)
    return {"version": version, "checksum": checksum, "max_layer": max_layer, "layers": layers}


def write_imf(imf):
    out = bytearray()
    out += struct.pack("<f", imf["version"])
    out += struct.pack("<ii", imf["checksum"], imf["max_layer"])
    for actions in imf["layers"]:
        out += struct.pack("<i", len(actions))
        for motions in actions:
            out += struct.pack("<i", len(motions))
            for prio, cx, cy in motions:
                out += struct.pack("<iii", prio, cx, cy)
    return bytes(out)


def is_magenta(c):
    return c[0] >= MAGENTA_MIN_RB and c[1] <= MAGENTA_MAX_G and c[2] >= MAGENTA_MIN_RB


def fix_magenta(c):
    c = tuple(c[:3])
    if is_magenta(c):
        return (c[0], c[1], c[2] - 2)
    return c


# ---------------------------------------------------------------------------
# Colors
# ---------------------------------------------------------------------------

def hex_rgb(text):
    text = text.lstrip("#")
    return (int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16))


def ramp_color(base, t):
    if t <= 0.75:
        k = 0.2 + 0.8 * (t / 0.75)
        return tuple(min(255, int(round(c * k))) for c in base)
    f = 0.35 * ((t - 0.75) / 0.25)
    return tuple(min(255, int(round(c + (255 - c) * f))) for c in base)


def make_ramp(base):
    return [fix_magenta(ramp_color(base, i / (RAMP_LEN - 1))) for i in range(RAMP_LEN)]


GREY = (200, 200, 200)


def load_styles():
    with open(HERE / "styles.json", encoding="utf-8") as f:
        return json.load(f)


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def save_json(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(obj, f, ensure_ascii=False, indent=1)


# ---------------------------------------------------------------------------
# Original-file helpers
# ---------------------------------------------------------------------------

def sprite_pixels_opaque(spr, clip):
    """Return (w, h, set of opaque-row flags per row) for the clip's image, or None."""
    idx = clip["sprite_index"]
    if idx < 0:
        return None
    if clip["sprite_type"] == 1:
        if idx >= len(spr["rgba"]):
            return None
        im = spr["rgba"][idx]
        w, h, d = im["w"], im["h"], im["data"]
        rows = [False] * h
        for y in range(h):
            dst = h - 1 - y
            for x in range(w):
                if d[(y * w + x) * 4] != 0:
                    rows[dst] = True
        return w, h, rows
    if idx >= len(spr["indexed"]):
        return None
    im = spr["indexed"][idx]
    w, h, d = im["w"], im["h"], im["data"]
    pal = spr["palette"]
    rows = [False] * h
    for y in range(h):
        for x in range(w):
            v = d[y * w + x]
            if v == 0:
                continue
            if pal is not None and is_magenta(pal[v]):
                continue
            rows[y] = True
            break
    return w, h, rows


def original_foot_line(act, spr):
    """Bottom edge (screen px relative to origin) of the lowest opaque row of idle dir0 motion0."""
    motion = act["actions"][0][0]
    best = None
    for clip in motion["clips"]:
        info = sprite_pixels_opaque(spr, clip)
        if info is None:
            continue
        w, h, rows = info
        last = max((i for i, v in enumerate(rows) if v), default=None)
        if last is None:
            continue
        edge = clip["y"] - h / 2.0 + last + 1
        best = edge if best is None else max(best, edge)
    if best is None:
        raise FormatError("original idle frame has no opaque pixels")
    return best


# ---------------------------------------------------------------------------
# layout
# ---------------------------------------------------------------------------

def cmd_layout(args):
    data = Path(args.data)
    rel = body_base_rel(args.sex)
    act = parse_act(orig_path(data, with_suffix(rel, ".act")).read_bytes())
    spr = parse_spr(orig_path(data, with_suffix(rel, ".spr")).read_bytes())
    layout = {
        "sex": args.sex,
        "action_count": len(act["actions"]),
        "motions": [len(m) for m in act["actions"]],
        "event_ids": [[mo["event_id"] for mo in m] for m in act["actions"]],
        "events": act["events"],
        "delays": act["delays"],
        "foot_line": original_foot_line(act, spr),
    }
    if layout["action_count"] != GROUPS * DIRECTIONS:
        print("warning: original body has %d actions, expected %d" %
              (layout["action_count"], GROUPS * DIRECTIONS), file=sys.stderr)
    save_json(Path(args.out), layout)
    print("layout %s: %d actions, foot_line %.1f -> %s" %
          (args.sex, layout["action_count"], layout["foot_line"], args.out))
    return 0


# ---------------------------------------------------------------------------
# synthetic frames
# ---------------------------------------------------------------------------

STYLE_ACCENT = [(200, 60, 60), (60, 140, 200), (80, 180, 90), (220, 180, 60)]


def cmd_synthetic(args):
    Image, ImageDraw, _ = _pil()
    styles = load_styles()
    out = Path(args.out)
    layout_path = Path(args.layout) if args.layout else out.parent / ("layout_%s.json" % args.sex)
    layout = load_json(layout_path)
    names = styles["sex"][args.sex]["hair_styles"]
    skin = hex_rgb(styles["skin"])
    scale = styles["render"]["supersample"]
    canvas = styles["render"]["canvas_px"] * scale
    ox, oy = (v * scale for v in styles["render"]["origin_px"])
    layers = ["body"] + ["head_" + n for n in names]
    frames = {"sex": args.sex, "scale": scale, "canvas": [canvas, canvas], "hair_styles": names, "layers": {}}
    facing_x = [0, -0.7, -1, -0.7, 0, 0.7, 1, 0.7]

    class Pair:
        def __init__(self):
            self.img = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
            self.mask = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
            self.d = ImageDraw.Draw(self.img)
            self.m = ImageDraw.Draw(self.mask)

        def pt(self, x, y):
            return (ox + x * scale, oy + y * scale)

        def box(self, x0, y0, x1, y1):
            return [ox + x0 * scale, oy + y0 * scale, ox + x1 * scale, oy + y1 * scale]

        def ell(self, x0, y0, x1, y1, rgb, masked=False):
            b = self.box(x0, y0, x1, y1)
            self.d.ellipse(b, fill=rgb + (255,))
            self.m.ellipse(b, fill=(255, 255, 255, 255) if masked else (0, 0, 0, 255))

        def rect(self, x0, y0, x1, y1, rgb, masked=False):
            b = self.box(x0, y0, x1, y1)
            self.d.rectangle(b, fill=rgb + (255,))
            self.m.rectangle(b, fill=(255, 255, 255, 255) if masked else (0, 0, 0, 255))

        def poly(self, pts, rgb, masked=False):
            p = [self.pt(x, y) for x, y in pts]
            self.d.polygon(p, fill=rgb + (255,))
            self.m.polygon(p, fill=(255, 255, 255, 255) if masked else (0, 0, 0, 255))

        def pie(self, x0, y0, x1, y1, a0, a1, rgb, masked=False):
            b = self.box(x0, y0, x1, y1)
            self.d.pieslice(b, a0, a1, fill=rgb + (255,))
            self.m.pieslice(b, a0, a1, fill=(255, 255, 255, 255) if masked else (0, 0, 0, 255))

    def draw_body(p, group, d, m, count):
        ph = m / float(max(count, 1))
        walk = math.sin(2 * math.pi * ph) if group == 1 else 0.0
        drop = 20 if group == 2 else 0
        side = abs(facing_x[d]) > 0.5
        for sgn in (-1, 1):
            sw = walk * sgn
            if group == 2:
                p.rect(sgn * 7 - 5, -8, sgn * 7 + 14, -2, (40, 36, 34))
                continue
            if side:
                p.rect(sgn * 3 - 5 + sw * 10, -34 + drop, sgn * 3 + 5 + sw * 10, -2, (40, 36, 34))
            else:
                lift = max(0.0, sw) * 5
                p.rect(sgn * 7 - 5, -34 + drop - lift, sgn * 7 + 5, -2 - lift, (40, 36, 34))
        p.poly([(-13, -36 + drop), (13, -36 + drop), (17, -6), (-17, -6)], (120, 120, 120), True)
        p.poly([(-13, -36 + drop), (0, -36 + drop), (-7, -6), (-17, -6)], (170, 170, 170), True)
        p.rect(-13, -66 + drop, 13, -34 + drop, (150, 150, 150), True)
        p.ell(-13, -66 + drop, 0, -40 + drop, (200, 200, 200), True)
        p.rect(-13, -42 + drop, 13, -37 + drop, (140, 50, 50))
        for sgn in (-1, 1):
            sw = -walk * sgn * 5
            p.ell(sgn * 15 - 3, -62 + drop + sw, sgn * 15 + 5 if sgn > 0 else sgn * 15 + 3, -36 + drop + sw, skin)
        p.rect(-3, -70 + drop, 3, -64 + drop, skin)

    def draw_head(p, group, d, m, style_idx, turn):
        drop = 20 if group == 2 else 0
        cy = -82 + drop
        p.ell(-11, cy - 12, 11, cy + 12, skin)
        lum = 90 + 30 * style_idx
        p.pie(-12, cy - 13, 12, cy + 11, 180, 360, (lum, lum, lum), True)
        k = style_idx % 4
        if k in (0, 2):
            p.ell(-5, cy - 22, 5, cy - 12, (lum + 40,) * 3, True)
        if k in (1, 2):
            p.ell(-6, cy - 2, 6, cy + 24, (lum - 20,) * 3, True)
        if k == 3:
            p.rect(-12, cy - 8, 12, cy - 5, STYLE_ACCENT[style_idx % 4])
        if d not in (3, 4, 5):
            fx = facing_x[d] * 5 + (3 if turn == 1 else -3 if turn == 2 else 0)
            for ex in (-4, 4):
                p.ell(ex + fx - 1.5, cy + 1, ex + fx + 1.5, cy + 5, (20, 20, 24))

    def rotate_lying(p, rotate):
        if not rotate:
            return
        center = p.pt(0, -34)
        p.img = p.img.rotate(-90, center=center, resample=Image.NEAREST)
        p.mask = p.mask.rotate(-90, center=center, resample=Image.NEAREST)

    for layer in layers:
        ldir = out / layer
        ldir.mkdir(parents=True, exist_ok=True)
        entries = {}
        style_idx = names.index(layer[5:]) if layer.startswith("head_") else 0
        for a, count in enumerate(layout["motions"]):
            group, d = divmod(a, DIRECTIONS)
            for m in range(count):
                key = "a%03d_m%02d" % (a, m)
                turn = m if group in (0, 2) and m in (1, 2) else 0
                p = Pair()
                if layer == "body":
                    draw_body(p, group, d, m, count)
                else:
                    draw_head(p, group, d, m, style_idx, turn)
                rotate_lying(p, group == 8)
                p.img.save(ldir / (key + ".png"), compress_level=1)
                p.mask.save(ldir / (key + "_mask.png"), compress_level=1)
                drop = 20 if group == 2 else 0
                ax, ay = 0, -66 + drop
                if group == 8:
                    # Lying figure is rotated 90 degrees clockwise about (0, -34).
                    ax, ay = 32, -34
                entries[key] = {"origin": [ox, oy], "anchor": [ox + ax * scale, oy + ay * scale]}
        frames["layers"][layer] = entries
    save_json(out / "frames.json", frames)
    print("synthetic %s: %d layers -> %s" % (args.sex, len(layers), out))
    return 0


# ---------------------------------------------------------------------------
# preview
# ---------------------------------------------------------------------------

def cmd_preview(args):
    """Contact sheet: idle and walk, 8 directions, body + head composited by anchors."""
    Image, ImageDraw, _ = _pil()
    frames_dir = Path(args.frames)
    frames = load_json(frames_dir / "frames.json")
    s = frames["scale"]
    style = args.style or frames["hair_styles"][0]
    head_layer = "head_" + style
    walk_motions = sorted({int(k.split("_m")[1]) for k in frames["layers"]["body"] if k.startswith("a008_")})
    rows = [(0, 0)] + [(1, m) for m in walk_motions[::2]]
    cw, ch = 100 * 2, 130 * 2
    sheet = Image.new("RGBA", (cw * 8, ch * len(rows)), (96, 128, 100, 255))
    for r, (group, m) in enumerate(rows):
        for d in range(8):
            key = "a%03d_m%02d" % (group * 8 + d, m)
            if key not in frames["layers"]["body"]:
                continue
            body = Image.open(frames_dir / "body" / (key + ".png")).convert("RGBA")
            head = Image.open(frames_dir / head_layer / (key + ".png")).convert("RGBA")
            bi, hi = frames["layers"]["body"][key], frames["layers"][head_layer][key]
            dx = int(round(bi["anchor"][0] - hi["anchor"][0]))
            dy = int(round(bi["anchor"][1] - hi["anchor"][1]))
            comp = body.copy()
            comp.alpha_composite(head, (dx, dy)) if (dx, dy) == (0, 0) else comp.paste(head, (dx, dy), head)
            small = comp.convert("RGBa").reduce(s // 2).convert("RGBA")
            ox, oy = bi["origin"][0] / s * 2, bi["origin"][1] / s * 2
            left, top = int(ox - cw / 2), int(oy - ch * 0.88)
            cell = small.crop((left, top, left + cw, top + ch))
            sheet.alpha_composite(cell, (d * cw, r * ch))
    out = Path(args.out) if args.out else frames_dir / "preview_idle_walk.png"
    sheet.save(out)
    print("preview -> %s" % out)
    return 0


# ---------------------------------------------------------------------------
# pack
# ---------------------------------------------------------------------------

class Packer:
    def __init__(self, frames_dir, frames, styles, dy):
        self.dir = frames_dir
        self.frames = frames
        self.scale = frames["scale"]
        r = styles["render"]
        self.outline_px = int(r.get("outline_px", 1))
        self.outline_color = hex_rgb(r.get("outline_color", "#1e1816"))
        self.lum_lo = r.get("ramp_lum_lo", 50)
        self.lum_hi = r.get("ramp_lum_hi", 235)
        self.dy = dy
        self.frame_cache = {}
        self.part_cache = {}

    def resolve(self, layer, a, m):
        entries = self.frames["layers"][layer]
        key = "a%03d_m%02d" % (a, m)
        if key in entries:
            return key
        for mm in range(m - 1, -1, -1):
            k = "a%03d_m%02d" % (a, mm)
            if k in entries:
                return k
        k = "a%03d_m00" % a
        if k in entries:
            return k
        for aa in range(a - 1, -1, -1):
            k = "a%03d_m00" % aa
            if k in entries:
                return k
        raise FormatError("no frame for layer %s action %d motion %d" % (layer, a, m))

    def process(self, layer, key):
        ck = (layer, key)
        if ck in self.frame_cache:
            return self.frame_cache[ck]
        Image, _, ImageFilter = _pil()
        png = self.dir / layer / (key + ".png")
        mpng = self.dir / layer / (key + "_mask.png")
        s = self.scale
        small = Image.open(png).convert("RGBA").convert("RGBa").reduce(s).convert("RGBA")
        msmall = None
        if mpng.exists():
            msmall = Image.open(mpng).convert("RGBA").convert("RGBa").reduce(s).convert("RGBA")
        alpha = small.getchannel("A").point(lambda v: 255 if v >= 128 else 0)
        info = self.frames["layers"][layer][key]
        ox, oy = info["origin"][0] / float(s), info["origin"][1] / float(s)
        ax, ay = info["anchor"][0] / float(s), info["anchor"][1] / float(s)
        bbox = alpha.getbbox()
        if bbox is None:
            res = {"empty": True, "w": 1, "h": 1, "left": 0, "top": 0, "pix": [None],
                   "ox": ox, "oy": oy, "ax": ax, "ay": ay}
            self.frame_cache[ck] = res
            return res
        o = self.outline_px
        dil = alpha.filter(ImageFilter.MaxFilter(2 * o + 1)) if o > 0 else alpha
        x0, y0, x1, y1 = dil.getbbox()
        W, H = small.size
        left, top = max(0, x0 - 1), max(0, y0 - 1)
        right, bottom = min(W, x1 + 1), min(H, y1 + 1)
        sp, ap, dp = small.load(), alpha.load(), dil.load()
        mp = msmall.load() if msmall is not None else None
        pix = []
        lo, hi = float(self.lum_lo), float(self.lum_hi)
        for yy in range(top, bottom):
            for xx in range(left, right):
                if ap[xx, yy]:
                    r, g, b, _ = sp[xx, yy]
                    if mp is not None and mp[xx, yy][0] >= 128 and mp[xx, yy][3] >= 128:
                        lum = 0.299 * r + 0.587 * g + 0.114 * b
                        t = min(1.0, max(0.0, (lum - lo) / (hi - lo)))
                        pix.append(RAMP_FIRST + int(round(t * (RAMP_LEN - 1))))
                    else:
                        pix.append((r, g, b))
                elif dp[xx, yy]:
                    pix.append(self.outline_color)
                else:
                    pix.append(None)
        res = {"empty": False, "w": right - left, "h": bottom - top, "left": left, "top": top,
               "pix": pix, "ox": ox, "oy": oy, "ax": ax, "ay": ay}
        self.frame_cache[ck] = res
        return res

    def foot_edge(self, layer="body"):
        fr = self.process(layer, self.resolve(layer, 0, 0))
        if fr["empty"]:
            raise FormatError("idle frame of our body is empty")
        w = fr["w"]
        last = max(i // w for i, v in enumerate(fr["pix"]) if v is not None)
        return fr["top"] + last + 1 - fr["oy"]

    def build_part(self, layer, counts):
        ck = (layer, tuple(counts))
        if ck in self.part_cache:
            return self.part_cache[ck]
        Image, _, _ = _pil()
        keys = [(a, m) for a, c in enumerate(counts) for m in range(c)]
        resolved = {k: self.resolve(layer, *k) for k in keys}
        uniq = sorted(set(resolved.values()))
        frs = {k: self.process(layer, k) for k in uniq}

        hist = Counter()
        for fr in frs.values():
            for v in fr["pix"]:
                if isinstance(v, tuple):
                    hist[v] += 1
        total = sum(hist.values())
        cap = 600000
        samples = []
        if total > cap:
            for c, n in hist.items():
                samples.extend([c] * max(1, n * cap // total))
        else:
            for c, n in hist.items():
                samples.extend([c] * n)
        if samples:
            width = 1024
            rows = (len(samples) + width - 1) // width
            samples.extend([samples[0]] * (rows * width - len(samples)))
            im = Image.new("RGB", (width, rows))
            im.putdata(samples)
            q = im.quantize(colors=QUANT_COUNT, method=Image.MEDIANCUT)
            pal = q.getpalette()
            used = max(idx for _, idx in q.getcolors()) + 1
            qcolors = [fix_magenta(tuple(pal[i * 3:i * 3 + 3])) for i in range(used)]
        else:
            qcolors = [(24, 24, 24)]
        while len(qcolors) < QUANT_COUNT:
            qcolors.append(qcolors[-1])

        near_cache = {}

        def nearest(c):
            v = near_cache.get(c)
            if v is None:
                best, bd = 0, None
                for i, p in enumerate(qcolors):
                    d = (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 + (p[2] - c[2]) ** 2
                    if bd is None or d < bd:
                        best, bd = i, d
                        if d == 0:
                            break
                v = QUANT_FIRST + best
                near_cache[c] = v
            return v

        images = []
        image_index = {}
        frame_image = {}
        for k in uniq:
            fr = frs[k]
            px = bytearray(fr["w"] * fr["h"])
            for i, v in enumerate(fr["pix"]):
                if v is None:
                    continue
                px[i] = v if isinstance(v, int) else nearest(v)
            sig = (fr["w"], fr["h"], bytes(px))
            if sig not in image_index:
                image_index[sig] = len(images)
                images.append(sig)
            frame_image[k] = image_index[sig]

        base_pal = [(0, 0, 0)] + make_ramp(GREY) + qcolors
        assert len(base_pal) == 256
        refs = {}
        dy = self.dy
        for k in keys:
            fr = frs[resolved[k]]
            if fr["empty"]:
                x, y = 0, 0
            else:
                x = int(math.floor(fr["left"] + fr["w"] / 2.0 - fr["ox"] + 0.5))
                y = int(math.floor(fr["top"] + fr["h"] / 2.0 - fr["oy"] + 0.5)) + dy
            ax = int(math.floor(fr["ax"] - fr["ox"] + 0.5))
            ay = int(math.floor(fr["ay"] - fr["oy"] + 0.5)) + dy
            refs[k] = (frame_image[resolved[k]], x, y, ax, ay)
        part = {"images": images, "palette": base_pal, "refs": refs}
        self.part_cache[ck] = part
        return part


def build_act(orig_act, part):
    actions = []
    for a, orig_motions in enumerate(orig_act["actions"]):
        motions = []
        for m, om in enumerate(orig_motions):
            idx, x, y, ax, ay = part["refs"][(a, m)]
            w, h, _ = part["images"][idx]
            clip = {"x": x, "y": y, "sprite_index": idx, "width": w, "height": h}
            motions.append({"clips": [clip], "event_id": om["event_id"], "attach": [(ax, ay)]})
        actions.append(motions)
    delays = orig_act["delays"] or [4.0] * len(actions)
    return write_act(actions, orig_act["events"], delays)


def tinted_palette(base_pal, rgb):
    pal = list(base_pal)
    ramp = make_ramp(rgb)
    for i in range(RAMP_LEN):
        pal[RAMP_FIRST + i] = ramp[i]
    return pal


def safe_copy_backup(data, rel):
    src = data / rel
    dst = backup_root(data) / rel
    if dst.exists() or not src.exists():
        return
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    if not dst.exists() or dst.stat().st_size != src.stat().st_size:
        raise SystemExit("backup copy failed for %s" % rel)


def write_live(data, rel, payload):
    dst = data / rel
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_bytes(payload)


def cmd_pack(args):
    data = Path(args.data)
    sex = args.sex
    styles = load_styles()
    frames_dir = Path(args.frames)
    frames = load_json(frames_dir / "frames.json")
    body_rel = body_base_rel(sex)

    heads = find_heads(data, sex)
    if not heads:
        raise SystemExit("no heads found for sex %s" % sex)
    head_pals = find_head_pals(data, sex, set(heads))
    body_pals = find_body_pals(data, sex)

    to_write = [with_suffix(body_rel, ".spr"), with_suffix(body_rel, ".act"), imf_rel(sex)]
    to_write += [p for _, p in body_pals]
    hdir = head_dir_rel(sex)
    for n, stem in heads.items():
        to_write += [hdir / (stem + ".spr"), hdir / (stem + ".act")]
    to_write += [p for _, _, p in head_pals]

    for rel in to_write:
        safe_copy_backup(data, rel)

    style_names = frames["hair_styles"]
    layout_counts = None

    body_act = parse_act(orig_path(data, with_suffix(body_rel, ".act")).read_bytes())
    foot = original_foot_line(body_act, parse_spr(orig_path(data, with_suffix(body_rel, ".spr")).read_bytes()))
    packer = Packer(frames_dir, frames, styles, 0)
    ours = packer.foot_edge("body")
    packer.dy = int(round(foot - ours))
    print("foot line: original %.1f, ours %.1f, dy %d" % (foot, ours, packer.dy))

    # Body
    counts = [len(m) for m in body_act["actions"]]
    layout_counts = counts
    part = packer.build_part("body", counts)
    write_live(data, with_suffix(body_rel, ".spr"), write_spr(part["images"], part["palette"]))
    write_live(data, with_suffix(body_rel, ".act"), build_act(body_act, part))
    body_base = part["palette"]
    robe_colors = styles["robe_colors"][sex]
    for c, rel in body_pals:
        pal = tinted_palette(body_base, hex_rgb(robe_colors[c % len(robe_colors)]))
        write_live(data, rel, write_pal(pal))

    imf = parse_imf(orig_path(data, imf_rel(sex)).read_bytes())
    for action in imf["layers"][0]:
        for motion in action:
            motion[0] = 1
    write_live(data, imf_rel(sex), write_imf(imf))

    # Heads
    hair_colors = styles["hair_colors"]
    head_base = {}
    for n, stem in heads.items():
        style = style_names[(n - 1) % len(style_names)]
        act = parse_act(orig_path(data, hdir / (stem + ".act")).read_bytes())
        hcounts = [len(m) for m in act["actions"]]
        if hcounts != layout_counts:
            print("note: head %d motion counts differ from the body's" % n, file=sys.stderr)
        part = packer.build_part("head_" + style, hcounts)
        write_live(data, hdir / (stem + ".spr"), write_spr(part["images"], part["palette"]))
        write_live(data, hdir / (stem + ".act"), build_act(act, part))
        head_base[n] = part["palette"]
    for n, c, rel in head_pals:
        pal = tinted_palette(head_base[n], hex_rgb(hair_colors[c % len(hair_colors)]))
        write_live(data, rel, write_pal(pal))

    print("packed %s: body + %d heads, %d body pals, %d head pals" %
          (sex, len(heads), len(body_pals), len(head_pals)))
    return 0


# ---------------------------------------------------------------------------
# verify
# ---------------------------------------------------------------------------

def verify_sex(data, sex, errors):
    body_rel = body_base_rel(sex)
    bk = backup_root(data)
    if not (bk / with_suffix(body_rel, ".act")).exists():
        print("verify %s: no backup of the body, skipped" % sex)
        return False

    def err(msg):
        errors.append("[%s] %s" % (sex, msg))

    def check_pair(rel_base, label):
        try:
            new_act = parse_act((data / with_suffix(rel_base, ".act")).read_bytes())
            new_spr = parse_spr((data / with_suffix(rel_base, ".spr")).read_bytes(), strict=True)
            old_act = parse_act((bk / with_suffix(rel_base, ".act")).read_bytes())
        except (FormatError, OSError, UnicodeDecodeError, struct.error) as exc:
            err("%s: %s" % (label, exc))
            return None
        if new_act["version"] != (2, 5):
            err("%s: ACT version %s != 2.5" % (label, new_act["version"]))
        if len(new_act["actions"]) != len(old_act["actions"]):
            err("%s: action count %d != original %d" % (label, len(new_act["actions"]), len(old_act["actions"])))
            return None
        if len(new_act["actions"]) != GROUPS * DIRECTIONS:
            err("%s: action count %d != 104" % (label, len(new_act["actions"])))
        if new_act["events"] != old_act["events"]:
            err("%s: events differ from original" % label)
        if new_act["delays"] != old_act["delays"] and old_act["delays"]:
            err("%s: delays differ from original" % label)
        images = new_spr["indexed"]
        pal = new_spr["palette"]
        used = set()
        for img in images:
            used.update(img["data"])
            if len(img["data"]) != img["w"] * img["h"]:
                err("%s: image pixel count mismatch" % label)
        for i in sorted(used):
            if i and pal and is_magenta(pal[i]):
                err("%s: palette index %d is magenta" % (label, i))
        if pal and any(c[3] != 0 for c in pal):
            err("%s: palette alpha is not 0" % label)
        for a, (nm, om) in enumerate(zip(new_act["actions"], old_act["actions"])):
            if len(nm) != len(om):
                err("%s: action %d motion count %d != original %d" % (label, a, len(nm), len(om)))
                continue
            for m, (mn, mo) in enumerate(zip(nm, om)):
                if mn["event_id"] != mo["event_id"]:
                    err("%s: action %d motion %d event_id differs" % (label, a, m))
                if len(mn["attach"]) != 1:
                    err("%s: action %d motion %d has %d attach points" % (label, a, m, len(mn["attach"])))
                for c in mn["clips"]:
                    si = c["sprite_index"]
                    if not 0 <= si < len(images):
                        err("%s: action %d motion %d sprite_index %d out of range" % (label, a, m, si))
                    elif (c["width"], c["height"]) != (images[si]["w"], images[si]["h"]):
                        err("%s: action %d motion %d clip size != image size" % (label, a, m))
        return new_act

    check_pair(body_rel, "body")
    heads = find_heads(data, sex)
    hdir = head_dir_rel(sex)
    for n, stem in heads.items():
        check_pair(hdir / stem, "head %d" % n)

    try:
        new_imf = parse_imf((data / imf_rel(sex)).read_bytes())
        old_imf = parse_imf((bk / imf_rel(sex)).read_bytes())
        if [len(a) for a in new_imf["layers"]] != [len(a) for a in old_imf["layers"]]:
            err("imf structure differs")
        for ai, action in enumerate(new_imf["layers"][0]):
            for mi, motion in enumerate(action):
                if motion[0] != 1:
                    err("imf body priority %d at action %d motion %d" % (motion[0], ai, mi))
    except (FormatError, OSError, struct.error) as exc:
        err("imf: %s" % exc)

    pal_rels = [p for _, p in find_body_pals(data, sex)]
    pal_rels += [p for _, _, p in find_head_pals(data, sex, set(heads))]
    for rel in pal_rels:
        if not (bk / rel).exists():
            continue
        try:
            pal = parse_pal((data / rel).read_bytes())
        except (FormatError, OSError) as exc:
            err("pal %s: %s" % (rel.name, exc))
            continue
        if len((data / rel).read_bytes()) != 1024:
            err("pal %s: size != 1024" % rel.name)
        if any(c[3] != 0 for c in pal):
            err("pal %s: alpha not 0" % rel.name)
        if any(is_magenta(c) for c in pal[1:]):
            err("pal %s: contains magenta" % rel.name)
    print("verify %s: body, %d heads, %d palettes checked" % (sex, len(heads), len(pal_rels)))
    return True


def cmd_verify(args):
    data = Path(args.data)
    errors = []
    sexes = [args.sex] if args.sex else ["m", "f"]
    checked = [verify_sex(data, s, errors) for s in sexes]
    if not any(checked):
        print("verify: nothing to check (no backup found)", file=sys.stderr)
        return 1
    if errors:
        for e in errors[:50]:
            print("FAIL " + e, file=sys.stderr)
        print("verify: %d problem(s)" % len(errors), file=sys.stderr)
        return 1
    print("verify: OK")
    return 0


# ---------------------------------------------------------------------------
# restore
# ---------------------------------------------------------------------------

def cmd_restore(args):
    data = Path(args.data)
    bk = backup_root(data)
    if not bk.is_dir():
        raise SystemExit("no backup directory at %s" % bk)
    n = 0
    for src in sorted(bk.rglob("*")):
        if src.is_file():
            dst = data / src.relative_to(bk)
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            n += 1
    print("restored %d files from %s" % (n, bk))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="Pack rendered character frames into RO sprite files.")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("layout", help="write motion layout of the original Novice body")
    p.add_argument("--data", required=True)
    p.add_argument("--sex", required=True, choices=["m", "f"])
    p.add_argument("--out", required=True)
    p.set_defaults(fn=cmd_layout)

    p = sub.add_parser("synthetic", help="generate stand-in frames without Blender")
    p.add_argument("--sex", required=True, choices=["m", "f"])
    p.add_argument("--out", required=True)
    p.add_argument("--layout", help="layout json (default: <out>/../layout_<sex>.json)")
    p.set_defaults(fn=cmd_synthetic)

    p = sub.add_parser("preview", help="contact sheet of idle and walk from raw frames")
    p.add_argument("--frames", required=True)
    p.add_argument("--style")
    p.add_argument("--out")
    p.set_defaults(fn=cmd_preview)

    p = sub.add_parser("pack", help="back up originals and write new SPR/ACT/PAL/IMF")
    p.add_argument("--data", required=True)
    p.add_argument("--frames", required=True)
    p.add_argument("--sex", required=True, choices=["m", "f"])
    p.set_defaults(fn=cmd_pack)

    p = sub.add_parser("verify", help="re-parse written files and check invariants")
    p.add_argument("--data", required=True)
    p.add_argument("--sex", choices=["m", "f"], help="default: both")
    p.set_defaults(fn=cmd_verify)

    p = sub.add_parser("restore", help="copy the backup back over the data dir")
    p.add_argument("--data", required=True)
    p.set_defaults(fn=cmd_restore)

    args = ap.parse_args(argv)
    try:
        return args.fn(args)
    except FormatError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
