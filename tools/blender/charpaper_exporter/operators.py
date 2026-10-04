import fnmatch
import os
import traceback

import bpy
from bpy.props import EnumProperty
from bpy.props import IntProperty
from bpy.types import Operator

from . import anim
from . import exporters
from . import gltf
from . import manifest
from . import toml_io
from . import validate
from .isolate import object_mode
from .props import suite_of

LISTS = {
    "SKIN": ("skins", "skin_index"),
    "ANIMATION": ("animations", "animation_index"),
    "ENVIRONMENT": ("environments", "environment_index"),
    "CAMERA": ("cameras", "camera_index"),
}
KIND_ITEMS = tuple((k, k.title(), "") for k in LISTS)


def _store(suite, findings):
    suite.findings.clear()
    for finding in findings:
        item = suite.findings.add()
        item.name = finding.message
        item.severity = finding.severity
        item.object_name = finding.object_name
        item.bone_name = finding.bone_name
    suite.validated = True


def _report_findings(op, findings):
    warnings = sum(1 for f in findings if f.severity == "WARNING")
    if warnings:
        op.report({"WARNING"}, f"{warnings} warning(s), see Validation")


class CHARPAPER_OT_list_add(Operator):
    bl_idname = "charpaper.list_add"
    bl_label = "Add"
    bl_description = "Add an entry. Picks up the active collection or object"
    bl_options = {"REGISTER", "UNDO"}

    kind: EnumProperty(items=KIND_ITEMS)

    def execute(self, context):
        suite = suite_of(context)
        items, index = LISTS[self.kind]
        collection = getattr(suite, items)
        item = collection.add()
        active_collection = context.collection
        if self.kind in {"SKIN", "ENVIRONMENT"} and active_collection != context.scene.collection:
            item.collection = active_collection
            item.name = active_collection.name.lower()
        elif self.kind == "CAMERA" and context.object and context.object.type == "CAMERA":
            item.camera = context.object
            item.name = context.object.name.lower()
            data = context.object.animation_data
            item.action = data.action if data else None
        else:
            item.name = f"{self.kind.lower()}{len(collection)}"
        setattr(suite, index, len(collection) - 1)
        return {"FINISHED"}


class CHARPAPER_OT_list_remove(Operator):
    bl_idname = "charpaper.list_remove"
    bl_label = "Remove"
    bl_options = {"REGISTER", "UNDO"}

    kind: EnumProperty(items=KIND_ITEMS)

    def execute(self, context):
        suite = suite_of(context)
        items, index = LISTS[self.kind]
        collection = getattr(suite, items)
        current = getattr(suite, index)
        if 0 <= current < len(collection):
            collection.remove(current)
            setattr(suite, index, max(0, current - 1))
        return {"FINISHED"}


class CHARPAPER_OT_sync_clips(Operator):
    bl_idname = "charpaper.sync_clips"
    bl_label = "Sync clips"
    bl_description = "List every action that animates the armature. New ones are ticked if they match the pattern"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()

    def execute(self, context):
        suite = suite_of(context)
        if suite.armature is None:
            self.report({"WARNING"}, "Pick the armature first")
            return {"CANCELLED"}
        entry = suite.animations[self.index]
        for i in reversed(range(len(entry.clips))):
            if entry.clips[i].action is None:
                entry.clips.remove(i)
        known = {clip.action for clip in entry.clips}
        for action in sorted(anim.armature_actions(suite.armature), key=lambda a: a.name.lower()):
            if action not in known:
                clip = entry.clips.add()
                clip.action = action
                clip.name = action.name
                clip.enabled = fnmatch.fnmatchcase(action.name, entry.pattern or "*")
        return {"FINISHED"}


class CHARPAPER_OT_tick_clips(Operator):
    bl_idname = "charpaper.tick_clips"
    bl_label = "Tick clips"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()
    how: EnumProperty(items=(("ALL", "All", ""), ("NONE", "None", ""), ("MATCH", "Match", "")))

    def execute(self, context):
        entry = suite_of(context).animations[self.index]
        for clip in entry.clips:
            if self.how == "MATCH":
                clip.enabled = bool(clip.action) and fnmatch.fnmatchcase(clip.action.name, entry.pattern or "*")
            else:
                clip.enabled = self.how == "ALL"
        return {"FINISHED"}


