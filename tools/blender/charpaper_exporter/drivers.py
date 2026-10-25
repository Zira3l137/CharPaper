# Drivers on a skin's shape keys, written next to the skin as <skin>.drivers.json for the app to
# run. glTF has no place for drivers, and running them in the app rather than baking them into
# clips makes them follow whatever moves the bones: clips, blends between them, and gaze.
#
# Only what the app can run is written: average, sum, min, max and simple-expression drivers,
# reading bones of the suite's armature (Rotational Difference, Transform Channel, Distance, or
# a Single Property that is a bone's location, rotation or scale) or other shape keys of the
# skin (Single Property). A driver reading a bone the export leaves out, like a pose reader's
# helper bone, reads a stand-in: the nearest exported bone above it, turned the way the left-out
# bone sits from it in the rest pose. That holds while nothing in between moves on its own,
# which is checked.
#
# Positions and the stand-ins' turns are written in glTF's axes, as the model is exported:
# Blender's world with +Y up, so (x, y, z) becomes (x, z, -y). Each bone a Transform Channel or
# Distance reads also gets its rest rotation in Blender's own axes, from which the app works out
# how the exported bone's axes relate to Blender's.

import json
import math
import os
import re

import bpy

from . import anim

VERSION = 1
KINDS = {"SCRIPTED": "scripted", "AVERAGE": "average", "SUM": "sum", "MIN": "min", "MAX": "max"}
KEY_VALUE = re.compile(r'^key_blocks\["(.+)"\]\.value$')
BONE_CHANNEL = re.compile(
    r'^pose\.bones\["(.+)"\]\.(location|rotation_euler|rotation_quaternion|scale)\[(\d)\]$'
)
SPACES = {"WORLD_SPACE": "world", "TRANSFORM_SPACE": "local", "LOCAL_SPACE": "local"}
EULERS = {"XYZ", "XZY", "YXZ", "YZX", "ZXY", "ZYX"}
ROTATIONS = {"QUATERNION": "quaternion", "SWING_TWIST_X": "swing_twist_x",
             "SWING_TWIST_Y": "swing_twist_y", "SWING_TWIST_Z": "swing_twist_z"}
CURVE_SAMPLES = 64


class Problem:
    def __init__(self, message, obj=None, bone=""):
        self.message = message
        self.obj = obj
        self.bone = bone


def file_path(skin_path):
    return os.path.splitext(skin_path)[0] + ".drivers.json"


def driven_count(objects):
    """How many shape keys of these objects have drivers, and on how many objects."""
    keys, meshes = 0, 0
    for obj in objects:
        key = obj.data.shape_keys if obj.type == "MESH" else None
        drivers = key.animation_data.drivers if key and key.animation_data else ()
        found = sum(1 for d in drivers if KEY_VALUE.match(d.data_path))
        keys += found
        meshes += found > 0
    return keys, meshes


def driven_keys(obj):
    """The names of an object's shape keys that have drivers."""
    key = obj.data.shape_keys if obj.type == "MESH" else None
    if key is None or key.animation_data is None:
        return set()
    return {m.group(1) for d in key.animation_data.drivers if (m := KEY_VALUE.match(d.data_path))}


def collect(suite, armature, objects):
    """The drivers file's contents for these objects, None without a driver the app can run,
    and what can't be carried over."""
    found = _Found(suite, armature, objects)
    for obj in objects:
        key = obj.data.shape_keys if obj.type == "MESH" else None
        if key is None or key.animation_data is None:
            continue
        for fcurve in key.animation_data.drivers:
            match = KEY_VALUE.match(fcurve.data_path)
            if match is None:
                found.problem(f"{obj.name}: the app only runs drivers on shape key values, not on {fcurve.data_path}", obj)
                continue
            entry = found.entry(obj, key, match.group(1), fcurve)
            if entry is not None:
                found.entries.append(entry)
    if not found.entries:
        return None, found.problems
    data = {
        "version": VERSION,
        "drivers": found.entries,
        "stand_ins": found.stand_ins,
        "bones": found.bones,
    }
    return data, found.problems


