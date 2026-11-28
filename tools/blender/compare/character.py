"""Builds a test character in an empty scene, with something of everything a packed export
writes itself: a ~90-bone rig with bones that don't deform, two skinned bodies with shape keys,
one smoothed by a modifier on top of its armature, two material slots, a hat on a bone with an
empty on it, clips, an environment with props moving under one another, lights of every kind,
a small rig holding a lantern and a lamp on its bones, a still camera, and a moving one on a
rail that cuts from one shot to the next, with a focus object moving too."""

import math
import os
import random

import bpy
from mathutils import Matrix

CLIPS = 6
FRAMES = 120


def build(folder, textures):
    """Builds the character and points its suite at `folder`; images go in `textures`."""
    random.seed(1)
    os.makedirs(textures, exist_ok=True)
    scene = bpy.context.scene
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj)
    scene.frame_start, scene.frame_end = 1, FRAMES
    scene.render.fps = 24

    rig = _rig(scene)
    bones = [b.name for b in rig.data.bones]
    skins = []
    for name, segments in (("Casual", 96), ("Formal", 80)):
        collection = bpy.data.collections.new(name)
        scene.collection.children.link(collection)
        _body(f"{name}Body", collection, segments, _textured(name, 512, textures), rig, bones)
        skins.append(collection)
    casual, formal = skins

    bpy.ops.mesh.primitive_cone_add(radius1=0.2, depth=0.3)
    hat = _move_to(bpy.context.active_object, "Hat", casual)
    hat.data.materials.append(_textured("HatCloth", 64, textures))
    hat.parent, hat.parent_type, hat.parent_bone = rig, "BONE", "Head"
    hat.matrix_world = Matrix.Translation((0.02, 0.01, 1.85)) @ Matrix.Rotation(0.3, 4, "X")
    feather = bpy.data.objects.new("Feather", None)
    casual.objects.link(feather)
    feather.parent, feather.location = hat, (0, 0.1, 0.15)

    body = casual.objects["CasualBody"]
    body.data.materials.append(_textured("Belt", 64, textures))
    for polygon in body.data.polygons:
        if abs(polygon.center.z - 1.1) < 0.05:
            polygon.material_index = 1
    body = formal.objects["FormalBody"]
    bpy.context.view_layer.objects.active = body
    bpy.ops.object.shape_key_remove(all=True, apply_mix=False)
    body.modifiers.new("Smooth", "SUBSURF").levels = 1

    _clips(rig)
    room = _room(scene, textures)
    for name in ("Spine1", "Hair00", "Hair20", "Hand.L"):
        rig.data.bones[name].use_deform = False

    suite = scene.charpaper
    suite.custom_folder = True
    suite.folder = folder
    suite.suite_name = "Compare"
    suite.armature = rig
    for collection in skins:
        skin = suite.skins.add()
        skin.name, skin.collection = collection.name, collection
    environment = suite.environments.add()
    environment.name, environment.collection = "Room", room
    environment.export_sky = False
    for camera, action in _cameras(scene):
        entry = suite.cameras.add()
        entry.name, entry.camera, entry.action = camera.name, camera, action


def _rig(scene):
    data = bpy.data.armatures.new("Rig")
    rig = bpy.data.objects.new("Rig", data)
    scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")

    def bone(name, head, tail, parent=None):
        edit = data.edit_bones.new(name)
        edit.head, edit.tail = head, tail
        if parent:
            edit.parent = data.edit_bones[parent]
        return name

    spine = bone("Hips", (0, 0, 1), (0, 0, 1.1))
    for i in range(4):
        spine = bone(f"Spine{i}", (0, 0, 1.1 + i * 0.1), (0, 0, 1.2 + i * 0.1), spine)
    bone("Head", (0, 0, 1.5), (0, 0, 1.7), spine)
    for side, x in (("L", 1), ("R", -1)):
        arm = bone(f"Arm.{side}", (x * 0.1, 0, 1.45), (x * 0.4, 0, 1.45), spine)
        arm = bone(f"Fore.{side}", (x * 0.4, 0, 1.45), (x * 0.7, 0, 1.45), arm)
        hand = bone(f"Hand.{side}", (x * 0.7, 0, 1.45), (x * 0.8, 0, 1.45), arm)
        for f in range(5):
            finger = hand
            for j in range(3):
                head = (x * (0.8 + j * 0.03), f * 0.01, 1.45)
                tail = (x * (0.83 + j * 0.03), f * 0.01, 1.45)
                finger = bone(f"F{f}{j}.{side}", head, tail, finger)
        leg = bone(f"Leg.{side}", (x * 0.1, 0, 1), (x * 0.1, 0, 0.5), "Hips")
        bone(f"Shin.{side}", (x * 0.1, 0, 0.5), (x * 0.1, 0, 0.05), leg)
    for h in range(6):
        hair = "Head"
        for j in range(4):
            x = h * 0.02 - 0.05
            hair = bone(f"Hair{h}{j}", (x, -0.1, 1.7 - j * 0.1), (x, -0.1, 1.6 - j * 0.1), hair)
    bpy.ops.object.mode_set(mode="OBJECT")
    return rig


