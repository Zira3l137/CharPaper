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


def _numpy_float32():
    # The exporter moves a skinned mesh's values by np.array(matrix), whose precision follows
    # what NumPy makes of a Blender matrix: float64 in 4.2, float32 in 5.3.
    try:
        import numpy
    except ImportError:
        return False
    return numpy.array(Matrix.Identity(3)).dtype == numpy.float32


NUMPY_FLOAT32 = _numpy_float32()
TYPES = {"MESH", "EMPTY", "ARMATURE"}
WATTS_TO_LUMENS = 683
LIGHT_KINDS = {"POINT": "point", "SPOT": "spot", "SUN": "directional"}
# Blender's exporter added a light's exposure, normalization and temperature in 4.5, got the
# temperature right in 5.1, and from 5.2 reads a light's nodes whether or not it uses them,
# except in a scene that renders with EEVEE.
NEWER_LIGHTS = bpy.app.version >= (4, 5)


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
        tree.add_neutral_bones()
        for node in tree.nodes:
            if node.skin is not None:
                # Skinning places the vertices already, and the exporter writes no transform.
                node.local = Matrix.Identity(4)
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
        suite_files.call(suite_files.native.write_scene, path, description, materials)
    except ValueError as error:
        raise Unsupported(str(error)) from error


def _shared(index_of, found, key, describe, context):
    # One glTF light or camera for each Blender one, however many objects show it.
    if key not in index_of:
        description = describe(key, context)
        index_of[key] = None if description is None else len(found)
        if description is not None:
            found.append(description)
    return index_of[key]


def _frozen(matrix):
    return tuple(tuple(row) for row in matrix)


def _emits_through_nodes(lamp, scene):
    """Whether the exporter takes the light's color and strength from its nodes, as it does
    from an Emission shader feeding the active Light Output."""
    if bpy.app.version >= (5, 2):
        if scene.render.engine == "BLENDER_EEVEE":
            return False
        tree = lamp.node_tree
    else:
        tree = lamp.node_tree if lamp.use_nodes else None
    if tree is None:
        return False
    outputs = [n for n in tree.nodes if n.type == "OUTPUT_LIGHT" and n.is_active_output]
    sockets = [n.inputs["Surface"] for n in outputs]
    seen = set()
    while sockets:
        for link in sockets.pop().links:
            node = link.from_node
            if node.type == "EMISSION":
                return True
            if node not in seen:
                seen.add(node)
                sockets.extend(node.inputs)
    return False


def _area(lamp, matrix):
    # The keyword changed name in 5.2.
    try:
        return lamp.area(matrix_world=matrix)
    except TypeError:
        return lamp.area(world_matrix=matrix)


