import os

from . import anim
from . import gltf
from . import lens
from .isolate import isolated
from .isolate import showing
from .props import file_stem

SKINS_DIR = "skins"
ANIMATIONS_DIR = "animations"
CAMERAS_DIR = "cameras"
ENVIRONMENT_DIR = "environment"


class Result:
    def __init__(self, path):
        self.path = path
        self.skipped = []
        self.notes = []


def extension(suite):
    return gltf.EXTENSIONS[suite.gltf.file_format]


def target(folder, subdir, name, suite):
    directory = os.path.join(folder, subdir) if subdir else folder
    os.makedirs(directory, exist_ok=True)
    return os.path.join(directory, file_stem(name) + extension(suite))


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


def model(context, suite, folder):
    path = target(folder, "", suite.model_name, suite)
    with isolated(context) as iso:
        iso.add_with_ancestors(suite.armature)
        return _run(
            context, iso, path,
            _options(suite, export_animations=False, export_lights=False, export_cameras=False),
        )


def skin(context, suite, entry, folder):
    path = target(folder, SKINS_DIR, entry.name, suite)
    with isolated(context) as iso:
        iso.add_with_ancestors(suite.armature)
        meshes = []
        for obj in entry.collection.all_objects:
            if obj.type in {"MESH", "EMPTY"}:
                copy = iso.add(obj, own_mesh=_shape_keyed(obj))
                if _shape_keyed(copy):
                    meshes.append(copy)
        tracks = 0
        for expression in entry.expressions:
            if expression.enabled and expression.action:
                for mesh in meshes:
                    if anim.key_track(mesh, expression.action.name, expression.action):
                        tracks += 1
        return _run(
            context, iso, path,
            _options(
                suite,
                export_animations=tracks > 0,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=False,
            ),
        )


def animation(context, suite, entry, folder):
    path = target(folder, ANIMATIONS_DIR, entry.name, suite)
    with isolated(context) as iso:
        armature = iso.add_with_ancestors(suite.armature)
        meshes = []
        if entry.use_correctives and entry.correctives:
            for obj in entry.correctives.all_objects:
                if _shape_keyed(obj):
                    meshes.append(iso.add(obj, own_mesh=True))
        result_notes = []
        for clip in entry.clips:
            if not (clip.enabled and clip.action):
                continue
            name = clip.action.name
            anim.object_track(armature, name, clip.action)
            if clip.corrective and meshes:
                if not any(anim.key_track(mesh, name, clip.corrective) for mesh in meshes):
                    result_notes.append(f"{name}: {clip.corrective.name} keys none of the corrective meshes")
        result = _run(
            context, iso, path,
            _options(
                suite,
                export_animations=True,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=False,
            ),
        )
        result.notes.extend(result_notes)
        return result


def environment(context, suite, entry, folder):
    path = target(folder, ENVIRONMENT_DIR, entry.name, suite)
    with isolated(context) as iso:
        for obj in entry.collection.all_objects:
            if obj.type != "CAMERA":
                iso.add(obj, keep_animation=True)
        return _run(
            context, iso, path,
            _options(
                suite,
                export_animations=True,
                export_animation_mode="ACTIVE_ACTIONS",
                export_merge_animation="ACTION",
                export_lights=True,
                export_cameras=False,
            ),
        )


def _chain(obj):
    while obj is not None:
        yield obj
        obj = obj.parent


def _active_action(obj):
    data = obj.animation_data
    return data.action if data else None


def camera(context, suite, entry, folder):
    path = target(folder, CAMERAS_DIR, entry.name, suite)
    focus = lens.exported_with(suite, entry.camera)
    with isolated(context) as iso:
        iso.add_with_ancestors(entry.camera)
        chain = list(_chain(entry.camera))
        clip = None
        if entry.action:
            clip = entry.action.name
            owners = [obj for obj in chain if anim.users(entry.action, obj)] or [entry.camera]
            for owner in owners:
                anim.object_track(iso.copies[owner], clip, entry.action)
        if focus is not None:
            iso.add_with_ancestors(focus)
            # A camera file plays one clip, so the focus object's motion joins the camera's.
            for obj in _chain(focus):
                if obj in chain:
                    break
                action = entry.action if entry.action and anim.users(entry.action, obj) else _active_action(obj)
                if action is not None:
                    clip = clip or action.name
                    anim.object_track(iso.copies[obj], clip, action)
        return _run(
            context, iso, path,
            _options(
                suite,
                export_animations=clip is not None,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=True,
            ),
        )
