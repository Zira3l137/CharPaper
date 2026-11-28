"""The suite folder as an export sees it: loose files, or the suite.chr beside suite.toml.

A packed export writes into a staging folder inside the suite folder, then packs it into
suite.chr, carrying over what the pack already held that the export neither wrote nor removed.
An unpacked export writes straight into the suite folder, as it always has. Either way, an
exporter writes under `root` and asks this what's already there, so it works the same on both.

Paths are relative to the suite folder, with "/" between parts; an absolute path under `root`
is taken too.
"""

import hashlib
import os
import shutil
import tempfile

try:
    import charpaper_pack as native
except ImportError:
    native = None

PACK_FILE = "suite.chr"
STAGING_PREFIX = ".export~"
# Beside the pack, never in it: the add-on writes suite.toml, the app its pictures.
BESIDE_THE_PACK = {"suite.toml", "suite.toml.bak", PACK_FILE, PACK_FILE + ".part", "thumbnails"}
PANORAMA_EXTENSIONS = (".hdr", ".exr")
SKY_MAPS = ("skybox.ktx2", "specular.ktx2", "diffuse.ktx2")
# The panorama the add-on writes; one with another name was put there by hand.
EXPORTED_SKY = "sky"
# Written beside the maps in a pack, which has no panorama to tell an unchanged sky by.
BAKED_FROM = "sky.baked-from"


def available():
    """Whether the Rust module that packs and bakes is installed."""
    return native is not None


def call(write, *args, **kwargs):
    """Calls one of charpaper_pack's writers. PyO3 raises TypeError when what the add-on hands
    the module doesn't fit what it takes, which happens when the module comes from another
    build than the add-on's Python, so the error says which module Blender found."""
    try:
        return write(*args, **kwargs)
    except TypeError as error:
        raise RuntimeError(
            f"{_chain(error)}. The charpaper_pack module at {os.path.dirname(native.__file__)} "
            "doesn't match this add-on; install both from the same build"
        ) from error


def _chain(error):
    found = []
    while error is not None:
        found.append(f"missing {error}" if isinstance(error, KeyError) else str(error))
        error = error.__cause__
    return ": ".join(found)


def pack_path(folder):
    return os.path.join(folder, PACK_FILE)


def view(folder):
    """What an earlier export left in the suite folder, to look at only."""
    pack = _open_pack(folder)
    return SuiteFiles(folder, pack=pack, loose=pack is None)


def for_export(folder, packed):
    """Files for an export into `folder`. Returns them and notes for the viewer."""
    notes = []
    os.makedirs(folder, exist_ok=True)
    _remove_stale_staging(folder)
    if native is None and os.path.isfile(pack_path(folder)):
        # Loose files beside it would never show: the app reads the pack.
        raise RuntimeError(
            "this suite is packed into suite.chr, and without the charpaper_pack module the add-on "
            "can neither unpack nor pack it. Install the add-on from a release, or delete suite.chr "
            "and export everything again"
        )
    pack = _open_pack(folder)
    if not packed:
        if pack is not None:
            _unpack(pack, folder)
            os.remove(pack_path(folder))
            notes.append(f"suite.chr was unpacked into {folder}, since this export writes loose files")
        return SuiteFiles(folder, loose=True), notes
    if pack is None and os.path.isfile(pack_path(folder)):
        notes.append("suite.chr couldn't be read, so it was packed again from scratch")
    staging = tempfile.mkdtemp(prefix=STAGING_PREFIX, dir=folder)
    # Without a pack, the loose files of an earlier unpacked export are what's there.
    return SuiteFiles(folder, staging=staging, pack=pack, loose=pack is None), notes


def _open_pack(folder):
    if native is None or not os.path.isfile(pack_path(folder)):
        return None
    try:
        return native.Pack(pack_path(folder))
    except native.PackFailed as error:
        print(f"CharPaper: {error}")
        return None


def _remove_stale_staging(folder):
    # Left by an export Blender didn't live to finish.
    for name in os.listdir(folder):
        if name.startswith(STAGING_PREFIX):
            shutil.rmtree(os.path.join(folder, name), ignore_errors=True)


def _unpack(pack, folder):
    for path in pack.files():
        if not path.endswith("/" + BAKED_FROM):
            pack.extract(path, os.path.join(folder, *path.split("/")))


def _loose_files(folder):
    found = []
    for here, dirs, files in os.walk(folder):
        relative = os.path.relpath(here, folder).replace(os.sep, "/")
        if relative == ".":
            relative = ""
            dirs[:] = [d for d in dirs if d not in BESIDE_THE_PACK and not d.startswith(STAGING_PREFIX)]
            files = [f for f in files if f not in BESIDE_THE_PACK]
        found += [f"{relative}/{name}" if relative else name for name in files]
    return found


