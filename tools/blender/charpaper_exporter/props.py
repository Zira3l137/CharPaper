import re

import bpy
from bpy.props import BoolProperty
from bpy.props import CollectionProperty
from bpy.props import EnumProperty
from bpy.props import FloatProperty
from bpy.props import IntProperty
from bpy.props import PointerProperty
from bpy.props import StringProperty
from bpy.types import PropertyGroup

from . import anim

ORBIT = "orbit"


def suite_of(context):
    return context.scene.charpaper


def file_stem(name):
    return re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", name).strip().rstrip(".") or "unnamed"


def clip_names(suite):
    return [
        clip.name
        for entry in suite.animations
        if entry.enabled
        for clip in entry.clips
        if clip.enabled and clip.action
    ]


def shape_key_names(suite):
    names = set()
    for skin in suite.skins:
        if skin.collection is None:
            continue
        for obj in skin.collection.all_objects:
            if obj.type == "MESH" and obj.data.shape_keys:
                names.update(obj.data.shape_keys.key_blocks.keys())
    return sorted(names)


def _search(items):
    def search(self, context, edit_text):
        text = edit_text.lower()
        return [item for item in items(suite_of(context)) if text in item.lower()]

    return search


def _bone_search(self, context, edit_text):
    armature = suite_of(context).armature
    if armature is None:
        return []
    text = edit_text.lower()
    return [name for name in armature.data.bones.keys() if text in name.lower()]


def _is_armature(self, obj):
    return obj.type == "ARMATURE"


def _is_camera(self, obj):
    return obj.type == "CAMERA"


def _is_armature_action(self, action):
    return anim.is_armature_action(action)


def _is_shape_key_action(self, action):
    return anim.is_shape_key_action(action)


class CHARPAPER_PG_expression(PropertyGroup):
    action: PointerProperty(type=bpy.types.Action, poll=_is_shape_key_action)
    enabled: BoolProperty(name="Export", default=True)


