"""Pose the rig and render every sprite frame (beauty + recolor mask) with Workbench.

Usage:
  blender -b -P render.py -- --sex m|f --blend out/model_m.blend --out out/m
         [--layout out/layout_m.json] [--styles styles.json] [--groups 0,1] [--dirs 0,1,2]

Output: <out>/<layer>/a<action>_m<motion>.png and ..._mask.png plus <out>/frames.json.
Layers: "body" and "head_<style>". Masks: pure white where the recolor material is
(ROBE for the body, HAIR for heads), black elsewhere. They are produced by swapping the
materials' viewport colors and switching Workbench to FLAT lighting with Standard color
management, so the mask pixels are exact 0/255 (no shading, shadows, cavity or outline).

Pose conventions: every bone is created vertical with roll 0 (see build_model.py), so
local X is world X for all bones. "Up" bones (root, hips, spine, chest, neck, head) have
local Y = world Z and local Z = world -Y; "down" bones (arms, legs) have local Y = world -Z
and local Z = world +Y. set_bone() takes semantic angles in degrees:
  f = swing toward the facing direction, o = outward abduction, t = twist about world Z.
"""
import json
import math
import sys
from pathlib import Path

import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Vector

UP_BONES = {"root", "hips", "spine", "chest", "neck", "head"}


def parse_args():
    argv = sys.argv
    argv = argv[argv.index("--") + 1:] if "--" in argv else []
    opts = {"sex": None, "blend": None, "out": None, "layout": None, "styles": None,
            "groups": None, "dirs": None}
    i = 0
    while i < len(argv):
        key = argv[i].lstrip("-")
        if key in opts and i + 1 < len(argv):
            opts[key] = argv[i + 1]
            i += 2
        else:
            i += 1
    if opts["sex"] not in ("m", "f") or not opts["blend"] or not opts["out"]:
        raise SystemExit("usage: -- --sex m|f --blend file.blend --out dir [--layout json]")
    out = Path(opts["out"]).resolve()
    opts["out"] = out
    opts["layout"] = Path(opts["layout"]) if opts["layout"] else out.parent / ("layout_%s.json" % opts["sex"])
    opts["styles"] = Path(opts["styles"]) if opts["styles"] else Path(__file__).resolve().parent / "styles.json"
    for k in ("groups", "dirs"):
        if opts[k]:
            opts[k] = [int(v) for v in opts[k].split(",")]
    return opts


# ---------------------------------------------------------------------------
# Poses
# ---------------------------------------------------------------------------

def interp(p, keys):
    """Piecewise-linear interpolation over [(phase, value), ...] sorted by phase."""
    if p <= keys[0][0]:
        return keys[0][1]
    for (p0, v0), (p1, v1) in zip(keys, keys[1:]):
        if p <= p1:
            t = (p - p0) / (p1 - p0) if p1 > p0 else 0.0
            return v0 + (v1 - v0) * t
    return keys[-1][1]


def smoothstep(t):
    t = min(1.0, max(0.0, t))
    return t * t * (3 - 2 * t)


class Pose:
    def __init__(self, u):
        self.u = u
        self.bones = {}
        self.hips_dz = 0.0
        self.root_t = 0.0

    def set(self, name, f=0.0, o=0.0, t=0.0):
        self.bones[name] = (f, o, t)

    def both(self, base, f=0.0, o=0.0, t=0.0):
        self.set(base + ".L", f, o, t)
        self.set(base + ".R", f, o, t)


