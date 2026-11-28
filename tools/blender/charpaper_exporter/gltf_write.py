"""Model, skin, environment and camera files written by charpaper_pack, in place of
Blender's glTF exporter.

Blender's exporter spends most of a skin in Python, turning every corner of every mesh into a
glTF vertex. Here the add-on reads each mesh out in a few bulk calls, and charpaper_pack builds
the vertices, as Blender's exporter does, on as many cores as there are meshes.

Materials are a different matter: how a node tree becomes a glTF material is the exporter's to
say, setting by setting. So Blender's exporter still writes them, from a stand-in mesh holding
one triangle per material slot, and charpaper_pack takes the materials, textures and images
from that file. What this module can't write as the exporter would, it says, and the file is
then left to Blender's exporter whole.
"""

import array
import math
import os
import shutil
import tempfile

import bpy
from mathutils import Matrix
from mathutils import Quaternion

from . import gltf
from . import suite_files
from .isolate import showing

Y_UP = Matrix(((1.0, 0.0, 0.0, 0.0), (0.0, 0.0, 1.0, 0.0), (0.0, -1.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0)))
# glTF's cameras and lights look down their -Z with +Y up, Blender's down -Z with +Y up in their
# own Z-up space: the exporter turns them -90° about X to match.
TURN = Quaternion((2**0.5 / 2, -(2**0.5) / 2, 0.0, 0.0)).to_matrix().to_4x4()
MIN_INFLUENCE = 0.0001
TYPES = {"MESH", "EMPTY", "ARMATURE"}
WATTS_TO_LUMENS = 683
LIGHT_KINDS = {"POINT": "point", "SPOT": "spot", "SUN": "directional"}


class Unsupported(Exception):
    """What this module doesn't write as Blender's exporter would."""


def available():
    return hasattr(suite_files.native, "write_scene")


def write(context, iso, path, suite, lights=False, cameras=False, animation=None):
    """Writes the isolated scene as Blender's exporter does with the panel's settings, with its
    lights and cameras if asked. `animation`, if given, is called with the node tree once the
    meshes are read and returns what plays in the file, as write_scene takes it. Raises
    Unsupported, having written nothing, for what it can't."""
    settings = suite.gltf
    if settings.file_format != "GLB":
        raise Unsupported("files are .gltf + .bin")
    if settings.vertex_color == "ACTIVE":
        raise Unsupported("Vertex colors is Active")
    with showing(context, iso.scene):
        # The exporter writes the scene as it stands on frame 0, then goes back. A new scene
        # has no depsgraph until asked for one, and frame_set alone would then move nothing.
        frame = iso.scene.frame_current
        context.evaluated_depsgraph_get()
        iso.scene.frame_set(0)
        depsgraph = context.evaluated_depsgraph_get()
        tree = _Tree(iso.scene, settings, lights, cameras)
        meshes = []
        try:
            for node in tree.nodes:
                if node.obj is not None and node.obj.type == "MESH" and node.bone is None:
                    node.mesh = _mesh_index(meshes, node, tree, settings, depsgraph)
        finally:
            # Read out by now. Kept, they crash Blender once its exporter changes frame below.
            for mesh in meshes:
                mesh.release()
        description = {
            "name": iso.scene.name,
            "nodes": [node.description() for node in tree.nodes],
            "roots": tree.roots,
            "skins": tree.skin_descriptions(),
            "meshes": [m.description for m in meshes],
            "lights": tree.lights,
            "cameras": tree.cameras,
            "animation": animation(tree) if animation is not None else None,
        }
        iso.scene.frame_set(frame)
    materials = _materials(context, meshes, suite) if meshes else None
    try:
        suite_files.native.write_scene(path, description, materials)
    except ValueError as error:
        raise Unsupported(str(error)) from error


def _shared(index_of, found, data, describe, context):
    # One glTF light or camera for each Blender one, however many objects show it.
    if data not in index_of:
        description = describe(data, context)
        index_of[data] = None if description is None else len(found)
        if description is not None:
            found.append(description)
    return index_of[data]


def _emits_through_nodes(lamp):
    tree = lamp.node_tree if getattr(lamp, "use_nodes", False) else None
    return tree is not None and any(n.type == "EMISSION" for n in tree.nodes)


