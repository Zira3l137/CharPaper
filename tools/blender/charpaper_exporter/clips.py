"""Writes clips as the app's mixer reads them: per clip a .json, and a .glb beside it when the
clip moves bones or objects.

The .glb carries node transforms, which the glTF exporter puts in the app's axes. The .json
lists the nodes the action really keys, since the exporter bakes every bone, and holds the
shape-key and light curves by name, one value a frame: glTF animates a mesh's shape keys as one
block and needs the mesh in the file, and has no place for a light's Strength or Color.

A timeline clip covers the scene's frame range, unless its action has a Manual Frame Range: the
NLA strip it plays from starts at the scene's first frame, so before the action's first key the
clip holds its first pose, after the last its last, and a Cycles modifier repeats, as Blender
shows it. A Once clip covers its action's own keys; a Hold clip its first frame.
"""

import json
import os

from . import anim
from . import cues
from . import gltf
from .actors import TRANSFORM_PATH
from .actors import shape_keys_of
from .isolate import isolated
from .isolate import showing
from .props import file_stem

FORMAT = 1
CLIPS_DIR = "clips"
EXPRESSIONS_DIR = "expressions"
ENVIRONMENT_DIR = "environment"
CAMERAS_DIR = "cameras"
LIGHT_PATHS = {"energy", "color"}

# A clip's .glb holds nodes only: nothing in it is drawn.
NODES_ONLY = {
    "export_format": "GLB",
    "export_animations": True,
    "export_animation_mode": "NLA_TRACKS",
    "export_lights": False,
    "export_cameras": False,
    "export_materials": "NONE",
    "export_image_format": "NONE",
    "export_morph": False,
    # A keyed bone that holds still would otherwise lose its channel, and the app refuses a
    # clip whose .json names a node its .glb doesn't move.
    "export_optimize_animation_keep_anim_armature": True,
    "export_optimize_animation_keep_anim_object": True,
}


def fps(scene):
    return scene.render.fps / scene.render.fps_base


def span(scene, clip):
    """The frames a clip covers, first and last included: its Manual Frame Range, else the keys
    of what it plays, its slot's and no other's."""
    action = clip.action
    if action.use_frame_range:
        first, last = (round(f) for f in action.frame_range)
    else:
        keyed = [k.co.x for c in clip.curves() for k in c.keyframe_points]
        first, last = (round(min(keyed)), round(max(keyed))) if keyed else (round(f) for f in action.frame_range)
    if clip.mode == "HOLD":
        return first, first
    if clip.mode == "TIMELINE" and not action.use_frame_range:
        return scene.frame_start, scene.frame_end
    return first, max(last, first)


def stretched_by(clip):
    """The slot that keeps a once-played clip running past its body's keys, as (label, first,
    last), or None."""
    frames = lambda curves: [k.co.x for c in curves for k in c.keyframe_points]
    groups = clip.groups()
    body = next((frames(curves) for _, slot, curves in groups if slot is not None and slot == clip.slot), [])
    if not body:
        return None
    first, last = min(body), max(body)
    for label, slot, curves in groups:
        keyed = frames(curves)
        if keyed and not (slot is not None and slot == clip.slot) and (max(keyed) > last + 1 or min(keyed) < first - 1):
            return label, round(min(keyed)), round(max(keyed))
    return None


def clip_folder(actor):
    if actor.is_character:
        return CLIPS_DIR
    environment = file_stem(actor.environment.name)
    return os.path.join(ENVIRONMENT_DIR, environment, CLIPS_DIR, file_stem(actor.name))


def keyed_nodes(actor, clip, suite):
    """The nodes a clip moves on the actor: its bones the file has, and the actor itself when
    it keys the object's own transform. A bone moves when it is keyed, driven, or held by a
    constraint whose target moves, as a deform bone following a keyed control does in a rig."""
    obj = actor.obj
    nodes = []
    if obj.type == "ARMATURE":
        bones = obj.data.bones
        exported = {b.name for b in bones if b.use_deform or not suite.gltf.deform_bones_only}
        nodes.extend(sorted(moved_bones(obj, clip.bone_names()) & exported))
    if any(TRANSFORM_PATH.match(c.data_path) for c in clip.curves()):
        nodes.append(obj.name)
    return nodes


