import os

from . import actors
from . import anim
from . import animated
from . import clips
from . import drivers
from . import gltf
from . import lens
from . import lights
from . import sky
from .isolate import isolated
from .isolate import showing
from .props import file_stem

SKINS_DIR = "skins"
CAMERAS_DIR = "cameras"
ENVIRONMENT_DIR = "environment"


class Result:
    def __init__(self, path):
        self.path = path
        self.skipped = []
        self.notes = []
        # Files written besides the one at `path`.
        self.written = []
        # suite.toml values the export decided, None for the app's default.
        self.settings = {}
        # Notes that change how the result looks in the app, shown in a popup to be dismissed.
        self.alerts = []


def extension(suite):
    return gltf.EXTENSIONS[suite.gltf.file_format]


def target(files, subdir, name, suite, ext=None):
    stem = f"{subdir}/{file_stem(name)}" if subdir else file_stem(name)
    ext = ext or extension(suite)
    # Nobody can delete a file in a pack by hand, so the export does it.
    for other in gltf.EXTENSIONS.values():
        if files.packed and other != ext and files.isfile(stem + other):
            files.remove(stem + other)
    path = files.path(stem) + ext
    os.makedirs(os.path.dirname(path), exist_ok=True)
    return path


def _options(suite, **overrides):
    options = dict(gltf.settings_options(suite.gltf))
    options.update(gltf.LOCKED)
    options.update(overrides)
    return options


def _run(context, isolation, filepath, options):
    isolation.finish()
    result = Result(filepath)
    with showing(context, isolation.scene):
        result.skipped = gltf.export(filepath, options)
    return result


def _shape_keyed(obj):
    return obj.type == "MESH" and obj.data.shape_keys is not None


def _new_data(obj, objects):
    # Shape keys belong to the mesh, so objects sharing one get a single track between them.
    return all(other.data != obj.data for other in objects)


def model(context, suite, files):
    path = target(files, "", suite.model_name, suite)
    with isolated(context) as iso:
        iso.add_with_ancestors(suite.armature)
        return _run(
            context, iso, path,
            _options(suite, export_animations=False, export_lights=False, export_cameras=False),
        )


class Alert:
    def __init__(self, message, object_name="", fix=""):
        self.message = message
        self.object_name = object_name
        self.fix = fix


# Marks a warning one button fixes: Stop it, Unlink it.
STOP_SHAPE_KEYS = "SHAPE_KEYS"
UNLINK_ACTION = "OBJECT_ACTION"


def shape_keys_playing(obj, frame):
    """What plays on the mesh's shape keys at `frame`, said so the artist finds it, or None."""
    found = anim.playing_on_shape_keys(obj, frame)
    if found is None:
        return None
    action, track = found
    if track is None:
        return f"the action {action.name} is assigned to its shape keys", "Unassign it in the Shape Key Editor"
    return (
        f"its shape keys' NLA track {track!r} plays {action.name}",
        f"Mute that track in the NLA Editor, where it shows under {obj.name}'s Key",
    )


# A skin is exported as its sliders stand, and the app rests every shape key no clip keys at
# its slider. An action playing on a mesh's shape keys has written its values into the
# sliders, so they would export as that action's.
def skin(context, suite, entry, files):
    path = target(files, SKINS_DIR, entry.name, suite)
    alerts = []
    with isolated(context) as iso:
        iso.add_with_ancestors(suite.armature)
        meshes = []
        for obj in entry.collection.all_objects:
            if obj.type in {"MESH", "EMPTY"}:
                playing = shape_keys_playing(obj, context.scene.frame_current)
                if playing is not None and obj.data not in meshes:
                    what, where = playing
                    alerts.append(
                        Alert(
                            f"Skin {entry.name}: on the mesh {obj.name}, {what}, so its sliders were "
                            f"saved as that animation set them and the character rests on them in "
                            f"the app. {where}, or press Stop it, then export the skin again.",
                            obj.name,
                            STOP_SHAPE_KEYS,
                        )
                    )
                copy = iso.add(obj, own_mesh=_shape_keyed(obj))
                if _shape_keyed(copy) and _new_data(copy, meshes):
                    meshes.append(copy)
        result = _run(
            context, iso, path,
            _options(suite, export_animations=False, export_lights=False, export_cameras=False),
        )
    result.alerts.extend(alerts)
    # Read from the originals, once they have their names back.
    written, notes = drivers.write(suite, entry, path, files)
    result.written.extend(written)
    result.notes.extend(notes)
    return result


