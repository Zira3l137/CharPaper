# Which way her face points, for gaze. The app can't tell from the rig, so the author shows it:
# look her in the face in the viewport and click "She faces me". It is kept as one of the
# armature's own axes, so turning the armature, or the whole scene, afterwards keeps it right,
# and export writes it into suite.toml in world axes, as the model is exported. An arrow from
# her head shows it in the viewport.

import bpy
import gpu
from bpy.types import Operator
from gpu_extras.batch import batch_for_shader
from mathutils import Vector

from .props import redraw_views
from .props import suite_of

AXES = {
    "POS_X": Vector((1, 0, 0)),
    "NEG_X": Vector((-1, 0, 0)),
    "POS_Y": Vector((0, 1, 0)),
    "NEG_Y": Vector((0, -1, 0)),
    "POS_Z": Vector((0, 0, 1)),
    "NEG_Z": Vector((0, 0, -1)),
}
# What the app takes her to face without one: Blender's front view looks along +Y.
DEFAULT = Vector((0, -1, 0))

SET_COLOR = (0.3, 0.9, 0.4, 1.0)
UNSET_COLOR = (1.0, 0.55, 0.1, 1.0)

_handle = None


def _rotation(armature):
    return armature.matrix_world.to_3x3().normalized()


def world_front(armature, axis):
    """The front in world axes, or None while it isn't set."""
    if axis not in AXES:
        return None
    return (_rotation(armature) @ AXES[axis]).normalized()


def nearest_axis(armature, world):
    local = _rotation(armature).inverted() @ world
    index = max(range(3), key=lambda i: abs(local[i]))
    return ("POS_" if local[index] > 0 else "NEG_") + "XYZ"[index]


def _head(suite, armature):
    gaze = suite.gaze
    names = [gaze.head, gaze.neck] + [e.bone for e in gaze.eyes]
    for name in names:
        bone = armature.pose.bones.get(name) if name else None
        if bone is not None:
            return armature.matrix_world @ bone.head
    return armature.matrix_world.translation


def _draw():
    context = bpy.context
    suite = getattr(context.scene, "charpaper", None)
    if suite is None or not suite.gaze.show_front or suite.armature is None:
        return
    armature = suite.armature
    front = world_front(armature, suite.gaze.front)
    color = SET_COLOR if front is not None else UNSET_COLOR
    front = front if front is not None else DEFAULT
    start = _head(suite, armature)
    length = max(armature.dimensions.z, 1.0) * 0.3
    end = start + front * length
    side = front.cross(Vector((0, 0, 1)))
    side = side.normalized() if side.length > 1e-4 else Vector((1, 0, 0))
    barb = length * 0.15
    points = [
        start, end,
        end, end - front * barb + side * barb,
        end, end - front * barb - side * barb,
    ]
    shader = gpu.shader.from_builtin("POLYLINE_UNIFORM_COLOR")
    shader.uniform_float("viewportSize", tuple(float(v) for v in gpu.state.viewport_get()[2:]))
    shader.uniform_float("lineWidth", 3.0)
    shader.uniform_float("color", color)
    # Drawn over her, or the arrow would start hidden inside her head.
    depth_test = gpu.state.depth_test_get()
    gpu.state.depth_test_set("NONE")
    batch_for_shader(shader, "LINES", {"pos": points}).draw(shader)
    gpu.state.depth_test_set(depth_test)


class CHARPAPER_OT_gaze_front_from_view(Operator):
    bl_idname = "charpaper.gaze_front_from_view"
    bl_label = "She faces me"
    bl_description = (
        "Look her in the face in this viewport, then click: the way you look at her becomes her "
        "front, rounded to the nearest of the armature's axes"
    )
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        space = context.space_data
        return (
            suite_of(context).armature is not None
            and space is not None
            and space.type == "VIEW_3D"
            and space.region_3d is not None
        )

    def execute(self, context):
        gaze = suite_of(context).gaze
        armature = suite_of(context).armature
        view = context.space_data.region_3d.view_rotation @ Vector((0, 0, -1))
        gaze.front = nearest_axis(armature, -view)
        gaze.show_front = True
        label = gaze.front.replace("POS_", "+").replace("NEG_", "-")
        self.report({"INFO"}, f"Her front is {armature.name}'s {label} axis")
        redraw_views()
        return {"FINISHED"}


def register():
    global _handle
    bpy.utils.register_class(CHARPAPER_OT_gaze_front_from_view)
    _handle = bpy.types.SpaceView3D.draw_handler_add(_draw, (), "WINDOW", "POST_VIEW")


def unregister():
    global _handle
    if _handle is not None:
        bpy.types.SpaceView3D.draw_handler_remove(_handle, "WINDOW")
        _handle = None
    bpy.utils.unregister_class(CHARPAPER_OT_gaze_front_from_view)
