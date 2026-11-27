"""CharPaper suite packs and sky baking, through the same code the app runs.

Every call lets go of Python's lock while it works, so Blender keeps drawing.
"""

from collections.abc import Buffer, Callable, Iterable, Sequence
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

class Bones(TypedDict):
    """An armature's bones in the armature's order. Matrices are float32, column by column, as
    Blender's foreach_get gives them."""

    parents: Sequence[int]
    """Each bone's exported parent by index, -1 for none."""
    has_parent: Sequence[bool]
    """Whether each bone has a parent at all in Blender."""
    rest: Buffer
    """bone.matrix_local of every bone."""
    poses: Buffer
    """pose_bone.matrix of every bone, for each sample in turn."""
    written: Sequence[tuple[int, str]]
    """The bones the clip moves, by index and name."""

def write_clip(
    path: str | PathLike[str],
    name: str,
    samples: int,
    step_seconds: float,
    optimize: bool,
    bones: Bones | None = None,
    objects: Sequence[tuple[str, Buffer]] = (),
) -> None:
    """Writes a clip's .glb as Blender's glTF exporter would: the bones and objects it moves,
    sampled `samples` times `step_seconds` apart. `objects` are (name, float32 matrices relative
    to the parent in Blender, one per sample). With `optimize`, a channel that holds still keeps
    its first and last keys."""

class Influences(TypedDict):
    """A skinned mesh's vertex groups as joints. Integers are uint32, floats float32."""

    counts: Buffer
    """How many (joint, weight) pairs each vertex has, in vertex order."""
    joints: Buffer
    """Every vertex's joints by skin index, one vertex after another."""
    weights: Buffer
    """The weights beside `joints`, those above 0.0001 only."""
    object_world: Buffer
    """The mesh object's matrix_world."""
    armature_world: Buffer
    """The skinning armature's matrix_world."""

class ShapeKey(TypedDict):
    name: str
    weight: float
    positions: Buffer
    """The key's vertex positions, float32."""
    normals: Buffer | None
    """The key's corner normals, when shape key normals are exported."""

class Mesh(TypedDict):
    """A mesh as foreach_get reads it, in Blender's axes."""

    name: str
    positions: Buffer
    """The Basis's vertex positions, float32."""
    corner_verts: Buffer
    """Each corner's vertex, uint32."""
    normals: Buffer
    """Corner normals, float32."""
    uvs: Sequence[Buffer]
    """Each exported UV layer's corner coordinates, float32."""
    tangents: Buffer | None
    signs: Buffer | None
    """Corner tangents and bitangent signs, float32, or None for both."""
    triangles: Buffer
    """Loop triangles as three corners each, uint32."""
    triangle_slots: Buffer
    """Each triangle's material slot, uint32."""
    shape_keys: Sequence[ShapeKey]
    """The exported keys, the Basis left out."""
    skin: Influences | None

class Node(TypedDict):
    name: str
    matrix: Buffer
    """Relative to the parent, in Blender's axes, float32 column by column."""
    children: Sequence[int]
    mesh: int | None
    skin: int | None

class Skin(TypedDict):
    name: str
    joints: Sequence[int]
    """The joints' node indices."""
    inverse_binds: Buffer
    """One float32 matrix per joint, in glTF's axes."""

class Scene(TypedDict):
    name: str
    nodes: Sequence[Node]
    roots: Sequence[int]
    skins: Sequence[Skin]
    meshes: Sequence[Mesh]

def write_scene(path: str | PathLike[str], scene: Scene, materials: Buffer | None = None) -> None:
    """Writes a model or skin .glb as Blender's glTF exporter would. Its materials, textures and
    images are copied from `materials`, a .glb Blender's exporter wrote of stand-in meshes: mesh
    `i`'s stand-in is named "charpaper~proxy~{i}", with one primitive for each material slot.
    Raises ValueError, having written nothing, for what it doesn't write as Blender would."""

def missing_maps(folder: str | PathLike[str]) -> list[str]:
    """The names of the sky maps `folder` doesn't have yet."""