def _light(lamp, settings):
    """The light as the exporter's gather_lights_punctual makes it; None for an area light,
    which glTF has no light for."""
    kind = LIGHT_KINDS.get(lamp.type)
    if kind is None:
        return None
    if _emits_through_nodes(lamp):
        raise Unsupported(f"the light {lamp.name} shines through its nodes")
    intensity = lamp.energy
    if settings.lighting_mode != "RAW":
        if lamp.type != "SUN":
            intensity /= 4 * math.pi
        if settings.lighting_mode == "SPEC":
            intensity *= WATTS_TO_LUMENS
    cone = None
    if lamp.type == "SPOT":
        angle = lamp.spot_size * 0.5
        cone = (angle - angle * lamp.spot_blend, angle)
    return {
        "name": lamp.name,
        "kind": kind,
        "color": list(lamp.color),
        "intensity": intensity,
        "range": lamp.cutoff_distance if lamp.use_custom_distance else None,
        "cone": cone,
    }


def _camera(camera, scene):
    """The camera as the exporter's gather_camera makes it, its aspect the scene's render size;
    None for a panoramic one."""
    render = scene.render
    width = render.resolution_x * render.pixel_aspect_x
    height = render.resolution_y * render.pixel_aspect_y
    found = {"name": camera.name, "znear": camera.clip_start, "zfar": camera.clip_end}
    found.update(aspect_ratio=None, yfov=None, xmag=None, ymag=None)
    if camera.type == "PERSP":
        aspect = width / height
        fits_width = camera.sensor_fit != "VERTICAL" if width >= height else camera.sensor_fit == "HORIZONTAL"
        yfov = 2.0 * math.atan(math.tan(camera.angle * 0.5) / aspect) if fits_width else camera.angle
        found.update(kind="perspective", aspect_ratio=aspect, yfov=yfov)
    elif camera.type == "ORTHO":
        square = max(width, height)
        xmag = camera.ortho_scale * (width / square) / 2
        ymag = camera.ortho_scale * (height / square) / 2
        found.update(kind="orthographic", xmag=xmag, ymag=ymag)
    else:
        return None
    return found


def _flat(matrix):
    return [matrix[row][col] for col in range(4) for row in range(4)]


class _Node:
    def __init__(self, name, obj=None, bone=None, armature=None):
        self.name = name
        self.obj = obj
        self.bone = bone
        self.armature = armature
        self.children = []
        # Bones whose own parent isn't exported: the exporter adds them after the tree is made.
        self.late = []
        self.parent = None
        self.world = None
        # Before an object parented to a bone is moved back to the bone's rest, for its own
        # children: Blender's original_matrix_world.
        self.original = None
        self.mesh = None
        self.skin = None
        self.light = None
        self.camera = None

    def description(self):
        return {
            "name": self.name,
            "matrix": array.array("f", _flat(self.local)).tobytes(),
            "children": [c.index for c in self.children],
            "mesh": self.mesh,
            "skin": self.skin,
            "light": self.light,
            "camera": self.camera,
        }


