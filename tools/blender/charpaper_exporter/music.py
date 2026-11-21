"""An environment's music and ambience, written where the app finds them: music/ with one
.ogg per track, played in file-name order, and ambience.ogg, which loops under the music.
The order is kept by numbering the files: "01 - Rain.ogg"; the app shows the title alone.

With Sound on, the environment's folder mirrors the lists: .ogg files in music/ that aren't
tracks go, and so does ambience.ogg without an ambience. Converting is slow, so each track
remembers what it was written from, and an unchanged one is copied over instead."""

import os
import shutil
import tempfile

import bpy

from . import audio
from .exporters import ENVIRONMENT_DIR
from .props import file_stem

MUSIC_DIR = "music"
AMBIENCE_FILE = "ambience.ogg"


def environment_folder(folder, entry):
    return os.path.join(folder, ENVIRONMENT_DIR, file_stem(entry.name))


def source(item_path):
    return os.path.normpath(bpy.path.abspath(item_path)) if item_path else ""


def title(track):
    return track.name.strip() or os.path.splitext(os.path.basename(track.filepath))[0]


def file_names(entry):
    """The file each track is written to, in playlist order."""
    width = max(2, len(str(len(entry.music))))
    return [f"{i + 1:0{width}d} - {file_stem(title(t))}.ogg" for i, t in enumerate(entry.music)]


def _stamp(path):
    info = os.stat(path)
    return f"{path}|{info.st_mtime_ns}|{info.st_size}|{audio.BITRATE}"


def strays(folder, entry):
    """The .ogg files in music/ an export would remove, and whether ambience.ogg would go."""
    music = os.path.join(environment_folder(folder, entry), MUSIC_DIR)
    ours = {n.lower() for n in file_names(entry)}
    present = os.listdir(music) if os.path.isdir(music) else []
    extra = sorted(n for n in present if n.lower().endswith(".ogg") and n.lower() not in ours)
    ambience = os.path.join(environment_folder(folder, entry), AMBIENCE_FILE)
    return extra, not entry.ambience and os.path.isfile(ambience)


def export(entry, folder):
    """Writes the environment's music and ambience. Returns the files written and notes."""
    here = environment_folder(folder, entry)
    written, notes = [], []
    written += _write_music(entry, here, notes)
    ambience = _write_ambience(entry, here, notes)
    if ambience:
        written.append(ambience)
    return written, notes


def _write_music(entry, here, notes):
    music = os.path.join(here, MUSIC_DIR)
    os.makedirs(here, exist_ok=True)
    # Built in a folder beside music/ and moved in once all is well, so a failure midway
    # leaves music/ as the last export wrote it.
    staging = tempfile.mkdtemp(prefix=".music-", dir=here)
    try:
        done = []
        for track, name in zip(entry.music, file_names(entry)):
            path = source(track.filepath)
            if not os.path.isfile(path):
                notes.append(f"{entry.name}: the track {title(track)!r} has no file at {path or '(none)'}; left out")
                continue
            stamp = _stamp(path)
            previous = os.path.join(music, track.written) if track.written else ""
            if track.stamp == stamp and previous and os.path.isfile(previous):
                shutil.copyfile(previous, os.path.join(staging, name))
            else:
                print(f"CharPaper: {entry.name}: converting {os.path.basename(path)} to {name}")
                try:
                    audio.to_ogg(path, os.path.join(staging, name))
                except Exception as error:
                    notes.append(f"{entry.name}: the track {title(track)!r} could not be converted: {error}")
                    continue
            done.append((track, name, stamp))

        os.makedirs(music, exist_ok=True)
        for old in os.listdir(music):
            if old.lower().endswith(".ogg"):
                os.remove(os.path.join(music, old))
        for track in entry.music:
            track.written = track.stamp = ""
        for track, name, stamp in done:
            os.replace(os.path.join(staging, name), os.path.join(music, name))
            track.written, track.stamp = name, stamp
    finally:
        shutil.rmtree(staging, ignore_errors=True)
    return [os.path.join(music, name) for _, name, _ in done]


def _write_ambience(entry, here, notes):
    target = os.path.join(here, AMBIENCE_FILE)
    path = source(entry.ambience)
    if not path:
        if os.path.isfile(target):
            os.remove(target)
        entry.ambience_stamp = ""
        return None
    if not os.path.isfile(path):
        notes.append(f"{entry.name}: the ambience has no file at {path}; left as it was")
        return None
    stamp = _stamp(path)
    if entry.ambience_stamp == stamp and os.path.isfile(target):
        return target
    print(f"CharPaper: {entry.name}: converting {os.path.basename(path)} to {AMBIENCE_FILE}")
    try:
        audio.to_ogg(path, target)
    except Exception as error:
        notes.append(f"{entry.name}: the ambience could not be converted: {error}")
        return None
    entry.ambience_stamp = stamp
    return target


def check(suite, add, finding):
    folder = os.path.normpath(bpy.path.abspath(suite.folder))
    resolvable = not (suite.folder.startswith("//") and not bpy.data.filepath)
    for entry in suite.environments:
        if entry.enabled:
            _check(entry, folder if resolvable else None, lambda m, s="WARNING": add(finding(m, severity=s, sound=entry.name)))


def _check(entry, folder, warn):
    name = entry.name
    if not entry.export_sound:
        if len(entry.music) or entry.ambience:
            warn(
                f"environment {name!r} has music or an ambience picked, but Sound is off, so none of it "
                "is exported. Tick Sound in its Sound panel"
            )
        return
    if not len(entry.music) and not entry.ambience:
        warn(f"environment {name!r}: Sound is on, but no music or ambience is picked", "INFO")
    seen = {}
    for track in entry.music:
        path = source(track.filepath)
        ext = os.path.splitext(path)[1].lower()
        if not path:
            warn(f"environment {name!r}: the track {title(track)!r} has no file picked")
            continue
        if not os.path.isfile(path):
            warn(f"environment {name!r}: the track {title(track)!r} has no file at {path}")
        elif ext not in audio.AUDIO:
            warn(
                f"environment {name!r}: the track {title(track)!r} is a {ext or 'file without extension'}, "
                "which may not convert. Ogg, WAV, FLAC, MP3, AAC and Opus do"
            )
        seen.setdefault(path, []).append(title(track))
    for titles in seen.values():
        if len(titles) > 1:
            warn(f"environment {name!r}: one file is in the playlist {len(titles)} times ({', '.join(titles)})", "INFO")
    ambience = source(entry.ambience)
    if ambience and not os.path.isfile(ambience):
        warn(f"environment {name!r}: the ambience has no file at {ambience}")
    if ambience and ambience in seen:
        warn(f"environment {name!r}: its ambience is also a track, so it plays twice at once", "INFO")
    if folder is None:
        return
    extra, ambience_goes = strays(folder, entry)
    if extra:
        shown = ", ".join(extra[:3]) + (f" and {len(extra) - 3} more" if len(extra) > 3 else "")
        isnt, it = ("isn't", "it") if len(extra) == 1 else ("aren't", "them")
        warn(
            f"environment {name!r}: exporting removes {shown} from its music folder, which {isnt} in "
            f"the playlist. Add {it} to the playlist to keep {it}"
        )
    if ambience_goes:
        warn(
            f"environment {name!r}: exporting removes its ambience.ogg, since no ambience is picked. "
            "Pick the file as the ambience to keep it"
        )