def write(suite, entry, skin_path):
    """Writes the skin's drivers file, or removes a stale one. Returns (written, notes)."""
    objects = [o for o in entry.collection.all_objects if o.type == "MESH"]
    data, problems = collect(suite, suite.armature, objects)
    path = file_path(skin_path)
    notes = [p.message for p in problems]
    if data is None:
        if os.path.exists(path):
            os.remove(path)
        return [], notes
    _blender_rest_values(suite.armature, data["drivers"])
    with open(path, "w", encoding="utf-8") as file:
        json.dump(data, file, indent=1)
    return [path], notes


# What each shape key comes to in the rest pose by Blender's own driver evaluation, which the
# app checks itself against. The armature shows its rest pose only while this reads it.
def _blender_rest_values(armature, entries):
    data = armature.data
    before = data.pose_position
    data.pose_position = "REST"
    try:
        bpy.context.view_layer.update()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        for entry in entries:
            obj = bpy.data.objects.get(entry["object"])
            if obj is None or "rest" not in entry:
                continue
            try:
                key = obj.data.shape_keys.evaluated_get(depsgraph)
                entry["rest"]["value"] = key.key_blocks[entry["key"]].value
            except (AttributeError, KeyError, RuntimeError):
                pass
    finally:
        data.pose_position = before
        bpy.context.view_layer.update()


