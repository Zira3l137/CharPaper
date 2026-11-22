import os
from collections import Counter

import bpy

from . import anim
from . import animated
from . import checks
from . import drivers
from . import env_settings
from . import front
from . import paths
from . import sky
from . import gaze as gaze_bones
from . import gltf
from . import hotspots
from . import lens
from . import music
from .actors import TRANSFORM_PATH
from .checks import Finding
from .exporters import CAMERAS_DIR
from .exporters import ENVIRONMENT_DIR
from .exporters import SKINS_DIR
from .exporters import UNLINK_ACTION
from .props import ORBIT
from .props import file_stem

MAX_SHAPE_KEYS = 256


def _examples(names, limit=3):
    names = sorted(names)
    shown = ", ".join(names[:limit])
    return shown + (f" and {len(names) - limit} more" if len(names) > limit else "")


def _chain(obj):
    while obj is not None:
        yield obj
        obj = obj.parent


def _deformed_by(obj, armature):
    return any(m.type == "ARMATURE" and m.object == armature for m in obj.modifiers)


def check(context, suite):
    out = []
    add = out.append

    if not gltf.available():
        add(Finding("the glTF exporter add-on is disabled; enable it in Preferences > Add-ons"))
    folder, why = paths.folder(suite)
    if folder is None:
        add(Finding(f"nowhere to export to: {why[0].lower()}{why[1:]}", severity="ERROR"))
    if suite.gltf.image_format == "WEBP":
        add(Finding("WebP images only load if the app is built with Bevy's webp feature", severity="INFO"))

    armature = suite.armature
    if armature is None:
        add(Finding("no armature picked, so the model, skins and animations cannot be exported"))
    else:
        _check_rig(armature, add)
        if suite.gaze.mode == "WRITE":
            _check_gaze(suite, armature, add)

    _check_names(suite, add)
    _check_contents(suite, add)
    _check_defaults(suite, add)
    for entry in suite.skins:
        if entry.enabled:
            _check_skin(suite, entry, armature, add)
    checks.check(context, suite, add)
    for entry in suite.environments:
        if entry.enabled:
            _check_environment(context.scene, entry, armature, add)
    music.check(suite, add, Finding)
    hotspots.check(suite, add, Finding)
    for entry in suite.cameras:
        if entry.enabled:
            _check_camera(suite, entry, add)
    _check_stale_formats(suite, add)
    return out


def _check_rig(armature, add):
    for obj in _chain(armature):
        scale = obj.scale
        if max(scale) - min(scale) > 1e-4:
            add(Finding(f"{obj.name} has a non-uniform scale; bone rotations will skew in the app", obj))
    rotation = armature.matrix_world.to_quaternion()
    if rotation.angle > 1e-4:
        add(Finding(
            f"{armature.name} is rotated; that's fine only if the character still stands on +Z facing -Y",
            armature,
            severity="INFO",
        ))


def _check_gaze(suite, armature, add):
    gaze = suite.gaze
    bones = armature.data.bones
    eyes = [e.bone for e in gaze.eyes if e.bone]
    keys = gaze_bones.eye_keys(gaze)
    if not (eyes or keys or gaze.head or gaze.neck):
        add(Finding(
            "gaze names no bones and no shape keys, so the character never follows the cursor; "
            "try Detect bones",
            armature,
        ))
    for name in eyes + [gaze.head, gaze.neck]:
        if name and name not in bones:
            add(Finding(f"gaze: {name!r} is not a bone of {armature.name}; gaze will be off", armature))

    def below(child, parent):
        return child in bones and parent in bones and bones[parent] in bones[child].parent_recursive

    for eye in eyes:
        if gaze.head and not below(eye, gaze.head):
            add(Finding(f"gaze: {eye} is not below {gaze.head}; the eye stays behind when the head turns", armature, eye))
    if gaze.head and gaze.neck and not below(gaze.head, gaze.neck):
        add(Finding(f"gaze: {gaze.head} is not below {gaze.neck}; the neck turn won't carry the head", armature, gaze.head))

    for eye in eyes:
        if eye in bones:
            _check_eye(suite, armature, bones[eye], add)
    _check_gaze_front(gaze, armature, add)

    on_skins = set()
    for skin in suite.skins:
        if skin.collection:
            for obj in skin.collection.all_objects:
                if obj.type == "MESH" and obj.data.shape_keys:
                    on_skins.update(obj.data.shape_keys.key_blocks.keys())
    for key in keys.values():
        if key not in on_skins:
            add(Finding(f"gaze: no skin has a shape key named {key!r}, so it never moves"))


