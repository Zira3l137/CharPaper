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


class Clip:
    """A clip as export sees it, stored on the object or implied by its active action."""

    def __init__(self, action, extra=None, mode="TIMELINE", label="", gaze=True, stored=None):
        self.action = action
        self.reaction_only = bool(stored is not None and stored.reaction_only)
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
            Clip(c.action, c.extra, c.mode, c.label, c.gaze, stored=c)
            for c in stored
            if c.action is not None
        ]
    if actor.is_character:
        return []
    actions = own_actions(actor.obj)
    if not actions:
        return []
    return [Clip(actions[0], actions[1] if len(actions) > 1 else None)]


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
