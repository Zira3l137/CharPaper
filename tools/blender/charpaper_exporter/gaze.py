import math
import re

import bpy
from bpy.app.handlers import persistent
from bpy.types import Operator
from mathutils import Matrix
from mathutils import Quaternion
from mathutils import Vector

from . import aim

# Gaze on the Blender side: finding the gaze bones, telling eyeballs from other eyes, working
# out the character's front as the app does, and a preview that poses the character as the app
# would for a look, so limits are set by eye rather than by exporting and looking.

AXES = {
    "POS_X": Vector((1, 0, 0)),
    "NEG_X": Vector((-1, 0, 0)),
    "POS_Y": Vector((0, 1, 0)),
    "NEG_Y": Vector((0, -1, 0)),
    "POS_Z": Vector((0, 0, 1)),
    "NEG_Z": Vector((0, 0, -1)),
}
# What the app takes the character to face when nothing says: Blender's front view looks along +Y.
DEFAULT_FRONT = Vector((0, -1, 0))

# Bone names are read word by word: "mixamorig:LeftEye" is mixamorig, left, eye. A bone is
# an eye when one of its words is, and none of them marks a control, a helper or an end.
EYE_WORDS = {"eye", "eyeball", "目", "左目", "右目"}
HEAD_WORDS = {"head", "頭"}
NECK_WORDS = {"neck", "首"}
HELPERS = {"target", "track", "aim", "ik", "fk", "ctrl", "control", "tweak", "mch", "org", "master"}
NOT_EYES = HELPERS | {"brow", "lid", "lash", "eyes", "highlight", "socket", "両目"}
NOT_HEAD = HELPERS | {"top", "end", "nub", "tail"}
WORD = re.compile(r"[A-Z]+(?![a-z])|[A-Z]?[a-z]+|[^\W\d_A-Za-z]+")

# An eye is taken for a ball when its vertices lie on one this closely, as a share of its
# radius, the bone sits this near its center, and they cover this much of it from the middle
# of the patch. A flat or shallow eye fits a huge ball it only covers a sliver of.
BALL_STRAY = 0.1
BALL_OFF_CENTER = 0.3
BALL_COVERS = math.radians(50.0)


def bones_of(gaze, armature):
    """The gaze bones that exist, by name: (eyes, head, neck)."""
    bones = armature.data.bones
    eyes = [e.bone for e in gaze.eyes if e.bone in bones]
    return eyes, gaze.head if gaze.head in bones else "", gaze.neck if gaze.neck in bones else ""


def eye_keys(gaze):
    return {
        side: getattr(gaze, "key_" + side)
        for side in ("left", "right", "up", "down")
        if getattr(gaze, "key_" + side)
    }


def _rotation(armature):
    return armature.matrix_world.to_3x3().normalized()


def _up(armature):
    return _rotation(armature).inverted() @ Vector((0, 0, 1))


def worked_out_front(gaze, armature):
    """The front in the armature's axes, as the app works it out from the eye bones. None
    when the bones don't say."""
    eyes, head, neck = bones_of(gaze, armature)
    pivot = head or neck
    if not pivot:
        return None
    bones = armature.data.bones
    eyes_at = [bones[eye].head_local.copy() for eye in eyes]
    return aim.front_from_eyes(bones[pivot].head_local.copy(), eyes_at, _up(armature))


def front(gaze, armature):
    """The front in the armature's axes, and where it comes from: "stated" in the panel,
    worked out from the "bones", or the "default" when neither says."""
    stated = AXES.get(gaze.front)
    if stated is not None:
        return stated.copy(), "stated"
    worked_out = worked_out_front(gaze, armature)
    if worked_out is not None:
        return worked_out, "bones"
    return _rotation(armature).inverted() @ DEFAULT_FRONT, "default"


def setup(gaze, armature):
    """The panel's fields as the app reads them from suite.toml."""
    eyes, head, neck = bones_of(gaze, armature)

    def radians(yaw, pitch):
        return Vector((math.radians(yaw), math.radians(pitch)))

    zero = Vector((0.0, 0.0))
    head_limit = radians(gaze.head_yaw, gaze.head_pitch)
    neck_limit = radians(gaze.neck_yaw, gaze.neck_pitch)
    if head and neck:
        turn_limit, neck_part = head_limit + neck_limit, neck_limit
    else:
        turn_limit, neck_part = (head_limit if head else neck_limit if neck else zero), zero

    eye_limit = radians(gaze.eye_yaw, gaze.eye_pitch)
    reach = aim.DRAWN_REACH
    if eyes and gaze.eye_kind == "DRAWN":
        reach = Vector((max(reach.x, eye_limit.x), max(reach.y, eye_limit.y)))
        eyes_setup = aim.Eyes(eye_limit, reach)
    elif eyes:
        eyes_setup = aim.Eyes(eye_limit, eye_limit)
    elif eye_keys(gaze):
        eyes_setup = aim.Eyes(zero, reach.copy())
    else:
        eyes_setup = None
    return aim.Setup(turn_limit, neck_part, gaze.head_share, gaze.head_turn, eyes_setup)


