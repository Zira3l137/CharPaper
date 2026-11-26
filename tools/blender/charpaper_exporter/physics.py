"""Physics: bones of the character that the app swings with springs, following the animation,
so hair, skirts and tails move on their own without baking it.

A chain is bones from a root down, each the child of the one before. The app pulls each bone's
tail toward where the animation puts it, so a keyed swing still swings, and adds the
follow-through. A collider is a capsule around a bone of the body, from its head to its tail;
every chain keeps its tails out of every collider by its radius.

Only the character's armature has physics. A chain may run down bones only one skin has: the app
leaves it out while another skin is worn.
"""

import statistics

import bpy
import gpu
from bpy.props import EnumProperty
from bpy.props import IntProperty
from bpy.types import Operator
from gpu_extras.batch import batch_for_shader
from mathutils import Matrix
from mathutils import Vector
from mathutils.geometry import intersect_point_line

from . import multi
from .hotspots import _capsule
from .hotspots import exported_bone
from .hotspots import select
from .props import PHYSICS_PRESETS
from .props import PRESET_ITEMS
from .props import redraw_views
from .props import suite_of

# How many weighted vertices a mesh needs to say how thick a bone is.
FIT_VERTICES = 8
FIT_WEIGHT = 0.5
CHAIN_COLOR = (0.36, 0.71, 0.92)
TOUCHED_COLOR = (0.55, 0.85, 0.45)
COLLIDER_COLOR = (0.95, 0.64, 0.33)

_handle = None


def chain_name(chain):
    return chain.bones[0].bone if len(chain.bones) else "Chain"


def current_chain(suite):
    index = suite.physics_chain_index
    return suite.physics_chains[index] if 0 <= index < len(suite.physics_chains) else None


def current_collider(suite):
    index = suite.physics_collider_index
    return suite.physics_colliders[index] if 0 <= index < len(suite.physics_colliders) else None


def down_from(root, selected):
    """The chain starting at `root`: through its selected children if any of the bones below it
    are selected, otherwise all the way down, as long as there is one child to go to."""
    alone = not ({b.name for b in root.children_recursive} & selected)
    bones, bone = [root], root
    while True:
        if alone:
            bone = bone.children[0] if len(bone.children) == 1 else None
        else:
            bone = next((c for c in bone.children if c.name in selected), None)
        if bone is None:
            return bones
        bones.append(bone)


def chains_from(armature, names):
    """One chain per selected bone whose parent isn't selected."""
    selected = set(names)
    bones = [armature.data.bones[n] for n in names]
    return [down_from(b, selected) for b in bones if b.parent is None or b.parent.name not in selected]


def _closest(point, head, tail):
    closest, along = intersect_point_line(point, head, tail)
    return head if along < 0.0 else tail if along > 1.0 else closest


def fit_radius(suite, bone):
    """How thick the body is around the bone: of the meshes it deforms, the thinnest's median
    distance from the bone of the vertices mostly weighted to it."""
    armature = suite.armature
    head, tail = bone.head_local, bone.tail_local
    to_armature = armature.matrix_world.inverted()
    medians = []
    for obj in bpy.data.objects:
        if obj.type != "MESH" or not any(
            m.type == "ARMATURE" and m.object == armature for m in obj.modifiers
        ):
            continue
        group = obj.vertex_groups.get(bone.name)
        if group is None:
            continue
        matrix = to_armature @ obj.matrix_world
        distances = []
        for vertex in obj.data.vertices:
            weight = next((g.weight for g in vertex.groups if g.group == group.index), 0.0)
            if weight < FIT_WEIGHT:
                continue
            point = matrix @ vertex.co
            distances.append((point - _closest(point, head, tail)).length)
        if len(distances) >= FIT_VERTICES:
            medians.append(statistics.median(distances))
    return round(min(medians), 4) if medians else None


