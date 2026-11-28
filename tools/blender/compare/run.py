"""Exports a test character loose, through Blender's glTF exporter, and packed, through
charpaper_pack, then compares every model, skin, environment, camera and clip file between the
two. Run it with
the Blender the add-on is for, with charpaper_pack installed in Blender's Python:

    blender -b --factory-startup --python tools/blender/compare/run.py -- <work folder>

or with the bpy module from PyPI:

    python tools/blender/compare/run.py <work folder>

It exits with 1 if anything differs."""

import os
import shutil
import sys
import types

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:0] = [HERE, os.path.dirname(HERE)]
try:
    import aud  # noqa: F401
except ImportError:
    # The bpy module goes without Blender's audio module, which the add-on imports.
    sys.modules["aud"] = types.ModuleType("aud")

import bpy

import character
import charpaper_exporter
import compare
from charpaper_exporter import suite_files

VARIANTS = {
    "default": {},
    "deform-only": {"deform_bones_only": True},
    "every-2nd-frame": {"frame_step": 2},
}


def export(blend, folder, unpacked, settings):
    bpy.ops.wm.open_mainfile(filepath=blend)
    suite = bpy.context.scene.charpaper
    shutil.rmtree(folder, ignore_errors=True)
    suite.folder = folder
    suite.export_unpacked = unpacked
    for name, value in settings.items():
        setattr(suite.gltf, name, value)
    bpy.ops.charpaper.export(what="ALL", index=-1)


def main(work):
    if not suite_files.available():
        print("charpaper_pack isn't installed in this Python: see docs/development.md")
        return 2
    os.makedirs(work, exist_ok=True)
    blend = os.path.join(work, "character.blend")
    character.build(os.path.join(work, "unused"), os.path.join(work, "textures"))
    bpy.ops.wm.save_as_mainfile(filepath=blend)

    problems = 0
    for variant, settings in VARIANTS.items():
        print(f"\n== {variant}")
        loose = os.path.join(work, variant, "loose")
        packed = os.path.join(work, variant, "packed")
        export(blend, loose, True, settings)
        export(blend, packed, False, settings)
        pack = suite_files.native.Pack(os.path.join(packed, suite_files.native.PACK_FILE))
        scenes = ["model.glb", "environment/Room.glb"]
        scenes += sorted(p for f in ("skins", "cameras") for p in pack.files_in(f) if p.endswith(".glb"))
        for path in scenes:
            problems += compare.scene(loose, pack, path)
        problems += compare.clips(loose, pack, settings.get("frame_step", 1))
    print(f"\n{problems} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    if len(args) != 1:
        print(__doc__)
        code = 2
    else:
        charpaper_exporter.register()
        code = main(os.path.abspath(args[0]))
    sys.stdout.flush()
    # Leaving without Python's own shutdown: the bpy module can crash on it once an add-on
    # is registered, which would hide the exit code.
    os._exit(code)
