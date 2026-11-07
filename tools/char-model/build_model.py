"""Build an original wuxia-style humanoid (rigid-parented primitives) in Blender.

Usage: blender -b -P build_model.py -- --sex m|f --out out/model_m.blend [--styles styles.json]

Written against the Blender 4.2 LTS Python API. Not executable without Blender.
"""
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Euler, Matrix, Vector


def parse_args():
    argv = sys.argv
    argv = argv[argv.index("--") + 1:] if "--" in argv else []
    opts = {"sex": None, "out": None, "styles": None}
    i = 0
    while i < len(argv):
        key = argv[i].lstrip("-")
        if key in opts and i + 1 < len(argv):
            opts[key] = argv[i + 1]
            i += 2
        else:
            i += 1
    if opts["sex"] not in ("m", "f") or not opts["out"]:
        raise SystemExit("usage: -- --sex m|f --out file.blend [--styles styles.json]")
    if not opts["styles"]:
        opts["styles"] = str(Path(__file__).resolve().parent / "styles.json")
    return opts


def srgb_to_linear(c):
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def hex_linear(text):
    text = text.lstrip("#")
    rgb = [int(text[i:i + 2], 16) for i in (0, 2, 4)]
    return tuple(srgb_to_linear(v) for v in rgb) + (1.0,)


def make_material(name, hex_color):
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = hex_linear(hex_color)
    return mat


