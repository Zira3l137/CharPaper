"""Images as Blender's glTF exporter encodes them, kept for the session.

Every export encodes its textures again: a PNG written out as JPEG, or a roughness map packed
into glTF's metallic-roughness image, a tenth of a second to a third each, over and over for
files that haven't changed. While caching() is on, an image already encoded from the same
files, the same way, comes back from memory.

An image is known by its packed data, or, read from disk, by its file's path, modification time
and size: one generated, painted and unsaved, or tiled is encoded every time.
"""

import hashlib
import os
from collections import OrderedDict
from contextlib import contextmanager

import bpy

LIMIT = 512 * 1024 * 1024

_cache = OrderedDict()
_size = 0
# Packed data's hash by its block: a repacked image gets a new block, so it's never mistaken.
_digests = {}


class _Unkept(Exception):
    pass


def _export_image_class():
    # Where the exporter keeps it moved after 4.2.
    try:
        from io_scene_gltf2.blender.exp.material import encode_image as module
    except ImportError:
        try:
            from io_scene_gltf2.blender.exp.material.extensions import gltf2_blender_image as module
        except ImportError:
            return None
    found = getattr(module, "ExportImage", None)
    return found if found is not None and hasattr(found, "encode") else None


def _image(image):
    if image is None or image.source != "FILE" or image.is_dirty:
        raise _Unkept
    look = (image.colorspace_settings.name, image.alpha_mode)
    packed = image.packed_file
    if packed is not None:
        block = (packed.as_pointer(), packed.size)
        if block not in _digests:
            _digests[block] = hashlib.blake2b(packed.data, digest_size=16).hexdigest()
        return (image.name, "packed", _digests[block], look)
    path = bpy.path.abspath(image.filepath_raw, library=image.library)
    try:
        found = os.stat(path)
    except OSError:
        raise _Unkept from None
    where = os.path.normcase(os.path.realpath(path))
    return (image.name, where, found.st_mtime_ns, found.st_size, look)


def _fill(fill):
    kind = type(fill).__name__
    if kind in {"FillImage", "FillImageRGB2BW"}:
        return (kind, _image(fill.image), str(getattr(fill, "src_chan", None)))
    if kind == "FillWhite":
        return (kind,)
    if kind == "FillWith":
        return (kind, repr(fill.value))
    raise _Unkept


def _key(export_image, mime_type, settings):
    # Computed channels and kept originals are left to the exporter each time.
    if export_image.numpy_calc is not None or export_image.stored or getattr(export_image, "original", None):
        raise _Unkept
    fills = tuple(sorted((str(channel), _fill(fill)) for channel, fill in export_image.fills.items()))
    if not fills:
        raise _Unkept
    # What of the settings can bear on an encoded image: its format, quality and compression.
    words = ("image", "ktx", "zstd", "webp", "quality", "jpeg")
    chosen = tuple(
        sorted(
            (name, value)
            for name, value in settings.items()
            if any(word in name for word in words) and isinstance(value, (bool, int, float, str))
        )
    )
    return (mime_type, fills, getattr(export_image, "is_data", None), chosen)


def _keep(key, value):
    global _size
    _cache[key] = value
    _size += len(value[0])
    while _size > LIMIT and _cache:
        _, (data, _, _) = _cache.popitem(last=False)
        _size -= len(data)


@contextmanager
def caching():
    """Lets the exporter take images encoded by an earlier export, for as long as it runs."""
    found = _export_image_class()
    if found is None:
        yield
        return
    encode = found.encode

    def cached(self, mime_type, export_settings):
        try:
            key = _key(self, mime_type, export_settings)
        except (_Unkept, AttributeError, TypeError):
            key = None
        used = export_settings.get("exported_images")
        if key is not None and key in _cache:
            _cache.move_to_end(key)
            data, factor, marks = _cache[key]
            # Encoding also notes which images the file used: noted again, as it would be.
            if used is not None:
                used.update(marks)
            return data, factor
        before = dict(used) if used is not None else {}
        data, factor = encode(self, mime_type, export_settings)
        if key is not None and isinstance(data, (bytes, bytearray)):
            marks = {k: v for k, v in used.items() if before.get(k) != v} if used is not None else {}
            _keep(key, (bytes(data), factor, marks))
        return data, factor

    found.encode = cached
    try:
        yield
    finally:
        found.encode = encode