class CHARPAPER_OT_sync_expressions(Operator):
    bl_idname = "charpaper.sync_expressions"
    bl_label = "Sync expressions"
    bl_description = "List every shape key action that keys this skin's meshes. New ones start unticked"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()

    def execute(self, context):
        entry = suite_of(context).skins[self.index]
        if entry.collection is None:
            self.report({"WARNING"}, "Pick the skin's collection first")
            return {"CANCELLED"}
        for i in reversed(range(len(entry.expressions))):
            if entry.expressions[i].action is None:
                entry.expressions.remove(i)
        known = {e.action for e in entry.expressions}
        meshes = [o for o in entry.collection.all_objects if o.type == "MESH"]
        for action in sorted(anim.shape_key_actions(meshes), key=lambda a: a.name.lower()):
            if action not in known:
                item = entry.expressions.add()
                item.action = action
                item.enabled = False
        return {"FINISHED"}


class CHARPAPER_OT_eye_add(Operator):
    bl_idname = "charpaper.eye_add"
    bl_label = "Add eye"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        suite_of(context).gaze.eyes.add()
        return {"FINISHED"}


class CHARPAPER_OT_eye_remove(Operator):
    bl_idname = "charpaper.eye_remove"
    bl_label = "Remove eye"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()

    def execute(self, context):
        suite_of(context).gaze.eyes.remove(self.index)
        return {"FINISHED"}


class CHARPAPER_OT_validate(Operator):
    bl_idname = "charpaper.validate"
    bl_label = "Validate"
    bl_description = "Check the scene for things that would export wrong. Never blocks an export"

    def execute(self, context):
        suite = suite_of(context)
        findings = validate.check(context, suite)
        _store(suite, findings)
        if findings:
            _report_findings(self, findings)
        else:
            self.report({"INFO"}, "No problems found")
        return {"FINISHED"}


class CHARPAPER_OT_export(Operator):
    bl_idname = "charpaper.export"
    bl_label = "Export"
    bl_description = "Export to the suite folder and update suite.toml"

    what: EnumProperty(
        items=(
            ("ALL", "All", ""),
            ("MODEL", "Model", ""),
            ("SKIN", "Skin", ""),
            ("ANIMATION", "Animation", ""),
            ("ENVIRONMENT", "Environment", ""),
            ("CAMERA", "Camera", ""),
        )
    )
    index: IntProperty(default=-1)

    def _jobs(self, suite):
        def pick(kind, entries):
            if self.what == "ALL":
                return [(kind, e) for e in entries if e.enabled]
            if self.what == kind and 0 <= self.index < len(entries):
                return [(kind, entries[self.index])]
            return []

        jobs = []
        if self.what in {"ALL", "MODEL"} and suite.export_model and suite.armature:
            jobs.append(("MODEL", None))
        jobs += pick("SKIN", suite.skins)
        jobs += pick("ANIMATION", suite.animations)
        jobs += pick("ENVIRONMENT", suite.environments)
        jobs += pick("CAMERA", suite.cameras)
        return jobs

    def execute(self, context):
        suite = suite_of(context)
        if not gltf.available():
            self.report({"ERROR"}, "The glTF exporter add-on is disabled")
            return {"CANCELLED"}
        if suite.folder.startswith("//") and not bpy.data.filepath:
            self.report({"ERROR"}, "Save the .blend first, or pick an absolute folder")
            return {"CANCELLED"}
        folder = os.path.normpath(bpy.path.abspath(suite.folder))
        os.makedirs(folder, exist_ok=True)

        findings = validate.check(context, suite)
        _store(suite, findings)

        done, failed, exported_animations, model_exported = [], [], [], False
        with object_mode(context):
            for kind, entry in self._jobs(suite):
                label = entry.name if entry else "model"
                missing = self._missing(suite, kind, entry)
                if missing:
                    failed.append(f"{label}: {missing}")
                    continue
                try:
                    result = self._export(context, suite, kind, entry, folder)
                except Exception as error:
                    traceback.print_exc()
                    failed.append(f"{label}: {error}")
                    continue
                done.append(result.path)
                for note in result.notes:
                    self.report({"WARNING"}, note)
                if result.skipped:
                    print(f"CharPaper: {os.path.basename(result.path)}: exporter lacks {', '.join(result.skipped)}")
                if kind == "ANIMATION":
                    exported_animations.append(entry)
                model_exported |= kind == "MODEL"

        if suite.write_manifest and done:
            try:
                self._write_manifest(suite, folder, exported_animations, model_exported)
            except Exception as error:
                traceback.print_exc()
                failed.append(f"suite.toml: {error}")

        for failure in failed:
            self.report({"ERROR"}, failure)
        _report_findings(self, findings)
        self.report({"INFO"}, f"Exported {len(done)} file(s) to {folder}")
        return {"FINISHED"} if done else {"CANCELLED"}

    @staticmethod
    def _missing(suite, kind, entry):
        if kind in {"MODEL", "SKIN", "ANIMATION"} and suite.armature is None:
            return "no armature picked"
        if kind in {"SKIN", "ENVIRONMENT"} and entry.collection is None:
            return "no collection picked"
        if kind == "CAMERA" and entry.camera is None:
            return "no camera picked"
        return None

    @staticmethod
    def _export(context, suite, kind, entry, folder):
        if kind == "MODEL":
            return exporters.model(context, suite, folder)
        if kind == "SKIN":
            return exporters.skin(context, suite, entry, folder)
        if kind == "ANIMATION":
            return exporters.animation(context, suite, entry, folder)
        if kind == "ENVIRONMENT":
            return exporters.environment(context, suite, entry, folder)
        return exporters.camera(context, suite, entry, folder)

    @staticmethod
    def _write_manifest(suite, folder, exported_animations, model_exported):
        path = manifest.path(folder)
        existing = toml_io.load(path) if os.path.isfile(path) else {}
        data = manifest.update(
            existing, suite, exporters.extension(suite), exported_animations, model_exported
        )
        manifest.write(folder, data)


