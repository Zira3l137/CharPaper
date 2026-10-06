import re

import bpy

# Blender 4.4 moved F-curves into layered actions with slots, and 5.0 dropped the old
# `Action.fcurves`. Everything here reads both shapes so the rest of the add-on never
# has to care which Blender it runs in.

BONE_PATH = re.compile(r'^pose\.bones\["((?:[^"\\]|\\.)*)"\]')
KEY_PATH = re.compile(r'^key_blocks\["((?:[^"\\]|\\.)*)"\]')


def is_layered(action):
    return len(getattr(action, "layers", ())) > 0


def slots(action):
    return list(getattr(action, "slots", ()))


def fcurves(action, slot=None):
    if is_layered(action):
        curves = []
        for layer in action.layers:
            for strip in layer.strips:
                for bag in getattr(strip, "channelbags", ()):
                    if slot is None or bag.slot_handle == slot.handle:
                        curves.extend(bag.fcurves)
        return curves
    return list(getattr(action, "fcurves", ()))


def _names(action, pattern, slot=None):
    found = set()
    for curve in fcurves(action, slot):
        match = pattern.match(curve.data_path)
        if match:
            found.add(match.group(1).replace('\\"', '"').replace("\\\\", "\\"))
    return found


def bone_names(action, slot=None):
    return _names(action, BONE_PATH, slot)


def key_names(action, slot=None):
    return _names(action, KEY_PATH, slot)


def is_armature_action(action):
    return bool(bone_names(action))


def is_shape_key_action(action):
    return bool(key_names(action))


def armature_actions(armature):
    bones = set(armature.data.bones.keys())
    return [a for a in bpy.data.actions if bone_names(a) & bones]


def shape_key_actions(meshes):
    keys = set()
    for mesh in meshes:
        if mesh.data.shape_keys:
            keys.update(mesh.data.shape_keys.key_blocks.keys())
    return [a for a in bpy.data.actions if key_names(a) & keys]


def users(action, obj):
    data = obj.animation_data
    if data is None:
        return False
    if data.action == action:
        return True
    return any(strip.action == action for track in data.nla_tracks for strip in track.strips)


def pick_slot(action, id_type, wanted, identifier):
    # The slot named after the target wins; otherwise the first slot that animates
    # anything the target has. Legacy actions have no slots at all.
    candidates = [
        s for s in slots(action) if getattr(s, "target_id_type", "UNSPECIFIED") in (id_type, "UNSPECIFIED")
    ]
    for slot in candidates:
        if slot.identifier == identifier:
            return slot
    for slot in candidates:
        if wanted(slot):
            return slot
    return None


def add_track(anim_data, name, action, slot=None, frame_range=None):
    start, end = action.frame_range if frame_range is None else frame_range
    track = anim_data.nla_tracks.new()
    track.name = name
    strip = track.strips.new(name, int(round(start)), action)
    if slot is not None and hasattr(strip, "action_slot"):
        strip.action_slot = slot
    # Blender sizes a new strip by the keys of the slot it guesses for it, before the slot above
    # is set. With several slots and none named after this object it guesses none, and the strip
    # comes out one frame long. Each end is clamped against the other, hence `end` twice.
    strip.action_frame_end = end
    strip.action_frame_start = start
    strip.action_frame_end = end
    return track


def object_track(obj, name, action):
    slot = pick_slot(
        action, "OBJECT", lambda s: bool(fcurves(action, s)), "OB" + obj.name
    )
    data = obj.animation_data or obj.animation_data_create()
    return add_track(data, name, action, slot)


def key_track(mesh_obj, name, action, frame_range=None):
    key = mesh_obj.data.shape_keys
    own = set(key.key_blocks.keys())
    if is_layered(action):
        slot = pick_slot(action, "KEY", lambda s: bool(key_names(action, s) & own), "KE" + key.name)
        if slot is None:
            return None
    else:
        slot = None
        if not key_names(action) & own:
            return None
    data = key.animation_data or key.animation_data_create()
    return add_track(data, name, action, slot, frame_range)