def _skin_meshes(suite, armature):
    seen = set()
    for skin in suite.skins:
        if skin.collection is None:
            continue
        for obj in skin.collection.all_objects:
            if obj.type == "MESH" and obj.name not in seen:
                seen.add(obj.name)
                yield obj


def _deformed_by(obj, armature):
    return any(m.type == "ARMATURE" and m.object == armature for m in obj.modifiers)


def eyeball(suite, armature, bone):
    """The ball an eye bone's vertices lie on, in the armature's space: its center and
    radius, how far the bone's head is from the center, how far the vertices stray from the
    ball as a share of its radius, and how much of the ball they cover, as the widest angle
    from the middle of the patch. None with too few vertices to tell."""
    import numpy

    points = []
    to_armature = armature.matrix_world.inverted()
    for obj in _skin_meshes(suite, armature):
        group = obj.vertex_groups.get(bone.name)
        if group is None or not _deformed_by(obj, armature):
            continue
        matrix = to_armature @ obj.matrix_world
        index = group.index
        for vertex in obj.data.vertices:
            for g in vertex.groups:
                if g.group == index and g.weight >= 0.5:
                    points.append(tuple(matrix @ vertex.co))
                    break
    if len(points) < 8:
        return None
    # Fits a sphere, so a half-shell or an iris cap still finds the eyeball's center.
    p = numpy.array(points)
    a = numpy.column_stack((2 * p, numpy.ones(len(p))))
    b = (p * p).sum(axis=1)
    solution = numpy.linalg.lstsq(a, b, rcond=None)[0]
    center = solution[:3]
    radius_sq = solution[3] + center.dot(center)
    if radius_sq <= 0:
        return None
    radius = float(radius_sq**0.5)
    spokes = p - center
    lengths = numpy.linalg.norm(spokes, axis=1)
    stray = float(numpy.sqrt(numpy.mean((lengths - radius) ** 2))) / radius
    middle = spokes.mean(axis=0)
    covers = 0.0
    if numpy.linalg.norm(middle) > 1e-9 and lengths.min() > 1e-9:
        cosines = (spokes @ middle) / (lengths * numpy.linalg.norm(middle))
        covers = float(numpy.arccos(numpy.clip(cosines.min(), -1.0, 1.0)))
    else:
        # A whole ball's vertices average out to its center.
        covers = math.pi
    offset = float(numpy.linalg.norm(center - numpy.array(tuple(bone.head_local))))
    return {"center": Vector(center), "radius": radius, "offset": offset, "stray": stray, "covers": covers}


def is_ball(fit):
    return (
        fit is not None
        and fit["stray"] <= BALL_STRAY
        and fit["offset"] <= fit["radius"] * BALL_OFF_CENTER
        and fit["covers"] >= BALL_COVERS
    )


def _named(name, words, skip):
    found = {word.lower() for word in WORD.findall(name)}
    return bool(found & words) and not found & skip


def _weighted(suite, armature, names):
    """Which of these bones move any vertex of a skin."""
    found = set()
    for obj in _skin_meshes(suite, armature):
        if not _deformed_by(obj, armature):
            continue
        wanted = {g.index: g.name for g in obj.vertex_groups if g.name in names and g.name not in found}
        if not wanted:
            continue
        for vertex in obj.data.vertices:
            for g in vertex.groups:
                if g.group in wanted and g.weight > 0.0:
                    found.add(wanted.pop(g.group))
            if not wanted:
                break
    return found