def make_pose(u, group, phase, count, motion):
    """Return a Pose for a group (0..12), cycle phase in [0,1) and motion index."""
    P = Pose(u)
    s = math.sin(2 * math.pi * phase)
    c = math.cos(2 * math.pi * phase)
    one = motion / float(max(count - 1, 1))  # one-shot progress 0..1
    P.both("upper_arm", 0, 6)
    P.both("thigh", 0, 3)

    if group in (0, 2):  # Idle / Sit: motion index is the head turn
        turn = {1: 35.0, 2: -35.0}.get(motion, 0.0)
        P.set("head", 0, 0, turn)
        if group == 0:
            P.set("chest", 0)
        else:
            P.hips_dz = -(3.7 * u - 0.14)
            P.set("chest", 4)
            for sgn, side in ((1, ".L"), (-1, ".R")):
                P.set("thigh" + side, 62, 55)
                P.set("shin" + side, -118, -50)
            P.both("upper_arm", 15, 14)
            P.both("forearm", 60)
    elif group == 1:  # Walk
        P.set("thigh.L", 24 * s, 3)
        P.set("thigh.R", -24 * s, 3)
        P.set("shin.L", -35 * max(0.0, c))
        P.set("shin.R", -35 * max(0.0, -c))
        P.set("upper_arm.L", -25 * s, 6)
        P.set("upper_arm.R", 25 * s, 6)
        P.both("forearm", 18)
        P.hips_dz = -0.025 * abs(s)
        P.set("hips", 0, 0, 6 * s)
        P.set("chest", 0, 0, -8 * s)
    elif group == 3:  # Pickup
        a = math.sin(math.pi * (motion + 0.5) / max(count, 1))
        P.set("hips", 20 * a)
        P.set("spine", 25 * a)
        P.set("chest", 15 * a)
        P.both("thigh", 55 * a, 3)
        P.both("shin", -65 * a)
        P.both("upper_arm", 40 * a, 6)
        P.hips_dz = -0.12 * a
    elif group == 4:  # ReadyFight
        P.hips_dz = -0.08 + 0.01 * s
        P.set("hips", 0, 0, 25)
        P.set("chest", 2 * s)
        P.set("thigh.L", 25, 18)
        P.set("thigh.R", -20, 18)
        P.set("shin.L", -20)
        P.set("shin.R", -15)
        P.both("upper_arm", 40, 20)
        P.both("forearm", 70)
    elif group == 5:  # Attack1: right palm strike
        ext = interp(one, [(0, -0.5), (0.3, -0.3), (0.5, 1.0), (0.75, 0.9), (1.0, 0.0)])
        P.hips_dz = -0.05
        P.set("thigh.L", 20, 12)
        P.set("thigh.R", -10, 12)
        P.set("shin.L", -15)
        P.set("chest", 0, 0, 25 * ext)
        P.set("upper_arm.R", 15 + (75 * ext if ext > 0 else 40 * ext), 8)
        P.set("forearm.R", 100 * (1 - max(ext, 0.0)))
        P.set("upper_arm.L", 40, 15)
        P.set("forearm.L", 80)
    elif group == 6:  # Hurt
        a = interp(one, [(0, 0.6), (0.5, 1.0), (1.0, 0.4)])
        P.set("chest", -20 * a)
        P.set("head", -15 * a)
        P.hips_dz = -0.03 * a
        P.both("upper_arm", -10 * a, 6 + 19 * a)
        P.both("thigh", -8 * a, 4)
    elif group in (7, 9):  # Freeze: stiff
        P.both("upper_arm", 0, 3)
        P.both("thigh", 0, 2)
    elif group == 8:  # Die: lie on the back
        a = 1.0 if count <= 1 else smoothstep(one)
        P.set("hips", -90 * a)
        P.hips_dz = -(3.7 * u - 0.14) * a
        P.both("upper_arm", 0, 6 + 8 * a)
        P.both("thigh", 0, 3 + 5 * a)
    elif group == 10:  # Attack2: spinning two-hand sweep
        P.hips_dz = -0.1
        P.set("hips", 0, 0, 360.0 * motion / float(max(count, 1)))
        P.set("thigh.L", 20, 15)
        P.set("thigh.R", -15, 15)
        P.both("upper_arm", 60, 20)
        P.both("forearm", 5)
    elif group == 11:  # Attack3: front kick with the right leg
        ext = interp(one, [(0, 0.0), (0.25, -0.3), (0.5, 1.0), (0.7, 1.0), (1.0, 0.0)])
        P.hips_dz = -0.03
        P.set("chest", -10 * max(ext, 0.0))
        P.set("thigh.R", 20 + 80 * ext, 4)
        P.set("shin.R", -70 * (1 - max(ext, 0.0)))
        P.set("shin.L", -10)
        P.both("upper_arm", 20, 30)
    elif group == 12:  # Skill: gather, both palms forward
        a = interp(one, [(0, 0.0), (0.4, 0.6), (1.0, 1.0)])
        P.hips_dz = -0.06
        P.both("upper_arm", 35 + 45 * a, 20 - 15 * a)
        P.both("forearm", 60 * (1 - 0.5 * a))
        P.both("thigh", 22 * a, 14)
        P.both("shin", -25 * a)
    return P


def apply_pose(arm, P, direction):
    for pb in arm.pose.bones:
        pb.rotation_mode = "XYZ"
        pb.rotation_euler = (0.0, 0.0, 0.0)
        pb.location = (0.0, 0.0, 0.0)
    P.set("root", 0, 0, -45.0 * direction)
    for name, (f, o, t) in P.bones.items():
        pb = arm.pose.bones[name]
        up = name in UP_BONES
        sgn = -1.0 if name.endswith(".R") else 1.0
        rx = f if up else -f
        ry = (o if up else -o) * sgn
        rz = t
        if up:
            e = (rx, rz, -ry)
        else:
            e = (rx, -rz, ry)
        pb.rotation_euler = tuple(math.radians(v) for v in e)
    # Hips lowering: world dz maps to local Y of the (up) hips bone.
    arm.pose.bones["hips"].location = (0.0, P.hips_dz, 0.0)


# ---------------------------------------------------------------------------
# Scene setup
# ---------------------------------------------------------------------------

