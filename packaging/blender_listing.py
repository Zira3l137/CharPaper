"""Writes the Blender extensions listing for a release's add-on, as the website serves it.

Blender installs an add-on dragged in from a web page only from a repository that lists it:
a JSON file naming each package's zip, size and hash. Once that repository is added, Blender
also offers the add-on's updates. The listing is the one `blender --command extension
server-generate` writes, with the zip left where the release keeps it.

usage: blender_listing.py ADDON_ZIP ARCHIVE_URL LISTING_URL OUT_JSON

Prints the query that download links append for drag and drop: Blender reads the repository
and the versions it fits from it before downloading anything.
"""

import hashlib
import json
import os
import sys
import tomllib
import urllib.parse
import zipfile

# Kept out of the listing by Blender's own generator too: they only matter to the build.
BUILD_ONLY = {"build"}


def main(addon_zip, archive_url, listing_url, out):
    with zipfile.ZipFile(addon_zip) as archive:
        manifest = tomllib.loads(archive.read("blender_manifest.toml").decode("utf-8"))
    data = open(addon_zip, "rb").read()
    package = {key: value for key, value in manifest.items() if key not in BUILD_ONLY}
    package.update(
        {
            "archive_url": archive_url,
            "archive_size": len(data),
            "archive_hash": "sha256:" + hashlib.sha256(data).hexdigest(),
        }
    )
    listing = {"version": "v1", "blocklist": [], "data": [package]}
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    with open(out, "w", encoding="utf-8") as file:
        json.dump(listing, file, indent=2, ensure_ascii=False)
        file.write("\n")

    query = {"repository": listing_url, "blender_version_min": manifest["blender_version_min"]}
    if "blender_version_max" in manifest:
        query["blender_version_max"] = manifest["blender_version_max"]
    if "platforms" in manifest:
        query["platforms"] = ",".join(manifest["platforms"])
    print(urllib.parse.urlencode(query))


if __name__ == "__main__":
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    main(*sys.argv[1:])
