import textwrap

import bpy
from bpy.types import Panel
from bpy.types import UIList

from . import cues
from .props import suite_of


class _Entries(UIList):
    kind = ""

    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        row.prop(item, "enabled", text="")
        row.prop(item, "name", text="", emboss=False)
        self.draw_target(row, item)
        op = row.operator("charpaper.export", text="", icon="EXPORT", emboss=False)
        op.what = self.kind
        op.index = index

    def draw_target(self, row, item):
        pass


class CHARPAPER_UL_skins(_Entries):
    kind = "SKIN"

    def draw_target(self, row, item):
        row.label(text=item.collection.name if item.collection else "—", icon="OUTLINER_COLLECTION")


class CHARPAPER_UL_animations(_Entries):
    kind = "ANIMATION"

    def draw_target(self, row, item):
        ticked = sum(1 for c in item.clips if c.enabled and c.action)
        row.label(text=f"{ticked} clip{'s' if ticked != 1 else ''}", icon="ACTION")


class CHARPAPER_UL_environments(_Entries):
    kind = "ENVIRONMENT"

    def draw_target(self, row, item):
        row.label(text=item.collection.name if item.collection else "—", icon="OUTLINER_COLLECTION")


class CHARPAPER_UL_cameras(_Entries):
    kind = "CAMERA"

    def draw_target(self, row, item):
        row.label(text=item.camera.name if item.camera else "—", icon="CAMERA_DATA")
        row.label(text=item.action.name if item.action else "static", icon="PLAY")


class CHARPAPER_UL_clips(UIList):
    def draw_item(self, context, layout, data, item, icon, active_data, active_propname, index):
        row = layout.row(align=True)
        row.prop(item, "enabled", text="")
        row.prop(item, "name", text="", emboss=False)
        if item.action is None:
            row.label(text="action deleted", icon="ERROR")
            return
        if item.action.name != item.name:
            row.label(text=item.action.name, icon="ACTION")
        sounds = len(cues.markers(item.action))
        if sounds:
            row.label(text=str(sounds), icon="SPEAKER")


def _list(layout, suite, kind, list_cls, items, index):
    row = layout.row()
    row.template_list(list_cls.__name__, "", suite, items, suite, index, rows=3)
    side = row.column(align=True)
    side.operator("charpaper.list_add", text="", icon="ADD").kind = kind
    side.operator("charpaper.list_remove", text="", icon="REMOVE").kind = kind
    entries = getattr(suite, items)
    current = getattr(suite, index)
    return entries[current] if 0 <= current < len(entries) else None


class _Sidebar:
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "CharPaper"


class CHARPAPER_PT_suite(_Sidebar, Panel):
    bl_label = "Suite"

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        layout.use_property_split = True
        layout.use_property_decorate = False
        row = layout.row(align=True)
        row.prop(suite, "folder")
        row.operator("charpaper.open_folder", text="", icon="FILE_FOLDER")
        layout.prop(suite, "suite_name")
        layout.prop(suite, "armature")
        row = layout.row(heading="Model file")
        row.prop(suite, "export_model", text="")
        sub = row.row()
        sub.active = suite.export_model
        sub.prop(suite, "model_name", text="")

        col = layout.column(heading="Starts with", align=True)
        col.prop(suite, "default_skin")
        col.prop(suite, "default_animation")
        col.prop(suite, "default_environment")
        col.prop(suite, "default_camera")

        layout.use_property_split = False
        row = layout.row(align=True)
        row.prop(suite, "write_manifest")
        row.operator("charpaper.load_manifest", icon="IMPORT")
        row = layout.row(align=True)
        row.scale_y = 1.4
        row.operator("charpaper.validate", icon="VIEWZOOM")
        row.operator("charpaper.export", text="Export All", icon="EXPORT").what = "ALL"


