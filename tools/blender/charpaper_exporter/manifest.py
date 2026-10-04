import os
import shutil
from pathlib import PurePosixPath

import bpy

from . import toml_io
from .props import file_stem

FILE = "suite.toml"
MODES = {"LOOP": "loop", "ONCE": "once", "POSE": "pose"}


def path(folder):
    return os.path.join(folder, FILE)


def _same_file(a, b):
    # A .glb and a .gltf of the same name are one file exported in another format.
    a, b = PurePosixPath(a.replace("\\", "/")), PurePosixPath(b.replace("\\", "/"))
    gltf = {".glb", ".gltf"}
    return a.with_suffix("") == b.with_suffix("") and a.suffix.lower() in gltf and b.suffix.lower() in gltf


def _set_or_keep(table, key, value):
    # An empty field in the panel means "not managed here", so a value typed into
    # suite.toml by hand survives.
    if value:
        table[key] = value


def gaze_table(gaze):
    table = {"eyes": [e.bone for e in gaze.eyes if e.bone]}
    if gaze.head:
        table["head"] = gaze.head
    if gaze.neck:
        table["neck"] = gaze.neck
    for key, attr in (
        ("eye_yaw_deg", "eye_yaw"),
        ("eye_pitch_deg", "eye_pitch"),
        ("head_yaw_deg", "head_yaw"),
        ("head_pitch_deg", "head_pitch"),
        ("neck_yaw_deg", "neck_yaw"),
        ("neck_pitch_deg", "neck_pitch"),
    ):
        table[key] = float(getattr(gaze, attr))
    if gaze.head and gaze.neck:
        table["head_share"] = float(gaze.head_share)
    keys = {
        side: getattr(gaze, "key_" + side)
        for side in ("left", "right", "up", "down")
        if getattr(gaze, "key_" + side)
    }
    if keys:
        table["shape_keys"] = keys
    return table


def update(data, suite, extension, exported_animations, model_exported):
    data = dict(data)
    data.setdefault("schema", 1)
    _set_or_keep(data, "name", suite.suite_name)

    character = dict(data.get("character", {}))
    if model_exported:
        character["model"] = file_stem(suite.model_name) + extension
    _set_or_keep(character, "default_skin", suite.default_skin)
    _set_or_keep(character, "default_animation", suite.default_animation)
    if character:
        data["character"] = character

    files = {f"animations/{file_stem(e.name)}{extension}": e for e in exported_animations}
    animations = {
        name: entry
        for name, entry in data.get("animations", {}).items()
        if not any(_same_file(entry.get("file", ""), f) for f in files)
    }
    for file, entry in files.items():
        for clip in entry.clips:
            if not (clip.enabled and clip.action):
                continue
            item = {"file": file, "clip": clip.action.name}
            if clip.mode != "LOOP":
                item["mode"] = MODES[clip.mode]
            if not clip.gaze:
                item["gaze"] = False
            animations[clip.name or clip.action.name] = item
    if animations:
        data["animations"] = animations

    if suite.default_environment:
        data["environment"] = {**data.get("environment", {}), "default": suite.default_environment}
    if suite.default_camera:
        data["camera"] = {**data.get("camera", {}), "default": suite.default_camera}

    if suite.gaze.mode == "WRITE":
        data["gaze"] = gaze_table(suite.gaze)
    elif suite.gaze.mode == "REMOVE":
        data.pop("gaze", None)
    return data


def write(folder, data):
    """Writes suite.toml, keeping the previous one as suite.toml.bak. Returns whether it changed."""
    target = path(folder)
    text = toml_io.dumps(data)
    if os.path.isfile(target):
        with open(target, encoding="utf-8") as file:
            if file.read() == text:
                return False
        shutil.copyfile(target, target + ".bak")
    with open(target, "w", encoding="utf-8", newline="\n") as file:
        file.write(text)
    return True


def load_into(suite, data):
    """Fills the panel from suite.toml. Returns the clip entries that matched nothing here."""
    suite.suite_name = data.get("name", suite.suite_name)
    character = data.get("character", {})
    model = character.get("model")
    if model and "/" not in model.replace("\\", "/"):
        suite.model_name = PurePosixPath(model).stem
    suite.default_skin = character.get("default_skin", suite.default_skin)
    suite.default_animation = character.get("default_animation", suite.default_animation)
    suite.default_environment = data.get("environment", {}).get("default", suite.default_environment)
    suite.default_camera = data.get("camera", {}).get("default", suite.default_camera)

    unmatched = []
    for name, entry in data.get("animations", {}).items():
        file = PurePosixPath(entry.get("file", "").replace("\\", "/"))
        action = bpy.data.actions.get(entry.get("clip") or name)
        if file.parent != PurePosixPath("animations") or action is None:
            unmatched.append(name)
            continue
        group = next((a for a in suite.animations if file_stem(a.name) == file.stem), None)
        if group is None:
            group = suite.animations.add()
            group.name = file.stem
        clip = next((c for c in group.clips if c.action == action), None)
        if clip is None:
            clip = group.clips.add()
            clip.action = action
        clip.name = name
        clip.enabled = True
        clip.mode = {v: k for k, v in MODES.items()}.get(entry.get("mode", "loop"), "LOOP")
        clip.gaze = entry.get("gaze", True)

    gaze = data.get("gaze")
    if gaze is not None:
        g = suite.gaze
        g.mode = "WRITE"
        g.eyes.clear()
        for bone in gaze.get("eyes", []):
            g.eyes.add().bone = bone
        g.head = gaze.get("head", "")
        g.neck = gaze.get("neck", "")
        g.eye_yaw = gaze.get("eye_yaw_deg", 30.0)
        g.eye_pitch = gaze.get("eye_pitch_deg", 20.0)
        g.head_yaw = gaze.get("head_yaw_deg", 50.0)
        g.head_pitch = gaze.get("head_pitch_deg", 25.0)
        g.neck_yaw = gaze.get("neck_yaw_deg", 25.0)
        g.neck_pitch = gaze.get("neck_pitch_deg", 15.0)
        g.head_share = gaze.get("head_share", 0.7)
        keys = gaze.get("shape_keys", {})
        for side in ("left", "right", "up", "down"):
            setattr(g, "key_" + side, keys.get(side, ""))
    return unmatched