class Builder:
    def __init__(self, sex, styles):
        self.sex = sex
        self.styles = styles
        cfg = styles["sex"][sex]
        self.cfg = cfg
        self.height = cfg["height_m"]
        self.u = self.height / 7.0
        self.sw = cfg["shoulder_u"]
        self.hem = cfg["robe_hem_u"]
        self.hip_k = 1.0 if sex == "m" else 0.97
        self.mats = {}
        self.parent_cache = {}
        self.arm = None
        self.collections = {}

    # ----- materials -------------------------------------------------------
    def setup_materials(self):
        s = self.styles
        c = self.cfg
        defs = {
            "SKIN": s["skin"],
            "EYE": s["eye"],
            "BROW": s["brow"],
            "HAIR": s["hair_base"],
            "ROBE": s["robe_base"],
            "TRIM": c["trim"],
            "SASH": c["sash"],
            "WRAP": s["wrap"],
            "BOOT": c["boot"],
            "PANTS": s.get("pants", "#d8d2c2"),
        }
        for name, color in defs.items():
            self.mats[name] = make_material(name, color)

    # ----- rig -------------------------------------------------------------
    def build_armature(self):
        u, sw = self.u, self.sw
        arm_data = bpy.data.armatures.new("Rig")
        arm = bpy.data.objects.new("Rig", arm_data)
        bpy.context.scene.collection.objects.link(arm)
        bpy.context.view_layer.objects.active = arm
        arm.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        eb = arm_data.edit_bones

        def bone(name, head, tail, parent=None):
            b = eb.new(name)
            b.head = Vector(head)
            b.tail = Vector(tail)
            b.roll = 0.0
            if parent:
                b.parent = eb[parent]
                b.use_connect = False

        bone("root", (0, 0, 0), (0, 0, 0.3 * u))
        bone("hips", (0, 0, 3.7 * u), (0, 0, 4.1 * u), "root")
        bone("spine", (0, 0, 4.1 * u), (0, 0, 4.8 * u), "hips")
        bone("chest", (0, 0, 4.8 * u), (0, 0, 5.75 * u), "spine")
        bone("neck", (0, 0, 5.75 * u), (0, 0, 6.0 * u), "chest")
        bone("head", (0, 0, 6.0 * u), (0, 0, 7.0 * u), "neck")
        arm_x = (0.95 * sw + 0.15) * u
        hip_x = 0.48 * u
        for side, sgn in ((".L", 1), (".R", -1)):
            bone("upper_arm" + side, (sgn * arm_x, 0, 5.55 * u), (sgn * arm_x, 0, 4.2 * u), "chest")
            bone("forearm" + side, (sgn * arm_x, 0, 4.2 * u), (sgn * arm_x, 0, 3.0 * u), "upper_arm" + side)
            bone("hand" + side, (sgn * arm_x, 0, 3.0 * u), (sgn * arm_x, 0, 2.55 * u), "forearm" + side)
            bone("thigh" + side, (sgn * hip_x, 0, 3.7 * u), (sgn * hip_x, 0, 2.0 * u), "hips")
            bone("shin" + side, (sgn * hip_x, 0, 2.0 * u), (sgn * hip_x, 0, 0.45 * u), "thigh" + side)
            bone("foot" + side, (sgn * hip_x, 0, 0.45 * u), (sgn * hip_x, 0, 0.1 * u), "shin" + side)
        bpy.ops.object.mode_set(mode="OBJECT")
        arm.select_set(False)
        for pb in arm.pose.bones:
            pb.rotation_mode = "XYZ"
        self.arm = arm
        self.arm_x = arm_x
        self.hip_x = hip_x

    # ----- mesh helpers ----------------------------------------------------
    def collection(self, name):
        if name is None:
            return bpy.context.scene.collection
        if name not in self.collections:
            col = bpy.data.collections.new(name)
            bpy.context.scene.collection.children.link(col)
            self.collections[name] = col
        return self.collections[name]

    def parent_to_bone(self, obj, bone_name, world):
        """Parent obj to a bone, keeping `world` as its rest-pose transform.

        The parent matrix of bone parenting (which includes Blender's tail offset) is
        measured instead of computed: parent with identity inverse, evaluate, read it.
        """
        obj.parent = self.arm
        obj.parent_type = "BONE"
        obj.parent_bone = bone_name
        if bone_name not in self.parent_cache:
            obj.matrix_parent_inverse = Matrix.Identity(4)
            obj.matrix_basis = Matrix.Identity(4)
            bpy.context.view_layer.update()
            self.parent_cache[bone_name] = obj.matrix_world.copy()
        obj.matrix_parent_inverse = self.parent_cache[bone_name].inverted()
        obj.matrix_basis = world

    def add(self, name, prim, loc, bone, mat, scale=(1, 1, 1), rot=(0, 0, 0), part="body",
            hair=None, col=None, subsurf=False, smooth=True, **kw):
        loc = tuple(loc)
        if prim == "sphere":
            bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=12, radius=1.0, location=loc)
        elif prim == "cyl":
            bpy.ops.mesh.primitive_cylinder_add(vertices=24, radius=kw.get("r", 1.0), depth=kw.get("depth", 1.0), location=loc)
        elif prim == "cone":
            bpy.ops.mesh.primitive_cone_add(vertices=32, radius1=kw["r1"], radius2=kw["r2"],
                                            depth=kw["depth"], end_fill_type=kw.get("fill", "NGON"), location=loc)
        elif prim == "torus":
            bpy.ops.mesh.primitive_torus_add(major_radius=kw["major"], minor_radius=kw["minor"],
                                             major_segments=36, minor_segments=12, location=loc)
        elif prim == "cube":
            bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
        else:
            raise ValueError(prim)
        obj = bpy.context.active_object
        obj.name = name
        if smooth:
            for poly in obj.data.polygons:
                poly.use_smooth = True
        obj.data.materials.append(self.mats[mat])
        if subsurf:
            mod = obj.modifiers.new("Subsurf", "SUBSURF")
            mod.levels = 1
            mod.render_levels = 1
        if kw.get("solidify"):
            mod = obj.modifiers.new("Solidify", "SOLIDIFY")
            mod.thickness = kw["solidify"]
        obj["part"] = part
        if hair:
            obj["hair_style"] = hair
        for c in list(obj.users_collection):
            c.objects.unlink(obj)
        self.collection(col).objects.link(obj)
        world = Matrix.LocRotScale(Vector(loc), Euler(rot, "XYZ"), Vector(scale))
        self.parent_to_bone(obj, bone, world)
        return obj

    def limb(self, name, x, z_top, z_bot, r_top, r_bot, bone, mat, **kw):
        u = self.u
        zt, zb = z_top * u, z_bot * u
        return self.add(name, "cone", (x, 0, (zt + zb) / 2), bone, mat, r1=r_bot * u, r2=r_top * u,
                        depth=zt - zb, subsurf=kw.pop("subsurf", True), **kw)

    def strip(self, name, p_top, p_bot, width, thick, bone, mat, **kw):
        """Thin box between two points in the XZ plane (y is taken from p_top)."""
        u = self.u
        (xt, yt, zt), (xb, yb, zb) = p_top, p_bot
        dx, dz = (xt - xb) * u, (zt - zb) * u
        length = math.hypot(dx, dz)
        theta = math.atan2(dx, dz)
        mid = ((xt + xb) / 2 * u, (yt + yb) / 2 * u, (zt + zb) / 2 * u)
        return self.add(name, "cube", mid, bone, mat, scale=(width * u, thick * u, length),
                        rot=(0, theta, 0), smooth=False, **kw)

    def garment_r(self, z):
        """Torso garment half-width (in u) at height z (in u)."""
        sw = self.sw
        if z <= 4.8:
            t = max(0.0, (z - 3.7) / 1.1)
            return (0.70 * self.hip_k) * (1 - t) + 0.78 * sw * t
        t = min(1.0, (z - 4.8) / 0.95)
        return 0.78 * sw * (1 - t) + 0.92 * sw * t

    # ----- body ------------------------------------------------------------
    def build_body(self):
        u, sw = self.u, self.sw
        ax, hx = self.arm_x, self.hip_x
        depth_k = 0.58
        # Torso garment, two rigid halves so the spine can bend.
        for name, bone, z0, z1 in (("robe_lower", "spine", 3.7, 4.8), ("robe_upper", "chest", 4.8, 5.75)):
            r0, r1 = self.garment_r(z0), self.garment_r(z1)
            self.add(name, "cone", (0, 0, (z0 + z1) / 2 * u), bone, "ROBE", scale=(1, depth_k, 1),
                     r1=r0 * u, r2=r1 * u, depth=(z1 - z0) * u, subsurf=False)
        # Neck
        self.limb("neck_mesh", 0, 6.15, 5.6, 0.2, 0.22, "neck", "SKIN")
        # Crossed collar (trim strips) on the chest.
        yfront = -(depth_k * self.garment_r(5.3) + 0.03) * u / u
        for sgn in (1, -1):
            self.strip("collar_%s" % ("a" if sgn > 0 else "b"),
                       (sgn * 0.30 * sw, yfront, 5.70), (-sgn * 0.10 * sw, yfront, 4.78),
                       0.075, 0.05, "chest", "TRIM")
        # Sash with two hanging tails.
        rs = self.garment_r(3.95) + 0.03
        self.add("sash", "torus", (0, 0, 3.95 * u), "hips", "SASH", scale=(1, depth_k, 1),
                 major=rs * u, minor=0.1 * u)
        for i, (dx, ln, rot) in enumerate(((0.22, 0.75, 0.10), (0.34, 0.55, -0.12))):
            top = 3.95
            self.add("sash_tail_%d" % i, "cube", (dx * u, -(depth_k * rs + 0.07) * u, (top - ln / 2) * u),
                     "hips", "SASH", scale=(0.15 * u, 0.04 * u, ln * u), rot=(0, rot, 0), smooth=False)
        # Skirt: open cone with a trim hem, parented to the hips.
        z_top, z_hem = 3.85, self.hem
        r_top = self.garment_r(3.85) + 0.02
        r_hem = 1.0
        self.add("skirt", "cone", (0, 0, (z_top + z_hem) / 2 * u), "hips", "ROBE",
                 r1=r_hem * u, r2=r_top * u, depth=(z_top - z_hem) * u, fill="NOTHING",
                 solidify=0.015, subsurf=False)
        self.add("skirt_hem", "torus", (0, 0, z_hem * u), "hips", "TRIM", major=r_hem * u, minor=0.05 * u)
        # Arms: robe sleeves, wrist wraps, hands.
        for side, sgn in ((".L", 1), (".R", -1)):
            x = sgn * ax
            self.limb("sleeve_upper" + side, x, 5.55, 4.2, 0.26, 0.23, "upper_arm" + side, "ROBE")
            self.limb("sleeve_lower" + side, x, 4.2, 3.25, 0.23, 0.27, "forearm" + side, "ROBE")
            self.limb("wrap" + side, x, 3.35, 2.95, 0.17, 0.17, "forearm" + side, "WRAP")
            self.add("hand" + side, "sphere", (x, 0, 2.78 * u), "hand" + side, "SKIN",
                     scale=(0.15 * u, 0.12 * u, 0.2 * u))
            # Legs: trousers, boots, feet.
            lx = sgn * hx
            self.limb("thigh_mesh" + side, lx, 3.8, 2.0, 0.36, 0.28, "thigh" + side, "PANTS")
            self.limb("shin_mesh" + side, lx, 2.0, 1.0, 0.28, 0.24, "shin" + side, "PANTS")
            self.limb("boot_shaft" + side, lx, 1.15, 0.4, 0.29, 0.25, "shin" + side, "BOOT")
            self.add("boot_foot" + side, "sphere", (lx, -0.22 * u, 0.2 * u), "foot" + side, "BOOT",
                     scale=(0.26 * u, 0.55 * u, 0.2 * u))

    # ----- head ------------------------------------------------------------
    def build_head(self):
        u = self.u
        rx = 0.38 if self.sex == "m" else 0.36
        zc = 6.48
        self.add("head_mesh", "sphere", (0, 0, zc * u), "head", "SKIN", scale=(rx * u, 0.44 * u, 0.5 * u), part="head")
        for side, sgn in ((".L", 1), (".R", -1)):
            self.add("ear" + side, "sphere", (sgn * (rx - 0.01) * u, 0.02 * u, 6.42 * u), "head", "SKIN",
                     scale=(0.06 * u, 0.10 * u, 0.14 * u), part="head")
            self.add("eye" + side, "sphere", (sgn * 0.16 * u, -0.395 * u, 6.5 * u), "head", "EYE",
                     scale=(0.06 * u, 0.035 * u, 0.045 * u), part="head")
            self.add("brow" + side, "cube", (sgn * 0.16 * u, -0.375 * u, 6.62 * u), "head", "BROW",
                     scale=(0.14 * u, 0.03 * u, 0.022 * u), rot=(0, sgn * -0.15, 0), part="head", smooth=False)
        self.add("nose", "sphere", (0, -0.43 * u, 6.38 * u), "head", "SKIN",
                 scale=(0.05 * u, 0.07 * u, 0.1 * u), part="head")

    def hair_obj(self, style, name, prim, loc, scale=(1, 1, 1), rot=(0, 0, 0), **kw):
        u = self.u
        loc = tuple(v * u for v in loc)
        for k in ("r1", "r2", "depth", "major", "minor"):
            if k in kw:
                kw[k] = kw[k] * u
        scale = tuple(scale)
        if prim == "sphere":
            scale = tuple(s * u for s in scale)
        return self.add("hair_%s_%s" % (style, name), prim, loc, "head", "HAIR", scale=scale, rot=rot,
                        part="head", hair=style, col="hair_" + style, **kw)

    def hair_cap(self, style):
        self.hair_obj(style, "cap", "sphere", (0, 0.07, 6.58), scale=(0.405, 0.43, 0.47))

    def build_hair(self):
        styles = self.cfg["hair_styles"]
        builders = {
            "topknot": self.h_topknot,
            "long_tied": self.h_long_tied,
            "topknot_long": self.h_topknot_long,
            "headband_loose": self.h_headband_loose,
            "high_bun": self.h_high_bun,
            "long_straight": self.h_long_straight,
            "twin_buns": self.h_twin_buns,
            "side_braid": self.h_side_braid,
        }
        for name in styles:
            if name not in builders:
                raise SystemExit("unknown hair style: %s" % name)
            self.hair_cap(name)
            builders[name](name)
        # Only the first style is visible by default; render.py toggles per layer.
        for i, name in enumerate(styles):
            hide = i != 0
            col = self.collections["hair_" + name]
            for o in col.objects:
                o.hide_viewport = hide
                o.hide_render = hide

    def h_topknot(self, s):
        self.hair_obj(s, "bun", "sphere", (0, 0.05, 7.15), scale=(0.2, 0.2, 0.2))
        self.hair_obj(s, "band", "torus", (0, 0.05, 6.98), major=0.15, minor=0.035)

    def h_long_tied(self, s):
        self.hair_obj(s, "tie", "sphere", (0, 0.42, 6.25), scale=(0.08, 0.08, 0.08))
        self.hair_obj(s, "tail", "cone", (0, 0.62, 5.5), r1=0.05, r2=0.14, depth=1.4)

    def h_topknot_long(self, s):
        self.h_topknot(s)
        self.hair_obj(s, "back", "cone", (0, 0.55, 5.75), scale=(1, 0.4, 1), r1=0.22, r2=0.38, depth=1.5)

    def h_headband_loose(self, s):
        self.hair_obj(s, "band", "torus", (0, 0.03, 6.82), scale=(1, 1.08, 1), major=0.4, minor=0.03)
        self.hair_obj(s, "back", "cone", (0, 0.52, 5.95), scale=(1, 0.4, 1), r1=0.2, r2=0.36, depth=1.0)
        for sgn in (1, -1):
            self.hair_obj(s, "lock%d" % (0 if sgn > 0 else 1), "cone", (sgn * 0.4, 0.2, 6.0),
                          r1=0.05, r2=0.12, depth=0.9)

    def h_high_bun(self, s):
        self.hair_obj(s, "bun", "sphere", (0, 0.1, 7.2), scale=(0.26, 0.26, 0.24))
        for sgn in (1, -1):
            self.hair_obj(s, "lock%d" % (0 if sgn > 0 else 1), "cone", (sgn * 0.4, -0.08, 6.2),
                          r1=0.04, r2=0.08, depth=0.7)

    def h_long_straight(self, s):
        self.hair_obj(s, "back", "cone", (0, 0.5, 5.35), scale=(1, 0.35, 1), r1=0.4, r2=0.34, depth=2.2)
        for sgn in (1, -1):
            self.hair_obj(s, "lock%d" % (0 if sgn > 0 else 1), "cone", (sgn * 0.4, -0.05, 6.0),
                          r1=0.05, r2=0.1, depth=0.9)

    def h_twin_buns(self, s):
        for sgn in (1, -1):
            self.hair_obj(s, "bun%d" % (0 if sgn > 0 else 1), "sphere", (sgn * 0.3, 0.05, 7.0), scale=(0.2, 0.2, 0.2))
            self.hair_obj(s, "tie%d" % (0 if sgn > 0 else 1), "torus", (sgn * 0.3, 0.05, 6.88), major=0.12, minor=0.03)

    def h_side_braid(self, s):
        self.hair_obj(s, "back", "cone", (0, 0.5, 5.9), scale=(1, 0.35, 1), r1=0.3, r2=0.36, depth=1.0)
        zs = (5.95, 5.55, 5.15, 4.8)
        rs = (0.14, 0.13, 0.11, 0.08)
        for i, (z, r) in enumerate(zip(zs, rs)):
            self.hair_obj(s, "braid%d" % i, "sphere", (0.42, -0.5, z), scale=(r, r, r))

    # ----- entry -----------------------------------------------------------
    def run(self):
        self.setup_materials()
        self.build_armature()
        self.build_body()
        self.build_head()
        self.build_hair()


def main():
    opts = parse_args()
    with open(opts["styles"], encoding="utf-8") as f:
        styles = json.load(f)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    builder = Builder(opts["sex"], styles)
    builder.run()
    out = Path(opts["out"]).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out))
    print("saved %s (%d objects)" % (out, len(bpy.data.objects)))


main()