class CHARPAPER_PG_skin(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    collection: PointerProperty(
        type=bpy.types.Collection,
        description="Every mesh in this collection, and the armature, go into the skin",
    )
    expressions: CollectionProperty(type=CHARPAPER_PG_expression)


class CHARPAPER_PG_clip(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    action: PointerProperty(type=bpy.types.Action, poll=_is_armature_action)
    mode: EnumProperty(
        name="Mode",
        items=(
            ("LOOP", "Loop", "Repeats forever"),
            ("ONCE", "Once", "Plays once, then returns to the default animation"),
            ("POSE", "Pose", "Holds the clip's first frame"),
        ),
        default="LOOP",
    )
    gaze: BoolProperty(
        name="Gaze",
        default=True,
        description="Let her follow the cursor during this clip. Turn off for dances or lying poses",
    )
    corrective: PointerProperty(
        type=bpy.types.Action,
        poll=_is_shape_key_action,
        name="Correctives",
        description="Shape key action played together with this clip on the corrective meshes",
    )


class CHARPAPER_PG_animation(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    clips: CollectionProperty(type=CHARPAPER_PG_clip)
    clip_index: IntProperty()
    pattern: StringProperty(
        name="Pattern",
        default="*",
        description="Tick the clips whose action name matches, like Loco_*",
    )
    use_correctives: BoolProperty(
        name="Correctives",
        default=False,
        description="Export corrective meshes and the shape key actions the clips name",
    )
    correctives: PointerProperty(
        type=bpy.types.Collection,
        name="Corrective meshes",
        description="Meshes whose shape keys the clips key. They must be the same objects the skins use",
    )


class CHARPAPER_PG_environment(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    collection: PointerProperty(type=bpy.types.Collection)


class CHARPAPER_PG_camera(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    camera: PointerProperty(type=bpy.types.Object, poll=_is_camera)
    action: PointerProperty(
        type=bpy.types.Action,
        description="Played on loop. Goes on whichever of the camera and its parents uses it",
    )


class CHARPAPER_PG_bone(PropertyGroup):
    bone: StringProperty(name="Bone", search=_bone_search)


class CHARPAPER_PG_gaze(PropertyGroup):
    mode: EnumProperty(
        name="suite.toml",
        items=(
            ("KEEP", "Leave as is", "Don't touch the [gaze] section"),
            ("WRITE", "Write", "Write [gaze] from these fields"),
            ("REMOVE", "Remove", "Remove [gaze]: she never follows the cursor"),
        ),
        default="KEEP",
    )
    eyes: CollectionProperty(type=CHARPAPER_PG_bone)
    head: StringProperty(name="Head", search=_bone_search)
    neck: StringProperty(name="Neck", search=_bone_search)
    eye_yaw: FloatProperty(name="Eye yaw", default=30.0, min=0.0, max=180.0)
    eye_pitch: FloatProperty(name="Eye pitch", default=20.0, min=0.0, max=90.0)
    head_yaw: FloatProperty(name="Head yaw", default=50.0, min=0.0, max=180.0)
    head_pitch: FloatProperty(name="Head pitch", default=25.0, min=0.0, max=90.0)
    neck_yaw: FloatProperty(name="Neck yaw", default=25.0, min=0.0, max=180.0)
    neck_pitch: FloatProperty(name="Neck pitch", default=15.0, min=0.0, max=90.0)
    head_share: FloatProperty(name="Head share", default=0.7, min=0.0, max=1.0, subtype="FACTOR")
    key_left: StringProperty(name="Left", search=_search(shape_key_names))
    key_right: StringProperty(name="Right", search=_search(shape_key_names))
    key_up: StringProperty(name="Up", search=_search(shape_key_names))
    key_down: StringProperty(name="Down", search=_search(shape_key_names))


class CHARPAPER_PG_gltf(PropertyGroup):
    file_format: EnumProperty(
        name="Format",
        items=(
            ("GLB", "GLB", "One binary file"),
            ("GLTF_SEPARATE", "glTF + .bin + textures", "Several files side by side"),
        ),
        default="GLB",
    )
    image_format: EnumProperty(
        name="Images",
        items=(
            ("AUTO", "Automatic (PNG/JPEG)", "Keep each image's format where possible"),
            ("JPEG", "JPEG", "Smaller files, lossy"),
            ("WEBP", "WebP", "Needs Bevy's webp feature"),
            ("NONE", "None", "No images"),
        ),
        default="AUTO",
    )
    apply_modifiers: BoolProperty(name="Apply modifiers", default=True)
    tangents: BoolProperty(name="Tangents", default=True)
    vertex_color: EnumProperty(
        name="Vertex colors",
        items=(
            ("MATERIAL", "Used by materials", ""),
            ("ACTIVE", "Active", ""),
            ("NONE", "None", ""),
        ),
        default="MATERIAL",
    )
    lighting_mode: EnumProperty(
        name="Lights",
        items=(
            ("SPEC", "Standard", "Physical units, what Bevy expects"),
            ("COMPAT", "Unitless", ""),
            ("RAW", "Raw", "Blender strengths unconverted"),
        ),
        default="SPEC",
    )
    deform_bones_only: BoolProperty(name="Deform bones only", default=False)
    frame_step: IntProperty(name="Sampling rate", default=1, min=1, max=120)
    force_sampling: BoolProperty(name="Force sampling", default=True)
    optimize_animation: BoolProperty(name="Optimize keys", default=True)


class CHARPAPER_PG_finding(PropertyGroup):
    severity: StringProperty()
    object_name: StringProperty()
    bone_name: StringProperty()


class CHARPAPER_PG_suite(PropertyGroup):
    folder: StringProperty(name="Folder", subtype="DIR_PATH", default="//charpaper/")
    suite_name: StringProperty(name="Name", description="Name the app shows. Empty: the folder name")
    armature: PointerProperty(type=bpy.types.Object, poll=_is_armature, name="Armature")
    export_model: BoolProperty(
        name="Model file",
        default=True,
        description="Export the armature alone next to suite.toml",
    )
    model_name: StringProperty(name="Model", default="model")
    write_manifest: BoolProperty(name="Write suite.toml", default=True)

    skins: CollectionProperty(type=CHARPAPER_PG_skin)
    skin_index: IntProperty()
    animations: CollectionProperty(type=CHARPAPER_PG_animation)
    animation_index: IntProperty()
    environments: CollectionProperty(type=CHARPAPER_PG_environment)
    environment_index: IntProperty()
    cameras: CollectionProperty(type=CHARPAPER_PG_camera)
    camera_index: IntProperty()

    default_skin: StringProperty(
        name="Skin", search=_search(lambda s: [e.name for e in s.skins])
    )
    default_animation: StringProperty(name="Animation", search=_search(clip_names))
    default_environment: StringProperty(
        name="Environment", search=_search(lambda s: [e.name for e in s.environments])
    )
    default_camera: StringProperty(
        name="Camera", search=_search(lambda s: [ORBIT] + [e.name for e in s.cameras])
    )

    gaze: PointerProperty(type=CHARPAPER_PG_gaze)
    gltf: PointerProperty(type=CHARPAPER_PG_gltf)

    findings: CollectionProperty(type=CHARPAPER_PG_finding)
    finding_index: IntProperty()
    validated: BoolProperty()


classes = (
    CHARPAPER_PG_expression,
    CHARPAPER_PG_skin,
    CHARPAPER_PG_clip,
    CHARPAPER_PG_animation,
    CHARPAPER_PG_environment,
    CHARPAPER_PG_camera,
    CHARPAPER_PG_bone,
    CHARPAPER_PG_gaze,
    CHARPAPER_PG_gltf,
    CHARPAPER_PG_finding,
    CHARPAPER_PG_suite,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.charpaper = PointerProperty(type=CHARPAPER_PG_suite)


def unregister():
    del bpy.types.Scene.charpaper
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
