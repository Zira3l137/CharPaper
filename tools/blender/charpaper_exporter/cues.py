import os
import tempfile

import bpy

from . import audio
from .audio import AUDIO
from .props import file_stem

SOUNDS_DIR = "sounds"


def sound_name(name):
    """A marker's or Sound's name, less any audio extension, as a file name."""
    stem, ext = os.path.splitext(name)
    return file_stem(stem if ext.lower() in AUDIO else name)


def path(name):
    """Where a sound goes in the suite."""
    return f"{SOUNDS_DIR}/{name}.ogg"


def find_sound(name):
    """The Sound named so, with or without its file's extension: Blender names a Sound after
    its file."""
    exact = bpy.data.sounds.get(name)
    if exact is not None:
        return exact
    return next((s for s in bpy.data.sounds if sound_name(s.name) == name), None)


def markers(action):
    return list(getattr(action, "pose_markers", ()))


def within(action, start, end, fps):
    """(seconds from `start`, sound name) for each pose marker of the action from frame `start`
    to `end`."""
    found = [m for m in markers(action) if start <= m.frame <= end]
    return sorted((round((m.frame - start) / fps, 4), sound_name(m.name)) for m in found)


def write_sounds(files, names):
    """Writes each named Sound into the suite's sounds folder. Returns the written files, the
    names whose file is there now, and notes."""
    written, present, notes = [], set(), []
    for name in sorted(names):
        target = files.path(path(name))
        sound = find_sound(name)
        if sound is None:
            if files.isfile(target):
                present.add(name)
            else:
                notes.append(f"no sound named {name!r} here or in {SOUNDS_DIR}/; its cues are left out")
            continue
        try:
            _write_ogg(sound, target)
            written.append(target)
            present.add(name)
        except Exception as error:
            notes.append(f"sound {sound.name!r}: {error}")
    return written, present, notes


def source_path(sound):
    return bpy.path.abspath(sound.filepath, library=sound.library)


def _write_ogg(sound, target):
    if sound.packed_file is None:
        audio.to_ogg(source_path(sound), target)
        return
    with tempfile.TemporaryDirectory() as scratch:
        ext = os.path.splitext(sound.filepath or sound.name)[1].lower()
        source = os.path.join(scratch, "packed" + ext)
        with open(source, "wb") as file:
            file.write(sound.packed_file.data)
        audio.to_ogg(source, target)
