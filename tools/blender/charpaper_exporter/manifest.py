import os
import shutil
from pathlib import PurePosixPath

import bpy
from mathutils import Vector

from . import env_settings
from . import front
from . import gaze as gaze_bones
from . import toml_io
from .actors import start_clip
from .props import file_stem

FILE = "suite.toml"


def path(folder):
    return os.path.join(folder, FILE)


def _set_or_keep(table, key, value):
    # An empty field in the panel means "not managed here", so a value typed into
    # suite.toml by hand survives.
    if value:
        table[key] = value


def gaze_table(gaze, armature):
    table = {}
    eyes = [e.bone for e in gaze.eyes if e.bone]
    if eyes:
        table["eyes"] = eyes
        if gaze.eye_kind == "DRAWN":
            table["eye_kind"] = "drawn"
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
    keys = gaze_bones.eye_keys(gaze)
    if eyes or keys:
        table["head_turn"] = float(gaze.head_turn)
    # Only a front picked by hand is written; left out, the app works it out as the panel
    # does. As the model is exported: the armature turned as it stands in the scene.
    facing = front.world_front(armature, gaze.front) if armature else None
    if facing is not None:
        table["front"] = [round(v, 4) + 0.0 for v in facing]
    if keys:
        table["shape_keys"] = keys
    return table


SCHEMA = 2
MODES = {"ONCE": "once", "HOLD": "hold"}


def clip_table(clip, cues, previous, character):
    """A clip's entry: only what differs from the app's defaults."""
    table = {}
    if clip.mode in MODES:
        table["mode"] = MODES[clip.mode]
    if clip.label:
        table["label"] = clip.label
    if character and not clip.gaze:
        table["gaze"] = False
    if character and clip.reaction_only:
        table["reaction_only"] = True
    # Pose markers decide the cues; an action without any keeps those typed by hand.
    if cues:
        table["cues"] = [{"at": at, "sound": sound} for at, sound in cues]
    elif not cues and "cues" in previous:
        table["cues"] = previous["cues"]
    return table


def clips_tables(clips, cue_tables, previous, character):
    tables = {}
    for clip in clips:
        table = clip_table(clip, cue_tables.get(clip.name), previous.get(clip.name, {}), character)
        if table:
            tables[clip.name] = table
    return tables