def tables(suite):
    """[physics] for suite.toml, and what was left out of it. A bone in two chains is left out
    of the second: the app refuses a suite.toml that has one."""
    chains, colliders, left_out = {}, {}, []
    armature = suite.armature
    if armature is None:
        return {}, left_out
    taken = set()
    for chain in suite.physics_chains:
        name = chain_name(chain)
        bones = [exported_bone(suite, armature, b.bone) for b in chain.bones]
        if not bones or None in bones:
            left_out.append(f"physics chain {name!r}: left out, with a bone that isn't exported")
            continue
        if any(b.name in taken for b in bones) or name in chains:
            left_out.append(f"physics chain {name!r}: left out, as a bone of it is in another chain")
            continue
        if any(b.parent != a for a, b in zip(bones, bones[1:])):
            left_out.append(f"physics chain {name!r}: left out, as its bones aren't parent and child")
            continue
        taken.update(b.name for b in bones)
        chains[name] = {
            "bones": [b.name for b in bones],
            "length": round(bones[-1].length, 6),
            "radius": round(chain.radius, 6),
            "stiffness": round(chain.stiffness, 4),
            "drag": round(chain.drag, 4),
            "gravity": round(chain.gravity, 4),
        }
        if chain.cursor != "PUSHES":
            chains[name]["cursor"] = chain.cursor.lower()
    for collider in suite.physics_colliders:
        bone = exported_bone(suite, armature, collider.bone)
        if bone is None:
            left_out.append(f"collider {collider.bone!r}: left out, as its bone isn't exported")
            continue
        if bone.name in colliders:
            continue
        colliders[bone.name] = {
            "bone": bone.name,
            "length": round(bone.length, 6),
            "radius": round(collider.radius, 6),
        }
    out = {}
    if chains:
        out["chains"] = chains
    if colliders:
        out["colliders"] = colliders
    return out, left_out


def load_into(suite, data):
    """Fills the panel from suite.toml's [physics]. Returns what matches nothing here."""
    found = data.get("physics")
    if found is None:
        return []
    armature = suite.armature
    bones = armature.data.bones if armature is not None else {}
    unmatched = []
    suite.physics_chains.clear()
    suite.physics_colliders.clear()
    suite.physics_owned = True
    for name, table in found.get("chains", {}).items():
        names = table.get("bones", [])
        missing = [n for n in names if n not in bones]
        if missing or not names:
            unmatched.append(f"chain {name}: {', '.join(missing) or 'no bones'}")
            continue
        chain = suite.physics_chains.add()
        for bone in names:
            chain.bones.add().bone = bone
        chain.preset = "CUSTOM"
        chain.stiffness = table.get("stiffness", 0.2)
        chain.drag = table.get("drag", 0.1)
        chain.gravity = table.get("gravity", 1.0)
        chain.radius = table.get("radius", 0.0)
        chain.preset = _matching_preset(chain)
        chain.cursor = {"ignores": "IGNORES", "only": "ONLY"}.get(table.get("cursor"), "PUSHES")
    for name, table in found.get("colliders", {}).items():
        if table.get("bone") not in bones:
            unmatched.append(f"collider {name}: {table.get('bone')}")
            continue
        collider = suite.physics_colliders.add()
        collider.bone = table["bone"]
        collider.radius = table.get("radius", 0.05)
    return unmatched


def _matching_preset(chain):
    values = (chain.stiffness, chain.drag, chain.gravity, chain.radius)
    for key, preset in PHYSICS_PRESETS.items():
        if all(abs(a - b) < 1e-4 for a, b in zip(values, preset)):
            return key
    return "CUSTOM"


def _rest_capsules(suite):
    """Each collider as (name, head, tail, radius), at rest, in the armature's space."""
    out = []
    for collider in suite.physics_colliders:
        bone = suite.armature.data.bones.get(collider.bone)
        if bone is not None:
            out.append((collider.bone, bone.head_local, bone.tail_local, collider.radius))
    return out


def _first_inside(chain, chained, capsules):
    """The first of the chain's bones whose tail is in a collider at rest, and that collider."""
    names = {b.name for b in chained}
    for bone in chained:
        tail_at = bone.tail_local
        for collider, head, tail, radius in capsules:
            if collider not in names and (tail_at - _closest(tail_at, head, tail)).length < radius + chain.radius:
                return bone.name, collider
    return None