class _Found:
    def __init__(self, suite, armature, objects):
        self.suite = suite
        self.armature = armature
        self.objects = objects
        self.entries = []
        self.stand_ins = {}
        self.bones = {}
        self.problems = []
        self._checked = set()

    def problem(self, message, obj=None, bone=""):
        self.problems.append(Problem(message, obj, bone))

    def entry(self, obj, key, name, fcurve):
        driver = fcurve.driver
        where = f"{obj.name}: {name}"
        block = key.key_blocks.get(name)
        if block is None or fcurve.mute or not driver.is_valid:
            self.problem(f"{where}: its driver is muted or broken in Blender, so the app leaves it out", obj)
            return None
        kind = KINDS.get(driver.type)
        if kind is None:
            self.problem(f"{where}: the app can't run a {driver.type.lower()} driver", obj)
            return None
        if kind == "scripted" and not driver.is_simple_expression:
            self.problem(
                f"{where}: its expression needs Python, which the app can't run; keep to what "
                "Blender runs without it (no 'Slow Python expression' warning)",
                obj,
            )
            return None
        variables = []
        for var in driver.variables:
            variable = self._variable(obj, where, var)
            if variable is None:
                return None
            variables.append(variable)
        entry = {
            "object": obj.name,
            "key": name,
            "kind": kind,
            "expression": driver.expression if kind == "scripted" else "",
            "variables": variables,
            "curve": _curve(fcurve),
            "range": [block.slider_min, block.slider_max],
        }
        if len(fcurve.modifiers) and len(fcurve.keyframe_points) < 2:
            self.problem(f"{where}: its curve has modifiers but under two keys; the app follows it only near its keys", obj)
        rest = self._rest(driver, kind, fcurve, entry)
        if rest is not None:
            entry["rest"] = rest
        return entry

    def _unsupported(self, obj, where, var, why):
        self.problem(f"{where}: variable {var.name} {why}", obj)
        return None

    def _variable(self, obj, where, var):
        bones = self.armature.data.bones
        targets = list(var.targets)

        def bone_of(target):
            if target.id == self.armature and target.bone_target in bones:
                return bones[target.bone_target]
            return None

        if var.type == "ROTATION_DIFF":
            found = [bone_of(t) for t in targets[:2]]
            if None in found:
                return self._unsupported(obj, where, var, f"reads something other than two bones of {self.armature.name}")
            return {"name": var.name, "type": "rotation_difference", "bones": [self._bone(b) for b in found]}

        if var.type == "TRANSFORM_CHANNEL":
            target = targets[0]
            bone = bone_of(target)
            space = SPACES.get(target.transform_space)
            if bone is None:
                return self._unsupported(obj, where, var, f"reads something other than a bone of {self.armature.name}")
            if space is None:
                return self._unsupported(obj, where, var, f"reads {target.transform_space.replace('_', ' ').lower()}, which the app can't")
            return {
                "name": var.name,
                "type": "transform_channel",
                "bone": self._bone(bone, axes=True),
                "channel": target.transform_type.lower(),
                "space": space,
                "rotation": self._rotation(bone, target.rotation_mode),
            }

        if var.type == "LOC_DIFF":
            found = [bone_of(t) for t in targets[:2]]
            spaces = [SPACES.get(t.transform_space) for t in targets[:2]]
            if None in found:
                return self._unsupported(obj, where, var, f"measures from something other than bones of {self.armature.name}")
            if None in spaces:
                return self._unsupported(obj, where, var, "measures in a space the app can't")
            return {
                "name": var.name,
                "type": "distance",
                "bones": [self._bone(b, axes=True) for b in found],
                "spaces": spaces,
            }

        if var.type == "SINGLE_PROP":
            return self._single_property(obj, where, var, targets[0])

        label = var.type.replace("_", " ").title()
        return self._unsupported(obj, where, var, f"is a {label}, which the app can't read")

    # A bone's own location, rotation or scale reads as a Transform Channel in local space; a
    # shape key's value reads that shape key.
    def _single_property(self, obj, where, var, target):
        path = target.data_path
        if target.id_type == "KEY" and KEY_VALUE.match(path):
            name = KEY_VALUE.match(path).group(1)
            owner = next((o for o in self.objects if o.data.shape_keys == target.id), None)
            if owner is None:
                return self._unsupported(obj, where, var, f"reads a shape key of a mesh outside this skin ({name})")
            return {"name": var.name, "type": "shape_key", "object": owner.name, "key": name}
        match = BONE_CHANNEL.match(path)
        bones = self.armature.data.bones
        if target.id != self.armature or match is None or match.group(1) not in bones:
            return self._unsupported(
                obj, where, var,
                f"reads {path or 'nothing'}; the app only reads a bone's location, rotation or scale, or a shape key",
            )
        bone, prop, index = bones[match.group(1)], match.group(2), int(match.group(3))
        pose_bone = self.armature.pose.bones[bone.name]
        if prop == "rotation_euler" and pose_bone.rotation_mode not in EULERS:
            return self._unsupported(obj, where, var, f"reads {bone.name}'s Euler rotation, which it doesn't use")
        channel = {
            "location": "loc_" + "xyz"[min(index, 2)],
            "scale": "scale_" + "xyz"[min(index, 2)],
            "rotation_euler": "rot_" + "xyz"[min(index, 2)],
            "rotation_quaternion": "rot_" + "wxyz"[min(index, 3)],
        }[prop]
        rotation = "quaternion" if prop == "rotation_quaternion" else self._rotation(bone, "AUTO")
        return {
            "name": var.name,
            "type": "transform_channel",
            "bone": self._bone(bone, axes=True),
            "channel": channel,
            "space": "local",
            "rotation": rotation,
        }

    # Auto means the bone's own Euler order, or XYZ for a bone that turns some other way.
    def _rotation(self, bone, mode):
        if mode in ROTATIONS:
            return ROTATIONS[mode]
        if mode in EULERS:
            return mode.lower()
        own = self.armature.pose.bones[bone.name].rotation_mode
        return own.lower() if own in EULERS else "xyz"

    def _exported(self, bone):
        return not self.suite.gltf.deform_bones_only or bone.use_deform

    def _world(self):
        return self.armature.matrix_world.to_3x3().normalized().to_quaternion()

    def _bone(self, bone, axes=False):
        world = self._world()
        if axes and bone.name not in self.bones:
            rest = world @ bone.matrix_local.to_quaternion()
            self.bones[bone.name] = {"rest": [rest.x, rest.y, rest.z, rest.w]}
        if self._exported(bone) or bone.name in self.stand_ins:
            return bone.name
        between = [bone]
        anchor = bone.parent
        while anchor is not None and not self._exported(anchor):
            between.append(anchor)
            anchor = anchor.parent
        anchor_rest = world @ anchor.matrix_local.to_quaternion() if anchor else world
        turn = (world @ bone.matrix_local.to_quaternion()) @ anchor_rest.inverted()
        head = self.armature.matrix_world @ bone.head_local
        self.stand_ins[bone.name] = {
            "parent": anchor.name if anchor else self.armature.name,
            "turn": [turn.x, turn.z, -turn.y, turn.w],
            "head": [head.x, head.z, -head.y],
        }
        for still in between:
            self._check_still(still)
        return bone.name

    def _check_still(self, bone):
        if bone.name in self._checked:
            return
        self._checked.add(bone.name)
        pose = self.armature.pose.bones.get(bone.name)
        if pose and any(c.enabled and c.influence > 0.0 for c in pose.constraints):
            self.problem(
                f"{bone.name} isn't exported and has constraints, so the app can't follow it; "
                "the drivers reading it get it wrong",
                self.armature,
                bone.name,
            )
        path = f'pose.bones["{bone.name}"].'
        for entry in self.suite.animations:
            for clip in entry.clips:
                if clip.enabled and clip.action and any(
                    c.data_path.startswith(path) and _moves(c) for c in anim.fcurves(clip.action)
                ):
                    self.problem(
                        f"{bone.name} isn't exported but clip {clip.name or clip.action.name!r} "
                        "moves it, so the drivers reading it get it wrong",
                        self.armature,
                        bone.name,
                    )
                    return

    # What the driver's variables read in the rest pose, from the real bones, for the app to
    # compare. The value itself comes from Blender's own evaluation once the file is written;
    # this one, worked out here, stands in where that fails.
    def _rest(self, driver, kind, fcurve, entry):
        values = [self._rest_variable(v) for v in entry["variables"]]
        if None in values:
            return {"variables": values, "value": 0.0}
        if kind == "scripted":
            names = {v["name"]: value for v, value in zip(entry["variables"], values)}
            try:
                raw = float(eval(driver.expression, {"__builtins__": {}}, {**_FUNCTIONS, **names}))
            except Exception:
                return {"variables": values, "value": 0.0}
        elif not values:
            raw = 0.0
        else:
            raw = {"average": sum(values) / len(values), "sum": sum(values),
                   "min": min(values), "max": max(values)}[kind]
        value = fcurve.evaluate(raw) if entry["curve"] is not None else raw
        low, high = entry["range"]
        return {"variables": values, "value": min(max(value, min(low, high)), max(low, high))}

    def _rest_variable(self, variable):
        bones = self.armature.data.bones
        kind = variable["type"]
        if kind == "rotation_difference":
            first, second = (bones[b].matrix_local.to_quaternion() for b in variable["bones"])
            return _rotation_difference(first, second)
        if kind == "transform_channel":
            matrix = self._rest_matrix(bones[variable["bone"]], variable["space"])
            return _channel(matrix, variable["channel"], variable["rotation"])
        if kind == "distance":
            heads = [self._rest_matrix(bones[b], s).translation for b, s in zip(variable["bones"], variable["spaces"])]
            return (heads[0] - heads[1]).length
        return None

    def _rest_matrix(self, bone, space):
        from mathutils import Matrix

        if space == "local":
            return Matrix.Identity(4)
        return self.armature.matrix_world @ bone.matrix_local