def _check_gaze_front(gaze, armature, add):
    worked_out = gaze_bones.worked_out_front(gaze, armature)
    facing = front.world_front(armature, gaze.front)
    if facing is None:
        if worked_out is None:
            add(Finding(
                "gaze: the bones don't say which way the character faces, so it's taken to face "
                "-Y. If it faces another way, turn on the arrow, look it in the face and click "
                "Front from view",
                armature,
                severity="INFO",
            ))
    elif abs(facing.z) > 0.7:
        add(Finding(
            "gaze: the front points up or down; the character has to stand upright for gaze to turn it",
            armature,
        ))
    elif worked_out is not None and worked_out.dot(gaze_bones.AXES[gaze.front]) < 0.0:
        add(Finding(
            "gaze: Front points to the back of the head, going by the eye bones, so the character "
            "will turn away from the cursor. Set Front back to Automatic",
            armature,
        ))


def _check_eye(suite, armature, bone, add):
    fit = gaze_bones.eyeball(suite, armature, bone)
    if fit is None:
        return
    ball = gaze_bones.is_ball(fit)
    if suite.gaze.eye_kind == "DRAWN":
        if ball:
            add(Finding(
                f"gaze: {bone.name} turns a round eyeball, which Eyeballs would aim exactly; "
                "Drawn only moves it in proportion",
                armature,
                bone.name,
                severity="INFO",
            ))
        return
    round_enough = fit["stray"] <= gaze_bones.BALL_STRAY and fit["covers"] >= gaze_bones.BALL_COVERS
    if round_enough and fit["offset"] > max(0.0005, fit["radius"] * 0.05):
        add(Finding(
            f"gaze: {bone.name}'s head is {fit['offset'] * 1000:.1f} mm from its eyeball's center; "
            "the eye will swing instead of spin",
            armature,
            bone.name,
        ))
    elif not round_enough:
        add(Finding(
            f"gaze: {bone.name}'s eye is no ball turning about the bone, so aiming it will shoot "
            "past; set Eyes are to Drawn",
            armature,
            bone.name,
        ))


def _check_names(suite, add):
    groups = (
        ("skin", [e.name for e in suite.skins if e.enabled]),
        ("environment", [e.name for e in suite.environments if e.enabled]),
        ("camera", [e.name for e in suite.cameras if e.enabled]),
    )
    for kind, names in groups:
        for name in names:
            if file_stem(name) != name:
                add(Finding(f"{kind} {name!r} has characters a file name can't hold; saved as {file_stem(name)!r}"))
        for name, count in Counter(file_stem(n) for n in names).items():
            if count > 1:
                add(Finding(f"{count} {kind}s are named {name!r}; they overwrite each other"))

    if any(c.name == "orbit" for c in suite.cameras if c.enabled):
        add(Finding("a camera named 'orbit' hides the built-in orbit camera"))


def _on_disk(suite, subdir):
    """What an earlier export left in a subfolder, by file stem or folder name."""
    folder, _ = paths.folder(suite)
    if folder is None:
        return set()
    folder = os.path.join(folder, subdir)
    return {os.path.splitext(name)[0] for name in os.listdir(folder)} if os.path.isdir(folder) else set()


def _check_contents(suite, add):
    if not any(e.enabled for e in suite.skins) and not _on_disk(suite, SKINS_DIR):
        add(Finding("no skins, so there is no character to show"))
    if not any(e.enabled for e in suite.environments) and not _on_disk(suite, ENVIRONMENT_DIR):
        add(Finding("no environments, so nothing lights the character and it renders black"))


# The app refuses to load a suite whose default skin, environment or camera doesn't exist.
def _check_defaults(suite, add):
    offered = (
        ("skin", suite.default_skin, {e.name for e in suite.skins} | _on_disk(suite, SKINS_DIR)),
        (
            "environment",
            suite.default_environment,
            {e.name for e in suite.environments} | _on_disk(suite, ENVIRONMENT_DIR),
        ),
        (
            "camera",
            suite.default_camera,
            {ORBIT} | {e.name for e in suite.cameras} | _on_disk(suite, CAMERAS_DIR),
        ),
    )
    for kind, name, names in offered:
        if name and name not in names:
            add(Finding(f"the default {kind} {name!r} is not one of the suite's {kind}s"))


