"""Hotspots: places on the character's or an environment armature's bones that play clips when
clicked or when the cursor rests on them. Each area is a capsule around one bone, from its
head to its tail and as wide as its radius, drawn in the viewport while the panel is open.

Only armatures carry hotspots: an object that should react gets an armature. In suite.toml an
area names its armature by object name, which the app finds in the environment shown, and a
clip names its actor by folder, as the clips folder does.
"""

import math
import os

import bpy
import gpu
from bpy.props import IntProperty
from bpy.props import StringProperty
from bpy.types import Menu
from bpy.types import Operator
from gpu_extras.batch import batch_for_shader
from mathutils import Vector

from . import actors
from . import clips as clip_files
from .props import file_stem
from .props import redraw_views
from .props import suite_of

PICKED_COLOR = (1.0, 0.55, 0.1, 0.95)
AREA_COLOR = (1.0, 0.85, 0.4, 1.0)
OTHER_COLOR = (0.45, 0.7, 1.0, 0.45)
TRIGGERS = {"CLICK": "click", "HOVER": "hover"}

_handle = None


def exported_bone(suite, armature, name):
    """The bone if the exported file has it: Deform bones only leaves the others out."""
    bone = armature.data.bones.get(name) if armature is not None and armature.type == "ARMATURE" else None
    if bone is None or (suite.gltf.deform_bones_only and not bone.use_deform):
        return None
    return bone


def exported_armature(suite, armature):
    """Whether the armature is in the suite: the character's, or in an exported environment."""
    if armature is None:
        return False
    if armature == suite.armature:
        return True
    return any(
        entry.enabled and entry.collection is not None and armature.name in entry.collection.all_objects
        for entry in suite.environments
    )


def resolve_clip(suite, item):
    """The actor and clip a hotspot clip names, or None and why not."""
    if item.actor is None or item.action is None:
        return None, "has a clip with nothing picked"
    actor = next((a for a in actors.all_actors(suite) if a.obj == item.actor), None)
    if actor is None:
        return None, f"plays a clip of {item.actor.name}, which isn't the character or in an exported environment"
    clip = next((c for c in actors.clips(actor) if c.action == item.action), None)
    if clip is None:
        return None, (
            f"plays {item.action.name}, which isn't one of {item.actor.name}'s clips, so it isn't "
            "exported. Add it in Animation"
        )
    return (actor, clip), None


def _clip_file(folder, actor, clip):
    return os.path.join(folder, clip_files.clip_folder(actor), clip.name + ".json")


def tables(suite, folder):
    """[hotspots] for suite.toml, and what was left out of it. Only what is exported is named:
    the app refuses a suite.toml whose hotspots name a clip or expression it doesn't have."""
    out, left_out = {}, []
    expressions = {file_stem(e.action.name) for e in suite.expressions if e.action is not None}
    for spot in suite.hotspots:
        name = spot.name.strip() or "Hotspot"
        areas = []
        for area in spot.areas:
            bone = exported_bone(suite, area.armature, area.bone)
            if bone is None or not exported_armature(suite, area.armature):
                continue
            table = {} if area.armature == suite.armature else {"actor": area.armature.name}
            table.update({"bone": bone.name, "length": round(bone.length, 6), "radius": round(area.radius, 6)})
            areas.append(table)
        played = []
        for item in spot.clips:
            found, _ = resolve_clip(suite, item)
            if found is None:
                continue
            actor, clip = found
            if not os.path.isfile(_clip_file(folder, actor, clip)):
                left_out.append(f"hotspot {name!r}: {clip.name} of {actor.name} isn't exported yet")
                continue
            table = {} if actor.is_character else {"actor": file_stem(actor.name)}
            table["clip"] = clip.name
            played.append(table)
        if not areas or not played:
            left_out.append(f"hotspot {name!r}: left out, with no exported bone or clip")
            continue
        table = {}
        if spot.trigger != "CLICK":
            table["trigger"] = TRIGGERS[spot.trigger]
        if spot.layer:
            table["layer"] = True
        expression = file_stem(spot.expression) if spot.expression else ""
        if expression:
            if expression in expressions and os.path.isfile(os.path.join(folder, "expressions", expression + ".json")):
                table["expression"] = expression
            else:
                left_out.append(f"hotspot {name!r}: its expression {expression!r} isn't exported")
        if spot.cooldown > 0:
            table["cooldown"] = round(spot.cooldown, 3)
        table["areas"] = areas
        table["clips"] = played
        out[name] = table
    return out, left_out


