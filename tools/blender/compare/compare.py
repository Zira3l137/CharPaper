"""Compares the .glb files Blender's glTF exporter wrote into a loose suite with the ones
charpaper_pack wrote into a packed one. Each check prints what differs and returns how many
problems it found."""

import hashlib
import json
import struct

CLOSE = 1e-5

_WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
_FORMAT = {5126: "f", 5121: "B", 5123: "H", 5125: "I"}


class Glb:
    def __init__(self, data):
        length = struct.unpack_from("<I", data, 12)[0]
        self.doc = json.loads(data[20:20 + length])
        rest = data[20 + length:]
        self.blob = rest[8:8 + struct.unpack_from("<I", rest, 0)[0]] if rest else b""

    def accessor(self, index):
        accessor = self.doc["accessors"][index]
        view = self.doc["bufferViews"][accessor["bufferView"]]
        width = _WIDTH[accessor["type"]]
        start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        count = accessor["count"] * width
        values = struct.unpack_from(f"<{count}{_FORMAT[accessor['componentType']]}", self.blob, start)
        return [values[k:k + width] for k in range(0, len(values), width)]

    def name(self, node):
        return self.doc["nodes"][node]["name"]


def _gap(a, b):
    return max((abs(x - y) for x, y in zip(a, b)), default=0.0)


def _turn_gap(a, b):
    # q and -q are the same rotation.
    return min(_gap(a, b), _gap(a, [-x for x in b]))


class Report:
    def __init__(self, what):
        self.what = what
        self.problems = 0

    def differs(self, part, theirs, ours):
        self.problems += 1
        print(f"DIFFERS: {self.what} {part}\n    Blender's: {str(theirs)[:600]}\n    ours:      {str(ours)[:600]}")

    def same(self, part, theirs, ours):
        if theirs != ours:
            self.differs(part, theirs, ours)


# --- Clips ---


def _tracks(glb):
    tracks = {}
    for animation in glb.doc.get("animations", [])[:1]:
        for channel in animation["channels"]:
            sampler = animation["samplers"][channel["sampler"]]
            key = (glb.name(channel["target"]["node"]), channel["target"]["path"])
            times = [t[0] for t in glb.accessor(sampler["input"])]
            tracks[key] = (times, glb.accessor(sampler["output"]), sampler.get("interpolation", "LINEAR"))
    return tracks


def _at(track, t):
    times, values, how = track
    if t <= times[0] or len(times) == 1:
        return values[0]
    for k in range(1, len(times)):
        if t <= times[k] + 1e-6:
            if how == "STEP":
                return values[k - 1] if t < times[k] - 1e-6 else values[k]
            f = (t - times[k - 1]) / (times[k] - times[k - 1])
            return [a + (b - a) * f for a, b in zip(values[k - 1], values[k])]
    return values[-1]


def clips(loose, pack, step=1):
    """Every clip in the pack against Blender's: each channel of every node its .json names,
    at every sampled frame. Keys may sit at other frames as long as the motion is the same."""
    report = Report("clips")
    worst = {"translation": 0.0, "rotation": 0.0, "scale": 0.0}
    channels = 0
    for path in pack.files():
        glb_path = path[:-5] + ".glb"
        if not path.endswith(".json") or not pack.is_file(glb_path):
            continue
        meta = json.loads(pack.read(path))
        ours = _tracks(Glb(pack.read(glb_path)))
        with open(f"{loose}/{glb_path}", "rb") as file:
            theirs = _tracks(Glb(file.read()))
        for node in meta["nodes"]:
            for prop in worst:
                a, b = theirs.get((node, prop)), ours.get((node, prop))
                if a is None and b is None:
                    continue
                if a is None or b is None:
                    report.differs(f"{glb_path} {node} {prop} is there", a is not None, b is not None)
                    continue
                # Keys held still may be written either way, two of them.
                if len(a[0]) > 2 and len(b[0]) > 2:
                    report.same(f"{glb_path} {node} {prop} interpolation", a[2], b[2])
                gap = _turn_gap if prop == "rotation" else _gap
                for i in range(0, meta["frames"], step):
                    t = i / meta["fps"]
                    worst[prop] = max(worst[prop], gap(_at(a, a[0][0] + t), _at(b, b[0][0] + t)))
                channels += 1
    print(f"clips: {channels} channels, largest difference {max(worst.values()):.2e}")
    for prop, gap in worst.items():
        if gap > 1e-4:
            report.differs(f"{prop} by {gap:.2e}", "", "")
    return report.problems


