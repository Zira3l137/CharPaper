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
from . import gaze as gaze_preview

ORBIT = "orbit"


def suite_of(context):
    return context.scene.charpaper


# Gaze needs the front as the character stands in the rest pose, which a clip may turn it away
# from, so while the arrow shows, so does the rest pose.
def _show_front_changed(gaze, context):
    if gaze.show_front and gaze.preview:
        gaze.preview = False
    armature = context.scene.charpaper.armature
    if armature is not None and armature.type == "ARMATURE":
        armature.data.pose_position = "REST" if gaze.show_front else "POSE"
    redraw_views()


# The preview shows what the fields say, so it follows every change to them.
def _gaze_changed(self, context):
    if context.scene.charpaper.gaze.preview:
        gaze_preview.refresh(context)
    redraw_views()


# The arrow shows the rest pose, where a preview would not be seen.
def _preview_changed(gaze, context):
    if gaze.preview and gaze.show_front:
        gaze.show_front = False
    gaze_preview.refresh(context)
    redraw_views()


def redraw_views():
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type == "VIEW_3D":
                area.tag_redraw()


# Blender 5 warns on a "//" path in a field not marked as taking one; 4.2 has no such mark.
RELATIVE_PATH = (
    {"PATH_SUPPORTS_BLEND_RELATIVE"}
    if "is_path_supports_blend_relative" in bpy.types.StringProperty.bl_rna.properties
    else set()
)


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


def _object_slots(action):
    from .actors import object_slots

    return object_slots(action) if action is not None else []


def _is_armature(self, obj):
    return obj.type == "ARMATURE"


def _is_camera(self, obj):
    return obj.type == "CAMERA"


def _is_armature_action(self, action):
    return anim.is_armature_action(action)


def _is_shape_key_action(self, action):
    return anim.is_shape_key_action(action)


