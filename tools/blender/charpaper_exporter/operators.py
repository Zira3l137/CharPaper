import os
import re
import textwrap
import time
import traceback

import bpy
from bpy.props import CollectionProperty
from bpy.props import EnumProperty
from bpy.props import IntProperty
from bpy.props import StringProperty
from bpy.types import Operator

from . import actors
from . import anim
from . import clips
from . import cues
from . import env_settings
from . import exporters
from . import gaze
from . import gltf
from . import hotspots
from . import lens
from . import manifest
from . import music
from . import sky
from . import toml_io
from . import validate
from .audio import AUDIO
from .isolate import object_mode
from .props import file_stem
from .props import suite_of

LISTS = {
    "SKIN": ("skins", "skin_index"),
    "EXPRESSION": ("expressions", "expression_index"),
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
        item.clip = getattr(finding, "clip", "")
        item.sound = getattr(finding, "sound", "")
        item.hotspot = getattr(finding, "hotspot", "")
        item.fix = getattr(finding, "fix", "")
    suite.validated = True


def _report_findings(op, findings):
    warnings = sum(1 for f in findings if f.severity == "WARNING")
    if warnings:
        op.report({"WARNING"}, f"{warnings} warning(s), see Warnings")


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
        elif self.kind != "EXPRESSION":
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


def current_actor(suite):
    """The actor the Animation panel shows: the one picked, else the character, else the first."""
    found = actors.all_actors(suite)
    picked = next((a for a in found if a.name == suite.actor), None)
    return picked or (found[0] if found else None)


def _listed(actor):
    """The actor's own list, which first takes on the clip its active action implies, so that
    editing the list never drops what played until then."""
    stored = actor.obj.charpaper.clips
    if len(stored) == 0:
        for clip in actors.clips(actor):
            item = stored.add()
            item.action, item.extra = clip.action, clip.extra
    return stored


class CHARPAPER_OT_actor_select(Operator):
    bl_idname = "charpaper.actor_select"
    bl_label = "Show its clips"
    bl_description = "Show this actor's clips"

    name: StringProperty()

    def execute(self, context):
        suite_of(context).actor = self.name
        return {"FINISHED"}


class CHARPAPER_OT_clip_add(Operator):
    bl_idname = "charpaper.clip_add"
    bl_label = "Add clip"
    bl_description = "Add this action as a clip of the actor"
    bl_options = {"REGISTER", "UNDO"}

    action: StringProperty()

    def execute(self, context):
        suite = suite_of(context)
        actor = current_actor(suite)
        action = bpy.data.actions.get(self.action)
        if actor is None or action is None:
            return {"CANCELLED"}
        stored = _listed(actor)
        if not any(c.action == action for c in stored):
            stored.add().action = action
        actor.obj.charpaper.clip_index = next(i for i, c in enumerate(stored) if c.action == action)
        return {"FINISHED"}


class CHARPAPER_OT_clip_add_all(Operator):
    bl_idname = "charpaper.clip_add_all"
    bl_label = "Add all that fit"
    bl_description = "Add every action that keys this actor's bones, shape keys, light or transform"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        suite = suite_of(context)
        actor = current_actor(suite)
        if actor is None:
            return {"CANCELLED"}
        stored = _listed(actor)
        added = 0
        for action in actors.fitting_actions(actor, suite):
            if not any(c.action == action for c in stored):
                stored.add().action = action
                added += 1
        self.report({"INFO"}, f"{added} clip(s) added")
        return {"FINISHED"}


class CHARPAPER_OT_clip_remove(Operator):
    bl_idname = "charpaper.clip_remove"
    bl_label = "Remove clip"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        actor = current_actor(suite_of(context))
        if actor is None:
            return {"CANCELLED"}
        stored = _listed(actor)
        index = actor.obj.charpaper.clip_index
        if 0 <= index < len(stored):
            stored.remove(index)
            actor.obj.charpaper.clip_index = max(0, index - 1)
        return {"FINISHED"}


class CHARPAPER_OT_clip_edit(Operator):
    bl_idname = "charpaper.clip_edit"
    bl_label = "Edit as a list"
    bl_description = "Make the active action a clip of the list, to change how it plays or add others"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        actor = current_actor(suite_of(context))
        if actor is None:
            return {"CANCELLED"}
        _listed(actor)
        return {"FINISHED"}


class CHARPAPER_OT_clip_star(Operator):
    bl_idname = "charpaper.clip_star"
    bl_label = "Starts with"
    bl_description = "The clip the character starts with. Click again to start in the rest pose"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()

    def execute(self, context):
        suite = suite_of(context)
        clip = suite.armature.charpaper.clips[self.index] if suite.armature else None
        if clip is None or clip.action is None:
            return {"CANCELLED"}
        name = file_stem(clip.action.name)
        suite.default_clip = "" if suite.default_clip == name else name
        return {"FINISHED"}


def _environment(context):
    suite = suite_of(context)
    index = suite.environment_index
    return suite.environments[index] if 0 <= index < len(suite.environments) else None


def _natural(name):
    return [int(part) if part.isdigit() else part.casefold() for part in re.split(r"(\d+)", name)]


class CHARPAPER_OT_music_add(Operator):
    bl_idname = "charpaper.music_add"
    bl_label = "Add music"
    bl_description = "Add sound files to the end of the environment's playlist, in file-name order"
    bl_options = {"REGISTER", "UNDO"}

    # The file browser fills these. `filepath` is the file under the cursor, and the fallback
    # for the folder should `directory` come back empty.
    filepath: StringProperty(subtype="FILE_PATH", options={"HIDDEN", "SKIP_SAVE"})
    directory: StringProperty(subtype="FILE_PATH", options={"HIDDEN", "SKIP_SAVE"})
    files: CollectionProperty(type=bpy.types.OperatorFileListElement, options={"HIDDEN", "SKIP_SAVE"})
    filter_glob: StringProperty(default=";".join("*" + e for e in sorted(AUDIO)), options={"HIDDEN"})

    @classmethod
    def poll(cls, context):
        return _environment(context) is not None

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {"RUNNING_MODAL"}

    def execute(self, context):
        entry = _environment(context)
        folder = self.directory or os.path.dirname(self.filepath)
        folder = os.path.normpath(bpy.path.abspath(folder)) if folder else ""
        names = sorted((f.name for f in self.files if f.name), key=_natural)
        if not names and self.filepath:
            names = [os.path.basename(self.filepath)]
        found = [n for n in names if folder and os.path.isfile(os.path.join(folder, n))]
        if len(found) < len(names):
            lost = ", ".join(n for n in names if n not in found)
            self.report({"ERROR"}, f"Blender didn't say which folder these are in, so they weren't added: {lost}")
        for name in found:
            track = entry.music.add()
            track.name = os.path.splitext(name)[0]
            track.filepath = music.stored_path(os.path.join(folder, name))
        if found:
            entry.music_index = len(entry.music) - 1
            entry.export_sound = True
        return {"FINISHED"}


class CHARPAPER_OT_music_remove(Operator):
    bl_idname = "charpaper.music_remove"
    bl_label = "Remove track"
    bl_description = "Take the track out of the playlist. The next export removes its file"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        entry = _environment(context)
        return entry is not None and 0 <= entry.music_index < len(entry.music)

    def execute(self, context):
        entry = _environment(context)
        entry.music.remove(entry.music_index)
        entry.music_index = min(entry.music_index, len(entry.music) - 1)
        return {"FINISHED"}


class CHARPAPER_OT_music_move(Operator):
    bl_idname = "charpaper.music_move"
    bl_label = "Move track"
    bl_description = "Play the track earlier or later"
    bl_options = {"REGISTER", "UNDO"}

    direction: EnumProperty(items=(("UP", "Up", ""), ("DOWN", "Down", "")))

    @classmethod
    def poll(cls, context):
        entry = _environment(context)
        return entry is not None and len(entry.music) > 1

    def execute(self, context):
        entry = _environment(context)
        index = entry.music_index
        other = index - 1 if self.direction == "UP" else index + 1
        if not (0 <= index < len(entry.music) and 0 <= other < len(entry.music)):
            return {"CANCELLED"}
        entry.music.move(index, other)
        entry.music_index = other
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
        if suite_of(context).gaze.preview:
            gaze.refresh(context)
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
            ("ACTOR", "Actor", "One actor's clips"),
            ("EXPRESSIONS", "Expressions", ""),
            ("ENVIRONMENT", "Environment", ""),
            ("SOUND", "Sound", "An environment's music and ambience"),
            ("CAMERA", "Camera", ""),
        )
    )
    index: IntProperty(default=-1)
    actor: StringProperty(description="The actor whose clips to export, by its object's name")

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
        if self.what == "ALL":
            jobs += [("ACTOR", actor) for actor in actors.all_actors(suite)]
        elif self.what == "ACTOR":
            jobs += [("ACTOR", a) for a in [actors.find(suite, self.actor)] if a is not None]
        if self.what in {"ALL", "EXPRESSIONS"}:
            jobs.append(("EXPRESSIONS", None))
        environments = pick("ENVIRONMENT", suite.environments)
        jobs += environments
        if self.what == "SOUND" and 0 <= self.index < len(suite.environments):
            jobs.append(("SOUND", suite.environments[self.index]))
        else:
            jobs += [("SOUND", entry) for _, entry in environments if entry.export_sound]
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
        # A previewed look is not the character's own pose, and must not be exported or validated as it.
        gaze.stop(context)

        findings = validate.check(context, suite)
        _store(suite, findings)

        done, failed, model_exported = [], [], False
        lenses, environments, exported, alerts = {}, {}, {}, []
        with object_mode(context):
            for kind, entry in self._jobs(suite):
                label = entry.name if entry is not None else kind.lower()
                missing = self._missing(context.scene, suite, kind, entry)
                if missing:
                    failed.append(f"{label}: {missing}")
                    continue
                try:
                    if kind == "ACTOR":
                        actor_clips = actors.clips(entry)
                        written = clips.export_actor(context, suite, entry, actor_clips, folder)
                        done += written.files
                        for note in written.notes:
                            self.report({"WARNING"}, note)
                        environment = entry.environment.name if entry.environment else None
                        exported[(environment, entry.name)] = (entry, actor_clips, written.cues)
                        continue
                    if kind == "EXPRESSIONS":
                        done += clips.export_expressions(context, suite, folder)
                        continue
                    if kind == "SOUND":
                        written, notes = music.export(entry, folder)
                        done += written
                        for note in notes:
                            self.report({"WARNING"}, note)
                        continue
                    result = self._export(context, suite, kind, entry, folder)
                except Exception as error:
                    traceback.print_exc()
                    failed.append(f"{label}: {error}")
                    continue
                done.append(result.path)
                done += result.written
                alerts += result.alerts
                for note in result.notes:
                    self.report({"WARNING"}, note)
                if result.skipped:
                    print(f"CharPaper: {os.path.basename(result.path)}: exporter lacks {', '.join(result.skipped)}")
                if kind == "CAMERA":
                    lenses[file_stem(entry.name)] = lens.table(entry.camera, context.scene.render)
                if kind == "ENVIRONMENT":
                    table = env_settings.table(entry.collection) if entry.collection else {}
                    environments[file_stem(entry.name)] = {**table, **result.settings}
                model_exported |= kind == "MODEL"

        # Cues name sounds; a cue whose sound isn't there is left out.
        wanted = {sound for _, _, tables in exported.values() for cues_of in tables.values() for _, sound in cues_of}
        if wanted:
            try:
                written, present, notes = cues.write_sounds(folder, wanted)
                done += written
                for note in notes:
                    self.report({"WARNING"}, note)
            except Exception as error:
                traceback.print_exc()
                failed.append(f"sounds: {error}")
                present = set()
            for _, _, tables in exported.values():
                for name, cues_of in tables.items():
                    tables[name] = [(at, sound) for at, sound in cues_of if sound in present]

        if suite.write_manifest and done:
            try:
                left_out = self._write_manifest(
                    context.scene, suite, folder, model_exported, lenses, environments, exported
                )
                for note in left_out:
                    self.report({"WARNING"}, note)
            except Exception as error:
                traceback.print_exc()
                failed.append(f"suite.toml: {error}")

        for failure in failed:
            self.report({"ERROR"}, failure)
            # Kept in the Warnings panel too: the status bar's report soon goes.
            finding = suite.findings.add()
            finding.name, finding.severity = f"export failed: {failure}", "ERROR"
            suite.validated = True
        _report_findings(self, findings)
        self.report({"INFO"}, f"Exported {len(done)} file(s) to {folder}")
        warnings = sum(1 for f in suite.findings if f.severity == "WARNING")
        errors = len(failed) + len(alerts)
        problems = f"{errors} errors · {warnings} warnings" if errors else f"{warnings} warnings"
        suite.last_export = f"{time.strftime('%H:%M')} · {len(done)} files · {problems}"
        if alerts:
            for alert in alerts:
                finding = suite.findings.add()
                finding.name, finding.severity = alert.message, "ERROR"
                finding.object_name, finding.fix = alert.object_name, alert.fix
            suite.alerts.clear()
            for alert in alerts:
                item = suite.alerts.add()
                item.name, item.object_name, item.fix = alert.message, alert.object_name, alert.fix
            if context.window is not None:
                bpy.ops.charpaper.export_alerts("INVOKE_DEFAULT")
        return {"FINISHED"} if done else {"CANCELLED"}

    @staticmethod
    def _missing(scene, suite, kind, entry):
        if kind in {"MODEL", "SKIN"} and suite.armature is None:
            return "no armature picked"
        if kind == "SKIN" and entry.collection is None:
            return "no collection picked"
        if kind == "ENVIRONMENT" and entry.collection is None and sky.world_of(scene, entry) is None:
            return "no collection, and Sky is off" if not entry.export_sky else "no collection, and no World for its sky"
        if kind == "CAMERA" and entry.camera is None:
            return "no camera picked"
        return None

    @staticmethod
    def _export(context, suite, kind, entry, folder):
        if kind == "MODEL":
            return exporters.model(context, suite, folder)
        if kind == "SKIN":
            return exporters.skin(context, suite, entry, folder)
        if kind == "ENVIRONMENT":
            return exporters.environment(context, suite, entry, folder)
        return exporters.camera(context, suite, entry, folder)

    @staticmethod
    def _write_manifest(scene, suite, folder, model_exported, lenses, environments, exported):
        """Writes suite.toml. Returns what of the hotspots it left out."""
        path = manifest.path(folder)
        existing = toml_io.load(path) if os.path.isfile(path) else {}
        spots, left_out = hotspots.tables(suite, folder)
        data = manifest.update(
            existing,
            suite,
            scene,
            exporters.extension(suite),
            model_exported,
            lenses,
            environments,
            exported,
            hotspots=spots,
        )
        manifest.write(folder, data)
        return left_out if suite.hotspots_owned else []


