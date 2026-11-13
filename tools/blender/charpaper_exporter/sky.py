import math
import os
import shutil
import tempfile

import bpy

from .isolate import showing

# An environment's World as the panorama the app bakes its sky and reflections from. A World
# that is just an .hdr or .exr Environment Texture into a Background has the file copied as
# it is, with its Strength and Mapping rotation going to suite.toml. Any other World is
# rendered to a panorama by a Cycles panoramic camera, so whatever Blender shows comes over.

KEYS = ("brightness", "sky_rotation_deg")
PANORAMA_EXTENSIONS = (".hdr", ".exr")
MAPS = ("skybox.ktx2", "specular.ktx2", "diffuse.ktx2")
RENDERED_NAME = "sky.exr"
RENDER_SIZE = (2048, 1024)
RENDER_SAMPLES = 32
# The app's brightness for a Strength of 1: Bevy's exposure matching Blender's takes
# 1.2 × 2^9.7 cd/m² to white, rounded.
BRIGHTNESS_PER_STRENGTH = 1000.0
EPSILON = 1e-4


class Plain:
    """A World the file can be copied from."""

    def __init__(self, image, strength, rotation_deg):
        self.image = image
        self.strength = strength
        self.rotation_deg = rotation_deg


def _linked(socket):
    return socket.links[0].from_node if socket.is_linked else None


def _output(world):
    tree = world.node_tree if getattr(world, "use_nodes", True) else None
    if tree is None:
        return None
    for target in ("ALL", "CYCLES", "EEVEE"):
        node = tree.get_output_node(target)
        if node is not None:
            return node
    return None


def _image_extension(image):
    if image.packed_file is not None:
        name = image.filepath or image.name
    else:
        name = image.filepath
    return os.path.splitext(name)[1].lower()


def _mapping_rotation(node):
    """The Z rotation in degrees, as a Point mapping turns the sky, or None if it does more."""
    if node.vector_type not in {"POINT", "TEXTURE"}:
        return None
    # Unlinked, the vector is a constant, and the sky one color.
    vector = node.inputs["Vector"]
    if not vector.is_linked:
        return None
    link = vector.links[0]
    if link.from_node.bl_idname != "ShaderNodeTexCoord" or link.from_socket.name != "Generated":
        return None
    location = node.inputs["Location"].default_value
    rotation = node.inputs["Rotation"].default_value
    scale = node.inputs["Scale"].default_value
    if any(abs(v) > EPSILON for v in location) or any(abs(v) > EPSILON for v in rotation[:2]):
        return None
    if min(scale) <= 0 or max(scale) - min(scale) > EPSILON:
        return None
    degrees = math.degrees(rotation[2])
    return degrees if node.vector_type == "POINT" else -degrees


def plain(world):
    """The World as a Plain sky, or None if it has to be rendered."""
    output = _output(world)
    background = _linked(output.inputs["Surface"]) if output else None
    if background is None or background.bl_idname != "ShaderNodeBackground":
        return None
    if background.inputs["Strength"].is_linked:
        return None
    texture = _linked(background.inputs["Color"])
    if texture is None or texture.bl_idname != "ShaderNodeTexEnvironment":
        return None
    if texture.projection != "EQUIRECTANGULAR" or texture.image is None:
        return None
    if texture.image.source != "FILE" or _image_extension(texture.image) not in PANORAMA_EXTENSIONS:
        return None
    rotation = 0.0
    mapping = _linked(texture.inputs["Vector"])
    if mapping is not None:
        if mapping.bl_idname != "ShaderNodeMapping":
            return None
        rotation = _mapping_rotation(mapping)
        if rotation is None:
            return None
    strength = background.inputs["Strength"].default_value
    return Plain(texture.image, strength, rotation)


def _image_bytes(image):
    if image.packed_file is not None:
        return image.packed_file.data
    path = bpy.path.abspath(image.filepath, library=image.library)
    with open(path, "rb") as file:
        return file.read()


