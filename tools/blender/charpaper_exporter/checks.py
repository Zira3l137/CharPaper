"""Checks on what moves: every actor's clips, the character's expressions, and the shape keys
both play on. Each looks for a way the app would show something other than Blender does, and
says what to do about it. They only warn; nothing here stops an export."""

import os
import re

import bpy

from . import actors
from . import anim
from . import clips as clip_files
from . import cues
from . import drivers
from .actors import TRANSFORM_PATH
from .actors import shape_keys_of
from .exporters import playing_on_shape_keys
from .props import file_stem

# What a clip can carry; any other keyed property stays as the file has it.
BONE_TRANSFORM = re.compile(
    r'^pose\.bones\["(?:[^"\\]|\\.)*"\]\.(location|rotation_euler|rotation_quaternion|rotation_axis_angle|scale)$'
)
BONE_OTHER = re.compile(r'^pose\.bones\["(?:[^"\\]|\\.)*"\](\[|\.constraints)')
KEY_VALUE = re.compile(r'^key_blocks\["(?:[^"\\]|\\.)*"\]\.value$')
LIGHT_KEPT = {"energy", "color"}


class Finding:
    def __init__(self, message, obj=None, bone="", severity="WARNING", clip="", sound="", hotspot=""):
        self.message = message
        self.object_name = obj.name if obj is not None else ""
        self.bone_name = bone
        self.severity = severity
        self.clip = clip
        # The environment whose music or ambience it is about.
        self.sound = sound
        self.hotspot = hotspot


def _examples(names, limit=3):
    names = sorted(names)
    shown = ", ".join(names[:limit])
    return shown + (f" and {len(names) - limit} more" if len(names) > limit else "")


def _are(names):
    """`names` with the verb agreeing: "Head is", "Head and Neck are"."""
    names = list(names)
    return f"{_examples(names)} {'is' if len(names) == 1 else 'are'}"


def _keyed_range(action):
    """The first and last frame any of its unmuted curves keys."""
    frames = [k.co.x for c in anim.fcurves(action) if not c.mute for k in c.keyframe_points]
    return (round(min(frames)), round(max(frames))) if frames else None


def _cycles(action):
    curves = [c for c in anim.fcurves(action) if not c.mute and c.keyframe_points]
    cycling = [c for c in curves if any(m.type == "CYCLES" and not m.mute for m in c.modifiers)]
    return len(cycling) == len(curves) and bool(curves), bool(cycling)


def check(context, suite, add):
    scene = context.scene
    found = actors.all_actors(suite)
    for actor in found:
        _check_actor(scene, suite, actor, add)
    _check_names(found, add)
    _check_default(suite, add)
    _check_expressions(suite, add)
    _check_skin_sliders(suite, add)


def _check_names(found, add):
    by_folder = {}
    for actor in found:
        if not actor.is_character:
            key = (actor.environment.name, file_stem(actor.name))
            by_folder.setdefault(key, []).append(actor.name)
    for (environment, folder), names in by_folder.items():
        if len(names) > 1:
            add(Finding(f"environment {environment!r}: {_examples(names)} share the clip folder {folder!r}; rename one"))


def _check_default(suite, add):
    armature = suite.armature
    if armature is None or not suite.default_clip:
        return
    clip = next((c for c in actors.clips(actors.character(suite)) if c.name == suite.default_clip), None)
    if clip is None:
        add(Finding(f"the character starts with {suite.default_clip!r}, which is none of its clips; star one", armature))
    elif clip.mode == "ONCE":
        add(Finding(
            f"the character starts with {clip.name!r}, a Once clip, which would play once and leave nothing under "
            "it; the app starts in the rest pose instead. Star a Timeline or Hold clip",
            armature,
            clip=clip.name,
        ))


def _check_actor(scene, suite, actor, add):
    obj = actor.obj
    listed = actors.clips(actor)
    if actor.is_character and not listed:
        add(Finding(f"{obj.name} has no clips, so the character stands in its rest pose; add some in Animation", obj))
    names = {}
    for item in obj.charpaper.clips:
        if item.action is None:
            add(Finding(f"{obj.name}: a clip has no action; pick one or remove it", obj))
    for clip in listed:
        names.setdefault(clip.name, []).append(clip.action.name)
    for name, actions in names.items():
        if len(actions) > 1:
            same = len(set(actions)) == 1
            add(Finding(
                f"{obj.name}: {'the same action is listed twice' if same else 'actions ' + _examples(actions) + ' save to one file'} "
                f"({name!r}); only the first is exported",
                obj,
                clip=name,
            ))
    checked = set()
    for clip in listed:
        if clip.name not in checked:
            checked.add(clip.name)
            _check_clip(scene, suite, actor, clip, add)
    _check_unlisted(actor, listed, add)
    _check_nla(actor, add)


