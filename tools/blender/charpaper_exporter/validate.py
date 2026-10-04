import os
from collections import Counter

import bpy

from . import anim
from . import gltf
from .props import file_stem

MAX_SHAPE_KEYS = 256


class Finding:
    def __init__(self, message, obj=None, bone="", severity="WARNING"):
        self.message = message
        self.object_name = obj.name if obj is not None else ""
        self.bone_name = bone
        self.severity = severity


def _examples(names, limit=3):
    names = sorted(names)
    shown = ", ".join(names[:limit])
    return shown + (f" and {len(names) - limit} more" if len(names) > limit else "")


def _chain(obj):
    while obj is not None:
        yield obj
        obj = obj.parent


def _deformed_by(obj, armature):
    return any(m.type == "ARMATURE" and m.object == armature for m in obj.modifiers)


def check(context, suite):
    out = []
    add = out.append

    if not gltf.available():
        add(Finding("the glTF exporter add-on is disabled; enable it in Preferences > Add-ons"))
    if suite.folder.startswith("//") and not bpy.data.filepath:
        add(Finding("the folder is relative to the .blend, which is not saved yet"))
    if suite.gltf.image_format == "WEBP":
        add(Finding("WebP images only load if the app is built with Bevy's webp feature", severity="INFO"))

    armature = suite.armature
    if armature is None:
        add(Finding("no armature picked, so the model, skins and animations cannot be exported"))
    else:
        _check_rig(armature, add)
        if suite.gaze.mode == "WRITE":
            _check_gaze(suite, armature, add)

    _check_names(suite, add)
    for entry in suite.skins:
        if entry.enabled:
            _check_skin(suite, entry, armature, add)
    for entry in suite.animations:
        if entry.enabled:
            _check_animation(suite, entry, armature, add)
    for entry in suite.environments:
        if entry.enabled:
            _check_environment(entry, add)
    for entry in suite.cameras:
        if entry.enabled:
            _check_camera(entry, add)
    _check_stale_formats(suite, add)
    return out


def _check_rig(armature, add):
    for obj in _chain(armature):
        scale = obj.scale
        if max(scale) - min(scale) > 1e-4:
            add(Finding(f"{obj.name} has a non-uniform scale; bone rotations will skew in the app", obj))
    rotation = armature.matrix_world.to_quaternion()
    if rotation.angle > 1e-4:
        add(Finding(
            f"{armature.name} is rotated; that's fine only if she still stands on +Z facing -Y",
            armature,
            severity="INFO",
        ))


def _check_gaze(suite, armature, add):
    gaze = suite.gaze
    bones = armature.data.bones
    eyes = [e.bone for e in gaze.eyes if e.bone]
    if not eyes:
        add(Finding("gaze lists no eye bones, so she never follows the cursor", armature))
    for name in eyes + [gaze.head, gaze.neck]:
        if name and name not in bones:
            add(Finding(f"gaze: {name!r} is not a bone of {armature.name}; gaze will be off", armature))

    def below(child, parent):
        return child in bones and parent in bones and bones[parent] in bones[child].parent_recursive

    for eye in eyes:
        if gaze.head and not below(eye, gaze.head):
            add(Finding(f"gaze: {eye} is not below {gaze.head}; the eye stays behind when the head turns", armature, eye))
    if gaze.head and gaze.neck and not below(gaze.head, gaze.neck):
        add(Finding(f"gaze: {gaze.head} is not below {gaze.neck}; the neck turn won't carry the head", armature, gaze.head))

    for eye in eyes:
        if eye in bones:
            _check_eye_pivot(suite, armature, bones[eye], add)

    keys = set()
    for skin in suite.skins:
        if skin.collection:
            for obj in skin.collection.all_objects:
                if obj.type == "MESH" and obj.data.shape_keys:
                    keys.update(obj.data.shape_keys.key_blocks.keys())
    for side in ("left", "right", "up", "down"):
        key = getattr(gaze, "key_" + side)
        if key and key not in keys:
            add(Finding(f"gaze: no skin has a shape key named {key!r}, so it never moves"))


