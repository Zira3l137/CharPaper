"""CharPaper suite packs and sky baking, through the same code the app runs.

Every call lets go of Python's lock while it works, so Blender keeps drawing.
"""

from collections.abc import Callable, Iterable
from os import PathLike
from typing import TypedDict

VERSION: int
"""The pack format this module reads and writes."""
PACK_FILE: str
"""The pack's name in a suite folder: "suite.chr"."""
DEFAULT_LEVEL: int

class PackFailed(Exception):
    """A pack can't be read or written."""

class PackReplaced(PackFailed):
    """The pack was replaced since it was opened: open it again."""

class BakeFailed(Exception):
    """A sky can't be baked."""

class FileInfo(TypedDict):
    size: int
    stored: int
    compressed: bool
    hash: str

class Summary(TypedDict):
    files: int
    size: int
    packed: int
    reused: int
    shared: int

class Pack:
    """A suite pack, opened. Only its list of files is kept; each read opens the file again.

    Paths in a pack are relative to the suite folder, with "/" between parts.
    """

    def __init__(self, path: str | PathLike[str]) -> None: ...
    @property
    def path(self) -> str: ...
    @property
    def id(self) -> str:
        """Changes whenever the pack's contents do."""
    def files(self) -> list[str]:
        """Every file's path, sorted."""
    def files_in(self, folder: str = "") -> list[str]:
        """The paths of the files right in `folder`, not in its sub-folders."""
    def folders_in(self, folder: str = "") -> list[str]:
        """The names of the folders right in `folder`."""
    def is_file(self, path: str) -> bool: ...
    def is_dir(self, path: str) -> bool: ...
    def info(self, path: str) -> FileInfo:
        """Raises KeyError for a file the pack doesn't have."""
    def read(self, path: str) -> bytes:
        """Raises KeyError for a file the pack doesn't have."""
    def extract(self, path: str, to: str | PathLike[str]) -> None:
        """Writes one file of the pack to `to`, making its folders."""
    def verify(self) -> None:
        """Reads every file and checks it against its hash."""
    def __contains__(self, path: str) -> bool: ...
    def __len__(self) -> int: ...

def pack(
    folder: str | PathLike[str],
    out: str | PathLike[str],
    *,
    reuse: bool = True,
    keep: Iterable[str] = (),
    level: int = ...,
    progress: Callable[[str | None, int, int], object] | None = None,
) -> Summary:
    """Packs every file under `folder` into `out`, except suite.toml and its backup.

    With `reuse`, files whose contents are already in the pack at `out` are copied from it
    instead of being compressed again. `keep` lists paths and folders, such as "skins", to
    carry over from that pack where `folder` doesn't have them. `progress(path, done, total)`
    is called before each file, and once with None at the end. An exception it raises comes
    out of pack(); raised before the end, it stops packing and leaves `out` as it was.
    """

def bake(panorama: str | PathLike[str], folder: str | PathLike[str]) -> list[str]:
    """Bakes the sky maps from an .hdr or .exr panorama into `folder`.

    Maps already in `folder` are left alone. Returns the paths written.
    """

def missing_maps(folder: str | PathLike[str]) -> list[str]:
    """The names of the sky maps `folder` doesn't have yet."""