# Rest values of a bone's transform channels, by property and index.
_REST = {"location": 0.0, "rotation_euler": 0.0, "scale": 1.0, "rotation_quaternion": (1.0, 0.0, 0.0, 0.0),
         "rotation_axis_angle": (0.0, 0.0, 1.0, 0.0)}


# Whether a curve takes its bone away from the rest pose. Keying every bone keys helper bones
# too, at rest, which moves nothing.
def _moves(fcurve):
    prop = fcurve.data_path.rsplit(".", 1)[-1]
    rest = _REST.get(prop)
    if rest is None:
        return True
    if isinstance(rest, tuple):
        rest = rest[min(fcurve.array_index, len(rest) - 1)]
    for point in fcurve.keyframe_points:
        values = (point.co.y, point.handle_left.y, point.handle_right.y)
        if any(abs(v - rest) > 1e-4 for v in values):
            return True
    return len(fcurve.modifiers) > 0


# As Blender computes Rotational Difference (dvar_eval_rotDiff): the angle of the turn from one
# to the other, folded into 0..pi.
def _rotation_difference(a, b):
    between = a.inverted() @ b
    between.normalize()
    angle = abs(2.0 * math.acos(max(-1.0, min(1.0, between.w))))
    return 2.0 * math.pi - angle if angle > math.pi else angle