def setup_scene(styles):
    r = styles["render"]
    scale = r["supersample"]
    canvas = r["canvas_px"] * scale
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = canvas
    scene.render.resolution_y = canvas
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.color_depth = "8"
    scene.render.dither_intensity = 0.0
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.display_settings.display_device = "sRGB"
    sh = scene.display.shading
    sh.color_type = "MATERIAL"
    scene.display.render_aa = "8"

    elev = math.radians(r["elevation_deg"])
    pxpm = r["stand_height_px"] / (1.75 * math.cos(elev))
    cam_data = bpy.data.cameras.new("Cam")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = r["canvas_px"] / pxpm
    cam_data.clip_start = 0.1
    cam_data.clip_end = 100.0
    cam = bpy.data.objects.new("Cam", cam_data)
    bpy.context.scene.collection.objects.link(cam)
    cam.rotation_euler = (math.radians(90.0 - r["elevation_deg"]), 0.0, 0.0)
    # Put the world origin at origin_px: the view center sits (origin_y - center) px
    # below the origin on screen, i.e. along the camera's screen-up vector.
    up = Vector((0.0, math.sin(elev), math.cos(elev)))
    center = r["canvas_px"] / 2.0
    target = up * ((r["origin_px"][1] - center) / pxpm)
    target.x = -(r["origin_px"][0] - center) / pxpm
    view_dir = Vector((0.0, math.cos(elev), -math.sin(elev)))
    cam.location = target - view_dir * 20.0
    scene.camera = cam
    return scene, cam, canvas, scale


def set_beauty_mode(scene):
    sh = scene.display.shading
    sh.light = "STUDIO"
    sh.show_shadows = True
    sh.show_cavity = True
    sh.show_object_outline = True


def set_mask_mode(scene, target):
    sh = scene.display.shading
    sh.light = "FLAT"
    sh.show_shadows = False
    sh.show_cavity = False
    sh.show_object_outline = False
    for m in bpy.data.materials:
        m.diffuse_color = (1.0, 1.0, 1.0, 1.0) if m.name == target else (0.0, 0.0, 0.0, 1.0)


def main():
    opts = parse_args()
    with open(opts["styles"], encoding="utf-8") as f:
        styles = json.load(f)
    with open(opts["layout"], encoding="utf-8") as f:
        layout = json.load(f)
    bpy.ops.wm.open_mainfile(filepath=str(Path(opts["blend"]).resolve()))
    scene, cam, canvas, scale = setup_scene(styles)
    arm = bpy.data.objects["Rig"]
    arm.hide_render = True
    style_names = styles["sex"][opts["sex"]]["hair_styles"]
    u = styles["sex"][opts["sex"]]["height_m"] / 7.0
    objs = [o for o in bpy.data.objects if o.type == "MESH"]
    base_colors = {m.name: tuple(m.diffuse_color) for m in bpy.data.materials}

    def show(layer):
        style = layer[5:] if layer.startswith("head_") else None
        for o in objs:
            part = o.get("part")
            if style is None:
                vis = part == "body"
            else:
                vis = part == "head" and o.get("hair_style", style) == style
            o.hide_render = not vis
            o.hide_viewport = not vis

    def restore_colors():
        for m in bpy.data.materials:
            m.diffuse_color = base_colors[m.name]

    layers = ["body"] + ["head_" + n for n in style_names]
    out = opts["out"]
    for layer in layers:
        (out / layer).mkdir(parents=True, exist_ok=True)
    frames = {"sex": opts["sex"], "scale": scale, "canvas": [canvas, canvas],
              "hair_styles": style_names, "layers": {l: {} for l in layers}}

    def project(co):
        v = world_to_camera_view(scene, cam, co)
        return [round(v.x * canvas, 2), round((1.0 - v.y) * canvas, 2)]

    def render_to(path):
        scene.render.filepath = str(path)
        bpy.ops.render.render(write_still=True)

    cyclic = {0, 1, 2, 4, 7, 9}
    for a, count in enumerate(layout["motions"]):
        group, d = divmod(a, 8)
        if opts["groups"] and group not in opts["groups"]:
            continue
        if opts["dirs"] and d not in opts["dirs"]:
            continue
        for m in range(count):
            phase = m / float(max(count, 1)) if group in cyclic else m / float(max(count - 1, 1))
            P = make_pose(u, group, phase, count, m)
            apply_pose(arm, P, d)
            bpy.context.view_layer.update()
            dg = bpy.context.evaluated_depsgraph_get()
            arm_eval = arm.evaluated_get(dg)
            anchor = arm_eval.matrix_world @ arm_eval.pose.bones["head"].head
            origin = project(Vector((0.0, 0.0, 0.0)))
            anchor_px = project(anchor)
            key = "a%03d_m%02d" % (a, m)
            for layer in layers:
                show(layer)
                set_beauty_mode(scene)
                render_to(out / layer / (key + ".png"))
                set_mask_mode(scene, "ROBE" if layer == "body" else "HAIR")
                render_to(out / layer / (key + "_mask.png"))
                restore_colors()
                frames["layers"][layer][key] = {"origin": origin, "anchor": anchor_px}
        print("rendered action %d" % a)
    with open(out / "frames.json", "w", encoding="utf-8") as f:
        json.dump(frames, f, indent=1)
    print("frames.json written: %s" % out)


main()