def _light(key, context):
    """The light as the exporter's gather_lights_punctual makes it in this Blender; None for
    an area light, which glTF has no light for."""
    obj, settings, scene = context
    lamp = obj.data
    kind = LIGHT_KINDS.get(lamp.type)
    if kind is None:
        return None
    if _emits_through_nodes(lamp, scene):
        raise Unsupported(f"the light {lamp.name} shines through its nodes")
    intensity = lamp.energy
    color = list(lamp.color)
    if NEWER_LIGHTS:
        if not lamp.normalize:
            intensity *= _area(lamp, obj.matrix_world.copy().freeze())
        if lamp.use_temperature:
            if bpy.app.version < (5, 1):
                # These versions' exporter multiplies a number by a color here, and fails.
                raise Unsupported(f"the light {lamp.name} has a temperature")
            color = [c * t for c, t in zip(color, lamp.temperature_color)]
    if settings.lighting_mode != "RAW":
        if lamp.type != "SUN":
            intensity /= 4 * math.pi
        if settings.lighting_mode == "SPEC":
            intensity *= WATTS_TO_LUMENS
    if NEWER_LIGHTS:
        intensity *= 2 ** lamp.exposure
    cone = None
    if lamp.type == "SPOT":
        angle = lamp.spot_size * 0.5
        cone = (angle - angle * lamp.spot_blend, angle)
    return {
        "name": lamp.name,
        "kind": kind,
        "color": color,
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
    size = len(matrix)
    return [matrix[row][col] for col in range(size) for row in range(size)]


class _Node:
    def __init__(self, name, obj=None, bone=None, armature=None):
        self.name = name
        self.obj = obj
        self.bone = bone
        self.armature = armature
        self.children = []
        # An instancer's collection's offset, which the exporter takes off its translation.
        self.offset = None
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
        self.neutral = set()
        objects = [o for o in scene.objects]
        for obj in objects:
            if obj.type not in TYPES | self.turned:
                raise Unsupported(f"{obj.name} is a {obj.type.lower()}")
            if obj.instance_type not in {"NONE", "COLLECTION"}:
                raise Unsupported(f"{obj.name} instances its {obj.instance_type.lower()}")
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
            if node.offset is not None:
                # Its own transform alone: its objects are placed from the instancer's matrix.
                node.local = node.local @ Matrix.Translation(-node.offset)
        lights_of, cameras_of = {}, {}
        for node in self.nodes:
            if node.obj is not None and node.obj.type == "LIGHT":
                lamp = node.obj.data
                # From 4.5 the exporter keeps one light per light and object matrix: the
                # matrix can scale a light that isn't normalized.
                key = (lamp, _frozen(node.obj.matrix_world)) if NEWER_LIGHTS else lamp
                node.light = _shared(lights_of, self.lights, key, _light, (node.obj, settings, scene))
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

    def _add_object(self, obj, parent, space=None):
        """The object's node and those under it. `space` is the matrix of the instancer showing
        it, for an object in an instanced collection."""
        node = self._add(_Node(obj.name, obj=obj), parent)
        node.world = obj.matrix_world.copy() if space is None else space @ obj.matrix_world
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
        children = self.children_of.get(obj, ())
        if obj.instance_type == "COLLECTION" and obj.instance_collection is not None:
            if children:
                # Blender's exporter versions place these differently.
                raise Unsupported(f"objects are parented to {obj.name}, which instances a collection")
            self._add_instances(obj, node)
        for child in sorted(children, key=lambda c: c.name):
            if space is not None and child.parent_type != "OBJECT":
                raise Unsupported(f"{child.name} in an instanced collection hangs from a bone")
            if child.parent_type == "BONE":
                holder = self.bone_nodes.get((obj, child.parent_bone))
                if holder is None:
                    raise Unsupported(f"{child.name} hangs from bone {child.parent_bone}, which isn't exported")
                self._add_object(child, holder)
            elif child.parent_type in {"OBJECT", "ARMATURE"}:
                self._add_object(child, node, space)
            else:
                raise Unsupported(f"{child.name} is parented to its {child.parent_type.lower()}")
        return node

    def _add_instances(self, instancer, node):
        """The objects of the collection the instancer shows, as the exporter adds them: under
        it, placed by its matrix as the collection's objects are placed in the file."""
        collection = instancer.instance_collection
        if len(collection.children) > 0:
            raise Unsupported(f"{collection.name}, which {instancer.name} instances, holds collections")
        members = set(collection.objects)
        for obj in members:
            if obj.type not in {"MESH", "EMPTY"} or obj.instance_type != "NONE":
                raise Unsupported(f"{obj.name} in the instanced {collection.name} is a {obj.type.lower()} or instances")
            if obj.parent is not None and obj.parent not in members:
                raise Unsupported(f"{obj.name} in the instanced {collection.name} has its parent outside it")
            for child in self.children_of.get(obj, ()):
                if child not in members:
                    raise Unsupported(f"{child.name} is under {obj.name} but outside the instanced {collection.name}")
        for obj in sorted(members, key=lambda o: o.name):
            if obj.parent is None:
                self._add_object(obj, node, space=node.world)
        node.offset = collection.instance_offset.copy()

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
        # The exporter keeps modifiers by type, so with two Armature modifiers the last counts.
        found = [m for m in obj.modifiers if m.type == "ARMATURE"]
        if not found:
            return None
        armature = found[-1].object
        if armature is None:
            return None
        if obj.parent_type == "BONE" and obj.parent == armature:
            return None
        if not any(n.obj == armature for n in self.nodes):
            return None
        return armature

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

    def add_neutral_bones(self):
        """As the exporter, once every mesh is read: a joint for vertices on no bone, last under
        the armature, standing in glTF's axes at the armature's origin."""
        for armature in self.neutral:
            top = next(n for n in self.nodes if n.obj == armature)
            node = self._add(_Node("neutral_bone"), top)
            node.local = Y_UP.copy()
            _, joints, binds = self.skins[armature]
            joints.append(node)
            binds.append((Y_UP @ armature.matrix_world).inverted_safe())

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
        # An object an instanced collection shows many times makes one mesh, as in Blender's.
        key = ("evaluated", obj) if armature is None else None
        for index, found in enumerate(meshes):
            if key is not None and found.key == key:
                return index
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
    if description["skin"] is not None and description["skin"].pop("neutral"):
        tree.neutral.add(armature)
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
    apply = armature.matrix_world.inverted_safe() @ obj.matrix_world
    normal = armature.matrix_world.to_3x3() @ apply.to_3x3().inverted_safe().transposed()
    tangent = apply.to_quaternion().to_matrix()
    counts, found, weights = array.array("i"), array.array("i"), array.array("f")
    unweighted = False
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
        if count == 0:
            # On no bone: the exporter gives it to a neutral bone, made once all meshes are read.
            found.append(len(joints))
            weights.append(1.0)
            count = 1
            unweighted = True
        counts.append(count)
    return {
        "neutral": unweighted,
        "counts": counts.tobytes(),
        "joints": found.tobytes(),
        "weights": weights.tobytes(),
        "object_world": array.array("f", _flat(obj.matrix_world)).tobytes(),
        # Made here, in Blender's own math, as the exporter makes them: see SkinData.
        "normal_matrix": array.array("f", _flat(normal)).tobytes(),
        "tangent_matrix": array.array("f", _flat(tangent)).tobytes(),
        "float32": NUMPY_FLOAT32,
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
