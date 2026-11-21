import textwrap

import bpy
from bpy.types import Menu
from bpy.types import Panel
from bpy.types import UIList

from . import actors
from . import anim
from . import clips
from . import cues
from . import drivers
from . import env_settings
from .operators import current_actor
from .props import file_stem
from .props import suite_of

MODE_ICONS = {"TIMELINE": "TIME", "ONCE": "PLAY", "HOLD": "PAUSE"}
MODE_NAMES = {"TIMELINE": "Timeline", "ONCE": "Once", "HOLD": "Hold"}
LAYERED = bpy.app.version >= (4, 4, 0)


def warnings_about(suite, obj_name="", clip=""):
    return [
        f
        for f in suite.findings
        if f.severity == "WARNING"
        and (not obj_name or f.object_name == obj_name)
        and (not clip or f.clip == clip)
    ]


def _warn_icon(row, suite, obj_name="", clip=""):
    if warnings_about(suite, obj_name, clip):
        row.label(text="", icon="ERROR")


def _wrapped(layout, text, context, icon="NONE"):
    width = max(20, int(context.region.width / 7.5) - 6)
    col = layout.column(align=True)
    for i, line in enumerate(textwrap.wrap(text, width) or [""]):
        col.label(text=line, icon=icon if i == 0 else ("BLANK1" if icon != "NONE" else "NONE"))


def _list(layout, suite, kind, list_cls, items, index, rows=3):
    row = layout.row()
    row.template_list(list_cls.__name__, "", suite, items, suite, index, rows=rows)
    side = row.column(align=True)
    side.operator("charpaper.list_add", text="", icon="ADD").kind = kind
    side.operator("charpaper.list_remove", text="", icon="REMOVE").kind = kind
    entries = getattr(suite, items)
    current = getattr(suite, index)
    return entries[current] if 0 <= current < len(entries) else None


class _Entries(UIList):
    kind = ""

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        row.prop(item, "enabled", text="")
        row.prop(item, "name", text="", emboss=False)
        self.draw_target(context, row, item)
        op = row.operator("charpaper.export", text="", icon="EXPORT", emboss=False)
        op.what = self.kind
        op.index = index

    def draw_target(self, context, row, item):
        pass


class CHARPAPER_UL_skins(_Entries):
    kind = "SKIN"

    def draw_target(self, context, row, item):
        suite = suite_of(context)
        if item.collection and any(warnings_about(suite, o.name) for o in item.collection.all_objects):
            row.label(text="", icon="ERROR")
        row.label(text=item.collection.name if item.collection else "-", icon="OUTLINER_COLLECTION")


class CHARPAPER_UL_environments(_Entries):
    kind = "ENVIRONMENT"

    def draw_target(self, context, row, item):
        row.label(text=item.collection.name if item.collection else "-", icon="OUTLINER_COLLECTION")


class CHARPAPER_UL_cameras(_Entries):
    kind = "CAMERA"

    def draw_target(self, context, row, item):
        row.label(text="moves" if item.action else "still", icon="PLAY" if item.action else "CAMERA_DATA")