def _check_eye_pivot(suite, armature, bone, add):
    import numpy

    points = []
    to_armature = armature.matrix_world.inverted()
    for skin in suite.skins:
        if skin.collection is None:
            continue
        for obj in skin.collection.all_objects:
            if obj.type != "MESH" or not _deformed_by(obj, armature):
                continue
            group = obj.vertex_groups.get(bone.name)
            if group is None:
                continue
            matrix = to_armature @ obj.matrix_world
            index = group.index
            for vertex in obj.data.vertices:
                for g in vertex.groups:
                    if g.group == index and g.weight >= 0.5:
                        points.append(matrix @ vertex.co)
                        break
    if len(points) < 8:
        return
    # Fits a sphere, so a half-shell or an iris cap still finds the eyeball's center.
    p = numpy.array([tuple(v) for v in points])
    a = numpy.column_stack((2 * p, numpy.ones(len(p))))
    b = (p * p).sum(axis=1)
    solution = numpy.linalg.lstsq(a, b, rcond=None)[0]
    center = solution[:3]
    radius_sq = solution[3] + center.dot(center)
    if radius_sq <= 0:
        return
    radius = radius_sq ** 0.5
    offset = numpy.linalg.norm(center - numpy.array(tuple(bone.head_local)))
    if offset > max(0.0005, radius * 0.05):
        add(Finding(
            f"gaze: {bone.name}'s head is {offset * 1000:.1f} mm from its eyeball's center; "
            "the eye will swing instead of spin",
            armature,
            bone.name,
        ))


def _check_names(suite, add):
    groups = (
        ("skin", [e.name for e in suite.skins if e.enabled]),
        ("animation file", [e.name for e in suite.animations if e.enabled]),
        ("environment", [e.name for e in suite.environments if e.enabled]),
        ("camera", [e.name for e in suite.cameras if e.enabled]),
    )
    for kind, names in groups:
        for name in names:
            if file_stem(name) != name:
                add(Finding(f"{kind} {name!r} has characters a file name can't hold; saved as {file_stem(name)!r}"))
        for name, count in Counter(file_stem(n) for n in names).items():
            if count > 1:
                add(Finding(f"{count} {kind}s are named {name!r}; they overwrite each other"))

    clips = Counter(
        clip.name or clip.action.name
        for entry in suite.animations
        if entry.enabled
        for clip in entry.clips
        if clip.enabled and clip.action
    )
    for name, count in clips.items():
        if count > 1:
            add(Finding(f"{count} clips are named {name!r}; the app keeps only one"))
    if any(c.name == "orbit" for c in suite.cameras if c.enabled):
        add(Finding("a camera named 'orbit' hides the built-in orbit camera"))


def _check_skin(suite, entry, armature, add):
    if entry.collection is None:
        add(Finding(f"skin {entry.name!r} has no collection"))
        return
    meshes = [o for o in entry.collection.all_objects if o.type == "MESH"]
    if not meshes:
        add(Finding(f"skin {entry.name!r}: {entry.collection.name} holds no meshes"))
    loose = []
    for obj in meshes:
        if armature is not None and not _deformed_by(obj, armature) and armature not in _chain(obj.parent):
            loose.append(obj.name)
        for modifier in obj.modifiers:
            if modifier.type == "ARMATURE" and modifier.object not in (None, armature):
                add(Finding(f"{obj.name} is deformed by {modifier.object.name}, not the suite's armature", obj))
        key = obj.data.shape_keys
        if key is None:
            continue
        if len(key.key_blocks) - 1 > MAX_SHAPE_KEYS:
            add(Finding(f"{obj.name} has {len(key.key_blocks) - 1} shape keys; Bevy supports {MAX_SHAPE_KEYS}", obj))
        others = [m for m in obj.modifiers if m.type != "ARMATURE" and m.show_viewport]
        if others and suite.gltf.apply_modifiers:
            add(Finding(
                f"{obj.name} has shape keys and modifiers ({others[0].name}); "
                "with Apply modifiers on, the exporter may drop its shape keys",
                obj,
            ))
    if loose:
        add(Finding(
            f"skin {entry.name!r}: {len(loose)} mesh(es) are neither skinned nor parented to the "
            f"armature, so they won't follow her: {_examples(loose)}",
            bpy.data.objects.get(loose[0]),
        ))
    others = [o.name for o in entry.collection.all_objects if o.type not in {"MESH", "EMPTY", "ARMATURE"}]
    if others:
        add(Finding(f"skin {entry.name!r}: left out (not meshes): {_examples(others)}", severity="INFO"))

    keys = set()
    for obj in meshes:
        if obj.data.shape_keys:
            keys.update(obj.data.shape_keys.key_blocks.keys())
    for expression in entry.expressions:
        if expression.enabled and expression.action and not anim.key_names(expression.action) & keys:
            add(Finding(f"skin {entry.name!r}: expression {expression.action.name!r} keys none of its shape keys"))


