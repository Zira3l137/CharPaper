# Which way her face points in her rest pose, for gaze, which turns from there. It is worked
# out from where her eye bones sit, as the app does it, and an arrow from her head shows the
# result on her rest pose. Where that is wrong, or she has no eye bones, the author shows it:
# look her in the face in the viewport and click "She faces me". That is kept as one of the
# armature's own axes, so turning the armature, or the whole scene, afterwards keeps it right,
# and only then does export write it into suite.toml, in world axes, as the model is exported.

import bpy
import gpu
from bpy.types import Operator
from gpu_extras.batch import batch_for_shader
from mathutils import Vector

from . import gaze as gaze_bones
from .gaze import AXES
from .props import redraw_views
from .props import suite_of

# Green for a front she is known to have, orange for the guess made when nothing says.
KNOWN_COLOR = (0.3, 0.9, 0.4, 1.0)
GUESSED_COLOR = (1.0, 0.55, 0.1, 1.0)

_handle = None


def _rotation(armature):
    return armature.matrix_world.to_3x3().normalized()


def world_front(armature, axis):
    """A front picked by hand, in world axes. None while it is left to be worked out."""
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
        bone = armature.data.bones.get(name) if name else None
        if bone is not None:
            return armature.matrix_world @ bone.head_local
    return armature.matrix_world.translation


def _draw():
    context = bpy.context
    suite = getattr(context.scene, "charpaper", None)
    if suite is None or not suite.gaze.show_front or suite.armature is None:
        return
    armature = suite.armature
    local, source = gaze_bones.front(suite.gaze, armature)
    color = GUESSED_COLOR if source == "default" else KNOWN_COLOR
    front = (_rotation(armature) @ local).normalized()
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
        "For when the arrow points wrong. Look her in the face in this viewport, then click: the "
        "way you look at her becomes her front, rounded to the nearest of the armature's axes"
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
        if armature.data.pose_position != "REST":
            gaze.show_front = True
            self.report({"WARNING"}, "This is her rest pose: look her in the face and click again")
            return {"CANCELLED"}
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