class CHARPAPER_UL_expressions(UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        if item.action is None:
            row.label(text="Pick an action", icon="ERROR")
            return
        row.label(text=item.action.name, icon="SHAPEKEY_DATA")
        _warn_icon(row, suite_of(context), clip=file_stem(item.action.name))
        if item.label:
            row.label(text=item.label)
        row.label(text=f"{len(anim.key_names(item.action))} keys")


class CHARPAPER_UL_clips(UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        suite = suite_of(context)
        row = layout.row(align=True)
        if data == getattr(suite.armature, "charpaper", None):
            name = file_stem(item.action.name) if item.action else ""
            starred = bool(name) and suite.default_clip == name
            star = row.operator(
                "charpaper.clip_star", text="", icon="SOLO_ON" if starred else "SOLO_OFF", emboss=False
            )
            star.index = index
        if item.action is None:
            row.label(text="Pick an action", icon="ERROR")
            return
        row.label(text=item.action.name)
        if item.label:
            row.label(text=item.label)
        _warn_icon(row, suite, clip=file_stem(item.action.name))
        sounds = len(cues.markers(item.action))
        if sounds:
            row.label(text=str(sounds), icon="SPEAKER")
        row.label(text=MODE_NAMES[item.mode], icon=MODE_ICONS[item.mode])


class CHARPAPER_MT_add_clip(Menu):
    bl_label = "Add a clip"
    bl_description = "Actions that key this actor's bones, shape keys, light or transform"

    def draw(self, context):
        suite = suite_of(context)
        actor = current_actor(suite)
        if actor is None:
            return
        listed = {c.action for c in actors.clips(actor)}
        offered = [a for a in actors.fitting_actions(actor, suite) if a not in listed]
        if not offered:
            self.layout.label(text="No other action fits this actor")
        for action in offered:
            self.layout.operator("charpaper.clip_add", text=action.name, icon="ACTION").action = action.name


class _Sidebar:
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "CharPaper"


def _split(layout):
    layout.use_property_split = True
    layout.use_property_decorate = False
    return layout


class CHARPAPER_PT_suite(_Sidebar, Panel):
    bl_label = "Suite"

    def draw_header_preset(self, context):
        warnings = len(warnings_about(suite_of(context)))
        if warnings:
            self.layout.label(text=str(warnings), icon="ERROR")

    def draw(self, context):
        suite = suite_of(context)
        scene = context.scene
        layout = _split(self.layout)
        row = layout.row(align=True)
        row.prop(suite, "folder")
        row.operator("charpaper.open_folder", text="", icon="FILE_FOLDER")
        layout.prop(suite, "suite_name")

        box = layout.box()
        frames = scene.frame_end - scene.frame_start + 1
        fps = clips.fps(scene)
        box.label(text="Timeline", icon="TIME")
        box.label(text=f"Frames {scene.frame_start}–{scene.frame_end} at {fps:g} fps: a {frames / fps:.1f} s loop")
        hint = box.row()
        hint.active = False
        hint.label(text="From Output Properties. Every Timeline clip plays on it.")

        col = layout.column(heading="Starts with", align=True)
        col.prop(suite, "default_skin")
        col.prop(suite, "default_clip")
        col.prop(suite, "default_environment")
        col.prop(suite, "default_camera")

        layout.use_property_split = False
        row = layout.row(align=True)
        row.prop(suite, "write_manifest")
        row.operator("charpaper.load_manifest", icon="IMPORT")
        row = layout.row(align=True)
        row.scale_y = 1.4
        row.operator("charpaper.validate", text="Check", icon="VIEWZOOM")
        row.operator("charpaper.export", text="Export all", icon="EXPORT").what = "ALL"
        if suite.last_export:
            row = layout.row()
            row.active = False
            row.label(text=f"Last export {suite.last_export}", icon="CHECKMARK")


class CHARPAPER_PT_character(_Sidebar, Panel):
    bl_label = "Character"

    def draw_header_preset(self, context):
        suite = suite_of(context)
        self.layout.label(text=f"{suite.default_skin or '-'} · {len(suite.skins)} skins")

    def draw(self, context):
        suite = suite_of(context)
        layout = _split(self.layout)
        layout.prop(suite, "armature")
        row = layout.row(heading="Model file")
        row.prop(suite, "export_model", text="")
        sub = row.row()
        sub.active = suite.export_model
        sub.prop(suite, "model_name", text="")


class CHARPAPER_PT_skins(_Sidebar, Panel):
    bl_label = "Skins"
    bl_parent_id = "CHARPAPER_PT_character"

    def draw_header_preset(self, context):
        self.layout.label(text="→ skins/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(self.layout, suite, "SKIN", CHARPAPER_UL_skins, "skins", "skin_index")
        if entry is None:
            return
        layout = self.layout
        layout.prop(entry, "collection", text="Collection")
        if entry.collection is None:
            return
        meshes = [o for o in entry.collection.all_objects if actors.shape_keys_of(o)]
        keys = sum(len(actors.shape_keys_of(o).key_blocks) - 1 for o in meshes)
        driven, _ = drivers.driven_count(entry.collection.all_objects)
        box = layout.box()
        box.label(text=f"{keys} shape keys on {len(meshes)} meshes · {driven} driven", icon="SHAPEKEY_DATA")
        hint = box.column()
        hint.active = False
        _wrapped(hint, "Exported with its sliders as they are: no action may play on its shape keys.", context)
        for finding in warnings_about(suite):
            if any(finding.object_name == o.name for o in entry.collection.all_objects):
                _wrapped(layout.box(), finding.name, context, icon="ERROR")


class CHARPAPER_PT_expressions(_Sidebar, Panel):
    bl_label = "Expressions"
    bl_parent_id = "CHARPAPER_PT_character"

    def draw_header_preset(self, context):
        self.layout.label(text="→ expressions/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(
            self.layout, suite, "EXPRESSION", CHARPAPER_UL_expressions, "expressions", "expression_index"
        )
        if entry is None:
            return
        layout = _split(self.layout)
        layout.prop(entry, "action")
        layout.prop(entry, "label")
        if entry.action is None:
            return
        keyed = anim.key_names(entry.action)
        box = layout.box()
        _wrapped(box, "Touches " + ", ".join(sorted(keyed)), context, icon="SHAPEKEY_DATA")
        for skin in suite.skins:
            if skin.collection is None:
                continue
            meshes = [
                o.name
                for o in skin.collection.all_objects
                if actors.shape_keys_of(o) and keyed & set(actors.shape_keys_of(o).key_blocks.keys())
            ]
            _wrapped(box, f"{skin.name}: {', '.join(meshes) or 'none of its meshes'}", context)
        hint = box.row()
        hint.active = False
        hint.label(text="Matched by shape key name on whichever skin is worn.")


class CHARPAPER_PT_animation(_Sidebar, Panel):
    bl_label = "Animation"

    def draw_header_preset(self, context):
        suite = suite_of(context)
        self.layout.label(text=f"{len(actors.all_actors(suite))} actors")

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        found = actors.all_actors(suite)
        if not found:
            layout.label(text="Pick the character's armature first", icon="INFO")
            return
        actor = current_actor(suite)

        box = layout.box()
        col = box.column(align=True)
        group = object()
        for each in found:
            heading = "Character" if each.is_character else each.environment.name
            if heading != group:
                group = heading
                label = col.row()
                label.active = False
                label.label(text=heading.upper() if each.is_character else heading)
            row = col.row(align=True)
            icon = {"ARMATURE": "ARMATURE_DATA", "LIGHT": "LIGHT"}.get(each.obj.type, "OBJECT_DATA")
            selected = each.name == actor.name
            op = row.operator("charpaper.actor_select", text=each.name, icon=icon, depress=selected, emboss=selected)
            op.name = each.name
            _warn_icon(row, suite, each.name)
            count = len(actors.clips(each))
            row.label(text=f"{count} clip{'s' if count != 1 else ''}")

        where = clips.clip_folder(actor).replace("\\", "/") + "/"
        layout.label(text=f"Clips of {actor.name} → {where}", icon="ACTION")
        stored = actor.obj.charpaper.clips
        if len(stored) == 0 and not actor.is_character:
            implied = actors.clips(actor)
            box = layout.box()
            if implied:
                _wrapped(box, f"Plays its active action {implied[0].action.name} on the timeline.", context, icon="TIME")
            else:
                _wrapped(box, "Plays nothing yet: it has no action and no clips.", context, icon="INFO")
            row = box.row(align=True)
            if implied:
                row.operator("charpaper.clip_edit", icon="PRESET")
            row.menu("CHARPAPER_MT_add_clip", text="Add", icon="ADD")
            return

        row = layout.row()
        row.template_list("CHARPAPER_UL_clips", "", actor.obj.charpaper, "clips", actor.obj.charpaper, "clip_index", rows=4)
        side = row.column(align=True)
        side.menu("CHARPAPER_MT_add_clip", text="", icon="ADD")
        side.operator("charpaper.clip_remove", text="", icon="REMOVE")
        row = layout.row(align=True)
        row.menu("CHARPAPER_MT_add_clip", text="Add", icon="ADD")
        row.operator("charpaper.clip_add_all", icon="ADD")
        op = row.operator("charpaper.export", text="", icon="EXPORT")
        op.what, op.actor = "ACTOR", actor.name

        index = actor.obj.charpaper.clip_index
        if not 0 <= index < len(stored):
            return
        clip = stored[index]
        box = _split(layout.box())
        box.prop(clip, "action")
        box.prop(clip, "label")
        box.row().prop(clip, "mode", expand=True)
        if actor.is_character:
            box.prop(clip, "gaze")
        if clip.extra or actor.obj.type == "LIGHT" or not LAYERED:
            box.prop(clip, "extra")
        if clip.action is None:
            return
        view = actors.Clip(clip.action, clip.extra, clip.mode)
        nodes = clips.keyed_nodes(actor, clip.action, suite)
        keys = clips.shape_key_curves(view)
        start, end = clips.span(context.scene, view)
        sounds = cues.within(clip.action, start, end, clips.fps(context.scene))
        parts = [f"{len(nodes)} {'node' if len(nodes) == 1 else 'nodes'}"]
        if keys:
            parts.append(f"{len(keys)} shape keys ({', '.join(sorted(keys)[:4])}{'…' if len(keys) > 4 else ''})")
        if sounds:
            parts.append(f"{len(sounds)} sounds: " + ", ".join(f"{s} {at:g} s" for at, s in sounds[:3]))
        hint = box.column()
        hint.active = False
        _wrapped(hint, f"Frames {start}–{end} · keys " + " · ".join(parts), context)
        for finding in warnings_about(suite, clip=file_stem(clip.action.name)):
            _wrapped(layout.box(), finding.name, context, icon="ERROR")


class CHARPAPER_PT_environments(_Sidebar, Panel):
    bl_label = "Environments"

    def draw_header_preset(self, context):
        self.layout.label(text="→ environment/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(
            self.layout, suite, "ENVIRONMENT", CHARPAPER_UL_environments, "environments", "environment_index"
        )
        if entry is None:
            return
        col = self.layout.column()
        col.prop(entry, "collection", text="Collection")
        row = col.row(align=True)
        row.prop(entry, "export_sky", text="")
        world = row.row(align=True)
        world.active = entry.export_sky
        world.prop(entry, "world", text="Sky")
        if entry.export_sky and entry.world is None:
            scene_world = context.scene.world
            hint = f"From the scene: {scene_world.name}" if scene_world else "No World in the scene"
            col.label(text=hint, icon="WORLD")
        if entry.collection is None:
            return
        objects = [o for o in entry.collection.all_objects if o.type != "CAMERA"]
        lights = [o for o in objects if o.type == "LIGHT"]
        found = actors.environment_actors(entry)
        box = col.box()
        box.label(text=f"Found in {entry.collection.name}")
        if found:
            row = box.row(align=True)
            row.label(text=f"{len(found)} actors: " + ", ".join(a.name for a in found[:4]), icon="ACTION")
            row.operator("charpaper.actor_select", text="", icon="FORWARD").name = found[0].name
        unshadowed = sum(1 for o in lights if not o.data.use_shadow)
        unlit_fog = sum(1 for o in lights if o.data.volume_factor <= 0.0)
        box.label(text=f"{len(lights)} lights · {unshadowed} without shadows · {unlit_fog} not lighting fog", icon="LIGHT")
        fog = env_settings.fog_objects(objects)
        if fog:
            box.label(text=f"{len(fog)} fog boxes: " + ", ".join(o.name for o in fog), icon="MOD_FLUID")


class CHARPAPER_PT_cameras(_Sidebar, Panel):
    bl_label = "Cameras"

    def draw_header_preset(self, context):
        self.layout.label(text="→ cameras/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(self.layout, suite, "CAMERA", CHARPAPER_UL_cameras, "cameras", "camera_index")
        if entry is None:
            return
        col = self.layout.column()
        col.prop(entry, "camera", text="Camera")
        col.prop(entry, "action", text="Moves with")
        if entry.camera is None:
            return
        dof = entry.camera.data.dof
        box = col.box()
        if entry.action:
            box.label(text=f"Moves with {entry.action.name} on the timeline", icon="TIME")
        if dof.use_dof:
            focus = dof.focus_object.name if dof.focus_object else f"{dof.focus_distance:g} m"
            box.label(text=f"Lens from Depth of Field: f/{dof.aperture_fstop:g}, focus on {focus}", icon="CAMERA_DATA")
        else:
            box.label(text="Depth of Field off: the whole image is sharp", icon="CAMERA_DATA")


class CHARPAPER_PT_hotspots(_Sidebar, Panel):
    bl_label = "Hotspots"
    bl_options = {"DEFAULT_CLOSED"}

    def draw(self, context):
        row = self.layout.row()
        row.active = False
        row.label(text="Clicks and hovers that play clips come with reactions.", icon="INFO")


class CHARPAPER_PT_gaze(_Sidebar, Panel):
    bl_label = "Gaze"
    bl_parent_id = "CHARPAPER_PT_character"
    bl_options = {"DEFAULT_CLOSED"}

    def draw_header_preset(self, context):
        gaze = suite_of(context).gaze
        eyes = sum(1 for e in gaze.eyes if e.bone)
        parts = [f"{eyes} eyes"] if eyes else []
        parts += [p for p in ("head" if gaze.head else "", "neck" if gaze.neck else "") if p]
        self.layout.label(text=" · ".join(parts) if gaze.mode == "WRITE" else gaze.mode.lower().replace("_", " "))

    def draw(self, context):
        gaze = suite_of(context).gaze
        layout = self.layout
        layout.prop(gaze, "mode", expand=True)
        if gaze.mode != "WRITE":
            return
        layout.operator("charpaper.gaze_detect", icon="VIEWZOOM")

        col = layout.column(align=True)
        col.label(text="Eyes", icon="HIDE_OFF")
        for i, eye in enumerate(gaze.eyes):
            row = col.row(align=True)
            row.prop(eye, "bone", text="")
            row.operator("charpaper.eye_remove", text="", icon="X").index = i
        col.operator("charpaper.eye_add", text="Add eye", icon="ADD")
        row = layout.row()
        row.active = any(eye.bone for eye in gaze.eyes)
        row.prop(gaze, "eye_kind", expand=True)

        col = _split(layout.column())
        col.prop(gaze, "head")
        col.prop(gaze, "neck")

        col = layout.column(align=True)
        col.label(text="Shape keys that follow the eyes", icon="SHAPEKEY_DATA")
        _split(col)
        for side in ("left", "right", "up", "down"):
            col.prop(gaze, "key_" + side)

        box = layout.box()
        box.prop(gaze, "preview", toggle=True, icon="HIDE_OFF")
        col = box.column(align=True)
        col.active = gaze.preview
        col.prop(gaze, "look_side", slider=True)
        col.prop(gaze, "look_up", slider=True)

        grid = layout.grid_flow(columns=2, even_columns=True, align=True)
        for part in ("eye", "head", "neck"):
            grid.prop(gaze, part + "_yaw", text=f"{part.title()} yaw °")
            grid.prop(gaze, part + "_pitch", text=f"{part.title()} pitch °")
        col = layout.column(align=True)
        col.prop(gaze, "head_turn")
        row = col.row(align=True)
        row.active = bool(gaze.head and gaze.neck)
        row.prop(gaze, "head_share")

        col = layout.column(align=True)
        col.label(text="Front, in the rest pose", icon="ORIENTATION_VIEW")
        row = col.row(align=True)
        row.prop(gaze, "front", text="")
        row.prop(gaze, "show_front", text="", icon="EMPTY_SINGLE_ARROW")
        row.operator("charpaper.gaze_front_from_view", text="", icon="HIDE_OFF")


class CHARPAPER_PT_warnings(_Sidebar, Panel):
    bl_label = "Warnings"

    def draw_header_preset(self, context):
        suite = suite_of(context)
        if not suite.validated:
            return
        warnings = len(warnings_about(suite))
        if warnings:
            self.layout.label(text=str(warnings), icon="ERROR")
        else:
            self.layout.label(text="clean", icon="CHECKMARK")

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        if not suite.validated:
            layout.label(text="Not checked yet. Check or export to see.")
            return
        if not suite.findings:
            layout.label(text="Nothing to warn about", icon="CHECKMARK")
            return
        for i, finding in enumerate(suite.findings):
            box = layout.box()
            row = box.row()
            col = row.column()
            _wrapped(col, finding.name, context, icon="ERROR" if finding.severity == "WARNING" else "INFO")
            if finding.object_name:
                row.operator("charpaper.show_finding", text="", icon="RESTRICT_SELECT_OFF", emboss=False).index = i


class CHARPAPER_PT_gltf(_Sidebar, Panel):
    bl_label = "Export settings"
    bl_options = {"DEFAULT_CLOSED"}

    def draw_header_preset(self, context):
        self.layout.label(text="glTF")

    def draw(self, context):
        gltf = suite_of(context).gltf
        layout = _split(self.layout)
        layout.prop(gltf, "file_format")
        layout.prop(gltf, "image_format")
        layout.prop(gltf, "apply_modifiers")
        layout.prop(gltf, "tangents")
        layout.prop(gltf, "vertex_color")
        layout.prop(gltf, "lighting_mode")
        layout.prop(gltf, "deform_bones_only")
        col = layout.column(heading="Animation")
        col.prop(gltf, "frame_step")
        col.prop(gltf, "force_sampling")
        col.prop(gltf, "optimize_animation")
        box = layout.box()
        box.label(text="Always on: +Y Up, Use Rest Position, Shape Keys", icon="LOCKED")
        box.label(text="Always off: Draco and meshopt compression", icon="LOCKED")
        box.label(text="Clips and moving cameras are always .glb", icon="LOCKED")


classes = (
    CHARPAPER_UL_skins,
    CHARPAPER_UL_environments,
    CHARPAPER_UL_cameras,
    CHARPAPER_UL_expressions,
    CHARPAPER_UL_clips,
    CHARPAPER_MT_add_clip,
    CHARPAPER_PT_suite,
    CHARPAPER_PT_character,
    CHARPAPER_PT_skins,
    CHARPAPER_PT_expressions,
    CHARPAPER_PT_gaze,
    CHARPAPER_PT_animation,
    CHARPAPER_PT_environments,
    CHARPAPER_PT_cameras,
    CHARPAPER_PT_hotspots,
    CHARPAPER_PT_warnings,
    CHARPAPER_PT_gltf,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