def _check_unlisted(actor, listed, add):
    obj = actor.obj
    if not actor.is_character or not listed:
        return
    active = obj.animation_data.action if obj.animation_data else None
    if active is not None and all(c.action != active for c in listed):
        add(Finding(
            f"{obj.name} plays {active.name!r} in Blender, but it is none of its clips, so the app doesn't "
            "have it. Add it in Animation if it should be",
            obj,
            severity="INFO",
        ))


def _check_nla(actor, add):
    """The app plays a clip alone, so what other strips add in Blender doesn't show."""
    obj = actor.obj
    data = obj.animation_data
    tracks = [t for t in (data.nla_tracks if data else ()) if not t.mute and any(not s.mute for s in t.strips)]
    if tracks and not actor.is_character and len(obj.charpaper.clips) == 0 and data.action is None:
        add(Finding(
            f"{obj.name} only plays NLA strips, which the app doesn't play; add their actions as clips",
            obj,
        ))
    elif tracks and data.action is not None:
        add(Finding(
            f"{obj.name} plays NLA tracks on top of its action ({_examples(t.name for t in tracks)}); the app plays "
            "each clip alone, so what the strips add doesn't show",
            obj,
            severity="INFO",
        ))


def _check_clip(scene, suite, actor, clip, add):
    obj, action, name = actor.obj, clip.action, clip.name
    if not actors.fits(actor, action, suite) and not clip.extra:
        if not _check_paths(actor, clip, add):
            add(Finding(f"clip {name!r} keys nothing {obj.name} has, so it plays as the rest pose", obj, clip=name))
        return
    nodes = clip_files.keyed_nodes(actor, action, suite)
    if obj.type == "ARMATURE":
        keyed = anim.bone_names(action)
        missing = keyed - set(obj.data.bones.keys())
        if missing:
            add(Finding(
                f"clip {name!r} keys {len(missing)} bone(s) {obj.name} lacks, which do nothing: {_examples(missing)}. "
                "Bone names must match exactly",
                obj,
                clip=name,
            ))
        dropped = keyed & set(obj.data.bones.keys()) - set(nodes)
        if suite.gltf.deform_bones_only and dropped and not any(n != obj.name for n in nodes):
            add(Finding(
                f"clip {name!r} only moves bones that Deform bones only leaves out of the file "
                f"({_examples(dropped)}), so it plays as the rest pose. Turn Deform bones only off, or have "
                "deform bones follow them by constraints",
                obj,
                clip=name,
            ))
        if obj.name in nodes and actor.is_character:
            add(Finding(
                f"clip {name!r} moves {obj.name} itself, not only its bones; on the timeline the character "
                "jumps back to where it started on every loop",
                obj,
                severity="INFO",
                clip=name,
            ))
    _check_paths(actor, clip, add)
    _check_span(scene, actor, clip, add)
    _check_seam(scene, actor, clip, add)
    _check_markers(suite, scene, actor, clip, add)
    _check_shape_keys(suite, actor, clip, add)
    if actor.is_character and clip.gaze and suite.gaze.mode == "WRITE":
        eyes = {e.bone for e in suite.gaze.eyes if e.bone}
        moved = anim.bone_names(action) & eyes
        if moved:
            add(Finding(
                f"clip {name!r} keys {_examples(moved)}, which gaze sets itself while the character follows "
                "the cursor; untick the clip's Gaze for its own eye movement to show",
                obj,
                severity="INFO",
                clip=name,
            ))


def _check_paths(actor, clip, add):
    """Keyed properties a clip can't carry: materials, visibility, modifiers, camera settings."""
    lost = set()
    for action in (clip.action, clip.extra):
        for curve in anim.fcurves(action) if action else ():
            path = curve.data_path
            if curve.mute or TRANSFORM_PATH.match(path) or BONE_TRANSFORM.match(path) or BONE_OTHER.match(path):
                continue
            if KEY_VALUE.match(path) or (actor.obj.type == "LIGHT" and path in LIGHT_KEPT):
                continue
            lost.add(path)
    if not lost:
        return False
    if lost:
        kept = "transforms, bones, shape keys and a lamp's Strength and Color"
        add(Finding(
            f"clip {clip.name!r} keys {_examples(lost)}, which doesn't carry over: only {kept} move in the app",
            actor.obj,
            clip=clip.name,
        ))
    return True


