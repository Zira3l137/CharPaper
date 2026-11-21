"""Turning sound files into what the app plays: Ogg Vorbis."""

import os
import shutil
import tempfile

import aud

OGG = {".ogg", ".oga"}
AUDIO = OGG | {".wav", ".flac", ".mp3", ".aac", ".m4a", ".opus", ".aif", ".aiff"}
# Vorbis at this rate is indistinguishable from the source for music, effects and voices.
BITRATE = 192_000


def is_vorbis(path):
    """Whether the file is Ogg Vorbis. An .ogg may hold Opus instead, which the app can't play."""
    with open(path, "rb") as file:
        head = file.read(512)
    return head.startswith(b"OggS") and b"\x01vorbis" in head


def to_ogg(source, target):
    """Writes `source` to `target` as Ogg Vorbis, copying it when it already is. Converted in a
    scratch folder first, so a failed conversion never leaves half a file."""
    if not os.path.isfile(source):
        raise FileNotFoundError(f"its file {source} is missing")
    os.makedirs(os.path.dirname(target), exist_ok=True)
    if os.path.splitext(source)[1].lower() in OGG and is_vorbis(source):
        shutil.copyfile(source, target)
        return
    with tempfile.TemporaryDirectory() as scratch:
        converted = os.path.join(scratch, "converted.ogg")
        aud.Sound.file(source).write(
            converted,
            format=aud.FORMAT_FLOAT32,
            container=aud.CONTAINER_OGG,
            codec=aud.CODEC_VORBIS,
            bitrate=BITRATE,
        )
        shutil.copyfile(converted, target)