def detect(suite, armature):
    """The eye, head and neck bones, going by their names and what hangs from what:
    (eyes, head, neck), with "" for what isn't found."""
    bones = armature.data.bones
    candidates = [b.name for b in bones if _named(b.name, EYE_WORDS, NOT_EYES)]
    # A rig often has several bones per eye; the ones that move the mesh are the ones to turn.
    moving = _weighted(suite, armature, set(candidates))
    eyes = sorted(
        [n for n in candidates if n in moving] or [n for n in candidates if bones[n].use_deform]
    )

    head = ""
    if eyes:
        above = bones[eyes[0]].parent_recursive
        shared = [b for b in above if all(b in bones[eye].parent_recursive for eye in eyes)]
        by_name = [b for b in shared if _named(b.name, HEAD_WORDS, NOT_HEAD)]
        deforming = [b for b in shared if b.use_deform]
        nearest = (by_name or deforming or shared or [None])[0]
        head = nearest.name if nearest else ""
    else:
        by_name = [b for b in bones if _named(b.name, HEAD_WORDS, NOT_HEAD)]
        by_name.sort(key=lambda b: (not b.use_deform, len(b.name)))
        head = by_name[0].name if by_name else ""

    neck = ""
    if head:
        necks = [b for b in bones[head].parent_recursive if _named(b.name, NECK_WORDS, HELPERS)]
        neck = necks[0].name if necks else ""
    return eyes, head, neck


class CHARPAPER_OT_gaze_detect(Operator):
    bl_idname = "charpaper.gaze_detect"
    bl_label = "Detect bones"
    bl_description = (
        "Find the eye, head and neck bones by their names and by what moves the mesh, and tell "
        "whether the eyes are eyeballs. Check what it found, and fix it by hand where it's wrong"
    )
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        armature = context.scene.charpaper.armature
        return armature is not None and armature.type == "ARMATURE"

    def execute(self, context):
        suite = context.scene.charpaper
        gaze, armature = suite.gaze, suite.armature
        eyes, head, neck = detect(suite, armature)
        if not (eyes or head):
            self.report({"WARNING"}, "No eye or head bones found by name; pick them by hand")
            return {"CANCELLED"}
        gaze.eyes.clear()
        for eye in eyes:
            gaze.eyes.add().bone = eye
        gaze.head, gaze.neck = head, neck
        found = [f"eyes {', '.join(eyes)}" if eyes else "no eyes", f"head {head}" if head else "no head"]
        found.append(f"neck {neck}" if neck else "no neck")
        if eyes:
            fits = [eyeball(suite, armature, armature.data.bones[eye]) for eye in eyes]
            if any(fit is not None for fit in fits):
                balls = all(is_ball(fit) for fit in fits if fit is not None)
                gaze.eye_kind = "BALL" if balls else "DRAWN"
                found.append("eyeballs" if balls else "eyes that are no eyeballs, set to Drawn")
        self.report({"INFO"}, "Found " + "; ".join(found))
        return {"FINISHED"}


# What the preview changed, as it was before: the armature's name, each bone's own pose, and
# each shape key's value. Not saved with the .blend, since the pose is put back before a save.
_before = {}


def _restore():
    armature = bpy.data.objects.get(_before.get("armature", ""))
    if armature is not None and armature.type == "ARMATURE":
        for name, basis in _before.get("bones", {}).items():
            bone = armature.pose.bones.get(name)
            if bone is not None:
                bone.matrix_basis = basis
    for (obj_name, key), value in _before.get("keys", {}).items():
        obj = bpy.data.objects.get(obj_name)
        keys = obj.data.shape_keys if obj is not None and obj.type == "MESH" else None
        if keys is not None and key in keys.key_blocks:
            keys.key_blocks[key].value = value
    _before.clear()


def _about(pivot, turn):
    return Matrix.Translation(pivot) @ turn.to_matrix().to_4x4() @ Matrix.Translation(-pivot)


def _turned(pose_bone, matrix):
    """How far the bone has turned from its rest pose, in the armature's space, when its
    pose is `matrix`."""
    return aim.turn_from_rest(matrix.to_quaternion(), pose_bone.bone.matrix_local.to_quaternion())