def load_into(suite, data):
    """Fills the panel from suite.toml's [hotspots]. Returns what matches nothing here."""
    found = data.get("hotspots")
    if found is None:
        return []
    unmatched = []
    suite.hotspots.clear()
    suite.hotspots_owned = True
    for name, table in found.items():
        spot = suite.hotspots.add()
        spot.name = name
        spot.trigger = "HOVER" if table.get("trigger") == "hover" else "CLICK"
        spot.layer = table.get("layer", False)
        spot.expression = table.get("expression", "")
        spot.cooldown = table.get("cooldown", 0.0)
        for area_table in table.get("areas", []):
            owner = area_table.get("actor")
            armature = bpy.data.objects.get(owner) if owner else suite.armature
            if armature is None or armature.type != "ARMATURE":
                unmatched.append(f"{name}: armature {owner or 'of the character'}")
                continue
            area = spot.areas.add()
            area.armature = armature
            area.bone = area_table.get("bone", "")
            area.radius = area_table.get("radius", 0.05)
        for clip_table in table.get("clips", []):
            owner = clip_table.get("actor")
            actor = _actor_named(suite, owner)
            clip = next((c for c in actors.clips(actor) if c.name == clip_table.get("clip")), None) if actor else None
            if clip is None:
                unmatched.append(f"{name}: {owner + '/' if owner else ''}{clip_table.get('clip')}")
                continue
            item = spot.clips.add()
            item.actor, item.action = actor.obj, clip.action
    return unmatched


def _actor_named(suite, folder):
    for actor in actors.all_actors(suite):
        if (folder is None and actor.is_character) or (folder is not None and not actor.is_character and file_stem(actor.name) == folder):
            return actor
    return None


def check(suite, add, finding):
    names = {}
    used = set()
    for spot in suite.hotspots:
        name = spot.name.strip() or "Hotspot"
        names[name] = names.get(name, 0) + 1

        def warn(message, severity="WARNING", obj=None):
            add(finding(f"hotspot {name!r} {message}", obj, severity=severity, hotspot=spot.name))

        if not len(spot.areas):
            warn("has no bones, so nothing sets it off. Select bones in Pose Mode and click Add selected bones")
        if not len(spot.clips):
            warn("plays nothing: add a clip")
        for area in spot.areas:
            armature = area.armature
            if armature is None or not area.bone:
                warn("has an area with no bone picked")
                continue
            if armature.data.bones.get(area.bone) is None:
                warn(f"names the bone {area.bone!r}, which {armature.name} doesn't have", obj=armature)
            elif exported_bone(suite, armature, area.bone) is None:
                warn(
                    f"sits on {area.bone}, which Deform bones only leaves out of the file. Use a deform "
                    "bone near it, or turn Deform bones only off",
                    obj=armature,
                )
            if not exported_armature(suite, armature):
                warn(
                    f"sits on {armature.name}, which isn't the character's armature or in an exported "
                    "environment, so that area isn't there",
                    obj=armature,
                )
        for item in spot.clips:
            resolved, problem = resolve_clip(suite, item)
            if resolved is None:
                warn(problem, obj=item.actor)
                continue
            actor, clip = resolved
            used.add((actor.obj, clip.name))
            if clip.mode != "ONCE":
                warn(
                    f"plays {clip.name}, a {clip.mode.title()} clip. Reactions play through once whatever "
                    "the mode, so set it to Once to see it as the app does",
                    severity="INFO",
                    obj=actor.obj,
                )
        if spot.expression and file_stem(spot.expression) not in {
            file_stem(e.action.name) for e in suite.expressions if e.action is not None
        }:
            warn(f"shows the expression {spot.expression!r}, which isn't one of the character's expressions")
        if spot.expression and not any(
            item.actor == suite.armature for item in spot.clips if item.actor is not None
        ):
            warn(
                "sets an expression but plays nothing on the character, so the expression shows only "
                "while the other clips play",
                severity="INFO",
            )
    for name, count in names.items():
        if count > 1:
            add(finding(f"{count} hotspots are named {name!r}; only the last is exported. Rename the others", hotspot=name))
    character = actors.character(suite)
    for clip in actors.clips(character) if character else ():
        if clip.reaction_only and (character.obj, clip.name) not in used:
            add(finding(
                f"clip {clip.name!r} is only for hotspots, but no hotspot plays it, so the app never does",
                character.obj,
                severity="INFO",
                clip=clip.name,
            ))
        if clip.reaction_only and suite.default_clip == clip.name:
            add(finding(
                f"the character starts with {clip.name!r}, which is only for hotspots; the app starts in "
                "the rest pose instead. Star another clip",
                character.obj,
                clip=clip.name,
            ))