def _targets(constraint):
    pairs = []
    for target, sub in (("target", "subtarget"), ("pole_target", "pole_subtarget")):
        if hasattr(constraint, target):
            pairs.append((getattr(constraint, target), getattr(constraint, sub, "")))
    for item in getattr(constraint, "targets", ()):
        pairs.append((getattr(item, "target", None), getattr(item, "subtarget", "")))
    return [(obj, sub) for obj, sub in pairs if obj is not None]


def _animated(obj):
    while obj is not None:
        data = obj.animation_data
        if data is not None and (data.action is not None or len(data.nla_tracks) > 0 or len(data.drivers) > 0):
            return True
        obj = obj.parent
    return False


def moved_bones(armature, keyed):
    """The bones whose own transform changes when `keyed` are keyed."""
    local = set(keyed)
    data = armature.animation_data
    for driver in data.drivers if data else ():
        match = anim.BONE_PATH.match(driver.data_path)
        if match:
            local.add(match.group(1).replace('\\"', '"').replace("\\\\", "\\"))

    def world_moves(name):
        bone = armature.data.bones.get(name)
        while bone is not None:
            if bone.name in local:
                return True
            bone = bone.parent
        return False

    changed = True
    while changed:
        changed = False
        for pose_bone in armature.pose.bones:
            if pose_bone.name in local:
                continue
            for constraint in pose_bone.constraints:
                if constraint.mute or constraint.influence <= 0.0:
                    continue
                moves = any(
                    (obj == armature and sub and world_moves(sub)) or (obj != armature and _animated(obj))
                    for obj, sub in _targets(constraint)
                )
                if not moves:
                    continue
                chain = [pose_bone]
                if constraint.type in {"IK", "SPLINE_IK"}:
                    length = constraint.chain_count or len(pose_bone.parent_recursive) + 1
                    chain = [pose_bone, *pose_bone.parent_recursive][:length]
                for bone in chain:
                    if bone.name not in local:
                        local.add(bone.name)
                        changed = True
                break
    return local


def _curves(action, wanted):
    return [c for c in anim.fcurves(action) if not c.mute and wanted(c)] if action else []


def shape_key_curves(clip):
    """The clip's shape key curves by name, from its second action or else its own."""
    by_name = {}
    for action in (clip.action, clip.extra):
        for curve in _curves(action, lambda c: anim.KEY_PATH.match(c.data_path)):
            name = anim.KEY_PATH.match(curve.data_path).group(1)
            name = name.replace('\\"', '"').replace("\\\\", "\\")
            if curve.data_path.endswith(".value"):
                by_name[name] = curve
    return by_name


def light_curves(clip):
    curves = {}
    for action in (clip.action, clip.extra):
        for curve in _curves(action, lambda c: c.data_path in LIGHT_PATHS):
            key = "energy" if curve.data_path == "energy" else curve.array_index
            if key == "energy" or key < 3:
                curves[key] = curve
    return curves


def light_samples(light, clip, frames):
    """A lamp's Strength and linear Color at each frame, or None when the clip keys neither."""
    curves = light_curves(clip)
    if not curves:
        return None
    value = lambda key, still, f: curves[key].evaluate(f) if key in curves else still
    energies = [max(value("energy", light.energy, f), 0.0) for f in frames]
    colors = [[max(value(i, light.color[i], f), 0.0) for i in range(3)] for f in frames]
    return energies, colors, "energy" in curves, any(i in curves for i in range(3))


def light_base(actor, clips, scene):
    """What the environment's file gives a lamp, which its clips' curves multiply: its highest
    Strength over all its clips, and white when any keys its Color. None for a lamp no clip
    lights."""
    obj = actor.obj
    if obj.type != "LIGHT":
        return None
    peak, colored = None, False
    for clip in clips:
        start, end = span(scene, clip)
        samples = light_samples(obj.data, clip, range(start, end + 1))
        if samples is None:
            continue
        energies, _, keyed_strength, keyed_color = samples
        if keyed_strength:
            peak = max(peak or 0.0, max(energies))
        colored = colored or keyed_color
    if peak is None and not colored:
        return None
    return (peak if peak is not None else obj.data.energy), colored