# As Blender reads a Transform Channel off a matrix (dvar_eval_transChan and
# BKE_driver_target_matrix_to_rot_channels).
def _channel(matrix, channel, rotation):
    axis = channel[-1]
    if channel.startswith("loc_"):
        return matrix.translation["xyz".index(axis)]
    if channel == "scale_avg":
        volume = matrix.to_3x3().determinant()
        return math.copysign(abs(volume) ** (1.0 / 3.0), volume)
    if channel.startswith("scale_"):
        return matrix.col["xyz".index(axis)].to_3d().length
    index = "wxyz".index(axis)
    turn = matrix.to_3x3().normalized()
    if rotation == "quaternion":
        return turn.to_quaternion()[index]
    if rotation.startswith("swing_twist_"):
        twist_axis = rotation[-1].upper()
        swing, twist = turn.to_quaternion().to_swing_twist(twist_axis)
        if index == "XYZ".index(twist_axis) + 1:
            return twist
        value = max(-1.0, min(1.0, swing[index]))
        return 2.0 * math.acos(value) if index == 0 else 2.0 * math.asin(value)
    if index == 0:
        return 0.0
    return turn.to_euler(rotation.upper())[index - 1]


def _curve(fcurve):
    points = fcurve.keyframe_points
    if not len(points) and not len(fcurve.modifiers):
        return None
    if len(points):
        start = min(p.co.x for p in points)
        end = max(p.co.x for p in points)
    else:
        start, end = 0.0, 1.0
    count = CURVE_SAMPLES + 1 if end > start else 1
    values = [fcurve.evaluate(start + (end - start) * i / max(count - 1, 1)) for i in range(count)]
    step = max((end - start) * 1e-3, 1e-4)
    return {
        "from": start,
        "to": end,
        "values": values,
        "slope_before": (values[0] - fcurve.evaluate(start - step)) / step,
        "slope_after": (fcurve.evaluate(end + step) - values[-1]) / step,
    }


def _clamp(x, low=0.0, high=1.0):
    return max(low, min(high, x))


def _smoothstep(edge0, edge1, x):
    t = _clamp((x - edge0) / (edge1 - edge0))
    return t * t * (3.0 - 2.0 * t)


# What simple expressions may call, for working out the rest values here in Python.
_FUNCTIONS = {
    **{name: getattr(math, name) for name in (
        "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "log", "log10", "log2",
        "sqrt", "pow", "hypot", "floor", "ceil", "trunc", "fabs", "fmod", "radians", "degrees",
    )},
    "abs": abs,
    "min": min,
    "max": max,
    "round": round,
    "int": int,
    "clamp": _clamp,
    "lerp": lambda a, b, t: a + (b - a) * t,
    "smoothstep": _smoothstep,
    "pi": math.pi,
    "e": math.e,
    "True": True,
    "False": False,
}
