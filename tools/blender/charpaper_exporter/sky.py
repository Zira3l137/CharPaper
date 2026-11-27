import array
import hashlib
import math
import os
import shutil
import tempfile
from contextlib import contextmanager

import bpy

from . import suite_files
from .isolate import showing

# An environment's World as the panorama the app bakes its sky and reflections from, always
# named sky.hdr or sky.exr whatever the image is called.
#
# A World that is just an Environment Texture into a Background is "plain": its image goes
# over at full size, with Strength and the Mapping's Z rotation going to suite.toml. The file
# is copied as it is when the app is sure to read it as Blender does; otherwise Blender
# writes what it shades with into an EXR, which takes in any format, color space or layout
# Blender reads. Any other World is rendered to a panorama by a Cycles panoramic camera.

# brightness is what older add-ons wrote in place of sky_strength; owning it takes it out.
KEYS = ("sky_strength", "sky_rotation_deg", "brightness")
NAME = "sky"
PANORAMA_EXTENSIONS = (".hdr", ".exr")
LINEAR_REC709 = "lin_rec709_scene"
MAPS = ("skybox.ktx2", "specular.ktx2", "diffuse.ktx2")
# In a pack, what the maps were baked from, since the panorama itself stays out.
BAKED_FROM = suite_files.BAKED_FROM
RENDER_SIZE = (2048, 1024)
RENDER_SAMPLES = 32
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
    if texture.image.source != "FILE":
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


def image_path(image):
    return bpy.path.abspath(image.filepath, library=image.library)


def readable(image):
    """Whether Blender has the image's pixels, from its file or packed in the .blend."""
    if image.packed_file is not None or image.has_data:
        return True
    return os.path.isfile(image_path(image))


def _image_bytes(image):
    if image.packed_file is not None:
        return image.packed_file.data
    return _read(image_path(image))


def _linear_rec709(image):
    space = image.colorspace_settings
    if getattr(space, "interop_id", None) is not None:
        if space.interop_id != LINEAR_REC709:
            return False
    elif space.name not in {"Linear Rec.709", "Linear"}:
        return False
    working = getattr(bpy.data, "colorspace", None)
    return getattr(working, "working_space_interop_id", LINEAR_REC709) == LINEAR_REC709


def _hdr_as_app_reads(data):
    # The app's reader takes only this signature and this pixel format.
    if not data.startswith(b"#?RADIANCE"):
        return False
    header = data[: data.find(b"\n\n")] if b"\n\n" in data[:4096] else b""
    formats = [line for line in header.split(b"\n") if line.startswith(b"FORMAT=")]
    return bool(header) and all(line.strip() == b"FORMAT=32-bit_rle_rgbe" for line in formats)


def _exr_as_app_reads(data):
    # The app's reader takes the first layer with R, G and B, and Blender the unnamed layer.
    # They agree on a single part with plain R, G and B channels.
    if data[:4] != b"\x76\x2f\x31\x01" or len(data) < 8:
        return False
    if int.from_bytes(data[4:8], "little") & 0x1800:  # deep data, or several parts
        return False
    at = 8
    while at < len(data) and data[at] != 0:
        name_end = data.index(b"\0", at)
        type_end = data.index(b"\0", name_end + 1)
        size = int.from_bytes(data[type_end + 1 : type_end + 5], "little")
        value = data[type_end + 5 : type_end + 5 + size]
        if data[at:name_end] == b"channels":
            # Each channel: its name, then 16 bytes of pixel type, linearity and sampling.
            names, cursor = set(), 0
            while cursor < len(value) and value[cursor] != 0:
                end = value.index(b"\0", cursor)
                names.add(value[cursor:end])
                cursor = end + 1 + 16
            return {b"R", b"G", b"B"} <= names
        at = type_end + 5 + size
    return False


def _copyable(image, data):
    """Whether the file can go over as it is: the app reads it, and its numbers are those
    Blender shades with."""
    if not _linear_rec709(image):
        return False
    extension = _image_extension(image)
    if extension == ".hdr":
        return _hdr_as_app_reads(data)
    if extension == ".exr":
        return _exr_as_app_reads(data)
    return False


def _exr_settings(settings):
    settings.file_format = "OPEN_EXR"
    settings.color_depth = "32"
    settings.color_mode = "RGB"
    if hasattr(settings, "exr_codec"):
        settings.exr_codec = "ZIP"
    # The app takes linear Rec.709, whatever the .blend works in.
    if getattr(settings, "has_linear_colorspace", False):
        settings.color_management = "OVERRIDE"
        settings.linear_colorspace_settings.name = "Linear Rec.709"


def _reencode(image, path):
    scene = bpy.data.scenes.new("charpaper~sky")
    try:
        _exr_settings(scene.render.image_settings)
        image.save_render(path, scene=scene)
    finally:
        bpy.data.scenes.remove(scene)