def _pose(suite, armature):
    """Poses the character for the panel's look, as the app would on top of its current pose. Each
    bone's new pose is worked out from the poses read at the start, with what the bones above
    it did carried along, so nothing has to be evaluated in between."""
    gaze = suite.gaze
    eyes, head, neck = bones_of(gaze, armature)
    pose = armature.pose.bones
    frame = aim.Frame(front(gaze, armature)[0], _up(armature))
    gaze_setup = setup(gaze, armature)
    # A rig with a neck alone has it stand in as the head, as in the app.
    upper, lower = (head, neck) if head and neck else (head or neck, "")

    _before["armature"] = armature.name
    _before["bones"] = {name: pose[name].matrix_basis.copy() for name in eyes + [upper, lower] if name}
    start = {name: pose[name].matrix.copy() for name in _before["bones"]}
    # The turns made so far, outermost first: (bone, how it moved everything below it).
    moved = []

    def carry(name):
        total = Matrix.Identity(4)
        above = {b.name for b in armature.data.bones[name].parent_recursive}
        for bone, by in moved:
            if bone in above:
                total = by @ total
        return total

    def place(name, matrix, by):
        # Its own pose is what is left once its parents' new poses are taken out, and those
        # are their old ones carried along.
        bone = pose[name]
        bone.matrix_basis = bone.matrix_basis @ start[name].inverted() @ carry(name).inverted() @ matrix
        if by is not None:
            moved.append((name, by))

    lowest = lower or upper
    chest_bone = pose[lowest].parent if lowest else None
    chest = _turned(chest_bone, chest_bone.matrix) if chest_bone else Quaternion()
    reach = gaze_setup.reach()
    target = Vector((gaze.look_side * reach.x, gaze.look_up * reach.y))
    direction = chest @ frame.direction(target)

    head_now = None
    if upper:
        clip = frame.angles(chest.inverted() @ (_turned(pose[upper], start[upper]) @ frame.front))
        head_at = gaze_setup.head(target, clip)
        neck_at = gaze_setup.neck(clip, head_at)
        if lower:
            by = _about(start[lower].translation, aim.carried(chest, frame.turn(clip, neck_at)))
            place(lower, by @ start[lower], by)
        carried_head = carry(upper) @ start[upper]
        by = _about(carried_head.translation, aim.carried(chest, frame.turn(neck_at, head_at)))
        head_now = by @ carried_head
        place(upper, head_now, by)

    look = Vector((0.0, 0.0))
    if gaze_setup.eyes is not None:
        if upper:
            socket = _turned(pose[upper], head_now)
        elif eyes and pose[eyes[0]].parent is not None:
            socket = _turned(pose[eyes[0]].parent, pose[eyes[0]].parent.matrix)
        else:
            socket = Quaternion()
        look = gaze_setup.eyes.look(frame.angles(socket.inverted() @ direction))
        aimed = socket @ frame.toward(gaze_setup.eyes.turn(look))
        for eye in eyes:
            carried_eye = carry(eye) @ start[eye]
            rest = pose[eye].bone.matrix_local.to_quaternion()
            matrix = Matrix.LocRotScale(carried_eye.translation, aimed @ rest, carried_eye.to_scale())
            place(eye, matrix, None)

    values = {
        "left": max(look.x, 0.0),
        "right": max(-look.x, 0.0),
        "up": max(look.y, 0.0),
        "down": max(-look.y, 0.0),
    }
    _before["keys"] = {}
    for side, key in eye_keys(gaze).items():
        for obj in _skin_meshes(suite, armature):
            keys = obj.data.shape_keys
            if keys is not None and key in keys.key_blocks:
                _before["keys"][(obj.name, key)] = keys.key_blocks[key].value
                keys.key_blocks[key].value = values[side]


def refresh(context):
    """Brings the preview up to date with the panel: puts back what it changed, and while it
    is on, poses the character again from there."""
    _restore()
    suite = context.scene.charpaper
    armature = suite.armature
    if not suite.gaze.preview or armature is None or armature.type != "ARMATURE":
        return
    # The poses read below must be the ones just put back.
    context.view_layer.update()
    _pose(suite, armature)


def stop(context):
    gaze = context.scene.charpaper.gaze
    if gaze.preview:
        gaze.preview = False
    else:
        _restore()


def _scenes_previewing():
    return [s for s in bpy.data.scenes if getattr(s, "charpaper", None) and s.charpaper.gaze.preview]


# A previewed pose must not be saved as the character's own, and a new frame brings a new pose to start from.
@persistent
def _stop_previews(*_):
    for scene in _scenes_previewing():
        scene.charpaper.gaze.preview = False
    _restore()


# Undo puts the pose back itself, and what was kept from before no longer belongs to it.
@persistent
def _forget(*_):
    if not _scenes_previewing():
        _before.clear()


@persistent
def _loaded(*_):
    _before.clear()
    for scene in _scenes_previewing():
        scene.charpaper.gaze.preview = False


HANDLERS = (
    ("save_pre", _stop_previews),
    ("frame_change_pre", _stop_previews),
    ("undo_post", _forget),
    ("redo_post", _forget),
    ("load_post", _loaded),
)


def register():
    bpy.utils.register_class(CHARPAPER_OT_gaze_detect)
    for name, handler in HANDLERS:
        getattr(bpy.app.handlers, name).append(handler)


def unregister():
    _restore()
    for name, handler in HANDLERS:
        handlers = getattr(bpy.app.handlers, name)
        if handler in handlers:
            handlers.remove(handler)
    bpy.utils.unregister_class(CHARPAPER_OT_gaze_detect)