class CHARPAPER_OT_stop_shape_keys(Operator):
    bl_idname = "charpaper.stop_shape_keys"
    bl_label = "Stop it"
    bl_description = (
        "Stop what plays on this mesh's shape keys: unassign the action, keeping it in the file, "
        "and mute the NLA tracks that play. The sliders it moved go back to 0"
    )
    bl_options = {"REGISTER", "UNDO"}

    name: StringProperty()

    def execute(self, context):
        obj = bpy.data.objects.get(self.name)
        if obj is None or obj.type != "MESH" or obj.data.shape_keys is None:
            return {"CANCELLED"}
        key = obj.data.shape_keys
        stopped = anim.stop(key, context.scene.frame_current, anim.KEY_PATH.match)
        for action in stopped:
            for name in anim.key_names(action):
                block = key.key_blocks.get(name)
                if block is not None:
                    block.value = 0.0
        if not stopped:
            self.report({"INFO"}, f"Nothing plays on {obj.name}'s shape keys")
            return {"CANCELLED"}
        names = ", ".join(sorted({a.name for a in stopped}))
        self.report({"INFO"}, f"Stopped {names} on {obj.name}; its sliders are back at 0. Check again")
        return {"FINISHED"}


class CHARPAPER_OT_unlink_object_action(Operator):
    bl_idname = "charpaper.unlink_object_action"
    bl_label = "Unlink it"
    bl_description = (
        "Take the object's own action off it, keeping the action in the file, and mute the NLA "
        "tracks that play on it. The object stays where it is now"
    )
    bl_options = {"REGISTER", "UNDO"}

    name: StringProperty()

    def execute(self, context):
        obj = bpy.data.objects.get(self.name)
        if obj is None:
            return {"CANCELLED"}
        stopped = anim.stop(obj, context.scene.frame_current)
        if not stopped:
            self.report({"INFO"}, f"Nothing plays on {obj.name}")
            return {"CANCELLED"}
        names = ", ".join(sorted({a.name for a in stopped}))
        self.report({"INFO"}, f"Unlinked {names} from {obj.name}. Check again")
        return {"FINISHED"}