class SuiteFiles:
    def __init__(self, folder, staging=None, pack=None, loose=False):
        self.folder = folder
        self.staging = staging
        # What was there before: a pack, or the loose files in the folder.
        self._pack = pack
        self._loose = loose
        self._removed = set()

    @property
    def packed(self):
        return self.staging is not None

    @property
    def root(self):
        """Where an export writes."""
        return self.staging or self.folder

    def rel(self, path):
        if os.path.isabs(path):
            path = os.path.relpath(path, self.root)
        return path.replace(os.sep, "/").strip("/")

    def path(self, rel):
        """Where to write the file at `rel`."""
        return os.path.join(self.root, *self.rel(rel).split("/"))

    def _on_disk(self, rel):
        return os.path.join(self.folder, *rel.split("/"))

    def _staged(self, rel):
        return self.staging is not None and os.path.isfile(os.path.join(self.staging, *rel.split("/")))

    def _was_there(self, rel):
        if rel in self._removed:
            return False
        if self._pack is not None:
            return self._pack.is_file(rel)
        return self._loose and os.path.isfile(self._on_disk(rel))

    def isfile(self, path):
        rel = self.rel(path)
        return self._staged(rel) or self._was_there(rel)

    def isdir(self, path):
        return bool(self.listdir(path))

    def listdir(self, path):
        """The names of the files and folders right in a folder, sorted."""
        rel = self.rel(path)
        names = set()
        if self.staging is not None:
            names |= _names_in(os.path.join(self.staging, *rel.split("/")) if rel else self.staging)
        if self._pack is not None:
            names |= {p.rsplit("/", 1)[-1] for p in self._pack.files_in(rel) if p not in self._removed}
            names |= set(self._pack.folders_in(rel))
        elif self._loose:
            names |= _names_in(self._on_disk(rel) if rel else self.folder)
            if not rel:
                names = {n for n in names if n not in BESIDE_THE_PACK and not n.startswith(STAGING_PREFIX)}
        joined = (lambda n: f"{rel}/{n}") if rel else (lambda n: n)
        return sorted(n for n in names if joined(n) not in self._removed)

    def remove(self, path):
        rel = self.rel(path)
        target = os.path.join(self.root, *rel.split("/"))
        if os.path.isfile(target):
            os.remove(target)
        if self.packed:
            self._removed.add(rel)

    def local(self, path):
        """A path on disk holding the file, taken out of the pack first if it's there. None
        when the suite doesn't have it."""
        rel = self.rel(path)
        if self._staged(rel) or not self.packed:
            return self.path(rel) if self.isfile(rel) else None
        if not self._was_there(rel):
            return None
        if self._pack is not None:
            # Into the staging folder: packing it again is cheap, since it hasn't changed.
            self._pack.extract(rel, self.path(rel))
            return self.path(rel)
        return self._on_disk(rel)

    def finish(self, progress=None):
        """Packs what was staged, carrying over the rest of the old pack. Returns what packing
        reports, or None for an unpacked export. The staging folder goes either way."""
        if not self.packed:
            return None
        try:
            notes = self._bake_skies()
            taken_in = []
            if self._pack is None and self._loose:
                taken_in = _loose_files(self.folder)
                for rel in taken_in:
                    if rel not in self._removed and not self._staged(rel):
                        _link(self._on_disk(rel), os.path.join(self.staging, *rel.split("/")))
            keep = [p for p in self._pack.files() if p not in self._removed] if self._pack else []
            summary = native.pack(self.staging, pack_path(self.folder), keep=keep, progress=progress)
            # Now in the pack, the loose files would only be read by an older CharPaper.
            for rel in taken_in:
                _remove_with_empty_folders(self.folder, rel)
            summary["notes"] = notes
            return summary
        finally:
            self.discard()

    def discard(self):
        if self.staging is not None:
            shutil.rmtree(self.staging, ignore_errors=True)

    # The app can't bake into a pack, so the maps are baked here, and the panorama the add-on
    # wrote stays out: only baking needs it. One put there by hand is kept, for the next export.
    def _bake_skies(self):
        notes = []
        for name in self.listdir("environment"):
            folder = f"environment/{name}"
            panoramas = [n for n in self.listdir(folder) if os.path.splitext(n)[1].lower() in PANORAMA_EXTENSIONS]
            if not panoramas:
                continue
            missing = [m for m in SKY_MAPS if not self.isfile(f"{folder}/{m}")]
            if missing:
                source = self.local(f"{folder}/{panoramas[0]}")
                print(f"CharPaper: {name}: baking the sky from {panoramas[0]}")
                try:
                    native.bake(source, self.path(folder))
                except native.BakeFailed as error:
                    notes.append(f"{name}: its sky couldn't be baked, so it shows without one: {error}")
                    continue
                if not self.isfile(f"{folder}/{BAKED_FROM}"):
                    with open(source, "rb") as file:
                        digest = hashlib.sha256(file.read()).hexdigest()
                    with open(self.path(f"{folder}/{BAKED_FROM}"), "w", encoding="utf-8") as file:
                        file.write(digest + "\n")
            for panorama in panoramas:
                if os.path.splitext(panorama)[0] == EXPORTED_SKY:
                    self.remove(f"{folder}/{panorama}")
        return notes


def _names_in(directory):
    return set(os.listdir(directory)) if os.path.isdir(directory) else set()


def _link(source, target):
    os.makedirs(os.path.dirname(target), exist_ok=True)
    try:
        os.link(source, target)
    except OSError:
        shutil.copy2(source, target)


def _remove_with_empty_folders(folder, rel):
    path = os.path.join(folder, *rel.split("/"))
    if os.path.isfile(path):
        os.remove(path)
    parent = os.path.dirname(path)
    while os.path.normpath(parent) != os.path.normpath(folder):
        try:
            os.rmdir(parent)
        except OSError:
            break
        parent = os.path.dirname(parent)