def check(suite, add, finding):
    armature = suite.armature
    if armature is None or not (len(suite.physics_chains) or len(suite.physics_colliders)):
        return
    bones = armature.data.bones
    capsules = _rest_capsules(suite)
    owners = {}
    for chain in suite.physics_chains:
        name = chain_name(chain)
        key = f"chain:{name}"

        def warn(message, severity="WARNING"):
            add(finding(f"physics chain {name!r} {message}", armature, severity=severity, physics=key))

        if not len(chain.bones):
            warn("has no bones. Remove it, or select bones in Pose Mode and add a chain from them")
            continue
        unknown = [b.bone for b in chain.bones if b.bone not in bones]
        if unknown:
            warn(f"names {', '.join(unknown)}, which {suite.armature.name} doesn't have: it's left out")
            continue
        for item in chain.bones:
            other = owners.setdefault(item.bone, name)
            if other != name:
                warn(f"shares {item.bone} with the chain {other!r}: it's left out. A bone can only "
                     "swing one way")
        left = [b.bone for b in chain.bones if exported_bone(suite, armature, b.bone) is None]
        if left:
            warn(f"has {', '.join(left)}, which Deform bones only leaves out of the export: it's left out. "
                 "Tick Deform on them, or untick Deform bones only in glTF settings")
        chained = [bones[b.bone] for b in chain.bones]
        if any(b.parent != a for a, b in zip(chained, chained[1:])):
            warn("has bones that aren't each the child of the one before: it's left out")
        # One only the cursor moves keeps out of nothing else.
        inside = chain.cursor != "ONLY" and _first_inside(chain, chained, capsules)
        if inside:
            warn(f"starts with {inside[0]} inside the collider on {inside[1]}: it's pushed out the "
                 "moment it loads. Make the collider or the chain's radius smaller")
    seen = set()
    for collider in suite.physics_colliders:
        key = f"collider:{collider.bone}"
        if collider.bone not in bones:
            add(finding(f"collider {collider.bone!r} names a bone {armature.name} doesn't have: it's "
                        "left out", armature, physics=key))
        elif exported_bone(suite, armature, collider.bone) is None:
            add(finding(f"collider {collider.bone!r} is on a bone Deform bones only leaves out of the "
                        "export: it's left out", armature, physics=key))
        if collider.bone in seen:
            add(finding(f"collider {collider.bone!r} is there twice: only the first is used", armature,
                        severity="INFO", physics=key))
        seen.add(collider.bone)
    if any(c.cursor != "ONLY" for c in suite.physics_chains) and not len(suite.physics_colliders):
        add(finding("physics: no colliders, so chains swing through the body. Select body bones in "
                    "Pose Mode and add colliders from them", armature, severity="INFO"))


def _draw():
    suite = getattr(bpy.context.scene, "charpaper", None)
    if suite is None or not suite.show_physics or suite.armature is None:
        return
    armature = suite.armature
    if not (len(suite.physics_chains) or len(suite.physics_colliders)) or armature.pose is None:
        return
    groups = {}
    for index, collider in enumerate(suite.physics_colliders):
        pose_bone = armature.pose.bones.get(collider.bone)
        if pose_bone is None:
            continue
        picked = index == suite.physics_collider_index
        look = ((*COLLIDER_COLOR, 1.0 if picked else 0.45), 2.5 if picked else 1.0)
        matrix = armature.matrix_world @ pose_bone.matrix
        groups.setdefault(look, []).extend(_capsule(matrix, pose_bone.bone.length, collider.radius))
    for index, chain in enumerate(suite.physics_chains):
        picked = index == suite.physics_chain_index
        color = TOUCHED_COLOR if chain.cursor == "ONLY" else CHAIN_COLOR
        look = ((*color, 1.0 if picked else 0.45), 2.5 if picked else 1.0)
        points = groups.setdefault(look, [])
        for item in chain.bones:
            pose_bone = armature.pose.bones.get(item.bone)
            if pose_bone is None:
                continue
            matrix = armature.matrix_world @ pose_bone.matrix
            head, tail = matrix @ Vector((0, 0, 0)), matrix @ Vector((0, pose_bone.bone.length, 0))
            points.extend((head, tail))
            if chain.radius > 0:
                at_tail = matrix @ Matrix.Translation((0.0, pose_bone.bone.length, 0.0))
                points.extend(_capsule(at_tail, 0.0, chain.radius, 12))
    groups = {look: points for look, points in groups.items() if points}
    if not groups:
        return
    shader = gpu.shader.from_builtin("POLYLINE_UNIFORM_COLOR")
    shader.uniform_float("viewportSize", tuple(float(v) for v in gpu.state.viewport_get()[2:]))
    depth_test = gpu.state.depth_test_get()
    gpu.state.depth_test_set("NONE")
    gpu.state.blend_set("ALPHA")
    for (color, width), points in groups.items():
        shader.uniform_float("lineWidth", width)
        shader.uniform_float("color", color)
        batch_for_shader(shader, "LINES", {"pos": points}).draw(shader)
    gpu.state.blend_set("NONE")
    gpu.state.depth_test_set(depth_test)


