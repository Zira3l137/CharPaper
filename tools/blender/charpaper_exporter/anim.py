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


def shape_key_source(clip):
    """Where a clip's hand-keyed shape keys come from: the action picked for them, or else the
    clip's own action when it keys shape keys too, in a slot of their own (Blender 4.4 and
    newer). None when the clip keys none."""
    if clip.corrective is not None:
        return clip.corrective
    if clip.action is not None and key_names(clip.action):
        return clip.action
    return None


def shape_keyed_skin_meshes(suite):
    """Every mesh with shape keys in a skin's collection, each once."""
    meshes = []
    for skin in suite.skins:
        if skin.collection is None:
            continue
        for obj in skin.collection.all_objects:
            if obj.type == "MESH" and obj.data.shape_keys and obj not in meshes:
                meshes.append(obj)
    return meshes


def users(action, obj):
    data = obj.animation_data
    if data is None:
        return False
    if data.action == action:
        return True
    return any(strip.action == action for track in data.nla_tracks for strip in track.strips)


def _played_curves(action, slot, wanted):
    """The unmuted curves the action plays through `slot` whose paths `wanted` accepts. A
    layered action plays nothing without a slot."""
    if is_layered(action) and slot is None:
        return []
    return [c for c in fcurves(action, slot) if not c.mute and wanted(c.data_path)]


def _reaches(strip, frame):
    if strip.frame_start <= frame <= strip.frame_end:
        return True
    return strip.extrapolation == "HOLD" or (strip.extrapolation == "HOLD_FORWARD" and frame >= strip.frame_start)


def playing(holder, frame, wanted=lambda path: True):
    """What animates `holder`, an object or other ID, at `frame`, as (action, NLA track name or
    None for its assigned action, the curves that play), or None. Only what Blender evaluates
    there counts: an action without a slot, a muted track, a track other than the solo one and
    a strip that doesn't reach the frame play nothing."""
    data = getattr(holder, "animation_data", None)
    if data is None:
        return None
    action = data.action
    if action is not None and data.action_influence > 0:
        curves = _played_curves(action, getattr(data, "action_slot", None), wanted)
        if curves:
            return action, None, curves
    if not data.use_nla:
        return None
    tracks = [t for t in data.nla_tracks if not t.mute]
    for track in [t for t in tracks if t.is_solo] or tracks:
        for strip in track.strips:
            if strip.mute or strip.action is None or not _reaches(strip, frame):
                continue
            if strip.use_animated_influence and strip.influence <= 0:
                continue
            curves = _played_curves(strip.action, getattr(strip, "action_slot", None), wanted)
            if curves:
                return strip.action, track.name, curves
    return None


def changes(curve):
    """Whether the curve ever moves: keys at more than one value, or a modifier making it."""
    values = {round(k.co.y, 6) for k in curve.keyframe_points}
    return len(values) > 1 or any(not m.mute for m in curve.modifiers)


def playing_on_shape_keys(obj, frame):
    """What sets the mesh's shape key sliders at `frame`, as (action, NLA track name or None),
    or None."""
    key = obj.data.shape_keys if obj.type == "MESH" and obj.data else None
    found = playing(key, frame, KEY_PATH.match) if key is not None else None
    return found[:2] if found else None


def stop(holder, frame, wanted=lambda path: True):
    """Stops what plays on `holder`: unassigns its action, keeping it with a fake user if
    nothing else uses it, and mutes the NLA tracks that play. Returns the actions stopped."""
    data = holder.animation_data
    stopped = []
    for _ in range(len(data.nla_tracks) + 1 if data else 0):
        found = playing(holder, frame, wanted)
        if found is None:
            break
        action, track, _ = found
        if track is None:
            # Unassigned, an action nothing else uses would be gone once the file is saved.
            if action.users <= 1:
                action.use_fake_user = True
            data.action = None
        else:
            data.nla_tracks[track].mute = True
        stopped.append(action)
    return stopped


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


def object_track(obj, name, action, frame_range=None):
    slot = pick_slot(
        action, "OBJECT", lambda s: bool(fcurves(action, s)), "OB" + obj.name
    )
    data = obj.animation_data or obj.animation_data_create()
    return add_track(data, name, action, slot, frame_range)