def _check_skin(suite, entry, armature, add):
    if entry.collection is None:
        add(Finding(f"skin {entry.name!r} has no collection"))
        return
    meshes = [o for o in entry.collection.all_objects if o.type == "MESH"]
    if not meshes:
        add(Finding(f"skin {entry.name!r}: {entry.collection.name} holds no meshes"))
    loose = []
    for obj in meshes:
        if armature is not None and not _deformed_by(obj, armature) and armature not in _chain(obj.parent):
            loose.append(obj.name)
        for modifier in obj.modifiers:
            if modifier.type == "ARMATURE" and modifier.object not in (None, armature):
                add(Finding(f"{obj.name} is deformed by {modifier.object.name}, not the suite's armature", obj))
        key = obj.data.shape_keys
        if key is None:
            continue
        if len(key.key_blocks) - 1 > MAX_SHAPE_KEYS:
            add(Finding(f"{obj.name} has {len(key.key_blocks) - 1} shape keys; Bevy supports {MAX_SHAPE_KEYS}", obj))
        others = [m for m in obj.modifiers if m.type != "ARMATURE" and m.show_viewport]
        if others and suite.gltf.apply_modifiers:
            add(Finding(
                f"{obj.name} has shape keys and modifiers ({others[0].name}); "
                "with Apply modifiers on, the exporter may drop its shape keys",
                obj,
            ))
    if loose:
        add(Finding(
            f"skin {entry.name!r}: {len(loose)} mesh(es) are neither skinned nor parented to the "
            f"armature, so they won't follow the character: {_examples(loose)}",
            bpy.data.objects.get(loose[0]),
        ))
    others = [o.name for o in entry.collection.all_objects if o.type not in {"MESH", "EMPTY", "ARMATURE"}]
    if others:
        add(Finding(f"skin {entry.name!r}: left out (not meshes): {_examples(others)}", severity="INFO"))

    if armature is not None:
        _, problems = drivers.collect(suite, armature, meshes)
        for problem in problems:
            add(Finding(f"skin {entry.name!r}: {problem.message}", problem.obj, problem.bone))

    frame = bpy.context.scene.frame_current
    for obj in entry.collection.all_objects:
        _check_skin_object_action(entry, obj, frame, add)


# A skin is exported as it stands and moves only with the armature: what an object's own
# action does to it is frozen at this frame.
def _check_skin_object_action(entry, obj, frame, add):
    found = anim.playing(obj, frame)
    if found is None:
        return
    action, track, curves = found
    where = f"the action {action.name}" if track is None else f"its NLA track {track!r}, playing {action.name},"
    if not any(anim.changes(c) for c in curves):
        add(Finding(
            f"skin {entry.name!r}: {obj.name} has {where} of its own, which keys nothing that changes, so it "
            "does no harm. Importers often leave these; Unlink it to tidy up",
            obj,
            severity="INFO",
            fix=UNLINK_ACTION,
        ))
        return
    moved = [c for c in curves if anim.changes(c) and TRANSFORM_PATH.match(c.data_path)]
    if moved:
        add(Finding(
            f"skin {entry.name!r}: {where} moves {obj.name}, but a skin is exported still and moves only "
            "with the armature, so in the app it stays where it is at this frame. To move it with the "
            "character, parent it to a bone or weight it to the armature and key the bones; then Unlink "
            "it",
            obj,
            fix=UNLINK_ACTION,
        ))
        return
    paths = sorted({c.data_path for c in curves if anim.changes(c)})
    kept = "it as it is" if len(paths) == 1 else "them as they are"
    add(Finding(
        f"skin {entry.name!r}: on {obj.name}, {where} keys {_examples(paths)}, which a skin doesn't carry: "
        f"the app keeps {kept} at this frame. Unlink it if that's not what you want",
        obj,
        fix=UNLINK_ACTION,
    ))


def _check_world(name, world, add):
    found = sky.plain(world)
    if found is None:
        add(Finding(
            f"environment {name!r}: World {world.name!r} isn't a single Environment Texture, so "
            "the export renders it into a panorama with Cycles; a Light Path trick comes over "
            "as the camera sees it",
            severity="INFO",
        ))
        return
    if not sky.readable(found.image):
        add(Finding(
            f"environment {name!r}: the World's image {found.image.filepath} is missing, so its "
            "sky can't be exported"
        ))


