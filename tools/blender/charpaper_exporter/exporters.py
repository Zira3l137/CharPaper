import os

from . import anim
from . import animated
from . import drivers
from . import gltf
from . import lens
from . import lights
from .isolate import isolated
from .isolate import showing
from .props import file_stem

SKINS_DIR = "skins"
ANIMATIONS_DIR = "animations"
CAMERAS_DIR = "cameras"
ENVIRONMENT_DIR = "environment"

# An animation file's meshes are only there for their shape keys' weights to be keyed on: the
# app plays the keys on the worn skin's own meshes and never draws these. So they leave out
# what only drawing needs, above all their materials' textures, which would otherwise be copied
# into every animation file.
CARRIER_MESHES = {
    "export_materials": "NONE",
    "export_image_format": "NONE",
    "export_texcoords": False,
    "export_vertex_color": "NONE",
    "export_tangents": False,
    "export_morph_normal": False,
    "export_morph_tangent": False,
}


class Result:
    def __init__(self, path):
        self.path = path
        self.skipped = []
        self.notes = []
        # Files written besides the one at `path`.
        self.written = []


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


def _new_data(obj, objects):
    # Shape keys belong to the mesh, so objects sharing one get a single track between them.
    return all(other.data != obj.data for other in objects)


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
                if _shape_keyed(copy) and _new_data(copy, meshes):
                    meshes.append(copy)
        tracks = 0
        for expression in entry.expressions:
            if expression.enabled and expression.action:
                for mesh in meshes:
                    if anim.key_track(mesh, expression.action.name, expression.action):
                        tracks += 1
        result = _run(
            context, iso, path,
            _options(
                suite,
                export_animations=tracks > 0,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=False,
            ),
        )
    # Read from the originals, once they have their names back.
    written, notes = drivers.write(suite, entry, path)
    result.written.extend(written)
    result.notes.extend(notes)
    return result


# Shape keys a clip keys by hand go in with it, on copies of the skins' meshes that have them,
# which the app matches to the worn skin's own meshes by name. Driven shape keys need nothing
# here: the skin's drivers file carries them.
def animation(context, suite, entry, folder):
    path = target(folder, ANIMATIONS_DIR, entry.name, suite)
    clips = [c for c in entry.clips if c.enabled and c.action]
    sources = [anim.shape_key_source(c) for c in clips]
    keyed = set()
    for source in sources:
        if source is not None:
            keyed |= anim.key_names(source)
    with isolated(context) as iso:
        armature = iso.add_with_ancestors(suite.armature)
        meshes = []
        for obj in anim.shape_keyed_skin_meshes(suite):
            if keyed & set(obj.data.shape_keys.key_blocks.keys()):
                copy = iso.add(obj, own_mesh=True)
                if _new_data(copy, meshes):
                    meshes.append(copy)
        result_notes = []
        for clip, source in zip(clips, sources):
            name = clip.action.name
            anim.object_track(armature, name, clip.action)
            if source is not None and meshes:
                # The glTF exporter slides each object's strip to frame 0 on its own, so the
                # shape keys must start and end with the body or they drift out of step.
                span = tuple(clip.action.frame_range)
                tracks = [anim.key_track(mesh, name, source, span) for mesh in meshes]
                if not any(tracks):
                    result_notes.append(f"{name}: its shape keys are on none of the skins' meshes")
            elif source is not None:
                result_notes.append(f"{name}: its shape keys are on none of the skins' meshes")
        result = _run(
            context, iso, path,
            _options(
                suite,
                export_animations=True,
                export_animation_mode="NLA_TRACKS",
                export_lights=False,
                export_cameras=False,
                **CARRIER_MESHES,
            ),
        )
        result.notes.extend(result_notes)
        return result


def environment(context, suite, entry, folder):
    path = target(folder, ENVIRONMENT_DIR, entry.name, suite)
    threshold = context.scene.eevee.light_threshold
    objects = [obj for obj in entry.collection.all_objects if obj.type != "CAMERA"]
    with isolated(context) as iso, animated.stills(objects):
        tracks = 0
        keyed = []
        for obj in objects:
            copy = iso.add(obj, own_mesh=_shape_keyed(obj), own_light=True, keep_drivers=True)
            if copy.type == "LIGHT":
                lights.give_range(copy.data, threshold)
            # Each action becomes a clip of its own, as long as its own keys, and the app loops
            # each on its own. Merged into one clip, a short loop would wait for the longest.
            # Objects playing the same action share its clip and stay in step.
            if anim.active_track(copy, obj):
                tracks += 1
            if _shape_keyed(copy) and _new_data(copy, keyed):
                keyed.append(copy)
                if anim.active_track(copy.data.shape_keys, obj.data.shape_keys):
                    tracks += 1
        result = _run(
            context, iso, path,
            _options(
                suite,
                export_animations=tracks > 0,
                export_animation_mode="NLA_TRACKS",
                export_lights=True,
                export_cameras=False,
            ),
        )
    gifs = os.path.join(folder, ENVIRONMENT_DIR, file_stem(entry.name), animated.ANIMATED_DIR)
    result.written, notes = animated.copy_gifs(objects, gifs)
    result.notes.extend(notes)
    return result


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