def _check_span(scene, actor, clip, add):
    action, name, obj = clip.action, clip.name, actor.obj
    keyed = _keyed_range(action)
    if keyed is None:
        return
    first, last = keyed
    all_cycle, some_cycle = _cycles(action)
    start, end = scene.frame_start, scene.frame_end
    if clip.mode == "ONCE":
        if some_cycle:
            add(Finding(f"clip {name!r} plays Once, so its Cycles modifier repeats nothing", obj, severity="INFO", clip=name))
        return
    if clip.mode == "HOLD":
        if last > first:
            add(Finding(
                f"clip {name!r} is a Hold clip: only frame {first} shows, though it is keyed to frame {last}",
                obj,
                severity="INFO",
                clip=name,
            ))
        return
    length = end - start + 1
    if action.use_frame_range:
        own = round(action.frame_range[1]) - round(action.frame_range[0])
        own = own + 1 if own > 0 else 1
        if length % own:
            add(Finding(
                f"clip {name!r} loops on its Manual Frame Range of {own} frames, which doesn't divide the "
                f"{length}-frame timeline, so it jumps when the timeline starts over",
                obj,
                clip=name,
            ))
        return
    if all_cycle:
        period = last - first
        if period > 0 and length % period:
            add(Finding(
                f"clip {name!r} repeats every {period} frames by its Cycles modifier, which doesn't divide the "
                f"{length}-frame timeline, so it jumps when the timeline starts over",
                obj,
                clip=name,
            ))
        return
    # Keying the frame after the end as the first frame is how a loop is closed in Blender.
    closing = last == end + 1 and all(
        abs(c.evaluate(end + 1) - c.evaluate(start)) < 1e-4 for c in anim.fcurves(action) if not c.mute
    )
    if last > end and not closing:
        add(Finding(
            f"clip {name!r} is keyed up to frame {last}, but the timeline ends at {end}: frames {end + 1}–{last} "
            "never play. Lengthen the scene's frame range, or set the action's Manual Frame Range",
            obj,
            clip=name,
        ))
    if first < start:
        add(Finding(
            f"clip {name!r} starts at frame {first}, before the timeline's frame {start}: those frames never play",
            obj,
            clip=name,
        ))
    if last < end and not some_cycle and not closing:
        add(Finding(
            f"clip {name!r} ends at frame {last} and holds still until frame {end}. If it should keep going, "
            "add a Cycles modifier, or make the scene's frame range its loop",
            obj,
            severity="WARNING" if actor.is_character else "INFO",
            clip=name,
        ))


def _check_seam(scene, actor, clip, add):
    """A timeline loop goes from the last frame straight to the first; if they differ, it pops."""
    if clip.mode != "TIMELINE" or not actor.is_character or clip.action.use_frame_range:
        return
    start, end = scene.frame_start, scene.frame_end
    keyed = _keyed_range(clip.action)
    if keyed is None or keyed[1] < end:
        return
    popping = set()
    for curve in anim.fcurves(clip.action):
        if curve.mute or any(m.type == "CYCLES" for m in curve.modifiers):
            continue
        step = abs(curve.evaluate(end) - curve.evaluate(end - 1)) if end > start else 0.0
        jump = abs(curve.evaluate(start) - curve.evaluate(end))
        if jump > max(4.0 * step, 1e-3):
            match = anim.BONE_PATH.match(curve.data_path)
            popping.add(match.group(1) if match else curve.data_path)
    if popping:
        them, they = ("it", "it pops") if len(popping) == 1 else ("them", "they pop")
        add(Finding(
            f"clip {clip.name!r} doesn't loop smoothly: at frame {end} {_are(popping)} far from where "
            f"frame {start} has {them}, so {they} each loop. Key frame {end + 1} as frame {start}, or end "
            f"the scene a frame before the last key",
            actor.obj,
            clip=clip.name,
        ))


def _check_markers(suite, scene, actor, clip, add):
    start, end = clip_files.span(scene, clip)
    outside = [m.name for m in cues.markers(clip.action) if not start <= m.frame <= end]
    if outside:
        add(Finding(
            f"clip {clip.name!r}: pose markers outside frames {start}–{end} play nothing: {_examples(outside)}",
            actor.obj,
            clip=clip.name,
        ))
    folder = bpy.path.abspath(suite.folder)
    unknown = {
        cues.sound_name(m.name)
        for m in cues.markers(clip.action)
        if cues.find_sound(cues.sound_name(m.name)) is None
        and not os.path.isfile(cues.path(folder, cues.sound_name(m.name)))
    }
    if unknown:
        add(Finding(
            f"clip {clip.name!r}: no sound named {_examples(unknown)} in the .blend or in {cues.SOUNDS_DIR}/, so "
            "those markers play nothing. A marker is named after the Sound it plays",
            actor.obj,
            clip=clip.name,
        ))