@contextmanager
def _cycles():
    """Cycles, switched on for the render if it was off, and off again after."""
    import addon_utils  # Blender's, there once bpy is

    loaded = addon_utils.check("cycles")[1]
    if not loaded:
        addon_utils.enable("cycles", default_set=False)
    try:
        yield
    finally:
        if not loaded:
            addon_utils.disable("cycles", default_set=False)


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
        _exr_settings(scene.render.image_settings)
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
        if not os.path.isfile(path):
            raise RuntimeError(f"rendering World {world.name!r} wrote no image")
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


def _pixel_digest(path):
    image = bpy.data.images.load(path, check_existing=False)
    try:
        pixels = array.array("f", bytes(4 * len(image.pixels)))
        image.pixels.foreach_get(pixels)
        return hashlib.sha256(bytes(str(tuple(image.size)), "ascii") + pixels.tobytes()).hexdigest()
    finally:
        bpy.data.images.remove(image)


def _place(files, folder, name, data, same=None, digest=None):
    """Puts the panorama in the folder as the only one. The maps baked from an earlier one go,
    so they're baked again. `same` tells an unchanged panorama apart, by default by its bytes.

    A pack keeps the maps but not the panorama they came from, so it keeps what that was
    instead: `digest`, by default of the bytes."""
    path = files.path(f"{folder}/{name}")
    old = files.local(path)
    unchanged = old is not None and (same(old) if same else _read(old) == data)
    digest = digest or hashlib.sha256(data).hexdigest()
    if files.packed and old is None:
        maps = all(files.isfile(f"{folder}/{m}") for m in MAPS)
        unchanged = maps and _baked_from(files, folder) == digest
    others = [
        n for n in files.listdir(folder)
        if n != name and os.path.splitext(n)[1].lower() in PANORAMA_EXTENSIONS
    ]
    try:
        for other in others:
            files.remove(f"{folder}/{other}")
        if not unchanged or others:
            for map_name in MAPS:
                if files.isfile(f"{folder}/{map_name}"):
                    files.remove(f"{folder}/{map_name}")
        # Without its maps a packed panorama has to be there to bake them from again.
        if not unchanged or (others and old is None):
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as file:
                file.write(data)
        if files.packed and _baked_from(files, folder) != digest:
            os.makedirs(files.path(folder), exist_ok=True)
            with open(files.path(f"{folder}/{BAKED_FROM}"), "w", encoding="utf-8") as file:
                file.write(digest + "\n")
    except PermissionError as error:
        raise RuntimeError(
            f"can't replace {error.filename}: it's in use. If CharPaper is baking this sky, "
            "export again when it's done"
        ) from error
    return path


def _baked_from(files, folder):
    path = files.local(f"{folder}/{BAKED_FROM}")
    if path is None:
        return None
    with open(path, encoding="utf-8") as file:
        return file.read().strip()


def world_of(scene, entry):
    """The World an environment's sky comes from, or None for none."""
    if not entry.export_sky:
        return None
    return entry.world or scene.world


def hand_placed(files, folder):
    """A panorama in the folder that no export wrote, or None."""
    for name in files.listdir(folder):
        stem, extension = os.path.splitext(name)
        if extension.lower() in PANORAMA_EXTENSIONS and stem != NAME:
            return name
    return None


def export(context, world, files, folder):
    """Writes the World's panorama into the environment's folder. Returns its path, the
    suite.toml values it sets (None for the app's default), and notes for the viewer."""
    os.makedirs(files.path(folder), exist_ok=True)
    sky = plain(world)
    if sky is not None:
        image = sky.image
        if not readable(image):
            raise RuntimeError(
                f"World {world.name!r}: its image {image.filepath!r} is missing; find it with "
                "File > External Data > Find Missing Files, or pack it"
            )
        strength = round(sky.strength, 3)
        rotation = round(sky.rotation_deg, 2)
        values = {
            "sky_strength": None if strength == 1 else strength,
            "sky_rotation_deg": None if rotation == 0 else rotation,
            "brightness": None,
        }
        on_disk = image.packed_file is not None or os.path.isfile(image_path(image))
        data = _image_bytes(image) if on_disk else None
        if data is not None and _copyable(image, data):
            return _place(files, folder, NAME + _image_extension(image), data), values, []
        return _written(files, folder, lambda path: _reencode(image, path)), values, []

    with _cycles():
        path = _written(files, folder, lambda path: _render(context, world, path))
    note = (
        f"World {world.name!r} isn't a single Environment Texture, so it was rendered into "
        f"{NAME}.exr with Cycles"
    )
    return path, {key: None for key in KEYS}, [note]


def _written(files, folder, write):
    """Places the EXR `write` makes, telling an unchanged one by its pixels: the file also
    carries when it was made."""
    scratch = tempfile.mkdtemp(prefix="charpaper-sky-")
    try:
        made = os.path.join(scratch, NAME + ".exr")
        write(made)

        def same(old):
            return _same_pixels(old, made)

        # The file also carries when it was made, so only its pixels tell it apart.
        digest = _pixel_digest(made) if files.packed else None
        return _place(files, folder, NAME + ".exr", _read(made), same, digest)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
