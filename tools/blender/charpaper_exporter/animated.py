import os
import shutil
import uuid
from contextlib import contextmanager

import bpy

from .props import file_stem

# The app plays a GIF on the environment material it's named after, from
# `environment/<name>/animated/<material>.gif`. In Blender that's an Image Texture node showing
# the GIF, which Blender opens as a movie.

ANIMATED_DIR = "animated"
MOVING = {"MOVIE", "SEQUENCE"}


def is_gif(image):
    return image.source == "MOVIE" and os.path.splitext(image.filepath)[1].lower() == ".gif"


def source_path(image):
    return bpy.path.abspath(image.filepath, library=image.library)


def materials(objects):
    found = []
    for obj in objects:
        for slot in getattr(obj, "material_slots", ()):
            if slot.material is not None and slot.material not in found:
                found.append(slot.material)
    return found


def image_nodes(material):
    """Every Image Texture node the material uses, inside node groups too."""
    nodes, seen = [], set()

    def walk(tree):
        if tree is None or tree.as_pointer() in seen:
            return
        seen.add(tree.as_pointer())
        for node in tree.nodes:
            if node.type == "TEX_IMAGE" and node.image is not None:
                nodes.append(node)
            elif node.type == "GROUP":
                walk(node.node_tree)

    walk(material.node_tree)
    return nodes


def moving_images(material):
    images = []
    for node in image_nodes(material):
        if node.image.source in MOVING and node.image not in images:
            images.append(node.image)
    return images


def gif_of(material):
    return next((i for i in moving_images(material) if is_gif(i)), None)


@contextmanager
def stills(objects):
    """Shows a blank still in place of every movie for the length of an export.

    The glTF exporter would save a movie the way it saves a picture, which Blender can't do.
    The still only shows in the app for the moment before the GIF's first frame is decoded.
    Image sequences are left alone: the exporter reads their first file.
    """
    swapped = []
    still = None
    try:
        for material in materials(objects):
            for node in image_nodes(material):
                if node.image.source != "MOVIE":
                    continue
                if still is None:
                    still = bpy.data.images.new("charpaper~" + uuid.uuid4().hex[:8], 4, 4, alpha=True)
                    still.pixels.foreach_set([0.0] * 64)
                swapped.append((node, node.image))
                node.image = still
        yield
    finally:
        for node, image in swapped:
            node.image = image
        if still is not None:
            bpy.data.images.remove(still)


def copy_gifs(objects, directory):
    """Copies each material's GIF into the directory, and returns the written files and
    notes."""
    written, notes = [], []
    for material in materials(objects):
        gif = gif_of(material)
        if gif is None or file_stem(material.name) != material.name:
            continue
        target = os.path.join(directory, material.name + ".gif")
        source = source_path(gif)
        try:
            if not os.path.isfile(source):
                raise FileNotFoundError(f"its file {source} is missing")
            os.makedirs(directory, exist_ok=True)
            # The GIF may already be the one in the suite folder.
            if not (os.path.isfile(target) and os.path.samefile(source, target)):
                shutil.copyfile(source, target)
            written.append(target)
        except Exception as error:
            notes.append(f"GIF on {material.name!r}: {error}")
    return written, notes
