import os
import shutil
import tempfile

import aud
import bpy

from .props import file_stem

SOUNDS_DIR = "sounds"
OGG = {".ogg", ".oga"}
AUDIO = OGG | {".wav", ".flac", ".mp3", ".aac", ".m4a", ".opus", ".aif", ".aiff"}
# Vorbis at this rate is indistinguishable from the source for effects and voices.
BITRATE = 192_000


def sound_name(name):
    """A marker's or Sound's name, less any audio extension, as a file name."""
    stem, ext = os.path.splitext(name)
    return file_stem(stem if ext.lower() in AUDIO else name)


def path(folder, name):
    return os.path.join(folder, SOUNDS_DIR, name + ".ogg")


def find_sound(name):
    """The Sound named so, with or without its file's extension: Blender names a Sound after
    its file."""
    exact = bpy.data.sounds.get(name)
    if exact is not None:
        return exact
    return next((s for s in bpy.data.sounds if sound_name(s.name) == name), None)


def markers(action):
    return list(getattr(action, "pose_markers", ()))


def in_range(action):
    """Splits an action's pose markers into those inside its frame range and the rest."""
    start, end = action.frame_range
    inside, outside = [], []
    for marker in markers(action):
        (inside if start <= marker.frame <= end else outside).append(marker)
    return inside, outside


def of(action, fps):
    """(seconds from the clip's start, sound name) for each pose marker of the action.

    The glTF exporter starts a clip at its action's first frame, so that frame is 0 here too.
    """
    start = action.frame_range[0]
    inside, _ = in_range(action)
    return sorted((round((m.frame - start) / fps, 4), sound_name(m.name)) for m in inside)


def export(context, folder, entries):
    """Writes the sounds the exported clips' markers name, and returns their cue tables by
    action name, the written files, and notes.

    An action with markers gets a table, even an empty one, and its markers decide its cues.
    One without gets none, so cues written into suite.toml by hand are kept.
    """
    render = context.scene.render
    fps = render.fps / render.fps_base
    actions = {
        clip.action for entry in entries for clip in entry.clips if clip.enabled and clip.action
    }
    found = {a.name: of(a, fps) for a in actions if markers(a)}
    names = {name for table in found.values() for _, name in table}
    written, notes = [], []
    for name in sorted(names):
        target = path(folder, name)
        sound = find_sound(name)
        if sound is None:
            if not os.path.isfile(target):
                notes.append(f"no sound named {name!r} here or in {SOUNDS_DIR}/; its cues are left out")
            continue
        try:
            _write_ogg(sound, target)
            written.append(target)
        except Exception as error:
            notes.append(f"sound {sound.name!r}: {error}")
    tables = {
        action: [{"at": at, "sound": name} for at, name in table if os.path.isfile(path(folder, name))]
        for action, table in found.items()
    }
    return tables, written, notes


def source_path(sound):
    return bpy.path.abspath(sound.filepath, library=sound.library)


def _write_ogg(sound, target):
    # Written in a scratch folder first, so a failed conversion never leaves half a file.
    with tempfile.TemporaryDirectory() as scratch:
        if sound.packed_file is not None:
            ext = os.path.splitext(sound.filepath or sound.name)[1].lower()
            source = os.path.join(scratch, "packed" + ext)
            with open(source, "wb") as file:
                file.write(sound.packed_file.data)
        else:
            source = source_path(sound)
            if not os.path.isfile(source):
                raise FileNotFoundError(f"its file {source} is missing")
            ext = os.path.splitext(source)[1].lower()

        if ext in OGG:
            converted = source
        else:
            converted = os.path.join(scratch, "converted.ogg")
            aud.Sound.file(source).write(
                converted,
                format=aud.FORMAT_FLOAT32,
                container=aud.CONTAINER_OGG,
                codec=aud.CODEC_VORBIS,
                bitrate=BITRATE,
            )
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copyfile(converted, target)
