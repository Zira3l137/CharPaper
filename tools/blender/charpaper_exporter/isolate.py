import uuid
from contextlib import contextmanager

import bpy

# Every export runs on copies of the objects it needs, linked into a throwaway scene. The
# copies can have their animation cleared and new NLA tracks added without touching the
# user's scene, and the scene holds exactly what goes into the file.
#
# glTF names nodes after objects, and the app matches files to each other by those names,
# so a copy must carry its original's exact name. The original steps aside under a
# temporary name for the duration of the export.


class Isolation:
    def __init__(self, source):
        self.source = source
        self.scene = bpy.data.scenes.new("charpaper~" + uuid.uuid4().hex[:8])
        render, theirs = self.scene.render, source.render
        render.fps, render.fps_base = theirs.fps, theirs.fps_base
        # The glTF exporter takes a camera's aspect ratio, and so its vertical field of view,
        # from the scene's resolution.
        render.resolution_x, render.resolution_y = theirs.resolution_x, theirs.resolution_y
        render.pixel_aspect_x, render.pixel_aspect_y = theirs.pixel_aspect_x, theirs.pixel_aspect_y
        self.scene.frame_start, self.scene.frame_end = source.frame_start, source.frame_end
        self.scene.frame_current = source.frame_current
        self.copies = {}
        # Each data block copied once, so objects sharing a mesh or a light still share it in
        # the file.
        self._data = {}
        self._renamed = []

    def _step_aside(self, block):
        name = block.name
        block.name = "charpaper~" + uuid.uuid4().hex
        self._renamed.append((block, name))
        return name

    def add(self, obj, own_mesh=False, own_light=False, keep_drivers=False):
        """Copies the object into the scene, without its actions and NLA tracks. An export that
        wants them adds tracks to the copy. Drivers go too, unless kept: a driven bone or shape
        key then still moves when the exporter samples the animation."""
        if obj in self.copies:
            return self.copies[obj]
        name = self._step_aside(obj)
        copy = obj.copy()
        copy.name = name
        _strip(copy, keep_drivers)
        # glTF names a light after its light data, and the app finds lights by that name too.
        if (own_mesh and obj.type == "MESH") or (own_light and obj.type == "LIGHT"):
            copy.data = self._own(obj.data, keep_drivers)
        self.scene.collection.objects.link(copy)
        self.copies[obj] = copy
        return copy

    def _own(self, data, keep_drivers):
        if data in self._data:
            return self._data[data]
        name = self._step_aside(data)
        key = getattr(data, "shape_keys", None)
        key_name = self._step_aside(key) if key else None
        copy = data.copy()
        copy.name = name
        if key_name and copy.shape_keys:
            copy.shape_keys.name = key_name
            _strip(copy.shape_keys, keep_drivers)
        self._data[data] = copy
        return copy

    def add_with_ancestors(self, obj, **kwargs):
        chain = []
        current = obj
        while current is not None:
            chain.append(current)
            current = current.parent
        for item in reversed(chain):
            self.add(item, **kwargs)
        return self.copies[obj]

    def finish(self):
        for original, copy in self.copies.items():
            parent = original.parent
            if parent is not None and parent not in self.copies:
                copy.parent = None
                copy.matrix_world = original.matrix_world.copy()
            elif parent is not None:
                # Setting `parent` from Python resets the parent inverse, which moves anything
                # parented with an offset, like objects parented to a bone.
                inverse = copy.matrix_parent_inverse.copy()
                copy.parent = self.copies[parent]
                copy.matrix_parent_inverse = inverse
            for modifier in copy.modifiers:
                self._retarget(modifier)
            for constraint in copy.constraints:
                self._retarget(constraint)
            if copy.pose:
                for bone in copy.pose.bones:
                    for constraint in bone.constraints:
                        self._retarget(constraint)

    def _retarget(self, holder):
        for attr in ("object", "target"):
            target = getattr(holder, attr, None)
            if isinstance(target, bpy.types.Object) and target in self.copies:
                try:
                    setattr(holder, attr, self.copies[target])
                except (AttributeError, TypeError):
                    pass
        for target in getattr(holder, "targets", ()):
            if getattr(target, "target", None) in self.copies:
                target.target = self.copies[target.target]

    def close(self):
        for copy in self.copies.values():
            bpy.data.objects.remove(copy, do_unlink=True)
        for data in self._data.values():
            if data.users == 0:
                blocks = bpy.data.lights if isinstance(data, bpy.types.Light) else bpy.data.meshes
                blocks.remove(data)
        bpy.data.scenes.remove(self.scene)
        for block, name in reversed(self._renamed):
            block.name = name
        self.copies.clear()
        self._data.clear()


def _strip(block, keep_drivers):
    data = block.animation_data
    if data is None:
        return
    if not keep_drivers:
        block.animation_data_clear()
        return
    data.action = None
    for track in list(data.nla_tracks):
        data.nla_tracks.remove(track)


@contextmanager
def isolated(context):
    isolation = Isolation(context.scene)
    try:
        yield isolation
    finally:
        isolation.close()


@contextmanager
def showing(context, scene):
    # The glTF exporter reads `context.scene` and switches `window.scene` itself, so the
    # throwaway scene has to really be the window's scene while it runs.
    window = context.window
    previous = window.scene
    window.scene = scene
    try:
        yield
    finally:
        window.scene = previous


@contextmanager
def object_mode(context):
    obj = context.view_layer.objects.active
    mode = obj.mode if obj else "OBJECT"
    if mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    try:
        yield
    finally:
        if mode != "OBJECT" and context.view_layer.objects.active == obj:
            try:
                bpy.ops.object.mode_set(mode=mode)
            except RuntimeError:
                pass
