"""Where a suite is exported: into the app's characters folder, or a folder of the artist's own.

The app's folder is a preference, kept with Blender's and not in the .blend, since it's where
the app sits on this computer. Everything else is the suite's."""

import os

import bpy
from bpy.props import StringProperty
from bpy.types import AddonPreferences

from .props import file_stem

CHARACTERS = "characters"
# The app is one executable; its name differs only by the platform's extension.
EXECUTABLES = ("charpaper.exe", "charpaper")


class CHARPAPER_AP_preferences(AddonPreferences):
    bl_idname = __package__

    app_folder: StringProperty(
        name="CharPaper app",
        subtype="DIR_PATH",
        description="Where CharPaper is on this computer: the folder charpaper.exe is in. "
        "Characters are exported into its characters folder",
    )

    def draw(self, context):
        layout = self.layout
        layout.use_property_split = True
        layout.prop(self, "app_folder")
        root, why = app()
        row = layout.row()
        if root is None:
            row.alert = True
            row.label(text=why, icon="ERROR")
        else:
            row.label(text=f"Characters go to {os.path.join(root, CHARACTERS)}", icon="CHECKMARK")


def preferences():
    addon = bpy.context.preferences.addons.get(__package__)
    return addon.preferences if addon is not None else None


def _is_app(folder):
    return any(os.path.isfile(os.path.join(folder, name)) for name in EXECUTABLES)


def _subfolders(folder):
    try:
        return [e for e in os.scandir(folder) if e.is_dir()]
    except OSError:
        return []


def find_app(path):
    """The app's folder from whatever the artist picked: that folder, the executable, the
    characters folder in it, or the folder the download was unpacked into, which holds the
    app's folder. Of several apps unpacked side by side, the newest."""
    if not path.strip():
        return None
    path = os.path.normpath(bpy.path.abspath(path))
    if os.path.isfile(path):
        path = os.path.dirname(path)
    if not os.path.isdir(path):
        return None
    for folder in (path, os.path.dirname(path)):
        if _is_app(folder):
            return folder
    apps = [e for e in _subfolders(path) if _is_app(e.path)]
    if not apps:
        return None
    return max(apps, key=lambda e: e.stat().st_mtime).path


def app():
    """The app's folder, or None and what to do about it."""
    prefs = preferences()
    if prefs is None:
        return None, "Enable the add-on in Preferences to set where CharPaper is"
    if not prefs.app_folder.strip():
        return None, "Pick where CharPaper is: the folder with charpaper.exe"
    root = find_app(prefs.app_folder)
    if root is None:
        return None, "No charpaper.exe there: pick the folder CharPaper was unpacked to"
    return root, None


def folder_name(suite):
    """The character's folder in the app's characters folder: its own setting, else its Name,
    else the .blend's name."""
    blend = os.path.splitext(os.path.basename(bpy.data.filepath))[0]
    for name in (suite.character_folder, suite.suite_name, blend):
        if name.strip():
            return file_stem(name.strip())
    return None


def folder(suite):
    """The folder the suite is exported to, or None and why it can't be worked out."""
    if suite.custom_folder:
        if not suite.folder.strip():
            return None, "Pick a folder"
        if suite.folder.startswith("//") and not bpy.data.filepath:
            return None, "The folder is relative to the .blend, which isn't saved yet"
        return os.path.normpath(bpy.path.abspath(suite.folder)), None
    root, why = app()
    if root is None:
        return None, why
    name = folder_name(suite)
    if name is None:
        return None, "Give the character a Name"
    return os.path.join(root, CHARACTERS, name), None


def keep_name(suite):
    """Settles the character's folder name on its first export into the app, so renaming the
    character later doesn't start a second folder next to the first."""
    if not suite.custom_folder and not suite.character_folder.strip():
        suite.character_folder = folder_name(suite) or ""


def register():
    bpy.utils.register_class(CHARPAPER_AP_preferences)


def unregister():
    bpy.utils.unregister_class(CHARPAPER_AP_preferences)