def _meshes(suite, actor):
    if actor.is_character:
        return anim.shape_keyed_skin_meshes(suite)
    return [o for o in [actor.obj, *actor.obj.children_recursive] if shape_keys_of(o)]


def _check_shape_keys(suite, actor, clip, add):
    keys = set(clip_files.shape_key_curves(clip))
    if not keys:
        return
    meshes = _meshes(suite, actor)
    landing = [o for o in meshes if keys & set(shape_keys_of(o).key_blocks.keys())]
    if not landing:
        add(Finding(
            f"clip {clip.name!r} keys shape keys none of {'the skins' if actor.is_character else actor.obj.name}' "
            f"meshes have: {_examples(keys)}",
            actor.obj,
            clip=clip.name,
        ))
    for obj in landing:
        driven = keys & drivers.driven_keys(obj)
        if driven:
            add(Finding(
                f"clip {clip.name!r} keys {_examples(driven)} on {obj.name}, which drivers drive; the drivers win, "
                "so those keys do nothing",
                obj,
                clip=clip.name,
            ))


def _check_expressions(suite, add):
    skins = [s for s in suite.skins if s.enabled and s.collection]
    for expression in suite.expressions:
        action = expression.action
        if action is None:
            add(Finding("an expression has no action; pick one or remove it"))
            continue
        name = file_stem(action.name)
        if anim.bone_names(action) or any(TRANSFORM_PATH.match(c.data_path) for c in anim.fcurves(action)):
            add(Finding(
                f"expression {name!r} keys bones or objects too, which an expression never moves; "
                "only its shape keys play",
                clip=name,
            ))
        keys = anim.key_names(action)
        for skin in skins:
            meshes = [o for o in skin.collection.all_objects if shape_keys_of(o)]
            have = set().union(*(shape_keys_of(o).key_blocks.keys() for o in meshes)) if meshes else set()
            lacking = keys - have
            if lacking == keys:
                add(Finding(
                    f"expression {name!r} keys none of skin {skin.name!r}'s shape keys, so the app doesn't offer it there",
                    severity="INFO",
                    clip=name,
                ))
            elif lacking:
                add(Finding(
                    f"expression {name!r}: skin {skin.name!r} has no {_examples(lacking)}, so those don't move there",
                    clip=name,
                ))
            for obj in meshes:
                driven = keys & drivers.driven_keys(obj)
                if driven:
                    add(Finding(
                        f"expression {name!r} keys {_examples(driven)} on {obj.name}, which drivers drive; the "
                        "drivers win, so those keys do nothing",
                        obj,
                        clip=name,
                    ))


def _check_skin_sliders(suite, add):
    """The app rests a shape key no clip keys at its slider; a slider left up shows."""
    used = set()
    character = actors.character(suite)
    for clip in actors.clips(character) if character else ():
        used |= set(clip_files.shape_key_curves(clip))
    for expression in suite.expressions:
        if expression.action:
            used |= anim.key_names(expression.action)
    for skin in suite.skins:
        if not (skin.enabled and skin.collection):
            continue
        for obj in skin.collection.all_objects:
            key = shape_keys_of(obj)
            if key is None:
                continue
            action = playing_on_shape_keys(obj)
            if action is not None:
                add(Finding(
                    f"skin {skin.name!r}: the mesh {obj.name} has the action {action.name} playing on its shape keys, "
                    f"so its sliders would export as {action.name}'s values and the character rests on them in the "
                    "app. Unassign the action in the Shape Key Editor before exporting",
                    obj,
                ))
                continue
            if not key.use_relative:
                add(Finding(f"{obj.name}'s shape keys are not Relative, which glTF can't carry", obj))
                continue
            raised = [
                f"{b.name} at {b.value:.2f}"
                for b in key.key_blocks[1:]
                if b.name in used and abs(b.value) > 1e-4 and not b.mute
            ]
            if raised:
                one = len(raised) == 1
                add(Finding(
                    f"skin {skin.name!r}: on {obj.name}, {_examples(raised)}; while no clip or expression keys "
                    f"{'it, it shows at that value' if one else 'them, they show at those values'} in the app. "
                    f"Set {'it' if one else 'them'} to 0 unless that is the look you want",
                    obj,
                ))