# --- Models and skins ---


def _nodes(glb):
    parents = {c: i for i, n in enumerate(glb.doc["nodes"]) for c in n.get("children", [])}
    found = {}
    for i, node in enumerate(glb.doc["nodes"]):
        parent = glb.name(parents[i]) if i in parents else None
        trs = (node.get("translation", (0, 0, 0)), node.get("rotation", (0, 0, 0, 1)), node.get("scale", (1, 1, 1)))
        found[node["name"]] = ((parent, "mesh" in node, "skin" in node), trs)
    return found


def _compare_nodes(report, theirs, ours):
    a, b = _nodes(theirs), _nodes(ours)
    report.same("node names", sorted(a), sorted(b))
    # An object stepped aside for its copy, which leaked into the file under a temporary name.
    for side, nodes in (("Blender's", a), ("ours", b)):
        leaked = [name for name in nodes if name.startswith("charpaper~")]
        if leaked:
            report.differs(f"{side} has stepped-aside objects", leaked, "")
    for name in set(a) & set(b):
        report.same(f"node {name}", a[name][0], b[name][0])
        (t1, r1, s1), (t2, r2, s2) = a[name][1], b[name][1]
        if max(_gap(t1, t2), _turn_gap(r1, r2), _gap(s1, s2)) > CLOSE:
            report.differs(f"node {name} transform", a[name][1], b[name][1])


def _lights_and_cameras(glb):
    lights = glb.doc.get("extensions", {}).get("KHR_lights_punctual", {}).get("lights", [])
    found = {}
    for node in glb.doc["nodes"]:
        light = node.get("extensions", {}).get("KHR_lights_punctual", {}).get("light")
        if light is not None:
            found[node["name"]] = ("light", lights[light])
        if "camera" in node:
            found[node["name"]] = ("camera", glb.doc["cameras"][node["camera"]])
    return found


def _numbers_close(a, b):
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(_numbers_close(a[k], b[k]) for k in a)
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return abs(a - b) <= CLOSE * max(1.0, abs(a))
    if isinstance(a, (list, tuple)) and isinstance(b, (list, tuple)):
        return len(a) == len(b) and all(_numbers_close(x, y) for x, y in zip(a, b))
    return a == b


def _compare_lights_and_cameras(report, theirs, ours):
    a, b = _lights_and_cameras(theirs), _lights_and_cameras(ours)
    report.same("nodes with lights and cameras", sorted(a), sorted(b))
    for name in set(a) & set(b):
        if not _numbers_close(a[name], b[name]):
            report.differs(f"{name}'s {a[name][0]}", a[name][1], b[name][1])
    report.same("extensions used", sorted(theirs.doc.get("extensionsUsed", [])), sorted(ours.doc.get("extensionsUsed", [])))


def _compare_skins(report, theirs, ours):
    skins = theirs.doc.get("skins", []), ours.doc.get("skins", [])
    report.same("skin count", len(skins[0]), len(skins[1]))
    for a, b in zip(*skins):
        report.same(f"skin {a.get('name')} joints", [theirs.name(j) for j in a["joints"]], [ours.name(j) for j in b["joints"]])
        binds = zip(theirs.accessor(a["inverseBindMatrices"]), ours.accessor(b["inverseBindMatrices"]))
        if any(_gap(x, y) > CLOSE for x, y in binds):
            report.differs(f"skin {a.get('name')} inverse binds", "", "")


def _vertices(glb, primitive, joints):
    attributes = primitive["attributes"]
    columns = [glb.accessor(attributes[k]) for k in sorted(attributes) if not k.startswith(("JOINTS", "WEIGHTS"))]
    columns += [glb.accessor(t[k]) for t in primitive.get("targets", []) for k in sorted(t)]
    weights = None
    if "JOINTS_0" in attributes:
        weights = zip(glb.accessor(attributes["JOINTS_0"]), glb.accessor(attributes["WEIGHTS_0"]))
        weights = [tuple(sorted((joints[j], round(w, 4)) for j, w in zip(js, ws) if w > 0)) for js, ws in weights]
    count = len(columns[0])
    return [([x for c in columns for x in c[i]], weights[i] if weights else ()) for i in range(count)]


