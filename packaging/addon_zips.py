"""Builds the Blender add-on's zips, one for each platform with a charpaper_pack wheel.

Each holds the add-on, its platform's wheel under wheels/, and a manifest naming both, as
`blender --command extension build --split-platforms` makes them: Blender installs the wheel
with the add-on and refuses a zip made for another platform.

usage: addon_zips.py LABEL WHEEL_FOLDER OUT_FOLDER

Writes OUT_FOLDER/charpaper_exporter-LABEL-PLATFORM.zip for each wheel in WHEEL_FOLDER.
"""

import os
import re
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ADDON = os.path.join(HERE, "..", "tools", "blender", "charpaper_exporter")

# Blender's platform names, by the wheel's platform tag.
PLATFORMS = [
    (re.compile(r"win_amd64$"), "windows-x64"),
    (re.compile(r"win_arm64$"), "windows-arm64"),
    (re.compile(r"manylinux_\d+_\d+_x86_64$"), "linux-x64"),
    (re.compile(r"macosx_\d+_\d+_arm64$"), "macos-arm64"),
    (re.compile(r"macosx_\d+_\d+_x86_64$"), "macos-x64"),
]

# A fixed time for every entry, so the same files make the same zip.
STAMP = (2020, 1, 1, 0, 0, 0)


def platform_of(wheel):
    tag = wheel.removesuffix(".whl").rsplit("-", 1)[-1]
    for pattern, platform in PLATFORMS:
        if pattern.search(tag):
            return platform
    sys.exit(f"{wheel}: no Blender platform for its tag {tag}")


def with_wheel(manifest, platform, wheel):
    """The manifest with the platform and wheel added among its top-level keys, which TOML
    wants before the first table."""
    lines = manifest.splitlines(keepends=True)
    first_table = next((i for i, line in enumerate(lines) if line.startswith("[")), len(lines))
    added = f'platforms = ["{platform}"]\nwheels = ["./wheels/{wheel}"]\n\n'
    return "".join(lines[:first_table]) + added + "".join(lines[first_table:])


def addon_files():
    for folder, folders, files in os.walk(ADDON):
        folders[:] = sorted(f for f in folders if f not in {"__pycache__", "wheels"})
        for name in sorted(files):
            if name != "blender_manifest.toml" and not name.endswith(".pyc"):
                path = os.path.join(folder, name)
                yield path, os.path.relpath(path, ADDON).replace(os.sep, "/")


def write(path, name, data):
    entry = zipfile.ZipInfo(name, STAMP)
    entry.compress_type = zipfile.ZIP_DEFLATED
    entry.external_attr = 0o644 << 16
    path.writestr(entry, data)


def main(label, wheels, out):
    found = sorted(f for f in os.listdir(wheels) if f.endswith(".whl"))
    if not found:
        sys.exit(f"{wheels}: no wheels")
    platforms = [platform_of(w) for w in found]
    if len(set(platforms)) != len(platforms):
        sys.exit(f"{wheels}: more than one wheel for a platform: {found}")
    with open(os.path.join(ADDON, "blender_manifest.toml"), encoding="utf-8") as file:
        manifest = file.read()
    os.makedirs(out, exist_ok=True)
    for wheel, platform in zip(found, platforms):
        target = os.path.join(out, f"charpaper_exporter-{label}-{platform}.zip")
        # Blender's "Install from Disk" wants blender_manifest.toml at the root of the zip.
        with zipfile.ZipFile(target, "w") as archive:
            write(archive, "blender_manifest.toml", with_wheel(manifest, platform, wheel))
            for path, name in addon_files():
                with open(path, "rb") as file:
                    write(archive, name, file.read())
            with open(os.path.join(wheels, wheel), "rb") as file:
                write(archive, f"wheels/{wheel}", file.read())
        print(f"{target}: {platform}, {wheel}")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    main(*sys.argv[1:])