class CHARPAPER_OT_load_manifest(Operator):
    bl_idname = "charpaper.load_manifest"
    bl_label = "Load suite.toml"
    bl_description = "Fill the panel from the suite.toml in the folder: name, defaults, clip settings and gaze"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        suite = suite_of(context)
        path = manifest.path(bpy.path.abspath(suite.folder))
        if not os.path.isfile(path):
            self.report({"WARNING"}, f"No suite.toml in {os.path.dirname(path)}")
            return {"CANCELLED"}
        try:
            data = toml_io.load(path)
        except Exception as error:
            self.report({"ERROR"}, f"suite.toml: {error}")
            return {"CANCELLED"}
        unmatched = manifest.load_into(suite, data)
        if unmatched:
            self.report(
                {"WARNING"},
                f"{len(unmatched)} animation(s) match no action here and stay as they are: "
                + ", ".join(unmatched[:5]),
            )
        else:
            self.report({"INFO"}, "Loaded suite.toml")
        return {"FINISHED"}


class CHARPAPER_OT_open_folder(Operator):
    bl_idname = "charpaper.open_folder"
    bl_label = "Open folder"

    def execute(self, context):
        folder = bpy.path.abspath(suite_of(context).folder)
        if not os.path.isdir(folder):
            self.report({"WARNING"}, "Nothing exported there yet")
            return {"CANCELLED"}
        bpy.ops.wm.path_open(filepath=folder)
        return {"FINISHED"}


class CHARPAPER_OT_show_finding(Operator):
    bl_idname = "charpaper.show_finding"
    bl_label = "Show"
    bl_description = "Select what this finding is about"

    index: IntProperty()

    def execute(self, context):
        finding = suite_of(context).findings[self.index]
        obj = bpy.data.objects.get(finding.object_name)
        if obj is None or obj.name not in context.view_layer.objects:
            self.report({"WARNING"}, "Nothing to select in this view layer")
            return {"CANCELLED"}
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        for other in context.view_layer.objects:
            other.select_set(False)
        obj.select_set(True)
        context.view_layer.objects.active = obj
        if finding.bone_name and obj.type == "ARMATURE" and finding.bone_name in obj.data.bones:
            obj.data.bones.active = obj.data.bones[finding.bone_name]
        return {"FINISHED"}


classes = (
    CHARPAPER_OT_list_add,
    CHARPAPER_OT_list_remove,
    CHARPAPER_OT_sync_clips,
    CHARPAPER_OT_tick_clips,
    CHARPAPER_OT_sync_expressions,
    CHARPAPER_OT_eye_add,
    CHARPAPER_OT_eye_remove,
    CHARPAPER_OT_validate,
    CHARPAPER_OT_export,
    CHARPAPER_OT_load_manifest,
    CHARPAPER_OT_open_folder,
    CHARPAPER_OT_show_finding,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