# The expression, clip and animation groups below, a skin's expressions and the suite's
# animations and default_animation hold settings from before clips lived on objects. Only
# migrate.py reads them, to move a .blend's settings over the first time it opens.
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
        description="Let the character follow the cursor during this clip. Turn off for dances or lying poses",
    )
    # Named for what it first held; kept so older .blend files keep their choice.
    corrective: PointerProperty(
        type=bpy.types.Action,
        poll=_is_shape_key_action,
        name="Shape keys",
        description=(
            "A shape key action played together with this clip. Leave empty to use the shape "
            "keys keyed in the clip's own action, or none. Driven shape keys need nothing here"
        ),
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


def _keep_path(owner, name):
    from .music import stored_path

    kept = stored_path(getattr(owner, name))
    if kept != getattr(owner, name):
        setattr(owner, name, kept)


def _sound_picked(self, context):
    _keep_path(self, "ambience")
    if self.ambience:
        self.export_sound = True


def _track_picked(self, context):
    _keep_path(self, "filepath")


class CHARPAPER_PG_track(PropertyGroup):
    # `name` is the title the app shows.
    filepath: StringProperty(
        name="File",
        subtype="FILE_PATH",
        options=RELATIVE_PATH,
        description="The sound file, in any format Blender plays",
        update=_track_picked,
    )
    # What the last export wrote and from what, so an unchanged track isn't converted again.
    written: StringProperty(options={"HIDDEN"})
    stamp: StringProperty(options={"HIDDEN"})


class CHARPAPER_PG_environment(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    collection: PointerProperty(type=bpy.types.Collection)
    export_sky: BoolProperty(
        name="Sky",
        default=True,
        description="Export the sky and reflections from a World. Off leaves the panorama in "
        "the environment's folder as it is",
    )
    world: PointerProperty(
        type=bpy.types.World,
        description="The World to export the sky from. Empty takes the scene's, but keeps a "
        "panorama you put in the folder yourself",
    )
    export_sound: BoolProperty(
        name="Sound",
        description="Write the music and ambience below into the environment's folder, replacing "
        "the .ogg files in its music folder and its ambience.ogg. Off leaves them as they are",
    )
    music: CollectionProperty(type=CHARPAPER_PG_track)
    music_index: IntProperty()
    ambience: StringProperty(
        name="Ambience",
        subtype="FILE_PATH",
        options=RELATIVE_PATH,
        description="One sound that loops under the music while this environment shows. Its end "
        "should flow into its start",
        update=_sound_picked,
    )
    ambience_stamp: StringProperty(options={"HIDDEN"})


class CHARPAPER_PG_camera(PropertyGroup):
    enabled: BoolProperty(name="Export", default=True)
    camera: PointerProperty(type=bpy.types.Object, poll=_is_camera)
    action: PointerProperty(
        type=bpy.types.Action,
        description="Played on loop. Goes on whichever of the camera and its parents uses it",
    )


class CHARPAPER_PG_bone(PropertyGroup):
    bone: StringProperty(name="Bone", search=_bone_search, update=_gaze_changed)


class CHARPAPER_PG_gaze(PropertyGroup):
    mode: EnumProperty(
        name="suite.toml",
        items=(
            ("KEEP", "Leave as is", "Don't touch the [gaze] section"),
            ("WRITE", "Write", "Write [gaze] from these fields"),
            ("REMOVE", "Remove", "Remove [gaze]: the character never follows the cursor"),
        ),
        default="KEEP",
    )
    eyes: CollectionProperty(type=CHARPAPER_PG_bone)
    eye_kind: EnumProperty(
        name="Eyes are",
        description="What the eye bones turn. Detect bones sets it from the eyes' shape",
        items=(
            ("BALL", "Eyeballs", "Round balls that turn in their sockets. They are aimed exactly"),
            (
                "DRAWN",
                "Drawn",
                "Anything else, like irises that slide over a flat eye. Aimed, they would shoot "
                "past, so they move in proportion to how far aside the character looks",
            ),
        ),
        default="BALL",
        update=_gaze_changed,
    )
    head: StringProperty(name="Head", search=_bone_search, update=_gaze_changed)
    neck: StringProperty(name="Neck", search=_bone_search, update=_gaze_changed)
    eye_yaw: FloatProperty(
        name="Eye yaw",
        description="How far the eye bones turn sideways",
        default=30.0, min=0.0, max=180.0, update=_gaze_changed,
    )
    eye_pitch: FloatProperty(
        name="Eye pitch",
        description="How far the eye bones turn up or down",
        default=20.0, min=0.0, max=90.0, update=_gaze_changed,
    )
    head_yaw: FloatProperty(
        name="Head yaw",
        description=(
            "How far the head itself turns sideways. With the neck's, it is how far the head may "
            "point from the bone above the neck, whether a clip or gaze turned it there"
        ),
        default=50.0, min=0.0, max=180.0, update=_gaze_changed,
    )
    head_pitch: FloatProperty(
        name="Head pitch",
        description="How far the head itself turns up or down",
        default=25.0, min=0.0, max=90.0, update=_gaze_changed,
    )
    neck_yaw: FloatProperty(
        name="Neck yaw",
        description="How far the neck turns sideways",
        default=25.0, min=0.0, max=180.0, update=_gaze_changed,
    )
    neck_pitch: FloatProperty(
        name="Neck pitch",
        description="How far the neck turns up or down",
        default=15.0, min=0.0, max=90.0, update=_gaze_changed,
    )
    head_share: FloatProperty(
        name="Head share",
        description="The head's share of a turn. The neck takes the rest, up to its limit",
        default=0.7, min=0.0, max=1.0, subtype="FACTOR", update=_gaze_changed,
    )
    head_turn: FloatProperty(
        name="Head turn",
        description=(
            "How much of the way to the cursor the head and neck go; the eyes cover the rest. "
            "At 0 the head only moves for what the eyes can't reach, at 1 it points straight there"
        ),
        default=0.6, min=0.0, max=1.0, subtype="FACTOR", update=_gaze_changed,
    )
    front: EnumProperty(
        name="Front",
        description=(
            "Which way the face points in the rest pose. Automatic works it out from where the "
            "eye bones sit; pick one of the armature's axes only if the arrow points wrong"
        ),
        items=(
            ("NONE", "Automatic", "Worked out from the eye and head bones, as the app does"),
            ("NEG_Y", "-Y", ""),
            ("POS_Y", "+Y", ""),
            ("NEG_X", "-X", ""),
            ("POS_X", "+X", ""),
            ("NEG_Z", "-Z", ""),
            ("POS_Z", "+Z", ""),
        ),
        default="NONE",
        update=_gaze_changed,
    )
    show_front: BoolProperty(
        name="Show front",
        description="Show the rest pose, with an arrow from the head the way gaze takes the character to face",
        default=False,
        update=_show_front_changed,
    )
    key_left: StringProperty(name="Left", search=_search(shape_key_names), update=_gaze_changed)
    key_right: StringProperty(name="Right", search=_search(shape_key_names), update=_gaze_changed)
    key_up: StringProperty(name="Up", search=_search(shape_key_names), update=_gaze_changed)
    key_down: StringProperty(name="Down", search=_search(shape_key_names), update=_gaze_changed)
    preview: BoolProperty(
        name="Preview",
        description=(
            "Pose the character as the app would for the look below, to set the limits by eye. "
            "Its own pose is put back when this is turned off, on a frame change and before saving"
        ),
        default=False,
        update=_preview_changed,
    )
    look_side: FloatProperty(
        name="Sideways",
        description="-1 is as far to the character's right as it can look, 1 as far to its left",
        default=0.0, min=-1.0, max=1.0, update=_gaze_changed,
    )
    look_up: FloatProperty(
        name="Up",
        description="-1 is as far down as the character can look, 1 as far up",
        default=0.0, min=-1.0, max=1.0, update=_gaze_changed,
    )


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
    # The clip it is about, by name, to mark its row.
    clip: StringProperty()
    sound: StringProperty()
    hotspot: StringProperty()
    # The chain or collider it is about, as "chain:Name" or "collider:Bone".
    physics: StringProperty()
    fix: StringProperty()


CLIP_MODES = (
    ("TIMELINE", "Timeline", "Shows the timeline's frame and loops with it, in step with every other clip on it"),
    ("ONCE", "Once", "Plays through from its first frame when started, then hands back to what played before"),
    ("HOLD", "Hold", "Holds its first frame. For still poses"),
)


def _once_for_hotspots(clip, _context):
    # Most clips only hotspots play are played once; a loop for While is set to Timeline after.
    if clip.reaction_only and clip.mode == "TIMELINE":
        clip.mode = "ONCE"


class CHARPAPER_PG_actor_clip(PropertyGroup):
    action: PointerProperty(type=bpy.types.Action, name="Action")
    extra: PointerProperty(
        type=bpy.types.Action,
        name="Also plays",
        description=(
            "A second action played with this clip: its shape keys on Blender older than 4.4, "
            "where they can't share the action, or a lamp's Strength and Color"
        ),
    )
    mode: EnumProperty(name="Plays", items=CLIP_MODES, default="TIMELINE")
    label: StringProperty(name="Shown as", description="Name the app shows. Empty: the action's name")
    gaze: BoolProperty(
        name="Gaze",
        default=True,
        description="Let the character follow the cursor during this clip. Turn off for dances or lying poses",
    )
    slot: StringProperty(
        name="Slot",
        search=lambda self, context, text: [
            s.identifier for s in _object_slots(self.action) if text.lower() in s.identifier.lower()
        ],
        description=(
            "Which of the action's slots this clip plays, for an action holding several for "
            "objects. Empty: its only one"
        ),
    )
    reaction_only: BoolProperty(
        name="Only for hotspots",
        description=(
            "Keep it out of the app's list of animations: a flinch, or a loop for while a hotspot is "
            "held, only a hotspot should play"
        ),
        update=_once_for_hotspots,
    )


class CHARPAPER_PG_actor(PropertyGroup):
    clips: CollectionProperty(type=CHARPAPER_PG_actor_clip)
    clip_index: IntProperty()
    # A prop's starred clip, by clip name: the one it plays by itself. The character's is the
    # suite's default_clip.
    start_clip: StringProperty()


class CHARPAPER_PG_character_expression(PropertyGroup):
    action: PointerProperty(type=bpy.types.Action, poll=_is_shape_key_action, name="Action")
    label: StringProperty(name="Shown as", description="Name the app shows. Empty: the action's name")


def expression_names(suite):
    return [file_stem(e.action.name) for e in suite.expressions if e.action is not None]


def _redraw(self, context):
    redraw_views()


def _area_bone_search(self, context, edit_text):
    if self.armature is None or self.armature.type != "ARMATURE":
        return []
    text = edit_text.lower()
    return [name for name in self.armature.data.bones.keys() if text in name.lower()]


class CHARPAPER_PG_hotspot_area(PropertyGroup):
    armature: PointerProperty(type=bpy.types.Object, poll=_is_armature, name="Armature", update=_redraw)
    bone: StringProperty(name="Bone", search=_area_bone_search, update=_redraw)
    radius: FloatProperty(
        name="Radius",
        description="How far the capsule reaches from the bone on every side, in the armature's units",
        default=0.05,
        min=0.001,
        soft_max=1.0,
        precision=3,
        step=0.5,
        update=_redraw,
    )


class CHARPAPER_PG_hotspot_clip(PropertyGroup):
    actor: PointerProperty(type=bpy.types.Object, name="Actor")
    action: PointerProperty(type=bpy.types.Action, name="Clip")
    from_time: IntProperty(
        name="From the time in a row",
        description=(
            "Play it only from this time in a row on, instead of the clips with a lower number: 3 for a "
            "reaction to the third poke. In a row: going off again within 3 seconds of being able to, "
            "or for Mouse keeps still, without the mouse moving"
        ),
        default=1,
        min=1,
        soft_max=10,
    )


class CHARPAPER_PG_hotspot_entry(PropertyGroup):
    """An animation an actor plays on and on, in a hotspot's While, Then or Only during. No actor:
    the character's."""

    actor: PointerProperty(type=bpy.types.Object, name="Actor")
    action: PointerProperty(type=bpy.types.Action, name="Animation")


TRIGGERS = (
    ("CLICK", "Click", "A left click on its bones, pressed and let go without dragging", "MOUSE_LMB", 0),
    (
        "HOVER",
        "Hover",
        "The cursor resting on its bones for a moment. It goes off again once the cursor has left",
        "MOUSE_MOVE",
        1,
    ),
    (
        "IDLE",
        "Mouse keeps still",
        "The mouse keeping still for a while, and again after as long while it stays still",
        "TIME",
        2,
    ),
    (
        "WELCOME",
        "Mouse comes back",
        "The mouse moving on the desktop after keeping still, or being in other windows, for a while",
        "HAND",
        3,
    ),
)
# What After starts at for each, as the app has it when it's left out.
AFTER_DEFAULTS = {"IDLE": 60.0, "WELCOME": 300.0}


def _trigger_changed(spot, _context):
    defaults = AFTER_DEFAULTS.get(spot.trigger)
    if defaults is not None and spot.after in AFTER_DEFAULTS.values():
        spot.after = defaults
    _redraw(spot, _context)


class CHARPAPER_PG_hotspot(PropertyGroup):
    trigger: EnumProperty(name="When", items=TRIGGERS, default="CLICK", update=_trigger_changed)
    after: FloatProperty(
        name="After, seconds",
        description="How many seconds the mouse keeps still, or stays away, first",
        default=60.0,
        min=1.0,
        soft_max=1800.0,
        precision=0,
    )
    layer: BoolProperty(
        name="Layer",
        description=(
            "Play over the reactions other hotspots set off, moving only what its clips key. Off: "
            "stop them and take their place"
        ),
    )
    expression: StringProperty(
        name="Expression",
        search=_search(expression_names),
        description="The character's expression while its clips play. Empty: none",
    )
    cooldown: FloatProperty(
        name="Cooldown",
        description="Seconds after it goes off before it can again",
        min=0.0,
        soft_max=10.0,
        precision=1,
    )
    areas: CollectionProperty(type=CHARPAPER_PG_hotspot_area)
    area_index: IntProperty(update=_redraw)
    clips: CollectionProperty(type=CHARPAPER_PG_hotspot_clip)
    clip_index: IntProperty()
    # What actors play on and on while it's held, and after it lets go; and what they must be
    # playing for it to go off. Empty During: any.
    meanwhile: CollectionProperty(type=CHARPAPER_PG_hotspot_entry)
    meanwhile_index: IntProperty()
    then: CollectionProperty(type=CHARPAPER_PG_hotspot_entry)
    then_index: IntProperty()
    during: CollectionProperty(type=CHARPAPER_PG_hotspot_entry)
    during_index: IntProperty()


# Starting points for a chain, as (stiffness, drag, gravity, radius).
PHYSICS_PRESETS = {
    "HAIR": (0.1, 0.15, 1.0, 0.02),
    "CLOTH": (0.05, 0.2, 1.0, 0.03),
    "SOFT": (0.4, 0.3, 0.2, 0.0),
    "TAIL": (0.2, 0.1, 0.5, 0.03),
}
PRESET_ITEMS = (
    ("HAIR", "Hair", "Light and loose: ponytails, bangs, braids"),
    ("CLOTH", "Cloth", "Hangs and trails behind: skirts, capes, ribbons"),
    ("SOFT", "Soft", "Springs back to its pose: bouncy accessories, ears"),
    ("TAIL", "Tail", "Holds its shape but swings: tails, long earrings"),
    ("CUSTOM", "Custom", "Your own values"),
)
# Set while a preset writes its values, so they don't turn it into Custom.
_presetting = False


def _preset_picked(chain, _context):
    global _presetting
    values = PHYSICS_PRESETS.get(chain.preset)
    if values is None:
        return
    _presetting = True
    try:
        chain.stiffness, chain.drag, chain.gravity, chain.radius = values
    finally:
        _presetting = False
    redraw_views()


def _physics_tuned(chain, _context):
    if not _presetting and chain.preset != "CUSTOM":
        chain.preset = "CUSTOM"
    redraw_views()


class CHARPAPER_PG_physics_chain(PropertyGroup):
    """Bones of the character's armature that swing together, root first."""

    bones: CollectionProperty(type=CHARPAPER_PG_bone)
    preset: EnumProperty(name="Preset", items=PRESET_ITEMS, default="HAIR", update=_preset_picked)
    stiffness: FloatProperty(
        name="Stiffness",
        description="How strongly it springs back to the animated pose: 0 hangs loose, 1 follows the animation exactly",
        default=0.1, min=0.0, max=1.0, update=_physics_tuned,
    )
    drag: FloatProperty(
        name="Drag",
        description="How quickly it settles: 0 swings on and on, 1 stops dead",
        default=0.15, min=0.0, max=1.0, update=_physics_tuned,
    )
    gravity: FloatProperty(
        name="Gravity",
        description="How heavy it is, as a multiple of Earth's gravity",
        default=1.0, min=0.0, soft_max=2.0, update=_physics_tuned,
    )
    radius: FloatProperty(
        name="Radius",
        description="How far the bones' tails keep from the colliders, in the armature's units",
        default=0.02, min=0.0, soft_max=0.2, precision=3, step=0.5, update=_physics_tuned,
    )


class CHARPAPER_PG_physics_collider(PropertyGroup):
    """A capsule around one bone of the character, from its head to its tail."""

    bone: StringProperty(name="Bone", search=_bone_search, update=_redraw)
    radius: FloatProperty(
        name="Radius",
        description="How far the capsule reaches from the bone on every side, in the armature's units",
        default=0.05, min=0.001, soft_max=0.5, precision=3, step=0.5, update=_redraw,
    )


def character_clip_names(suite):
    armature = suite.armature
    if armature is None:
        return []
    return [file_stem(c.action.name) for c in armature.charpaper.clips if c.action is not None]


DEFAULT_FOLDER = "//charpaper/"


def _settle_folder(suite, _context):
    suite.folder_settled = True


class CHARPAPER_PG_suite(PropertyGroup):
    custom_folder: BoolProperty(
        name="Custom folder",
        description="Export to a folder of your own instead of the app's characters folder",
        update=_settle_folder,
    )
    folder: StringProperty(
        name="Folder",
        subtype="DIR_PATH",
        default=DEFAULT_FOLDER,
        options=RELATIVE_PATH,
        description="The suite's folder, where suite.toml goes",
    )
    character_folder: StringProperty(
        name="Folder name",
        description="The character's folder in the app's characters folder. Empty: its Name, "
        "or the .blend's name, kept from the first export on",
    )
    # Whether custom_folder was worked out for a .blend from before it existed.
    folder_settled: BoolProperty()
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

    expressions: CollectionProperty(type=CHARPAPER_PG_character_expression)
    expression_index: IntProperty()
    default_clip: StringProperty(name="Clip", search=_search(character_clip_names))
    # The actor the Animation panel shows, by its object's name.
    actor: StringProperty()
    # Set once this suite's settings from before clips lived on objects have been moved there.
    migrated: BoolProperty()
    # Set on every scene once the file's hotspot-only clips have been given their modes.
    clip_modes_settled: BoolProperty()
    last_export: StringProperty()
    gaze: PointerProperty(type=CHARPAPER_PG_gaze)
    gltf: PointerProperty(type=CHARPAPER_PG_gltf)

    hotspots: CollectionProperty(type=CHARPAPER_PG_hotspot)
    hotspot_index: IntProperty(update=_redraw)
    show_hotspots: BoolProperty(
        name="Show hotspots",
        default=True,
        update=_redraw,
        description="Draw the hotspots' capsules in the viewport. The picked hotspot is orange",
    )
    # Whether [hotspots] in suite.toml is the panel's to write: once it has had hotspots, or
    # read them from suite.toml. Until then hotspots written there by hand are kept.
    hotspots_owned: BoolProperty()

    physics_chains: CollectionProperty(type=CHARPAPER_PG_physics_chain)
    physics_chain_index: IntProperty(update=_redraw)
    physics_colliders: CollectionProperty(type=CHARPAPER_PG_physics_collider)
    physics_collider_index: IntProperty(update=_redraw)
    show_physics: BoolProperty(
        name="Show physics",
        default=True,
        update=_redraw,
        description="Draw the chains and the colliders' capsules in the viewport",
    )
    # As hotspots_owned, for [physics].
    physics_owned: BoolProperty()

    findings: CollectionProperty(type=CHARPAPER_PG_finding)
    # What the last export did that changes how the result looks, for its popup.
    alerts: CollectionProperty(type=CHARPAPER_PG_finding)
    finding_index: IntProperty()
    validated: BoolProperty()


classes = (
    CHARPAPER_PG_expression,
    CHARPAPER_PG_skin,
    CHARPAPER_PG_clip,
    CHARPAPER_PG_animation,
    CHARPAPER_PG_track,
    CHARPAPER_PG_environment,
    CHARPAPER_PG_camera,
    CHARPAPER_PG_bone,
    CHARPAPER_PG_gaze,
    CHARPAPER_PG_gltf,
    CHARPAPER_PG_finding,
    CHARPAPER_PG_actor_clip,
    CHARPAPER_PG_actor,
    CHARPAPER_PG_character_expression,
    CHARPAPER_PG_hotspot_area,
    CHARPAPER_PG_hotspot_clip,
    CHARPAPER_PG_hotspot_entry,
    CHARPAPER_PG_hotspot,
    CHARPAPER_PG_physics_chain,
    CHARPAPER_PG_physics_collider,
    CHARPAPER_PG_suite,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.charpaper = PointerProperty(type=CHARPAPER_PG_suite)
    bpy.types.Object.charpaper = PointerProperty(type=CHARPAPER_PG_actor)


def unregister():
    del bpy.types.Object.charpaper
    del bpy.types.Scene.charpaper
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