def environment(context, suite, entry, files):
    result = None
    if entry.collection is not None:
        result = _environment_scene(context, suite, entry, files)
    world = sky.world_of(context.scene, entry)
    if world is None:
        if entry.export_sky:
            note = f"{entry.name}: no World in the scene, so no sky was exported"
            result.notes.append(note)
        return result
    sky_folder = f"{ENVIRONMENT_DIR}/{file_stem(entry.name)}"
    kept = sky.hand_placed(files, sky_folder) if entry.world is None else None
    if kept is not None:
        path, settings = files.path(f"{sky_folder}/{kept}"), {}
        notes = [f"{entry.name}: kept {kept}, which you put there; pick a World to replace it"]
    else:
        path, settings, notes = sky.export(context, world, files, sky_folder)
    if result is None:
        result = Result(path)
    else:
        result.written.append(path)
    result.settings.update(settings)
    result.notes.extend(notes)
    return result


# Everything moves through its clips, so the file holds the scene as it stands. A lamp a clip
# lights is set to what its clips' curves multiply, its brightest Strength and white.
def _environment_scene(context, suite, entry, files):
    path = target(files, ENVIRONMENT_DIR, entry.name, suite)
    threshold = context.scene.eevee.light_threshold
    objects = [obj for obj in entry.collection.all_objects if obj.type != "CAMERA"]
    lamps = {}
    for actor in actors.environment_actors(entry):
        base = clips.light_base(actor, actors.clips(actor), context.scene)
        if base is not None:
            lamps[actor.obj] = base
    with isolated(context) as iso, animated.stills(objects):
        for obj in objects:
            copy = iso.add(obj, own_mesh=_shape_keyed(obj), own_light=True, keep_drivers=True)
            if copy.type == "LIGHT":
                copy.data.animation_data_clear()
                if obj in lamps:
                    peak, white = lamps[obj]
                    copy.data.energy = peak
                    if white:
                        copy.data.color = (1.0, 1.0, 1.0)
                lights.give_range(copy.data, threshold)
        result = _run(
            context, iso, path,
            _options(suite, export_animations=False, export_lights=True, export_cameras=False),
        )
    gifs = f"{ENVIRONMENT_DIR}/{file_stem(entry.name)}/{animated.ANIMATED_DIR}"
    result.written, notes = animated.copy_gifs(objects, files, gifs)
    result.notes.extend(notes)
    return result


def _chain(obj):
    while obj is not None:
        yield obj
        obj = obj.parent


def _active_action(obj):
    data = obj.animation_data
    return data.action if data else None


# A moving camera plays on the timeline, so its action is padded to the scene's frame range
# as a clip is, and its file is a .glb: the app reads it as its clip's .glb, beside a .json
# that names the nodes moving in it.
def camera(context, suite, entry, files):
    scene = context.scene
    moving = entry.action is not None
    path = target(files, CAMERAS_DIR, entry.name, suite, ".glb" if moving else None)
    focus = lens.exported_with(suite, entry.camera)
    frames = None if moving and entry.action.use_frame_range else (scene.frame_start, scene.frame_end)
    moved = []
    with isolated(context) as iso:
        iso.add_with_ancestors(entry.camera)
        chain = list(_chain(entry.camera))
        clip = None
        if entry.action:
            clip = entry.action.name
            owners = [obj for obj in chain if anim.users(entry.action, obj)] or [entry.camera]
            for owner in owners:
                anim.object_track(iso.copies[owner], clip, entry.action, frames)
                # The copy carries the name; the original steps aside meanwhile.
                moved.append(iso.copies[owner].name)
        if focus is not None:
            iso.add_with_ancestors(focus)
            # A camera file plays one clip, so the focus object's motion joins the camera's.
            for obj in _chain(focus):
                if obj in chain:
                    break
                action = entry.action if entry.action and anim.users(entry.action, obj) else _active_action(obj)
                if action is not None and clip is not None:
                    anim.object_track(iso.copies[obj], clip, action, frames)
                    moved.append(iso.copies[obj].name)
        result = _run(
            context, iso, path,
            _options(
                suite,
                export_animations=clip is not None,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=True,
                **({"export_format": "GLB"} if moving else {}),
            ),
        )
    json_path = os.path.splitext(path)[0] + ".json"
    if moving:
        if entry.action.use_frame_range:
            first, last = (round(f) for f in entry.action.frame_range)
            frames = (first, max(last, first))
        count = frames[1] - frames[0] + 1
        clips.write_json(json_path, {"format": clips.FORMAT, "fps": round(clips.fps(scene), 6), "frames": count, "nodes": moved})
        result.written.append(json_path)
    elif files.isfile(json_path):
        files.remove(json_path)
    return result