def _selected_names(context, suite):
    """The character's bones selected in Pose Mode, in the armature's order."""
    armature = suite.armature
    picked = {b.name for b in context.selected_pose_bones or [] if b.id_data == armature}
    return [name for name in armature.data.bones.keys() if name in picked] if armature else []


class CHARPAPER_OT_physics_chain_add(Operator):
    bl_idname = "charpaper.physics_chain_add"
    bl_label = "Add chains from selected bones"
    bl_description = (
        "Make chains of the character's bones selected in Pose Mode. Select the first bone of each "
        "and the chain runs down to the end; select more of it to stop it earlier or to pick a way at a fork"
    )
    bl_options = {"REGISTER", "UNDO"}

    preset: EnumProperty(items=PRESET_ITEMS[:-1], default="HAIR")

    @classmethod
    def poll(cls, context):
        return suite_of(context).armature is not None

    def execute(self, context):
        suite = suite_of(context)
        names = _selected_names(context, suite)
        if not names:
            self.report({"WARNING"}, f"Select bones of {suite.armature.name} in Pose Mode first")
            return {"CANCELLED"}
        taken = {b.bone for c in suite.physics_chains for b in c.bones}
        added = skipped = 0
        for bones in chains_from(suite.armature, names):
            if any(b.name in taken for b in bones):
                skipped += 1
                continue
            chain = suite.physics_chains.add()
            for bone in bones:
                chain.bones.add().bone = bone.name
            # Through Custom, so that picking the preset it already has still writes its values.
            chain.preset = "CUSTOM"
            chain.preset = self.preset
            taken.update(b.name for b in bones)
            added += 1
        suite.physics_owned = True
        if added:
            suite.physics_chain_index = len(suite.physics_chains) - 1
        redraw_views()
        note = f", {skipped} skipped: already in a chain" if skipped else ""
        self.report({"INFO"} if added else {"WARNING"}, f"{added} chain(s) added{note}")
        return {"FINISHED"} if added else {"CANCELLED"}


class CHARPAPER_OT_physics_chain_remove(Operator):
    bl_idname = "charpaper.physics_chain_remove"
    bl_label = "Remove chain"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return current_chain(suite_of(context)) is not None

    def execute(self, context):
        suite = suite_of(context)
        multi.remove_picked(suite, "physics_chains", "physics_chain_index")
        redraw_views()
        return {"FINISHED"}