def clip_json(actor, clip, suite, scene, base=None):
    start, end = span(scene, clip)
    frames = range(start, end + 1)
    data = {"format": FORMAT, "fps": round(fps(scene), 6), "frames": len(frames)}
    nodes = keyed_nodes(actor, clip, suite)
    if nodes:
        data["nodes"] = nodes
    keys = shape_key_curves(clip)
    if keys:
        data["shape_keys"] = _sampled(keys, frames)
    if actor.obj.type == "LIGHT" and base is not None:
        samples = light_samples(actor.obj.data, clip, frames)
        if samples is not None:
            energies, colors, keyed_strength, keyed_color = samples
            light = {}
            peak, white = base
            if keyed_strength and peak > 0.0:
                light["strength"] = [round(e / peak, 5) for e in energies]
            if keyed_color and white:
                light["color"] = [[round(c, 5) for c in color] for color in colors]
            if light:
                data["light"] = light
    return data


def _sampled(curves, frames):
    return {name: [round(curve.evaluate(f), 5) for f in frames] for name, curve in sorted(curves.items())}


def write_json(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as file:
        json.dump(data, file, separators=(",", ":"))
        file.write("\n")


def _export_nodes(context, suite, actor, clip, path):
    """The clip's .glb: the actor as a node, its parents above it, playing the action over the
    clip's frames."""
    options = dict(gltf.settings_options(suite.gltf))
    options.update(gltf.LOCKED)
    options.update(NODES_ONLY)
    with isolated(context) as iso:
        copy = iso.stand_in_with_ancestors(actor.obj)
        anim.object_track(copy, clip.name, clip.action, span(context.scene, clip), clip.slot)
        iso.finish()
        with showing(context, iso.scene):
            return gltf.export(path, options)


class Written:
    """What exporting one actor's clips did."""

    def __init__(self):
        self.files = []
        self.notes = []
        # By clip name: [(seconds, sound name)] from its pose markers.
        self.cues = {}


def export_actor(context, suite, actor, clips, files):
    """Writes the actor's clips into its clip folder, which then holds only them."""
    scene = context.scene
    directory = files.path(clip_folder(actor))
    done = Written()
    base = light_base(actor, clips, scene)
    kept = set()
    for clip in clips:
        name = clip.name
        if name in kept:
            done.notes.append(f"{actor.name}: two clips play {clip.action.name}; the second is left out")
            continue
        kept.add(name)
        data = clip_json(actor, clip, suite, scene, base)
        json_path = os.path.join(directory, name + ".json")
        if "nodes" in data:
            glb_path = os.path.join(directory, name + ".glb")
            os.makedirs(directory, exist_ok=True)
            # Older exporters lack some of NODES_ONLY's switches and do without.
            _export_nodes(context, suite, actor, clip, glb_path)
            done.files.append(glb_path)
        write_json(json_path, data)
        done.files.append(json_path)
        start, end = span(scene, clip)
        done.cues[name] = cues.within(clip.action, start, end, fps(scene))
    _remove_others(files, clip_folder(actor), {n + ext for n in kept for ext in (".json", ".glb")})
    return done


def export_expressions(context, suite, files):
    """Writes each expression's .json into expressions/, which then holds only them."""
    from .actors import Clip

    scene = context.scene
    directory = files.path(EXPRESSIONS_DIR)
    written, kept = [], set()
    for expression in suite.expressions:
        if expression.action is None:
            continue
        # Over its own keys: the app loops an expression on its own length.
        clip = Clip(expression.action, mode="ONCE")
        start, end = span(scene, clip)
        frames = range(start, end + 1)
        keys = shape_key_curves(clip)
        data = {"format": FORMAT, "fps": round(fps(scene), 6), "frames": len(frames)}
        data["shape_keys"] = _sampled(keys, frames)
        path = os.path.join(directory, clip.name + ".json")
        write_json(path, data)
        written.append(path)
        kept.add(clip.name + ".json")
    _remove_others(files, EXPRESSIONS_DIR, kept)
    return written


def _remove_others(files, folder, kept):
    # A clip removed from a list must not linger in the app. Only clip files are touched.
    for entry in files.listdir(folder):
        if entry.endswith((".json", ".glb")) and entry not in kept and files.isfile(f"{folder}/{entry}"):
            files.remove(f"{folder}/{entry}")