def _check_environment(scene, entry, armature, add):
    world = sky.world_of(scene, entry)
    if world is not None:
        _check_world(entry.name, world, add)
    elif entry.export_sky:
        add(Finding(f"environment {entry.name!r}: no World picked and none in the scene, so it has no sky"))
    if entry.collection is None:
        if world is None:
            add(Finding(f"environment {entry.name!r} has no collection, and no World for its sky"))
        return
    objects = list(entry.collection.all_objects)
    if world is None and not any(o.type == "LIGHT" for o in objects):
        add(Finding(
            f"environment {entry.name!r} has no lights; unless it ships reflection maps, "
            "the character renders black",
        ))
    cameras = [o.name for o in objects if o.type == "CAMERA"]
    if cameras:
        add(Finding(f"environment {entry.name!r}: cameras are left out, use Cameras: {_examples(cameras)}", severity="INFO"))
    _check_fog(entry.name, objects, world is not None, add)
    _check_shadows(entry.name, objects, add)
    _check_animated(objects, add)
    # The environment's file has no armature, so what hangs from the character stays put.
    riding = [o.name for o in objects if armature is not None and armature in _chain(o.parent)]
    if riding:
        one = len(riding) == 1
        add(Finding(
            f"environment {entry.name!r}: {_examples(riding)} {'hangs' if one else 'hang'} from the "
            f"character, which isn't in the environment's file, so {'it stays where it is' if one else 'they stay where they are'}. "
            f"Put {'it' if one else 'them'} in a skin to move with the character",
            bpy.data.objects.get(riding[0]),
        ))


def _check_animated(objects, add):
    for material in animated.materials(objects):
        moving = animated.moving_images(material)
        gifs = [image for image in moving if animated.is_gif(image)]
        others = [image.name for image in moving if not animated.is_gif(image)]
        if others:
            add(Finding(
                f"material {material.name!r}: only GIFs play in the app, so these show as a "
                f"still: {_examples(others)}",
            ))
        if not gifs:
            continue
        if len(gifs) > 1:
            add(Finding(
                f"material {material.name!r} has {len(gifs)} GIFs; the app plays one on its base "
                f"color, {gifs[0].name!r}",
            ))
        if file_stem(material.name) != material.name:
            add(Finding(
                f"material {material.name!r}: its GIF is saved under the material's name, which "
                "can't be a file name; rename the material",
            ))
        source = animated.source_path(gifs[0])
        if not os.path.isfile(source):
            add(Finding(f"GIF {gifs[0].name!r}: its file {source} is missing"))


def _check_fog(name, objects, has_sky, add):
    fog = env_settings.fog_objects(objects)
    for obj, node in fog.items():
        if not env_settings.is_principled(node):
            add(Finding(f"fog {obj.name}: only a Principled Volume is read; this one shows as its defaults", obj))
            continue
        linked = [socket for socket in env_settings.FOG_INPUTS if node.inputs[socket].is_linked]
        if "Density" in linked:
            linked.remove("Density")
            noise, notes = env_settings.density_noise(obj, node)
            if noise is None:
                add(Finding(f"fog {obj.name}: {notes[0]}, so the app uses the Density typed into the node", obj))
            for note in notes if noise is not None else ():
                add(Finding(f"fog {obj.name}: {note}", obj, severity="INFO"))
        if linked:
            add(Finding(
                f"fog {obj.name}: {', '.join(linked)} come from other nodes, which don't carry over; "
                "the app uses the values typed into the node",
                obj,
            ))
        glowing = [
            label
            for socket, label in (("Emission Strength", "emission"), ("Blackbody Intensity", "blackbody"))
            if socket in node.inputs and node.inputs[socket].default_value > 0.0
        ]
        if glowing:
            add(Finding(f"fog {obj.name}: {' and '.join(glowing)} don't carry over", obj))
        if len(obj.data.vertices) != 8:
            add(Finding(f"fog {obj.name} is not a box; the app fills its bounding box", obj, severity="INFO"))

    lights = [o for o in objects if o.type == "LIGHT"]
    partial = [o.name for o in lights if 0.0 < o.data.volume_factor != 1.0]
    if fog and partial:
        add(Finding(
            f"environment {name!r}: the app has no Volume Scatter strength, only on or off; "
            f"above 0 counts as on: {_examples(partial)}",
            severity="INFO",
        ))
    if fog and not has_sky and not any(o.data.volume_factor > 0.0 for o in lights):
        add(Finding(
            f"environment {name!r} has fog but no World and no light that lights it, so unless "
            "it ships a sky the fog never shows",
        ))
    dark_suns = [o.name for o in lights if o.data.type == "SUN" and not o.data.use_shadow and o.data.volume_factor > 0.0]
    if fog and dark_suns:
        add(Finding(
            f"environment {name!r}: a sun lights fog through its shadows, so with Shadow off "
            f"it doesn't: {_examples(dark_suns)}",
            severity="INFO",
        ))