class CHARPAPER_OT_physics_collider_add(Operator):
    bl_idname = "charpaper.physics_collider_add"
    bl_label = "Add colliders from selected bones"
    bl_description = (
        "Put a capsule around each of the character's bones selected in Pose Mode, as thick as the "
        "body around it"
    )
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return suite_of(context).armature is not None

    def execute(self, context):
        suite = suite_of(context)
        names = _selected_names(context, suite)
        if not names:
            self.report({"WARNING"}, f"Select bones of {suite.armature.name} in Pose Mode first")
            return {"CANCELLED"}
        have = {c.bone for c in suite.physics_colliders}
        chained = {b.bone for c in suite.physics_chains for b in c.bones}
        added, guessed = 0, 0
        for name in names:
            if name in have or name in chained:
                continue
            bone = suite.armature.data.bones[name]
            collider = suite.physics_colliders.add()
            collider.bone = name
            radius = fit_radius(suite, bone)
            if radius is None:
                radius = max(bone.length * 0.3, 0.01)
                guessed += 1
            collider.radius = max(radius, 0.001)
            added += 1
        suite.physics_owned = True
        if added:
            suite.physics_collider_index = len(suite.physics_colliders) - 1
        redraw_views()
        if not added:
            self.report({"WARNING"}, "Those bones already have colliders or are in chains")
            return {"CANCELLED"}
        note = f"; {guessed} had no mesh weighted to them, so their size is a guess" if guessed else ""
        self.report({"INFO"}, f"{added} collider(s) added{note}")
        return {"FINISHED"}


class CHARPAPER_OT_physics_collider_fit(Operator):
    bl_idname = "charpaper.physics_collider_fit"
    bl_label = "Fit to the mesh"
    bl_description = "Make the capsule as thick as the body around its bone, from the vertices weighted to it"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        suite = suite_of(context)
        return suite.armature is not None and current_collider(suite) is not None

    def execute(self, context):
        suite = suite_of(context)
        colliders = suite.physics_colliders
        fitted, unfit = 0, []
        for index in multi.picked(colliders, suite.physics_collider_index):
            collider = colliders[index]
            bone = suite.armature.data.bones.get(collider.bone)
            radius = fit_radius(suite, bone) if bone is not None else None
            if radius is None:
                unfit.append(collider.bone)
                continue
            # Each to its own mesh, not one to all.
            with multi.alone():
                collider.radius = max(radius, 0.001)
            fitted += 1
        if unfit:
            self.report({"WARNING"}, f"No mesh has enough vertices weighted to {', '.join(unfit)}")
        return {"FINISHED"} if fitted else {"CANCELLED"}


class CHARPAPER_OT_physics_collider_remove(Operator):
    bl_idname = "charpaper.physics_collider_remove"
    bl_label = "Remove collider"
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return current_collider(suite_of(context)) is not None

    def execute(self, context):
        suite = suite_of(context)
        multi.remove_picked(suite, "physics_colliders", "physics_collider_index")
        redraw_views()
        return {"FINISHED"}


class CHARPAPER_OT_physics_select(Operator):
    bl_idname = "charpaper.physics_select"
    bl_label = "Select bones"
    bl_description = "Select its bones in Pose Mode, to see them or to pose them"
    bl_options = {"REGISTER", "UNDO"}

    kind: EnumProperty(items=(("CHAIN", "Chain", ""), ("COLLIDER", "Collider", "")))
    index: IntProperty()

    def execute(self, context):
        suite = suite_of(context)
        armature = suite.armature
        items = suite.physics_chains if self.kind == "CHAIN" else suite.physics_colliders
        if armature is None or not 0 <= self.index < len(items):
            return {"CANCELLED"}
        item = items[self.index]
        names = [b.bone for b in item.bones] if self.kind == "CHAIN" else [item.bone]
        names = [n for n in names if n in armature.data.bones]
        if not names:
            return {"CANCELLED"}
        if context.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        for obj in context.view_layer.objects:
            obj.select_set(False)
        armature.select_set(True)
        context.view_layer.objects.active = armature
        bpy.ops.object.mode_set(mode="POSE")
        for pose_bone in armature.pose.bones:
            select(pose_bone, pose_bone.name in names)
        armature.data.bones.active = armature.data.bones[names[0]]
        if self.kind == "CHAIN":
            suite.physics_chain_index = self.index
        else:
            suite.physics_collider_index = self.index
        return {"FINISHED"}


classes = (
    CHARPAPER_OT_physics_chain_add,
    CHARPAPER_OT_physics_chain_remove,
    CHARPAPER_OT_physics_collider_add,
    CHARPAPER_OT_physics_collider_fit,
    CHARPAPER_OT_physics_collider_remove,
    CHARPAPER_OT_physics_select,
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