class CHARPAPER_PT_skins(_Sidebar, Panel):
    bl_label = "Skins"
    bl_parent_id = "CHARPAPER_PT_suite"

    def draw_header_preset(self, context):
        self.layout.label(text="→ skins/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(self.layout, suite, "SKIN", CHARPAPER_UL_skins, "skins", "skin_index")
        if entry is None:
            return
        layout = self.layout
        layout.prop(entry, "collection", text="Collection")
        box = layout.box()
        row = box.row()
        row.label(text="Expressions", icon="SHAPEKEY_DATA")
        row.operator("charpaper.sync_expressions", text="", icon="FILE_REFRESH").index = suite.skin_index
        if not entry.expressions:
            box.label(text="None yet: press refresh to list shape key actions")
        flow = box.grid_flow(columns=2, even_columns=True)
        for expression in entry.expressions:
            if expression.action:
                flow.prop(expression, "enabled", text=expression.action.name)


class CHARPAPER_PT_animations(_Sidebar, Panel):
    bl_label = "Animations"
    bl_parent_id = "CHARPAPER_PT_suite"

    def draw_header_preset(self, context):
        self.layout.label(text="→ animations/")

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        entry = _list(layout, suite, "ANIMATION", CHARPAPER_UL_animations, "animations", "animation_index")
        if entry is None:
            return
        index = suite.animation_index
        box = layout.box()
        row = box.row(align=True)
        row.label(text="Clips", icon="ACTION")
        row.operator("charpaper.sync_clips", text="", icon="FILE_REFRESH").index = index
        box.template_list("CHARPAPER_UL_clips", "", entry, "clips", entry, "clip_index", rows=4)
        row = box.row(align=True)
        op = row.operator("charpaper.tick_clips", text="All")
        op.index, op.how = index, "ALL"
        op = row.operator("charpaper.tick_clips", text="None")
        op.index, op.how = index, "NONE"
        row.prop(entry, "pattern", text="")
        op = row.operator("charpaper.tick_clips", text="Match")
        op.index, op.how = index, "MATCH"

        if 0 <= entry.clip_index < len(entry.clips):
            clip = entry.clips[entry.clip_index]
            col = box.column()
            col.use_property_split = True
            col.use_property_decorate = False
            col.prop(clip, "name", text="Shown as")
            col.prop(clip, "action")
            col.prop(clip, "mode")
            col.prop(clip, "gaze")
            sub = col.column()
            sub.active = entry.use_correctives
            sub.prop(clip, "corrective")

        row = layout.row(heading="Correctives")
        row.prop(entry, "use_correctives", text="")
        sub = row.row()
        sub.active = entry.use_correctives
        sub.prop(entry, "correctives", text="")


class CHARPAPER_PT_environments(_Sidebar, Panel):
    bl_label = "Environments"
    bl_parent_id = "CHARPAPER_PT_suite"

    def draw_header_preset(self, context):
        self.layout.label(text="→ environment/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(
            self.layout, suite, "ENVIRONMENT", CHARPAPER_UL_environments, "environments", "environment_index"
        )
        if entry is not None:
            self.layout.prop(entry, "collection", text="Collection")


class CHARPAPER_PT_cameras(_Sidebar, Panel):
    bl_label = "Cameras"
    bl_parent_id = "CHARPAPER_PT_suite"

    def draw_header_preset(self, context):
        self.layout.label(text="→ cameras/")

    def draw(self, context):
        suite = suite_of(context)
        entry = _list(self.layout, suite, "CAMERA", CHARPAPER_UL_cameras, "cameras", "camera_index")
        if entry is not None:
            col = self.layout.column()
            col.prop(entry, "camera", text="Camera")
            col.prop(entry, "action", text="Action")


class CHARPAPER_PT_gaze(_Sidebar, Panel):
    bl_label = "Gaze"
    bl_parent_id = "CHARPAPER_PT_suite"
    bl_options = {"DEFAULT_CLOSED"}

    def draw(self, context):
        gaze = suite_of(context).gaze
        layout = self.layout
        layout.prop(gaze, "mode", expand=True)
        if gaze.mode != "WRITE":
            return
        col = layout.column(align=True)
        col.label(text="Eyes", icon="HIDE_OFF")
        for i, eye in enumerate(gaze.eyes):
            row = col.row(align=True)
            row.prop(eye, "bone", text="")
            row.operator("charpaper.eye_remove", text="", icon="X").index = i
        col.operator("charpaper.eye_add", text="Add eye", icon="ADD")

        col = layout.column()
        col.use_property_split = True
        col.use_property_decorate = False
        col.prop(gaze, "head")
        col.prop(gaze, "neck")

        grid = layout.grid_flow(columns=2, even_columns=True, align=True)
        for part in ("eye", "head", "neck"):
            grid.prop(gaze, part + "_yaw", text=f"{part.title()} yaw °")
            grid.prop(gaze, part + "_pitch", text=f"{part.title()} pitch °")
        row = layout.row()
        row.active = bool(gaze.head and gaze.neck)
        row.prop(gaze, "head_share")

        col = layout.column(align=True)
        col.label(text="Shape keys that follow the eyes", icon="SHAPEKEY_DATA")
        col.use_property_split = True
        col.use_property_decorate = False
        for side in ("left", "right", "up", "down"):
            col.prop(gaze, "key_" + side)


class CHARPAPER_PT_gltf(_Sidebar, Panel):
    bl_label = "glTF Settings"
    bl_parent_id = "CHARPAPER_PT_suite"
    bl_options = {"DEFAULT_CLOSED"}

    def draw(self, context):
        gltf = suite_of(context).gltf
        layout = self.layout
        layout.use_property_split = True
        layout.use_property_decorate = False
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


class CHARPAPER_PT_validation(_Sidebar, Panel):
    bl_label = "Validation"
    bl_parent_id = "CHARPAPER_PT_suite"

    def draw_header_preset(self, context):
        suite = suite_of(context)
        if not suite.validated:
            return
        warnings = sum(1 for f in suite.findings if f.severity == "WARNING")
        if warnings:
            self.layout.label(text=f"{warnings} warning{'s' if warnings != 1 else ''}", icon="ERROR")
        else:
            self.layout.label(text="clean", icon="CHECKMARK")

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        if not suite.validated:
            layout.label(text="Not checked yet. Validate or export to check.")
            return
        if not suite.findings:
            layout.label(text="Nothing to warn about", icon="CHECKMARK")
            return
        width = max(20, int(context.region.width / 7.5) - 6)
        for i, finding in enumerate(suite.findings):
            box = layout.box()
            col = box.column(align=True)
            lines = textwrap.wrap(finding.name, width) or [""]
            row = col.row()
            row.label(text=lines[0], icon="ERROR" if finding.severity == "WARNING" else "INFO")
            if finding.object_name:
                row.operator("charpaper.show_finding", text="", icon="RESTRICT_SELECT_OFF", emboss=False).index = i
            for line in lines[1:]:
                col.label(text=line, icon="BLANK1")


classes = (
    CHARPAPER_UL_skins,
    CHARPAPER_UL_animations,
    CHARPAPER_UL_environments,
    CHARPAPER_UL_cameras,
    CHARPAPER_UL_clips,
    CHARPAPER_PT_suite,
    CHARPAPER_PT_skins,
    CHARPAPER_PT_animations,
    CHARPAPER_PT_environments,
    CHARPAPER_PT_cameras,
    CHARPAPER_PT_gaze,
    CHARPAPER_PT_gltf,
    CHARPAPER_PT_validation,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