# Above this many, the app's frame rate is usually held back by shadows.
MANY_SHADOWED_POINTS = 2


def _check_shadows(name, objects, add):
    """Each point light with Shadow on draws the scene six more times every frame."""
    shadowed = [o.name for o in objects if o.type == "LIGHT" and o.data.type == "POINT" and o.data.use_shadow]
    if len(shadowed) > MANY_SHADOWED_POINTS:
        add(Finding(
            f"environment {name!r}: {len(shadowed)} point lights cast shadows, and each draws the "
            "scene 6 more times a frame. Untick Light > Shadow on lamps that don't need it: "
            f"{_examples(shadowed)}",
            severity="INFO",
        ))


def _check_camera(suite, entry, add):
    if entry.camera is None:
        add(Finding(f"camera {entry.name!r} has no camera object"))
        return
    chain = list(_chain(entry.camera))
    for obj in chain[1:]:
        if obj.type != "EMPTY":
            add(Finding(f"camera {entry.name!r} hangs from {obj.name}, a {obj.type.lower()}; it gets exported too", obj))
    if entry.action and not any(anim.users(entry.action, o) for o in chain):
        add(Finding(
            f"camera {entry.name!r}: neither the camera nor its parents use {entry.action.name!r}; "
            "it goes on the camera",
            entry.camera,
            severity="INFO",
        ))
    _check_lens(suite, entry, add)


def _check_lens(suite, entry, add):
    camera = entry.camera
    dof = camera.data.dof
    if not dof.use_dof:
        return
    name = entry.name
    if camera.data.type != "PERSP":
        add(Finding(f"camera {name!r} is not perspective, so its depth of field is left out", camera))
        return

    if dof.aperture_fstop <= 0.0:
        add(Finding(f"camera {name!r}: an F-Stop of 0 can't work, so the app leaves depth of field off", camera))
    if dof.focus_object is None and dof.focus_distance <= 0.0:
        add(Finding(f"camera {name!r}: a Focus Distance of 0 can't work, so the app focuses 10 m away", camera))

    data = camera.data.animation_data
    keyed = {c.data_path for c in anim.fcurves(data.action)} if data and data.action else set()
    labels = {"dof.aperture_fstop": "F-Stop", "lens": "Focal Length"}
    if dof.focus_object is None:
        labels["dof.focus_distance"] = "Focus Distance"
    animated = [label for path, label in labels.items() if path in keyed]
    if animated:
        add(Finding(
            f"camera {name!r}: animated {' and '.join(animated)} does not carry over, the app "
            "uses the current value; animate a focus object instead",
            camera,
        ))
    if dof.aperture_blades or dof.aperture_rotation or dof.aperture_ratio != 1.0:
        add(Finding(
            f"camera {name!r}: aperture blades, rotation and ratio have no match in the app",
            camera,
            severity="INFO",
        ))

    target = dof.focus_object
    if target is None:
        return
    if target.type == "ARMATURE" and target != suite.armature:
        add(Finding(
            f"camera {name!r} focuses on {target.name}, an armature other than the character's; "
            "the app cannot find it",
            target,
        ))
    elif lens.on_character(suite, target) and target != suite.armature and not _in_skin(suite, target):
        add(Finding(
            f"camera {name!r} focuses on {target.name}, which hangs from the character but is in "
            "no skin; the app cannot find it",
            target,
        ))
    elif lens.exported_with(suite, camera) is target and target.type != "EMPTY":
        add(Finding(
            f"camera {name!r}: focus object {target.name} is a {target.type.lower()} and goes into "
            "the camera's file whole; an empty is enough",
            target,
            severity="INFO",
        ))


def _in_skin(suite, obj):
    return any(s.collection and obj.name in s.collection.all_objects for s in suite.skins)


def _check_stale_formats(suite, add):
    folder, _ = paths.folder(suite)
    if folder is None or not os.path.isdir(folder):
        return
    other = ".gltf" if suite.gltf.file_format == "GLB" else ".glb"
    groups = (
        ("skins", suite.skins),
        ("environment", suite.environments),
        ("cameras", suite.cameras),
    )
    stale = []
    for subdir, entries in groups:
        for entry in entries:
            path = os.path.join(folder, subdir, file_stem(entry.name) + other)
            if entry.enabled and os.path.isfile(path):
                stale.append(os.path.relpath(path, folder))
    if stale:
        add(Finding(f"older {other} files would sit next to the new ones; delete them: {_examples(stale)}"))
