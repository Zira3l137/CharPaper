# Drivers on a skin's shape keys, written next to the skin as <skin>.drivers.json for the app to
# run. glTF has no place for drivers, and running them in the app rather than baking them into
# clips makes them follow whatever moves the bones: clips, blends between them, and gaze.
#
# Only what the app can run is written: Blender's simple expressions, and the variable types
# below. A driver reading a bone the export leaves out, like a pose reader's helper bone, reads
# a stand-in: the nearest exported bone above it, turned the way the left-out bone sits from it
# in the rest pose. That holds while nothing in between moves on its own, which is checked.
#
# Rotations are written in glTF's axes, as the model is exported: Blender's world with +Y up,
# so (x, y, z) becomes (x, z, -y).

import json
import math
import os
import re

from . import anim

VERSION = 1
KINDS = {"SCRIPTED": "scripted", "AVERAGE": "average", "SUM": "sum", "MIN": "min", "MAX": "max"}
KEY_VALUE = re.compile(r'^key_blocks\["(.+)"\]\.value$')
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


def collect(suite, armature, objects):
    """The drivers file's contents for these objects, None without a driver the app can run,
    and what can't be carried over."""
    found = _Found(suite, armature)
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
    data = {"version": VERSION, "drivers": found.entries, "stand_ins": found.stand_ins}
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
    with open(path, "w", encoding="utf-8") as file:
        json.dump(data, file, indent=1)
    return [path], notes


class _Found:
    def __init__(self, suite, armature):
        self.suite = suite
        self.armature = armature
        self.entries = []
        self.stand_ins = {}
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

    def _variable(self, obj, where, var):
        if var.type != "ROTATION_DIFF":
            label = var.type.replace("_", " ").title()
            self.problem(f"{where}: variable {var.name} is a {label}; the app only reads Rotational Difference so far", obj)
            return None
        bones = []
        for target in var.targets[:2]:
            if target.id != self.armature or target.bone_target not in self.armature.data.bones:
                self.problem(f"{where}: variable {var.name} reads something other than a bone of {self.armature.name}", obj)
                return None
            bones.append(self._bone(self.armature.data.bones[target.bone_target]))
        return {"name": var.name, "type": "rotation_difference", "bones": bones}

    def _exported(self, bone):
        return not self.suite.gltf.deform_bones_only or bone.use_deform

    def _bone(self, bone):
        if self._exported(bone) or bone.name in self.stand_ins:
            return bone.name
        between = [bone]
        anchor = bone.parent
        while anchor is not None and not self._exported(anchor):
            between.append(anchor)
            anchor = anchor.parent
        world = self.armature.matrix_world.to_3x3().normalized().to_quaternion()
        anchor_rest = world @ anchor.matrix_local.to_quaternion() if anchor else world
        turn = (world @ bone.matrix_local.to_quaternion()) @ anchor_rest.inverted()
        self.stand_ins[bone.name] = {
            "parent": anchor.name if anchor else self.armature.name,
            "turn": [turn.x, turn.z, -turn.y, turn.w],
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
                    c.data_path.startswith(path) for c in anim.fcurves(clip.action)
                ):
                    self.problem(
                        f"{bone.name} isn't exported but clip {clip.name or clip.action.name!r} "
                        "moves it, so the drivers reading it get it wrong",
                        self.armature,
                        bone.name,
                    )
                    return

    # What the driver gives in the rest pose, from the real bones, for the app to compare.
    def _rest(self, driver, kind, fcurve, entry):
        bones = self.armature.data.bones
        values = []
        for var in driver.variables:
            first, second = (bones[t.bone_target].matrix_local.to_quaternion() for t in var.targets[:2])
            values.append(_rotation_difference(first, second))
        if kind == "scripted":
            names = {v.name: value for v, value in zip(driver.variables, values)}
            try:
                raw = float(eval(driver.expression, {"__builtins__": {}}, {**_FUNCTIONS, **names}))
            except Exception:
                return None
        elif not values:
            raw = 0.0
        else:
            raw = {"average": sum(values) / len(values), "sum": sum(values), "min": min(values), "max": max(values)}[kind]
        value = fcurve.evaluate(raw) if entry["curve"] is not None else raw
        low, high = entry["range"]
        return {"variables": values, "value": min(max(value, min(low, high)), max(low, high))}


# As Blender computes Rotational Difference (dvar_eval_rotDiff): the angle of the turn from one
# to the other, folded into 0..pi.
def _rotation_difference(a, b):
    between = a.inverted() @ b
    between.normalize()
    angle = abs(2.0 * math.acos(max(-1.0, min(1.0, between.w))))
    return 2.0 * math.pi - angle if angle > math.pi else angle


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
