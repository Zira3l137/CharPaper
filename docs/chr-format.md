[CharPaper](../README.md) › [Docs](README.md)

# The suite pack (`.chr`)

A suite folder used to hold dozens of files in sub-folders. A packed suite holds two:

```
characters/
  Aria/
    suite.toml    settings, plain text: the panel and you edit it
    suite.chr     every other file of the suite
```

The pack holds the files exactly as an unpacked suite has them, under the same paths
(`skins/casual.glb`, `environment/room/music/01 - Rain.ogg`). The app reads a pack and an
unpacked folder the same way, so an unpacked suite still works, for testing or by hand.

The code is in [`crates/charpaper-pack`](../crates/charpaper-pack). The app and the Blender
add-on's Python module both use it, so they can't read a pack differently.

## At a glance

```
┌──────────────┐ 0
│ header       │ 48 bytes
├──────────────┤ 48
│ file data    │ each file's bytes, starting on a 16-byte boundary
│ …            │
├──────────────┤ index offset
│ index        │ one entry per file, sorted by path
└──────────────┘ end of file
```

The index comes last, so the packer writes in one pass and fills in the header at the end.
All numbers are little-endian.

## Header

| Offset | Size | What |
|-------:|-----:|------|
| 0 | 8 | `CHRPACK\0` |
| 8 | 2 | Format version, `1` |
| 10 | 2 | Flags, `0`. None are defined; a reader ignores them. |
| 12 | 4 | Number of files |
| 16 | 8 | Where the index starts |
| 24 | 8 | The index's length. The index ends where the file ends. |
| 32 | 16 | Pack id: the first 16 bytes of the BLAKE3 hash of the index |

## Index

One entry after another, as many as the header says, sorted by path byte by byte, with
no path twice:

| Size | What |
|-----:|------|
| 2 | Path length in bytes |
| n | Path, UTF-8 |
| 8 | Where the data starts |
| 8 | Bytes stored in the pack |
| 8 | Bytes once unpacked |
| 1 | How it's stored: `0` as it is, `1` zstd |
| 16 | The first 16 bytes of the BLAKE3 hash of the unpacked contents |

A path is relative to the suite folder, with `/` between parts. It never starts or ends
with `/`, has no empty, `.` or `..` part, and has no `\`, `:` or NUL. Folders aren't
stored: a folder is there while some file's path runs through it. Two paths in a pack
never differ only in capitals, because Windows would see them as one file.

Files with the same contents may point at the same data.

## Compression

A file is compressed with zstd, as a single frame that carries its checksum, only if that
saves at least 5%. Files under 64 bytes, and `.ogg`, `.gif`, `.png`, `.jpg`, `.jpeg` and
`.webp` files, are never tried, since they don't shrink. A stored file's size and its
unpacked size are the same.

## In the app

- A packed suite's files come out of the pack through the app's own asset source, so Bevy
  loads them as it loads loose files. A pack holds no `.meta` files: everything in it loads
  with Bevy's default settings, as the app asks for them.
- A pack can't take the sky maps the app bakes on first load, so the add-on bakes them
  before it packs, and leaves the panorama out. An environment packed with a panorama but
  without all three maps shows without a sky, and the log says so.
- Outfit pictures are kept by each skin's content hash, so a skin exported again
  unchanged keeps its picture.

## What a reader checks

- The magic and version. A pack of another version is refused, never guessed at.
- That the index ends at the end of the file, and that it hashes to the pack id. A pack
  cut short or damaged is caught when it's opened.
- That every file's data lies between the header and the index, and that the paths are
  valid and sorted.
- Zstd checks each compressed file against its checksum as it unpacks. A stored file is
  only checked against its hash by a full check (`Pack::verify`), which reads everything.

## Replacing a pack while the app runs

The app keeps only the index in memory. For each file it reads, it opens the pack, checks
that the header still has the same id, reads, and closes the pack again. It never keeps the
pack open or maps it into memory, because on Windows that stops the add-on from replacing
the file.

The packer writes `suite.chr.part` beside the pack and renames it over `suite.chr` once
the pack is complete. A failed or cancelled export leaves the old pack as it was. When the
id has changed, the app opens the new pack and reads on from it, instead of reading old
offsets out of a new file. A suite already on screen keeps what it has loaded; switching to
it again shows the new export.

The same files always make the same pack, byte for byte, so exporting a suite that hasn't
changed gives it the same id.

## Packing again

When a suite is packed again, a file whose hash matches a file in the old pack is copied
from it as it is, without compressing it again. An export that changed one skin
compresses one skin. An export of only some parts can also carry the old pack's other
files over without having them on disk.