class _Tree:
    """The exporter's node tree: objects as Blender parents them, an armature's bones under it,
    and their world matrices as the exporter works them out, with armatures at rest."""

    def __init__(self, scene, settings, lights=False, cameras=False):
        self.settings = settings
        self.nodes = []
        self.bone_nodes = {}
        self.skins = {}
        # The types the exporter turns, as it does only when it exports them.
        self.turned = {t for t, on in (("LIGHT", lights), ("CAMERA", cameras)) if on}
        self.lights = []
        self.cameras = []
        objects = [o for o in scene.objects]
        for obj in objects:
            if obj.type not in TYPES | self.turned:
                raise Unsupported(f"{obj.name} is a {obj.type.lower()}")
            if obj.instance_type != "NONE":
                raise Unsupported(f"{obj.name} instances something")
        # Blender's obj.children looks through every object in the file for each call.
        self.children_of = {}
        for obj in bpy.data.objects:
            if obj.parent is not None:
                self.children_of.setdefault(obj.parent, []).append(obj)
        in_scene = set(objects)
        roots = [o for o in objects if o.parent not in in_scene]
        self.roots = [self._add_object(o, None).index for o in roots]
        for node in self.nodes:
            node.children.extend(node.late)
        for node in self.nodes:
            node.local = self._local(node)
        lights_of, cameras_of = {}, {}
        for node in self.nodes:
            if node.obj is not None and node.obj.type == "LIGHT":
                node.light = _shared(lights_of, self.lights, node.obj.data, _light, settings)
            elif node.obj is not None and node.obj.type == "CAMERA":
                node.camera = _shared(cameras_of, self.cameras, node.obj.data, _camera, scene)
        # Every armature gets its skin, as in Blender's files, used by a mesh or not.
        for node in self.nodes:
            if node.obj is not None and node.obj.type == "ARMATURE":
                self.skin_of(node.obj)

    def _add(self, node, parent, late=False):
        node.index = len(self.nodes)
        node.parent = parent
        if parent is not None:
            (parent.late if late else parent.children).append(node)
        self.nodes.append(node)
        return node

    def _add_object(self, obj, parent):
        node = self._add(_Node(obj.name, obj=obj), parent)
        node.world = obj.matrix_world.copy()
        if parent is not None and parent.bone is not None:
            armature = parent.armature
            pose = armature.matrix_world @ armature.pose.bones[parent.bone.name].matrix @ Y_UP
            rest = armature.matrix_world @ parent.bone.matrix_local @ Y_UP
            node.original = node.world
            node.world = rest @ (pose.inverted_safe() @ node.world)
        elif parent is not None and parent.original is not None:
            node.world = node.world @ self._turn(obj)
            node.original = node.world
            node.world = parent.world @ (parent.original.inverted_safe() @ node.world)
        # In the exporter's order: after moving to a bone's rest, before following a parent's
        # move to one.
        if node.original is None or parent.bone is not None:
            node.world = node.world @ self._turn(obj)
        if obj.type == "ARMATURE":
            self._add_bones(obj, node)
        for child in sorted(self.children_of.get(obj, ()), key=lambda c: c.name):
            if child.parent_type == "BONE":
                holder = self.bone_nodes.get((obj, child.parent_bone))
                if holder is None:
                    raise Unsupported(f"{child.name} hangs from bone {child.parent_bone}, which isn't exported")
                self._add_object(child, holder)
            elif child.parent_type in {"OBJECT", "ARMATURE"}:
                self._add_object(child, node)
            else:
                raise Unsupported(f"{child.name} is parented to its {child.parent_type.lower()}")
        return node

    def _turn(self, obj):
        return TURN if obj.type in self.turned else Matrix.Identity(4)

    def _exported(self, bone):
        return bone.use_deform or not self.settings.deform_bones_only

    def _add_bones(self, armature, under):
        def add(bone, parent, skipped):
            if self._exported(bone):
                node = _Node(bone.name, bone=bone, armature=armature)
                node = self._add(node, parent, late=skipped)
                node.world = armature.matrix_world @ bone.matrix_local @ Y_UP
                self.bone_nodes[(armature, bone.name)] = node
                parent, skipped = node, False
            else:
                skipped = True
            for child in bone.children:
                add(child, parent, skipped)

        for bone in armature.data.bones:
            if bone.parent is None:
                add(bone, under, False)

    def _local(self, node):
        parent = node.parent
        if parent is None:
            return node.world
        if parent.skin is not None or self._skinned(parent):
            grand = parent.parent
            if grand is not None and grand.obj is not None and grand.obj.type == "ARMATURE":
                return grand.world.inverted_safe() @ node.world
            return node.world
        return parent.world.inverted_safe() @ node.world

    def _skinned(self, node):
        return node.obj is not None and node.obj.type == "MESH" and self.armature_of(node.obj) is not None

    def armature_of(self, obj):
        """The armature skinning the object, as the exporter decides: its Armature modifier's,
        unless the object hangs from one of that armature's bones."""
        for modifier in obj.modifiers:
            if modifier.type == "ARMATURE":
                armature = modifier.object
                if armature is None:
                    return None
                if obj.parent_type == "BONE" and obj.parent == armature:
                    return None
                if not any(n.obj == armature for n in self.nodes):
                    return None
                return armature
        return None

    def skin_of(self, armature):
        """The skin of an armature's exported bones, joints in tree order, made once."""
        if armature not in self.skins:
            def bones(node):
                for child in node.children:
                    if child.bone is not None:
                        yield child
                        yield from bones(child)

            top = next(n for n in self.nodes if n.obj == armature)
            joints = list(bones(top))
            binds = [(Y_UP @ (armature.matrix_world @ n.bone.matrix_local)).inverted_safe() for n in joints]
            self.skins[armature] = (len(self.skins), joints, binds)
        return self.skins[armature]

    def skin_descriptions(self):
        found = sorted(self.skins.items(), key=lambda item: item[1][0])
        return [
            {
                "name": armature.name,
                "joints": [n.index for n in joints],
                "inverse_binds": array.array("f", [v for m in binds for v in _flat(m)]).tobytes(),
            }
            for armature, (_, joints, binds) in found
        ]