def _textured(name, size, textures):
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    shader = nodes["Principled BSDF"]
    for socket in ("Base Color", "Roughness"):
        image = bpy.data.images.new(f"{name}_{socket}", size, size)
        image.pixels.foreach_set([random.random() for _ in range(64)] * (size * size * 4 // 64))
        image.filepath_raw = os.path.join(textures, f"{name}_{socket}.png")
        image.file_format = "PNG"
        image.save()
        texture = nodes.new("ShaderNodeTexImage")
        texture.image = image
        material.node_tree.links.new(texture.outputs["Color"], shader.inputs[socket])
    return material


def _move_to(obj, name, collection):
    obj.name = name
    for old in obj.users_collection:
        old.objects.unlink(obj)
    collection.objects.link(obj)
    return obj


def _body(name, collection, segments, material, rig, bones):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segments, ring_count=segments // 2, radius=0.4, location=(0, 0, 1.1)
    )
    body = _move_to(bpy.context.active_object, name, collection)
    body.scale.z = 2.2
    bpy.ops.object.transform_apply(scale=True)
    body.data.materials.append(material)
    body.parent = rig
    body.modifiers.new("Armature", "ARMATURE").object = rig
    groups = {n: body.vertex_groups.new(name=n) for n in bones}
    heads = {b.name: b.head_local for b in rig.data.bones}
    for vertex in body.data.vertices:
        nearest = sorted(bones, key=lambda n: (heads[n] - vertex.co).length_squared)[:2]
        for bone, weight in zip(nearest, (0.7, 0.3)):
            groups[bone].add([vertex.index], weight, "REPLACE")
    body.shape_key_add(name="Basis")
    for k in range(6):
        key = body.shape_key_add(name=f"Key{k}")
        for point in key.data:
            point.co.x += 0.01 * math.sin(k + point.co.z)


def _clips(rig):
    rig.animation_data_create()
    for c in range(CLIPS):
        action = bpy.data.actions.new(f"Clip{c:02d}")
        rig.animation_data.action = action
        for pose in rig.pose.bones:
            pose.rotation_mode = "QUATERNION"
            for frame in range(1, FRAMES + 1, 8):
                a = math.sin(frame * 0.05 + c) * 0.3
                pose.rotation_quaternion = (math.cos(a), math.sin(a) * random.random(), 0, 0)
                pose.keyframe_insert("rotation_quaternion", frame=frame)
        rig.charpaper.clips.add().action = action
    rig.animation_data.action = None


def _room(scene, textures):
    room = bpy.data.collections.new("Room")
    scene.collection.children.link(room)
    material = _textured("Room", 64, textures)
    for i in range(3):
        bpy.ops.mesh.primitive_cube_add(location=(i - 1, 2, 0.5))
        cube = _move_to(bpy.context.active_object, f"Box{i}", room)
        cube.data.materials.append(material)

    # A fan spinning on an empty that slides about, each with an action of its own.
    pivot = bpy.data.objects.new("Pivot", None)
    room.objects.link(pivot)
    pivot.location = (1, 1, 1)
    pivot.animation_data_create()
    pivot.animation_data.action = bpy.data.actions.new("Slide")
    for frame, x in ((1, 0.0), (40, 2.0), (80, -1.0), (FRAMES, 0.0)):
        pivot.location.x, pivot.rotation_euler.z = x, x * 0.3
        pivot.keyframe_insert("location", frame=frame)
        pivot.keyframe_insert("rotation_euler", frame=frame)
    bpy.ops.mesh.primitive_cube_add(size=0.3)
    fan = _move_to(bpy.context.active_object, "Fan", room)
    fan.parent, fan.location, fan.scale = pivot, (0, 0, 0.5), (1, 2, 1)
    fan.animation_data_create()
    fan.animation_data.action = bpy.data.actions.new("Spin")
    for frame, a in ((1, 0.0), (60, 3.0), (FRAMES, 6.28)):
        fan.rotation_euler = (0.2, a * 0.5, a)
        fan.keyframe_insert("rotation_euler", frame=frame)

    _lights(room)
    _crane(scene, room)
    return room


def _lights(room):
    def lamp(name, kind, location, **settings):
        data = bpy.data.lights.new(name, kind)
        for key, value in settings.items():
            setattr(data, key, value)
        obj = bpy.data.objects.new(name, data)
        room.objects.link(obj)
        obj.location = location
        obj.rotation_euler = (0.6, 0.1, 0.4)
        return obj

    lamp("Bulb", "POINT", (0, 1, 2), energy=200.0, color=(1.0, 0.8, 0.6))
    lamp("Reach", "POINT", (2, 0, 2), energy=50.0, use_custom_distance=True, cutoff_distance=3.0)
    lamp("Cone", "SPOT", (-1, 1, 3), energy=500.0, spot_size=0.9, spot_blend=0.3)
    lamp("Sun", "SUN", (0, 0, 5), energy=3.0)
    lamp("Panel", "AREA", (1, -1, 3), energy=100.0)


def _crane(scene, room):
    # A rig of the environment's own: a lantern hangs from its arm, a lamp on its tip with an
    # empty under the lamp, so a turned light sits on a bone with something below it.
    data = bpy.data.armatures.new("Crane")
    crane = bpy.data.objects.new("Crane", data)
    room.objects.link(crane)
    crane.location = (-2, 1, 0)
    bpy.context.view_layer.objects.active = crane
    bpy.ops.object.mode_set(mode="EDIT")
    mast = data.edit_bones.new("Mast")
    mast.head, mast.tail = (0, 0, 0), (0, 0, 1.5)
    arm = data.edit_bones.new("Arm")
    arm.head, arm.tail, arm.parent = (0, 0, 1.5), (0.8, 0, 1.6), mast
    bpy.ops.object.mode_set(mode="OBJECT")
    crane.pose.bones["Arm"].rotation_mode = "XYZ"
    crane.pose.bones["Arm"].rotation_euler = (0.0, 0.3, 0.2)

    bpy.ops.mesh.primitive_cube_add(size=0.2)
    lantern = _move_to(bpy.context.active_object, "Lantern", room)
    lantern.parent, lantern.parent_type, lantern.parent_bone = crane, "BONE", "Arm"
    lantern.matrix_world = Matrix.Translation((-1.3, 1, 1.4))
    beam = bpy.data.objects.new("Beam", bpy.data.lights.new("Beam", "SPOT"))
    room.objects.link(beam)
    beam.parent, beam.parent_type, beam.parent_bone = crane, "BONE", "Arm"
    beam.matrix_world = Matrix.Translation((-1.2, 1, 1.6)) @ Matrix.Rotation(0.5, 4, "Y")
    glow = bpy.data.objects.new("Glow", None)
    room.objects.link(glow)
    glow.parent, glow.location = beam, (0, 0, -0.4)


def _cameras(scene):
    still = bpy.data.objects.new("Still", bpy.data.cameras.new("Still"))
    scene.collection.objects.link(still)
    still.location, still.rotation_euler = (0, -4, 1.6), (1.4, 0, 0)

    rail = bpy.data.objects.new("Rail", None)
    scene.collection.objects.link(rail)
    rail.location, rail.rotation_euler = (1, -3, 1), (0, 0, 0.3)
    lens = bpy.data.cameras.new("Dolly")
    lens.lens, lens.sensor_fit, lens.dof.use_dof = 35.0, "VERTICAL", True
    dolly = bpy.data.objects.new("Dolly", lens)
    scene.collection.objects.link(dolly)
    dolly.parent, dolly.rotation_euler = rail, (1.3, 0, 0)
    dolly.animation_data_create()
    shot = bpy.data.actions.new("Shot")
    dolly.animation_data.action = shot
    # A cut: the location jumps at frame 60, keyed Constant; the rotation pans smoothly.
    for frame, x in ((1, 0.0), (60, 2.0), (FRAMES, 2.5)):
        dolly.location.x = x
        dolly.keyframe_insert("location", frame=frame)
    for curve in shot.fcurves:
        if curve.data_path == "location":
            for point in curve.keyframe_points:
                point.interpolation = "CONSTANT"
    for frame, z in ((1, 0.0), (FRAMES, 0.5)):
        dolly.rotation_euler.z = z
        dolly.keyframe_insert("rotation_euler", frame=frame)
    dolly.animation_data.action = None

    target = bpy.data.objects.new("Target", None)
    scene.collection.objects.link(target)
    target.animation_data_create()
    target.animation_data.action = bpy.data.actions.new("Drift")
    for frame, y in ((1, 0.0), (FRAMES, 1.0)):
        target.location.y = y
        target.keyframe_insert("location", frame=frame)
    lens.dof.focus_object = target
    return [(still, None), (dolly, shot)]
