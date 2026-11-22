"""Moves a suite's settings from before clips lived on objects into the new places, once, when a
.blend that has them opens: the old animation files' clips onto the armature's clip list, the
skins' expressions onto the character's. The old settings are left as they were. A suite from
before the app's folder could be picked keeps exporting where it did."""

import os

import bpy
from bpy.app.handlers import persistent

from . import manifest
from .props import DEFAULT_FOLDER
from .props import file_stem

MODES = {"LOOP": "TIMELINE", "ONCE": "ONCE", "POSE": "HOLD"}


def migrate(suite):
    """Returns how many clips and expressions moved, or None when there was nothing to do."""
    if suite.migrated:
        return None
    suite.migrated = True
    clips = _clips(suite)
    expressions = _expressions(suite)
    if not (clips or expressions):
        return None
    return clips, expressions


def _clips(suite):
    armature = suite.armature
    if armature is None or len(armature.charpaper.clips) > 0:
        return 0
    moved = 0
    named = {}
    for entry in suite.animations:
        if not entry.enabled:
            continue
        for old in entry.clips:
            if not old.enabled or old.action is None:
                continue
            if any(c.action == old.action for c in armature.charpaper.clips):
                continue
            new = armature.charpaper.clips.add()
            new.action = old.action
            new.extra = old.corrective
            new.mode = MODES.get(old.mode, "TIMELINE")
            new.gaze = old.gaze
            # The old list showed a clip by its own name, the action's unless renamed.
            if old.name and old.name != old.action.name:
                new.label = old.name
            named[old.name] = file_stem(old.action.name)
            moved += 1
    if suite.default_animation and not suite.default_clip:
        suite.default_clip = named.get(suite.default_animation, "")
    return moved


def _expressions(suite):
    moved = 0
    for skin in suite.skins:
        for old in skin.expressions:
            if not old.enabled or old.action is None:
                continue
            if any(e.action == old.action for e in suite.expressions):
                continue
            suite.expressions.add().action = old.action
            moved += 1
    return moved


def settle_folder(suite):
    """Before a suite could go into the app's characters folder, each went to a folder of its
    own: one picked, or the default next to the .blend. Those that were exported there keep it."""
    if suite.folder_settled:
        return
    suite.folder_settled = True
    if suite.folder != DEFAULT_FOLDER:
        suite.custom_folder = True
    elif bpy.data.filepath:
        suite.custom_folder = os.path.isfile(manifest.path(bpy.path.abspath(suite.folder)))


def migrate_all():
    for scene in bpy.data.scenes:
        settle_folder(scene.charpaper)
        moved = migrate(scene.charpaper)
        if moved is not None:
            clips, expressions = moved
            print(
                f"CharPaper: scene {scene.name!r}: {clips} clip(s) moved onto the armature, "
                f"{expressions} expression(s) onto the character"
            )


@persistent
def _on_load(_file):
    migrate_all()


def register():
    bpy.app.handlers.load_post.append(_on_load)
    # While registering, bpy.data can't be written to; once Blender is idle it can.
    bpy.app.timers.register(lambda: migrate_all() or None, first_interval=0.0)


def unregister():
    if _on_load in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(_on_load)