class _Mesh:
    """One glTF mesh: the Blender mesh an object shows, and for an object with modifiers the
    mesh they make, which is let go of afterwards."""

    def __init__(self, key, description, materials, uv_names, colors, owner=None):
        self.key = key
        self.description = description
        self.materials = materials
        self.uv_names = uv_names
        self.colors = colors
        self._owner = owner

    def release(self):
        if self._owner is not None:
            self._owner.to_mesh_clear()
            self._owner = None


def _mesh_index(meshes, node, tree, settings, depsgraph):
    obj = node.obj
    armature = tree.armature_of(obj)
    modifiers = [m for m in obj.modifiers]
    owner = None
    if settings.apply_modifiers and modifiers:
        # As the exporter: skinning stays the app's to do, so armatures don't deform the mesh read.
        hidden = [m for m in modifiers if m.type == "ARMATURE" and m.show_viewport]
        for modifier in hidden:
            modifier.show_viewport = False
        try:
            depsgraph = bpy.context.evaluated_depsgraph_get()
            owner = obj.evaluated_get(depsgraph)
            mesh = owner.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)
        finally:
            for modifier in hidden:
                modifier.show_viewport = True
        # The evaluated mesh's materials are the depsgraph's copies, gone with the mesh: the
        # originals are what Blender's exporter reads, and what lasts.
        materials = tuple(m.original if m is not None else None for m in mesh.materials)
        if len(materials) == 1 and materials[0] is None:
            materials = tuple(slot.material for slot in obj.material_slots)
        key = None
    else:
        mesh = obj.data
        materials = tuple(slot.material for slot in obj.material_slots)
        # Objects sharing a mesh share one glTF mesh, when nothing changes it per object.
        key = (mesh, materials) if armature is None else None
        for index, found in enumerate(meshes):
            if key is not None and found.key == key:
                return index
    if armature is not None:
        _, joints, _ = tree.skin_of(armature)
        node.skin = tree.skins[armature][0]
    try:
        description = _read_mesh(obj, mesh, armature, joints if armature is not None else None, settings)
    except Exception:
        if owner is not None:
            owner.to_mesh_clear()
        raise
    uv_names = [uv.name for uv in mesh.uv_layers]
    colors = [(c.name, c.domain, c.data_type) for c in mesh.color_attributes]
    active = (mesh.uv_layers.active_index, _render_uv(mesh), mesh.color_attributes.active_color_index,
              mesh.color_attributes.render_color_index)
    meshes.append(_Mesh(key, description, materials, (uv_names, active), colors, owner))
    return len(meshes) - 1


def _render_uv(mesh):
    for i, layer in enumerate(mesh.uv_layers):
        if layer.active_render:
            return i
    return -1


def _buffer(kind, count):
    return array.array(kind, bytes(array.array(kind).itemsize * count))


def _read_mesh(obj, mesh, armature, joints, settings):
    vertices, corners = len(mesh.vertices), len(mesh.loops)
    keys = []
    if mesh.shape_keys is not None:
        blocks = mesh.shape_keys.key_blocks
        keys = [k for k in blocks if not (k == k.relative_key or k.mute or k.name == blocks[0].name)]

    positions = _buffer("f", 3 * vertices)
    if keys:
        keys[0].relative_key.points.foreach_get("co", positions)
    else:
        mesh.attributes["position"].data.foreach_get("vector", positions)
    corner_verts = _buffer("i", corners)
    mesh.attributes[".corner_vert"].data.foreach_get("value", corner_verts)
    if keys:
        normals = array.array("f", keys[0].relative_key.normals_split_get())
    else:
        normals = _buffer("f", 3 * corners)
        mesh.corner_normals.foreach_get("vector", normals)

    uvs = []
    if mesh.uv_layers.active:
        for layer in mesh.uv_layers:
            data = _buffer("f", 2 * corners)
            layer.uv.foreach_get("vector", data)
            uvs.append(data.tobytes())

    tangents = signs = None
    if settings.tangents and mesh.uv_layers.active:
        try:
            mesh.calc_tangents()
        except Exception as error:
            print(f"CharPaper: {mesh.name}: no tangents: {error}")
        else:
            tangents, signs = _buffer("f", 3 * corners), _buffer("f", corners)
            mesh.loops.foreach_get("tangent", tangents)
            mesh.loops.foreach_get("bitangent_sign", signs)

    mesh.calc_loop_triangles()
    triangles = _buffer("i", 3 * len(mesh.loop_triangles))
    mesh.loop_triangles.foreach_get("loops", triangles)
    slots = _buffer("i", len(mesh.loop_triangles))
    mesh.loop_triangles.foreach_get("material_index", slots)

    shape_keys = []
    for key in keys:
        points = _buffer("f", 3 * vertices)
        key.points.foreach_get("co", points)
        shape_keys.append({
            "name": key.name,
            "weight": key.value,
            "positions": points.tobytes(),
            "normals": array.array("f", key.normals_split_get()).tobytes(),
        })

    skin = None
    if armature is not None:
        skin = _influences(obj, mesh, joints, armature)
    return {
        "name": mesh.name,
        "positions": positions.tobytes(),
        "corner_verts": corner_verts.tobytes(),
        "normals": normals.tobytes(),
        "uvs": uvs,
        "tangents": tangents.tobytes() if tangents is not None else None,
        "signs": signs.tobytes() if signs is not None else None,
        "triangles": triangles.tobytes(),
        "triangle_slots": slots.tobytes(),
        "shape_keys": shape_keys,
        "skin": skin,
    }


