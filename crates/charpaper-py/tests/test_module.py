"""The Python side of charpaper_pack. With the wheel installed: python -m unittest discover crates/charpaper-py/tests"""

import array
import json
import os
import shutil
import struct
import tempfile
import unittest

import charpaper_pack as cp


def write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as file:
        file.write(data)


def panorama(path, width=64, height=32):
    # A flat Radiance .hdr: a bright top half over a dark bottom one.
    header = f"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n".encode()
    rows = [bytes([128, 128, 128, 130] if y < height // 2 else [128, 64, 0, 126]) * width for y in range(height)]
    write(path, header + b"".join(rows))


class PackTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="charpaper-py-")
        self.suite = os.path.join(self.dir, "Aria")
        self.out = os.path.join(self.suite, cp.PACK_FILE)
        self.staged = os.path.join(self.dir, "staged")
        write(os.path.join(self.staged, "model.glb"), os.urandom(300))
        write(os.path.join(self.staged, "skins", "casual.glb"), os.urandom(400))
        write(os.path.join(self.staged, "clips", "idle.json"), b'{"frames": 3}\n' * 500)
        write(os.path.join(self.staged, "suite.toml"), b"schema = 2\n")
        os.makedirs(self.suite)

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def test_round_trip(self):
        steps = []
        summary = cp.pack(self.staged, self.out, progress=lambda *step: steps.append(step))
        self.assertEqual(summary["files"], 3)
        self.assertEqual(steps[-1], (None, 3, 3))
        self.assertEqual([s[0] for s in steps[:-1]], ["clips/idle.json", "model.glb", "skins/casual.glb"])

        pack = cp.Pack(self.out)
        self.assertEqual(len(pack), 3)
        self.assertEqual(pack.files(), ["clips/idle.json", "model.glb", "skins/casual.glb"])
        self.assertNotIn("suite.toml", pack)
        self.assertEqual(pack.read("clips/idle.json"), b'{"frames": 3}\n' * 500)
        self.assertTrue(pack.info("clips/idle.json")["compressed"])
        self.assertFalse(pack.info("model.glb")["compressed"])
        self.assertEqual(pack.files_in(""), ["model.glb"])
        self.assertEqual(pack.folders_in(), ["clips", "skins"])
        self.assertTrue(pack.is_dir("skins"))
        pack.verify()

        target = os.path.join(self.dir, "out", "casual.glb")
        pack.extract("skins/casual.glb", target)
        with open(target, "rb") as file:
            self.assertEqual(file.read(), pack.read("skins/casual.glb"))

        with self.assertRaises(KeyError):
            pack.read("skins/nope.glb")
        with self.assertRaises(KeyError):
            pack.info("nope")

    def test_reuse_and_keep(self):
        cp.pack(self.staged, self.out)
        first = cp.Pack(self.out)
        again = cp.pack(self.staged, self.out)
        self.assertEqual(again["reused"], 3)
        self.assertEqual(cp.Pack(self.out).id, first.id)

        only_skins = os.path.join(self.dir, "skins-only")
        write(os.path.join(only_skins, "skins", "formal.glb"), os.urandom(100))
        cp.pack(only_skins, self.out, keep=["clips", "model.glb"])
        self.assertEqual(cp.Pack(self.out).files(), ["clips/idle.json", "model.glb", "skins/formal.glb"])

        with self.assertRaises(cp.PackReplaced):
            first.read("model.glb")

    def test_a_raise_at_the_end_still_comes_out(self):
        def fail_at_the_end(path, done, total):
            if path is None:
                raise RuntimeError("late")

        with self.assertRaisesRegex(RuntimeError, "late"):
            cp.pack(self.staged, self.out, progress=fail_at_the_end)
        self.assertEqual(len(cp.Pack(self.out)), 3)

    def test_keep_needs_a_pack(self):
        with self.assertRaises(cp.PackFailed):
            cp.pack(self.staged, self.out, keep=["skins"])

    def test_a_raising_progress_stops_and_keeps_the_old_pack(self):
        cp.pack(self.staged, self.out)
        before = cp.Pack(self.out).id
        write(os.path.join(self.staged, "model.glb"), os.urandom(300))

        def stop(path, done, total):
            if done == 1:
                raise RuntimeError("cancelled")

        with self.assertRaisesRegex(RuntimeError, "cancelled"):
            cp.pack(self.staged, self.out, progress=stop)
        self.assertEqual(cp.Pack(self.out).id, before)
        self.assertFalse(os.path.exists(self.out + ".part"))

    def test_not_a_pack(self):
        write(self.out, b"not a pack at all, just some bytes here for the header")
        with self.assertRaises(cp.PackFailed):
            cp.Pack(self.out)
        with self.assertRaises(cp.PackFailed):
            cp.Pack(os.path.join(self.dir, "nowhere.chr"))
        # Packing over an unreadable pack starts from scratch.
        self.assertEqual(cp.pack(self.staged, self.out)["reused"], 0)

    def test_bake(self):
        source = os.path.join(self.dir, "sky.hdr")
        panorama(source)
        maps = os.path.join(self.staged, "environment", "room")
        self.assertEqual(len(cp.missing_maps(maps)), 3)
        written = cp.bake(source, maps)
        self.assertEqual(len(written), 3)
        self.assertEqual(cp.missing_maps(maps), [])
        self.assertEqual(cp.bake(source, maps), [])
        with self.assertRaises(cp.BakeFailed):
            cp.bake(os.path.join(self.dir, "nowhere.hdr"), os.path.join(self.dir, "empty"))


def floats(values):
    return array.array("f", values).tobytes()


def ints(values):
    return array.array("I", values).tobytes()


IDENTITY = floats([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1])


def triangle(**changes):
    mesh = {
        "name": "Triangle",
        "positions": floats([0, 0, 0, 1, 0, 0, 0, 1, 0]),
        "corner_verts": ints([0, 1, 2]),
        "normals": floats([0, 0, 1] * 3),
        "uvs": [floats([0, 0, 1, 0, 0, 1])],
        "tangents": None,
        "signs": None,
        "triangles": ints([0, 1, 2]),
        "triangle_slots": ints([0]),
        "shape_keys": [{"name": "Lift", "weight": 0.5, "positions": floats([0, 0, 1, 1, 0, 0, 0, 1, 0]), "normals": None}],
        "skin": None,
    }
    mesh.update(changes)
    node = {"name": "Triangle", "matrix": IDENTITY, "children": [], "mesh": 0, "skin": None}
    return {"name": "Scene", "nodes": [node], "roots": [0], "skins": [], "meshes": [mesh]}


class SceneTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="charpaper-py-")
        self.path = os.path.join(self.dir, "model.glb")

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def test_write_scene(self):
        cp.write_scene(self.path, triangle())
        with open(self.path, "rb") as file:
            data = file.read()
        self.assertEqual(data[:4], b"glTF")
        length = struct.unpack_from("<I", data, 12)[0]
        document = json.loads(data[20:20 + length])
        mesh = document["meshes"][0]
        self.assertEqual(mesh["extras"]["targetNames"], ["Lift"])
        self.assertEqual(mesh["weights"], [0.5])
        self.assertIn("TEXCOORD_0", mesh["primitives"][0]["attributes"])

    def test_write_scene_refuses_what_it_cannot_write(self):
        with self.assertRaises(ValueError):
            cp.write_scene(self.path, triangle(normals=floats([0, 0, 1])))
        self.assertFalse(os.path.exists(self.path))


if __name__ == "__main__":
    unittest.main()