class CHARPAPER_OT_export_alerts(Operator):
    bl_idname = "charpaper.export_alerts"
    bl_label = "Exported, but the result will look wrong"
    bl_description = "What the last export did that changes how the character looks in the app"

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=520, confirm_text="OK, I understand")

    def draw(self, context):
        layout = self.layout
        width = 70
        for i, alert in enumerate(suite_of(context).alerts):
            box = layout.box()
            col = box.column(align=True)
            col.alert = True
            lines = textwrap.wrap(alert.name, width) or [""]
            col.label(text=lines[0], icon="CANCEL")
            for line in lines[1:]:
                col.label(text=line, icon="BLANK1")
            row = box.row()
            if alert.object_name:
                op = row.operator("charpaper.select_object", text="Show it", icon="RESTRICT_SELECT_OFF")
                op.name = alert.object_name
            if alert.fix == exporters.STOP_SHAPE_KEYS:
                row.operator("charpaper.stop_shape_keys", icon="PAUSE").name = alert.object_name

    def execute(self, context):
        return {"FINISHED"}


class CHARPAPER_OT_select_object(Operator):
    bl_idname = "charpaper.select_object"
    bl_label = "Show"
    bl_description = "Select this object"

    name: StringProperty()

    def execute(self, context):
        obj = bpy.data.objects.get(self.name)
        if obj is None or obj.name not in context.view_layer.objects:
            self.report({"WARNING"}, "Nothing to select in this view layer")
            return {"CANCELLED"}
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        for other in context.view_layer.objects:
            other.select_set(False)
        obj.select_set(True)
        context.view_layer.objects.active = obj
        return {"FINISHED"}


class CHARPAPER_OT_load_manifest(Operator):
    bl_idname = "charpaper.load_manifest"
    bl_label = "Load suite.toml"
    bl_description = "Fill the panel from the suite.toml in the folder: name, defaults, clip settings, gaze and hotspots"
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
        unmatched += hotspots.load_into(suite, data)
        if unmatched:
            self.report(
                {"WARNING"},
                f"{len(unmatched)} name(s) match nothing here and are left out: "
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
    CHARPAPER_OT_actor_select,
    CHARPAPER_OT_clip_add,
    CHARPAPER_OT_clip_add_all,
    CHARPAPER_OT_clip_remove,
    CHARPAPER_OT_clip_edit,
    CHARPAPER_OT_clip_star,
    CHARPAPER_OT_music_add,
    CHARPAPER_OT_music_remove,
    CHARPAPER_OT_music_move,
    CHARPAPER_OT_eye_add,
    CHARPAPER_OT_eye_remove,
    CHARPAPER_OT_validate,
    CHARPAPER_OT_export,
    CHARPAPER_OT_stop_shape_keys,
    CHARPAPER_OT_unlink_object_action,
    CHARPAPER_OT_export_alerts,
    CHARPAPER_OT_select_object,
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