def _meshes(glb):
    skins = glb.doc.get("skins")
    joints = [glb.name(j) for j in skins[0]["joints"]] if skins else []
    found = {}
    for node in glb.doc["nodes"]:
        if "mesh" not in node:
            continue
        mesh = glb.doc["meshes"][node["mesh"]]
        head = (mesh.get("name"), mesh.get("extras"), mesh.get("weights"))
        primitives = {}
        for primitive in mesh["primitives"]:
            indices = [i[0] for i in glb.accessor(primitive["indices"])]
            material = glb.doc["materials"][primitive["material"]]["name"] if "material" in primitive else None
            triangles = [indices[k:k + 3] for k in range(0, len(indices), 3)]
            primitives[material] = (_vertices(glb, primitive, joints), triangles)
        found[node["name"]] = (head, primitives)
    return found


def _match(theirs, ours):
    """Pairs each of Blender's vertices with ours holding the same values, to within CLOSE.
    Returns the pairing, the vertices left unpaired, and the largest difference."""
    grid = {}
    cell = lambda v: (tuple(round(x, 3) for x in v[0][:3]), v[1])
    for j, vertex in enumerate(ours):
        grid.setdefault(cell(vertex), []).append(j)
    pairs, unpaired, worst = {}, 0, 0.0
    for i, vertex in enumerate(theirs):
        gap = lambda j: _gap(vertex[0], ours[j][0])
        near = [j for j in grid.get(cell(vertex), []) if gap(j) <= CLOSE]
        if not near:
            # Rounding to a grid splits values that sit on a cell's edge: look further.
            near = [j for j in range(len(ours)) if ours[j][1] == vertex[1] and gap(j) <= CLOSE]
        if not near:
            unpaired += 1
            continue
        pairs[i] = min(near, key=gap)
        worst = max(worst, gap(pairs[i]))
    return pairs, unpaired, worst


def _compare_meshes(report, theirs, ours):
    a, b = _meshes(theirs), _meshes(ours)
    report.same("meshes", sorted(a), sorted(b))
    for name in sorted(set(a) & set(b)):
        (head_a, prims_a), (head_b, prims_b) = a[name], b[name]
        report.same(f"mesh {name} name, shape key names and weights", head_a, head_b)
        report.same(f"mesh {name} materials", sorted(map(str, prims_a)), sorted(map(str, prims_b)))
        for material in set(prims_a) & set(prims_b):
            (va, ta), (vb, tb) = prims_a[material], prims_b[material]
            pairs, unpaired, worst = _match(va, vb)
            print(f"  {name} / {material}: {len(va)} vertices, {unpaired} unpaired, largest difference {worst:.2e}")
            report.same(f"mesh {name} / {material} vertex count", len(va), len(vb))
            if unpaired:
                report.differs(f"mesh {name} / {material} vertices", f"{unpaired} without a match", "")
                continue
            ours_tris = sorted(sorted(t) for t in tb)
            report.same(f"mesh {name} / {material} triangles", sorted(sorted(pairs[i] for i in t) for t in ta), ours_tris)


def _materials(glb):
    doc = glb.doc

    def image(index):
        found = doc["images"][index]
        view = doc["bufferViews"][found["bufferView"]]
        start = view.get("byteOffset", 0)
        data = glb.blob[start:start + view["byteLength"]]
        return hashlib.sha256(data).hexdigest()[:16], found.get("mimeType")

    def resolve(value, key=None):
        if key == "index" and isinstance(value, int):
            texture = doc["textures"][value]
            sampler = doc["samplers"][texture["sampler"]] if "sampler" in texture else None
            return image(texture["source"]), sampler
        if isinstance(value, dict):
            return {k: resolve(v, k) for k, v in value.items()}
        if isinstance(value, list):
            return [resolve(v) for v in value]
        return value

    return sorted(json.dumps(resolve(m), sort_keys=True) for m in doc.get("materials", []))


def scene(loose, pack, path):
    """A model, skin, environment or camera file against Blender's: nodes, skins, meshes
    vertex by vertex and triangle by triangle, shape keys, lights, cameras, materials with their
    images, and the extensions used. What moves in a camera file is for `clips` to compare."""
    report = Report(path)
    with open(f"{loose}/{path}", "rb") as file:
        theirs = Glb(file.read())
    ours = Glb(pack.read(path))
    _compare_nodes(report, theirs, ours)
    _compare_skins(report, theirs, ours)
    _compare_meshes(report, theirs, ours)
    _compare_lights_and_cameras(report, theirs, ours)
    report.same("materials", _materials(theirs), _materials(ours))
    print(f"{path}: {report.problems} problems")
    return report.problems