def _render(context, world, path):
    scene = bpy.data.scenes.new("charpaper~sky")
    camera_data = bpy.data.cameras.new("charpaper~sky")
    camera = bpy.data.objects.new("charpaper~sky", camera_data)
    try:
        scene.world = world
        scene.render.engine = "CYCLES"
        scene.cycles.device = "CPU"
        scene.cycles.samples = RENDER_SAMPLES
        scene.cycles.use_denoising = False
        scene.render.resolution_x, scene.render.resolution_y = RENDER_SIZE
        scene.render.resolution_percentage = 100
        scene.render.film_transparent = False
        scene.view_settings.view_transform = "Standard"
        scene.render.image_settings.file_format = "OPEN_EXR"
        scene.render.image_settings.color_depth = "32"
        scene.render.filepath = path
        camera_data.type = "PANO"
        if hasattr(camera_data, "panorama_type"):
            camera_data.panorama_type = "EQUIRECTANGULAR"
        else:
            camera_data.cycles.panorama_type = "EQUIRECTANGULAR"
        # Its view's centre faces +X with +Z up, where an Environment Texture puts the middle
        # of its image.
        camera.rotation_euler = (math.radians(90), 0, math.radians(-90))
        scene.collection.objects.link(camera)
        scene.camera = camera
        with showing(context, scene):
            bpy.ops.render.render(write_still=True, scene=scene.name)
    finally:
        bpy.data.objects.remove(camera, do_unlink=True)
        bpy.data.cameras.remove(camera_data)
        bpy.data.scenes.remove(scene)


def _read(path):
    with open(path, "rb") as file:
        return file.read()


def _same_pixels(a, b):
    images = [bpy.data.images.load(p, check_existing=False) for p in (a, b)]
    try:
        first, second = images
        return tuple(first.size) == tuple(second.size) and first.pixels[:] == second.pixels[:]
    finally:
        for image in images:
            bpy.data.images.remove(image)


def _place(folder, name, data, same=None):
    """Puts the panorama in the folder as the only one. The maps baked from an earlier one go,
    so the app bakes them again. `same` tells an unchanged panorama apart, by default by its
    bytes."""
    path = os.path.join(folder, name)
    unchanged = os.path.isfile(path) and (same(path) if same else _read(path) == data)
    others = [
        n for n in os.listdir(folder)
        if n != name and os.path.splitext(n)[1].lower() in PANORAMA_EXTENSIONS
    ]
    for other in others:
        os.remove(os.path.join(folder, other))
    if not unchanged or others:
        for map_name in MAPS:
            if os.path.isfile(os.path.join(folder, map_name)):
                os.remove(os.path.join(folder, map_name))
    if not unchanged:
        with open(path, "wb") as file:
            file.write(data)
    return path


def export(context, world, folder):
    """Writes the World's panorama into the environment's folder. Returns its path, the
    suite.toml values it sets (None for the app's default), and notes for the viewer."""
    os.makedirs(folder, exist_ok=True)
    sky = plain(world)
    if sky is not None:
        stem = os.path.splitext(os.path.basename(sky.image.filepath or sky.image.name))[0]
        name = bpy.path.clean_name(stem) + _image_extension(sky.image)
        path = _place(folder, name, _image_bytes(sky.image))
        brightness = round(sky.strength * BRIGHTNESS_PER_STRENGTH, 2)
        rotation = round(sky.rotation_deg, 2)
        values = {
            "brightness": None if brightness == BRIGHTNESS_PER_STRENGTH else brightness,
            "sky_rotation_deg": None if rotation == 0 else rotation,
        }
        return path, values, []

    scratch = tempfile.mkdtemp(prefix="charpaper-sky-")
    try:
        rendered = os.path.join(scratch, RENDERED_NAME)
        _render(context, world, rendered)
        # A render's file carries the time it was made, so only its pixels tell.
        def same(old):
            return _same_pixels(old, rendered)

        path = _place(folder, RENDERED_NAME, _read(rendered), same)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    note = (
        f"World {world.name!r} isn't a single .hdr or .exr Environment Texture, so it was "
        f"rendered into {RENDERED_NAME} with Cycles"
    )
    return path, {key: None for key in KEYS}, [note]