def _capsule(matrix, length, radius, segments=24):
    """Line pairs outlining a capsule around a bone whose world matrix is `matrix`, as the app
    sizes it: the radius grows with the bone's scale."""
    basis = matrix.to_3x3()
    x, y, z = basis.col[0], basis.col[1], basis.col[2]
    radius *= (x.length + y.length + z.length) / 3.0
    head = matrix @ Vector((0.0, 0.0, 0.0))
    tail = matrix @ Vector((0.0, length, 0.0))
    x, y, z = x.normalized(), y.normalized(), z.normalized()
    points = []

    def arc(center, a, b, start, end):
        steps = max(2, round(segments * (end - start) / math.tau))
        ring = [center + radius * (math.cos(t) * a + math.sin(t) * b)
                for t in (start + (end - start) * i / steps for i in range(steps + 1))]
        for i in range(steps):
            points.extend((ring[i], ring[i + 1]))

    for center in (head, tail):
        arc(center, x, z, 0.0, math.tau)
    for side in (x, z):
        arc(head, side, -y, 0.0, math.pi)
        arc(tail, side, y, 0.0, math.pi)
    for side in (x, -x, z, -z):
        points.extend((head + side * radius, tail + side * radius))
    return points


def _draw():
    suite = getattr(bpy.context.scene, "charpaper", None)
    if suite is None or not suite.show_hotspots or not len(suite.hotspots):
        return
    groups = {}
    for index, spot in enumerate(suite.hotspots):
        for at, area in enumerate(spot.areas):
            armature = area.armature
            pose_bone = armature.pose.bones.get(area.bone) if armature is not None and armature.pose else None
            if pose_bone is None:
                continue
            if index != suite.hotspot_index:
                color = OTHER_COLOR
            else:
                color = AREA_COLOR if at == spot.area_index else PICKED_COLOR
            matrix = armature.matrix_world @ pose_bone.matrix
            groups.setdefault(color, []).extend(_capsule(matrix, pose_bone.bone.length, area.radius))
    if not groups:
        return
    shader = gpu.shader.from_builtin("POLYLINE_UNIFORM_COLOR")
    shader.uniform_float("viewportSize", tuple(float(v) for v in gpu.state.viewport_get()[2:]))
    # Drawn over the mesh: the capsules mostly sit inside it.
    depth_test = gpu.state.depth_test_get()
    gpu.state.depth_test_set("NONE")
    gpu.state.blend_set("ALPHA")
    for color, points in groups.items():
        shader.uniform_float("lineWidth", 2.0 if color is not OTHER_COLOR else 1.0)
        shader.uniform_float("color", color)
        batch_for_shader(shader, "LINES", {"pos": points}).draw(shader)
    gpu.state.blend_set("NONE")
    gpu.state.depth_test_set(depth_test)


def select(pose_bone, selected):
    # Blender 5 keeps selection on the pose bone, 4.2 on the bone.
    owner = pose_bone if hasattr(pose_bone, "select") else pose_bone.bone
    owner.select = selected


def current(suite):
    index = suite.hotspot_index
    return suite.hotspots[index] if 0 <= index < len(suite.hotspots) else None


class CHARPAPER_OT_hotspot_add(Operator):
    bl_idname = "charpaper.hotspot_add"
    bl_label = "Add hotspot"
    bl_description = "Add a hotspot. Bones selected in Pose Mode become its areas"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        suite = suite_of(context)
        spot = suite.hotspots.add()
        spot.name = f"Hotspot {len(suite.hotspots)}"
        suite.hotspot_index = len(suite.hotspots) - 1
        suite.hotspots_owned = True
        _add_bones(context, spot)
        return {"FINISHED"}


class CHARPAPER_OT_hotspot_remove(Operator):
    bl_idname = "charpaper.hotspot_remove"
    bl_label = "Remove hotspot"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return current(suite_of(context)) is not None

    def execute(self, context):
        suite = suite_of(context)
        suite.hotspots.remove(suite.hotspot_index)
        suite.hotspot_index = max(0, suite.hotspot_index - 1)
        redraw_views()
        return {"FINISHED"}


def _add_bones(context, spot):
    added = 0
    for pose_bone in context.selected_pose_bones or []:
        armature = pose_bone.id_data
        if any(a.armature == armature and a.bone == pose_bone.name for a in spot.areas):
            continue
        area = spot.areas.add()
        area.armature = armature
        area.bone = pose_bone.name
        # A start that fits most limbs; the drawn capsule shows what to change.
        area.radius = max(pose_bone.bone.length * 0.3, 0.01)
        added += 1
    if added:
        spot.area_index = len(spot.areas) - 1
    redraw_views()
    return added


class CHARPAPER_OT_hotspot_bones_add(Operator):
    bl_idname = "charpaper.hotspot_bones_add"
    bl_label = "Add selected bones"
    bl_description = (
        "Make the bones selected in Pose Mode areas of this hotspot. Several armatures in Pose "
        "Mode at once add their bones together"
    )
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return current(suite_of(context)) is not None

    def execute(self, context):
        added = _add_bones(context, current(suite_of(context)))
        if not added:
            self.report({"WARNING"}, "Select bones in Pose Mode first; bones already in it are skipped")
            return {"CANCELLED"}
        self.report({"INFO"}, f"{added} bone(s) added")
        return {"FINISHED"}


