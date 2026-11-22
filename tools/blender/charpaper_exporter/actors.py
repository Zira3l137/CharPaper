"""Who moves in a suite, and with which clips.

The character's armature is one actor. In an environment, any object with an action of its
own (on itself, its shape keys or its light) is one, and so is every armature, so it can carry
hotspots. Each actor's clips are the list on its object. An actor whose list is empty plays its
active action on the timeline, since that is what Blender's timeline shows for it; adding a clip
to the list makes the list what is exported.
"""

import re

from . import anim
from .props import file_stem

TRANSFORM_PATH = re.compile(
    r"^(delta_)?(location|rotation_euler|rotation_quaternion|rotation_axis_angle|scale)$"
)
LIGHT_PATH = re.compile(r"^(energy|color)$")


class Actor:
    def __init__(self, obj, environment=None):
        self.obj = obj
        # The environment entry it belongs to; None for the character.
        self.environment = environment

    @property
    def is_character(self):
        return self.environment is None

    @property
    def name(self):
        return self.obj.name


def object_slots(action):
    """The action's slots that animate an object, keys and all. Legacy actions have none."""
    return [
        s
        for s in anim.slots(action)
        if getattr(s, "target_id_type", "UNSPECIFIED") in ("OBJECT", "UNSPECIFIED") and anim.fcurves(action, s)
    ]


def resolve_slot(obj, action, picked=""):
    """The slot of `action` that plays on the object, and when that isn't certain, the slots it
    could be. Certain: the one picked, or the action's only one. Otherwise the one assigned to
    the object if it plays the action, else the one named after the object, else the first."""
    found = object_slots(action) if action is not None else []
    if not found:
        return None, []
    by_id = {s.identifier: s for s in found}
    if picked in by_id:
        return by_id[picked], []
    if len(found) == 1:
        return found[0], []
    data = obj.animation_data
    playing = getattr(data, "action_slot", None) if data is not None and data.action == action else None
    guess = playing if playing in found else by_id.get("OB" + obj.name, found[0])
    return guess, [s.identifier for s in found]


class Clip:
    """A clip as export sees it, stored on the object or implied by its active action."""

    def __init__(self, action, extra=None, mode="TIMELINE", label="", gaze=True, stored=None, obj=None):
        self.action = action
        self.reaction_only = bool(stored is not None and stored.reaction_only)
        # Which of the action's slots moves the object: an action can hold several, like an
        # idle copied with New and a reaction keyed beside it.
        self.slot, self.slots_unclear = resolve_slot(obj, action, stored.slot if stored else "") if obj else (None, [])
        # A second action played with it: a mesh's shape keys or a lamp's light.
        self.extra = extra
        self.mode = mode
        self.label = label
        self.gaze = gaze
        # The list entry it comes from; None when implied.
        self.stored = stored

    @property
    def name(self):
        """The clip's name in the app and its files' name."""
        return file_stem(self.action.name)

    def groups(self):
        """Its unmuted curves by where they come from, as (label, slot or None, curves): the
        action's object curves through its slot, each of its other slots (shape keys, a light),
        and its second action."""
        found = []
        object_slots_ = {s.handle for s in object_slots(self.action)}
        if anim.is_layered(self.action):
            for slot in anim.slots(self.action):
                if slot.handle in object_slots_ and slot != self.slot:
                    continue
                found.append((slot.identifier, slot, anim.fcurves(self.action, slot)))
        else:
            found.append((self.action.name, None, anim.fcurves(self.action)))
        if self.extra is not None:
            found.append((self.extra.name, None, anim.fcurves(self.extra)))
        return [(label, slot, [c for c in curves if not c.mute]) for label, slot, curves in found]

    def curves(self):
        """Its unmuted curves, from all its groups."""
        return [c for _, _, curves in self.groups() for c in curves]

    def bone_names(self):
        return {
            m.group(1).replace('\\"', '"').replace("\\\\", "\\")
            for m in (anim.BONE_PATH.match(c.data_path) for c in self.curves())
            if m
        }


