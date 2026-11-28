"""Writes the Blender extensions listing for a release's add-on, as the website serves it.

Blender installs an add-on dragged in from a web page only from a repository that lists it:
a JSON file naming each package's zip, size and hash, one zip for each platform. Once that
repository is added, Blender also offers the add-on's updates, each system its own zip. The
listing is the one `blender --command extension server-generate` writes, with the zips left
where the release keeps them.

usage: blender_listing.py RELEASE_URL LISTING_URL OUT_JSON LINKED_ZIP [OTHER_ZIP...]

RELEASE_URL is where the release keeps the zips. Prints the query the website's link to
LINKED_ZIP appends for drag and drop: Blender reads the repository, the versions and platforms
the zip fits from it before downloading anything.
"""

import hashlib
import json
import os
import sys
import tomllib
import urllib.parse
import zipfile

# Kept out of the listing by Blender's own generator too: they only matter to the build, or,
# for the wheels, to the zip. The listing names the Python versions they fit instead.
BUILD_ONLY = {"build", "wheels"}


def python_versions(wheels):
    """As Blender's generator reads them from the wheels' names: a stable-ABI wheel fits any
    Python 3 (the rest of the requirement is in its metadata), cp311 only 3.11."""
    found = set()
    for wheel in wheels:
        python, abi = os.path.basename(wheel).removesuffix(".whl").split("-")[-3:-1]
        if abi == "abi3":
            found.add("3")
        else:
            found.update(f"3.{tag[3:]}" for tag in python.split(".") if tag.startswith("cp3"))
    if any("." in v for v in found):
        found.discard("3")
    return sorted(found)


def package(addon_zip, release_url):
    with zipfile.ZipFile(addon_zip) as archive:
        manifest = tomllib.loads(archive.read("blender_manifest.toml").decode("utf-8"))
    data = open(addon_zip, "rb").read()
    found = {key: value for key, value in manifest.items() if key not in BUILD_ONLY}
    if manifest.get("wheels"):
        found["python_versions"] = python_versions(manifest["wheels"])
    found.update(
        {
            "archive_url": f"{release_url}/{os.path.basename(addon_zip)}",
            "archive_size": len(data),
            "archive_hash": "sha256:" + hashlib.sha256(data).hexdigest(),
        }
    )
    return found


def main(release_url, listing_url, out, *zips):
    packages = [package(z, release_url) for z in zips]
    listing = {"version": "v1", "blocklist": [], "data": packages}
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    with open(out, "w", encoding="utf-8") as file:
        json.dump(listing, file, indent=2, ensure_ascii=False)
        file.write("\n")

    linked = packages[0]
    query = {"repository": listing_url, "blender_version_min": linked["blender_version_min"]}
    if "blender_version_max" in linked:
        query["blender_version_max"] = linked["blender_version_max"]
    if "platforms" in linked:
        query["platforms"] = ",".join(linked["platforms"])
    print(urllib.parse.urlencode(query))


if __name__ == "__main__":
    if len(sys.argv) < 5:
        sys.exit(__doc__)
    main(*sys.argv[1:])