def _check_animation(suite, entry, armature, add):
    clips = [c for c in entry.clips if c.enabled and c.action]
    if not clips:
        add(Finding(f"animation file {entry.name!r} has no ticked clips"))
        return
    if armature is not None:
        bones = set(armature.data.bones.keys())
        for clip in clips:
            missing = anim.bone_names(clip.action) - bones
            if missing:
                add(Finding(
                    f"clip {clip.action.name!r} keys {len(missing)} bone(s) {armature.name} lacks: "
                    f"{_examples(missing)}",
                    armature,
                ))

    if not entry.use_correctives:
        if any(c.corrective for c in clips):
            add(Finding(
                f"animation file {entry.name!r}: some clips name correctives, but Correctives is off",
                severity="INFO",
            ))
        return
    if entry.correctives is None:
        add(Finding(f"animation file {entry.name!r}: Correctives is on but no collection is picked"))
        return
    skin_meshes = {
        o for s in suite.skins if s.collection for o in s.collection.all_objects if o.type == "MESH"
    }
    meshes = [o for o in entry.correctives.all_objects if o.type == "MESH" and o.data.shape_keys]
    if not meshes:
        add(Finding(f"animation file {entry.name!r}: {entry.correctives.name} has no shape-keyed meshes"))
    keyed = set()
    for clip in clips:
        if clip.corrective:
            keyed |= anim.key_names(clip.corrective)
    for obj in meshes:
        if obj not in skin_meshes:
            add(Finding(f"{obj.name} is in no skin, so its correctives move nothing", obj))
        if not keyed & set(obj.data.shape_keys.key_blocks.keys()):
            add(Finding(f"{obj.name}: no clip keys its shape keys, so it only takes up space", obj))


def _check_environment(entry, add):
    if entry.collection is None:
        add(Finding(f"environment {entry.name!r} has no collection"))
        return
    objects = list(entry.collection.all_objects)
    if not any(o.type == "LIGHT" for o in objects):
        add(Finding(
            f"environment {entry.name!r} has no lights; unless it ships reflection maps, "
            "the character renders black",
        ))
    cameras = [o.name for o in objects if o.type == "CAMERA"]
    if cameras:
        add(Finding(f"environment {entry.name!r}: cameras are left out, use Cameras: {_examples(cameras)}", severity="INFO"))


def _check_camera(entry, add):
    if entry.camera is None:
        add(Finding(f"camera {entry.name!r} has no camera object"))
        return
    chain = list(_chain(entry.camera))
    for obj in chain[1:]:
        if obj.type != "EMPTY":
            add(Finding(f"camera {entry.name!r} hangs from {obj.name}, a {obj.type.lower()}; it gets exported too", obj))
    if entry.action and not any(anim.users(entry.action, o) for o in chain):
        add(Finding(
            f"camera {entry.name!r}: neither the camera nor its parents use {entry.action.name!r}; "
            "it goes on the camera",
            entry.camera,
            severity="INFO",
        ))


def _check_stale_formats(suite, add):
    folder = bpy.path.abspath(suite.folder)
    if not os.path.isdir(folder):
        return
    other = ".gltf" if suite.gltf.file_format == "GLB" else ".glb"
    groups = (
        ("skins", suite.skins),
        ("animations", suite.animations),
        ("environment", suite.environments),
        ("cameras", suite.cameras),
    )
    stale = []
    for subdir, entries in groups:
        for entry in entries:
            path = os.path.join(folder, subdir, file_stem(entry.name) + other)
            if entry.enabled and os.path.isfile(path):
                stale.append(os.path.relpath(path, folder))
    if stale:
        add(Finding(f"older {other} files would sit next to the new ones; delete them: {_examples(stale)}"))