def own_actions(obj):
    """The actions playing on the object, its shape keys and its light, in that order."""
    found = []
    for owner in (obj, shape_keys_of(obj), obj.data if obj.type == "LIGHT" else None):
        data = getattr(owner, "animation_data", None)
        if data is not None and data.action is not None:
            found.append(data.action)
    return found


def shape_keys_of(obj):
    return obj.data.shape_keys if obj.type == "MESH" and obj.data else None


def is_actor(obj):
    return obj.type == "ARMATURE" or bool(own_actions(obj)) or len(obj.charpaper.clips) > 0


def character(suite):
    return Actor(suite.armature) if suite.armature is not None else None


def environment_actors(entry):
    if entry.collection is None:
        return []
    found = [obj for obj in entry.collection.all_objects if obj.type != "CAMERA" and is_actor(obj)]
    return [Actor(obj, entry) for obj in sorted(found, key=lambda o: o.name)]


def all_actors(suite):
    """The character first, then each exported environment's actors."""
    actors = [character(suite)] if suite.armature is not None else []
    for entry in suite.environments:
        if entry.enabled:
            actors.extend(environment_actors(entry))
    return actors


def clips(actor):
    stored = actor.obj.charpaper.clips
    if len(stored) > 0:
        return [
            Clip(c.action, c.extra, c.mode, c.label, c.gaze, stored=c, obj=actor.obj)
            for c in stored
            if c.action is not None
        ]
    if actor.is_character:
        return []
    actions = own_actions(actor.obj)
    if not actions:
        return []
    return [Clip(actions[0], actions[1] if len(actions) > 1 else None, obj=actor.obj)]


def shape_key_names(actor, suite):
    """The shape keys an actor's clips may key: those of every skin for the character, else the
    actor's own meshes and those under it."""
    if actor.is_character:
        meshes = anim.shape_keyed_skin_meshes(suite)
    else:
        meshes = [o for o in [actor.obj, *actor.obj.children_recursive] if shape_keys_of(o)]
    names = set()
    for mesh in meshes:
        names.update(shape_keys_of(mesh).key_blocks.keys())
    return names


def fits(actor, action, suite):
    """Whether the action keys anything the actor has: its bones, its transform, its shape
    keys or its light."""
    obj = actor.obj
    if obj.type == "ARMATURE" and anim.bone_names(action) & set(obj.data.bones.keys()):
        return True
    paths = {curve.data_path for curve in anim.fcurves(action)}
    if any(TRANSFORM_PATH.match(path) for path in paths):
        return True
    if obj.type == "LIGHT" and any(LIGHT_PATH.match(path) for path in paths):
        return True
    return bool(anim.key_names(action) & shape_key_names(actor, suite))


def fitting_actions(actor, suite):
    """Every action that fits, except one moving only objects that another object plays: an
    object's transform is all its actions share, so those would fit every actor."""
    import bpy

    others = [o for o in bpy.data.objects if o is not actor.obj and o.animation_data]

    def offered(action):
        if not fits(actor, action, suite):
            return False
        own = anim.bone_names(action) or anim.key_names(action) or not _moves_objects_only(action)
        return own or not any(anim.users(action, o) for o in others)

    return [a for a in sorted(bpy.data.actions, key=lambda a: a.name) if offered(a)]


def _moves_objects_only(action):
    paths = {curve.data_path for curve in anim.fcurves(action)}
    return bool(paths) and all(TRANSFORM_PATH.match(path) for path in paths)


def find(suite, name):
    """The actor whose object has this name."""
    return next((a for a in all_actors(suite) if a.name == name), None)


def start_clip(actor, found=None):
    """The clip a prop plays by itself: its starred one, if it plays on and on, else its first
    Timeline clip in the list, else its first Hold one. `found`: its clips, if already known."""
    looping = [c for c in (clips(actor) if found is None else found) if c.mode != "ONCE"]
    starred = next((c for c in looping if c.name == actor.obj.charpaper.start_clip), None)
    return starred or next((c for c in looping if c.mode == "TIMELINE"), None) or next(iter(looping), None)