def _influences(obj, mesh, joints, armature):
    joint_of = {node.name: i for i, node in enumerate(joints)}
    group_joint = [joint_of.get(group.name) for group in obj.vertex_groups]
    counts, found, weights = array.array("i"), array.array("i"), array.array("f")
    # Vertex groups have no bulk read, so this is the one loop over vertices left in Python.
    for vertex in mesh.vertices:
        count = 0
        for element in vertex.groups:
            weight = element.weight
            if weight <= MIN_INFLUENCE or element.group >= len(group_joint):
                continue
            joint = group_joint[element.group]
            if joint is None:
                continue
            found.append(joint)
            weights.append(weight)
            count += 1
        counts.append(count)
    return {
        "counts": counts.tobytes(),
        "joints": found.tobytes(),
        "weights": weights.tobytes(),
        "object_world": array.array("f", _flat(obj.matrix_world)).tobytes(),
        "armature_world": array.array("f", _flat(armature.matrix_world)).tobytes(),
    }


def _materials(context, meshes, suite):
    """The .glb Blender's exporter writes of every mesh's materials, from stand-ins: one
    triangle per material slot, each with the mesh's UV maps and color attributes, so the
    exporter sees what it would on the mesh itself."""
    scene = bpy.data.scenes.new("charpaper~materials")
    made = []
    scratch = tempfile.mkdtemp(prefix="charpaper-materials-")
    try:
        render, theirs = scene.render, context.scene.render
        render.fps, render.fps_base = theirs.fps, theirs.fps_base
        for index, found in enumerate(meshes):
            slots = max(1, len(found.materials))
            data = bpy.data.meshes.new(f"charpaper~proxy~{index}")
            data.from_pydata(
                [corner for s in range(slots) for corner in ((s, 0.0, 0.0), (s, 1.0, 0.0), (s, 0.0, 1.0))],
                [],
                [(3 * s, 3 * s + 1, 3 * s + 2) for s in range(slots)],
            )
            uv_names, (active_uv, render_uv, active_color, render_color) = found.uv_names
            for name in uv_names:
                data.uv_layers.new(name=name)
            if uv_names:
                data.uv_layers.active_index = active_uv
                if render_uv >= 0:
                    data.uv_layers[render_uv].active_render = True
            for name, domain, kind in found.colors:
                data.color_attributes.new(name=name, type=kind, domain=domain)
            if found.colors:
                data.color_attributes.active_color_index = active_color
                data.color_attributes.render_color_index = render_color
            for material in found.materials:
                data.materials.append(material)
            data.polygons.foreach_set("material_index", list(range(slots)))
            obj = bpy.data.objects.new(f"charpaper~proxy~{index}", data)
            scene.collection.objects.link(obj)
            made.append((obj, data))
        path = os.path.join(scratch, "materials.glb")
        options = dict(gltf.settings_options(suite.gltf))
        options.update(gltf.LOCKED)
        options.update(
            export_format="GLB",
            export_animations=False,
            export_skins=False,
            export_morph=False,
            export_lights=False,
            export_cameras=False,
            export_apply=False,
        )
        with showing(context, scene):
            gltf.export(path, options)
        with open(path, "rb") as file:
            return file.read()
    finally:
        for obj, data in made:
            bpy.data.objects.remove(obj)
            bpy.data.meshes.remove(data)
        bpy.data.scenes.remove(scene)
        shutil.rmtree(scratch, ignore_errors=True)