class CHARPAPER_OT_hotspot_area_remove(Operator):
    bl_idname = "charpaper.hotspot_area_remove"
    bl_label = "Remove area"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        spot = current(suite_of(context))
        return spot is not None and 0 <= spot.area_index < len(spot.areas)

    def execute(self, context):
        spot = current(suite_of(context))
        spot.areas.remove(spot.area_index)
        spot.area_index = max(0, spot.area_index - 1)
        redraw_views()
        return {"FINISHED"}


class CHARPAPER_OT_hotspot_clip_add(Operator):
    bl_idname = "charpaper.hotspot_clip_add"
    bl_label = "Add clip"
    bl_description = "Play this clip when the hotspot goes off. Several of one actor's: one at random"
    bl_options = {"REGISTER", "UNDO"}

    actor: StringProperty()
    action: StringProperty()

    def execute(self, context):
        spot = current(suite_of(context))
        obj, action = bpy.data.objects.get(self.actor), bpy.data.actions.get(self.action)
        if spot is None or obj is None or action is None:
            return {"CANCELLED"}
        if not any(c.actor == obj and c.action == action for c in spot.clips):
            item = spot.clips.add()
            item.actor, item.action = obj, action
            spot.clip_index = len(spot.clips) - 1
        return {"FINISHED"}


class CHARPAPER_OT_hotspot_clip_remove(Operator):
    bl_idname = "charpaper.hotspot_clip_remove"
    bl_label = "Remove clip"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        spot = current(suite_of(context))
        return spot is not None and 0 <= spot.clip_index < len(spot.clips)

    def execute(self, context):
        spot = current(suite_of(context))
        spot.clips.remove(spot.clip_index)
        spot.clip_index = max(0, spot.clip_index - 1)
        return {"FINISHED"}


class CHARPAPER_OT_hotspot_area_select(Operator):
    bl_idname = "charpaper.hotspot_area_select"
    bl_label = "Select bone"
    bl_description = "Select this bone in Pose Mode, to see it or to pose it"
    bl_options = {"REGISTER", "UNDO"}

    index: IntProperty()

    def execute(self, context):
        spot = current(suite_of(context))
        area = spot.areas[self.index] if spot and 0 <= self.index < len(spot.areas) else None
        if area is None or area.armature is None or area.armature.data.bones.get(area.bone) is None:
            return {"CANCELLED"}
        armature = area.armature
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        for obj in context.view_layer.objects:
            obj.select_set(False)
        armature.select_set(True)
        context.view_layer.objects.active = armature
        bpy.ops.object.mode_set(mode="POSE")
        for pose_bone in armature.pose.bones:
            select(pose_bone, pose_bone.name == area.bone)
        armature.data.bones.active = armature.data.bones[area.bone]
        spot.area_index = self.index
        return {"FINISHED"}


class CHARPAPER_MT_hotspot_clip(Menu):
    bl_label = "Play a clip"
    bl_description = "The clips of the character and of every exported environment's actors"

    def draw(self, context):
        suite = suite_of(context)
        layout = self.layout
        found = [(a, actors.clips(a)) for a in actors.all_actors(suite)]
        found = [(a, c) for a, c in found if c]
        if not found:
            layout.label(text="No actor has clips yet: add them in Animation")
        for actor, clips in found:
            layout.label(text="Character" if actor.is_character else actor.name)
            # Once clips first: they are what reactions are made of.
            for clip in sorted(clips, key=lambda c: (c.mode != "ONCE", c.name)):
                icon = "PLAY" if clip.mode == "ONCE" else "TIME" if clip.mode == "TIMELINE" else "PAUSE"
                op = layout.operator("charpaper.hotspot_clip_add", text=clip.action.name, icon=icon)
                op.actor, op.action = actor.obj.name, clip.action.name


classes = (
    CHARPAPER_OT_hotspot_add,
    CHARPAPER_OT_hotspot_remove,
    CHARPAPER_OT_hotspot_bones_add,
    CHARPAPER_OT_hotspot_area_remove,
    CHARPAPER_OT_hotspot_area_select,
    CHARPAPER_OT_hotspot_clip_add,
    CHARPAPER_OT_hotspot_clip_remove,
    CHARPAPER_MT_hotspot_clip,
)


def register():
    global _handle
    for cls in classes:
        bpy.utils.register_class(cls)
    _handle = bpy.types.SpaceView3D.draw_handler_add(_draw, (), "WINDOW", "POST_VIEW")


def unregister():
    global _handle
    if _handle is not None:
        bpy.types.SpaceView3D.draw_handler_remove(_handle, "WINDOW")
        _handle = None
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