def update(data, suite, scene, extension, model_exported, lenses, environments, exported, hotspots=None, physics=None):
    """`exported` maps (environment name or None, actor name) to the actor's exported clips
    and their cue tables; None is the character. What the add-on exports it owns: an exported
    actor's clips table is rewritten whole."""
    data = {k: v for k, v in data.items() if k != "animations"}
    data["schema"] = SCHEMA
    _set_or_keep(data, "name", suite.suite_name)
    data["timeline"] = {
        "start": scene.frame_start,
        "end": scene.frame_end,
        "fps": round(scene.render.fps / scene.render.fps_base, 6),
    }

    character = {k: v for k, v in data.get("character", {}).items() if k != "default_animation"}
    if model_exported:
        character["model"] = file_stem(suite.model_name) + extension
    _set_or_keep(character, "default_skin", suite.default_skin)
    _set_or_keep(character, "default_clip", suite.default_clip)
    for (environment, _), (_, clips, cue_tables) in exported.items():
        if environment is None:
            previous = character.get("clips", {})
            tables = clips_tables(clips, cue_tables, previous, character=True)
            character.pop("clips", None)
            if tables:
                character["clips"] = tables
    labels = {
        file_stem(e.action.name): {"label": e.label} for e in suite.expressions if e.action and e.label
    }
    character.pop("expressions", None)
    if labels:
        character["expressions"] = labels
    if character:
        data["character"] = character

    if suite.default_environment:
        data["environment"] = {**data.get("environment", {}), "default": suite.default_environment}
    if suite.default_camera:
        data["camera"] = {**data.get("camera", {}), "default": suite.default_camera}

    tables = dict(data.get("environments", {}))
    for name, exported_env in environments.items():
        # The sky's keys belong to the export only when it wrote the sky; None takes the key out.
        owned = set(env_settings.KEYS) | set(exported_env) | {"actors"}
        table = {k: v for k, v in tables.get(name, {}).items() if k not in owned}
        table.update({k: v for k, v in exported_env.items() if v is not None and v != [] and v != {}})
        # Light animation moved into lamps' clips.
        for light in table.get("lights", {}).values():
            light.pop("animation", None)
        tables[name] = table
    for (environment, actor), (entry, clips, cue_tables) in exported.items():
        if environment is None:
            continue
        # As the app finds them: by their folders' names.
        name, actor = file_stem(environment), file_stem(actor)
        table = tables.setdefault(name, {})
        actors = dict(table.get("actors", {}))
        previous = actors.get(actor, {}).get("clips", {})
        clips_of = clips_tables(clips, cue_tables, previous, character=False)
        actors.pop(actor, None)
        own = start_clip(entry, clips)
        actor_table = {"default_clip": own.name} if own is not None else {}
        if clips_of:
            actor_table["clips"] = clips_of
        if actor_table:
            actors[actor] = actor_table
        table.pop("actors", None)
        if actors:
            table["actors"] = actors
    tables = {name: table for name, table in tables.items() if table}
    if tables:
        data["environments"] = tables
    else:
        data.pop("environments", None)

    # An exported camera's table comes whole from its Depth of Field panel.
    cameras = dict(data.get("cameras", {}))
    for name, table in lenses.items():
        if table:
            cameras[name] = table
        else:
            cameras.pop(name, None)
    if cameras:
        data["cameras"] = cameras
    else:
        data.pop("cameras", None)

    if suite.gaze.mode == "WRITE":
        data["gaze"] = gaze_table(suite.gaze, suite.armature)
    elif suite.gaze.mode == "REMOVE":
        data.pop("gaze", None)

    # Hotspots typed into suite.toml stay until the panel has had its own.
    if suite.hotspots_owned:
        data.pop("hotspots", None)
        if hotspots:
            data["hotspots"] = hotspots
    if suite.physics_owned:
        data.pop("physics", None)
        if physics:
            data["physics"] = physics
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
    """Fills the panel from suite.toml. Returns the clips it names that match no action here."""
    suite.suite_name = data.get("name", suite.suite_name)
    character = data.get("character", {})
    model = character.get("model")
    if model and "/" not in model.replace("\\", "/"):
        suite.model_name = PurePosixPath(model).stem
    suite.default_skin = character.get("default_skin", suite.default_skin)
    suite.default_clip = character.get("default_clip", suite.default_clip)
    suite.default_environment = data.get("environment", {}).get("default", suite.default_environment)
    suite.default_camera = data.get("camera", {}).get("default", suite.default_camera)

    unmatched = []
    if suite.armature is not None:
        unmatched += _load_clips(suite.armature, character.get("clips", {}), character=True)
    by_stem = {file_stem(e.name): e for e in suite.environments}
    for env_name, table in data.get("environments", {}).items():
        entry = by_stem.get(env_name)
        for actor, actor_table in table.get("actors", {}).items():
            obj = bpy.data.objects.get(actor)
            if entry is None or obj is None:
                unmatched += [f"{env_name}/{actor}/{c}" for c in actor_table.get("clips", {})]
                continue
            unmatched += _load_clips(obj, actor_table.get("clips", {}), character=False)
            obj.charpaper.start_clip = actor_table.get("default_clip", "")
    for name, table in character.get("expressions", {}).items():
        expression = next((e for e in suite.expressions if e.action and file_stem(e.action.name) == name), None)
        if expression is None:
            action = next((a for a in bpy.data.actions if file_stem(a.name) == name), None)
            if action is None:
                unmatched.append(name)
                continue
            expression = suite.expressions.add()
            expression.action = action
        expression.label = table.get("label", "")

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
        g.head_turn = gaze.get("head_turn", 0.6)
        g.eye_kind = "DRAWN" if gaze.get("eye_kind") == "drawn" else "BALL"
        facing = gaze.get("front")
        if facing is not None and suite.armature is not None and len(facing) == 3:
            g.front = front.nearest_axis(suite.armature, Vector(facing))
        else:
            g.front = "NONE"
        keys = gaze.get("shape_keys", {})
        for side in ("left", "right", "up", "down"):
            setattr(g, "key_" + side, keys.get(side, ""))
    return unmatched


MODES_BACK = {"timeline": "TIMELINE", "once": "ONCE", "hold": "HOLD"}


def _load_clips(obj, tables, character):
    """Settings for the clips suite.toml names, onto the object's list, by action name."""
    unmatched = []
    for name, table in tables.items():
        action = next((a for a in bpy.data.actions if file_stem(a.name) == name), None)
        if action is None:
            unmatched.append(name)
            continue
        clip = next((c for c in obj.charpaper.clips if c.action == action), None)
        if clip is None:
            clip = obj.charpaper.clips.add()
            clip.action = action
        if character:
            clip.gaze = table.get("gaze", True)
            clip.reaction_only = table.get("reaction_only", False)
        # After Only for hotspots, which turns a Timeline clip into a Once one as it's ticked.
        clip.mode = MODES_BACK.get(table.get("mode", "timeline"), "TIMELINE")
        clip.label = table.get("label", "")
    return unmatched
