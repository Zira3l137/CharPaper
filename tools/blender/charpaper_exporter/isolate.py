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
        self._data = []
        self._renamed = []

    def _step_aside(self, block):
        name = block.name
        block.name = "charpaper~" + uuid.uuid4().hex
        self._renamed.append((block, name))
        return name

    def add(self, obj, own_mesh=False, own_light=False, keep_animation=False):
        if obj in self.copies:
            return self.copies[obj]
        name = self._step_aside(obj)
        copy = obj.copy()
        copy.name = name
        if own_mesh and obj.type == "MESH":
            mesh_name = self._step_aside(obj.data)
            key = obj.data.shape_keys
            key_name = self._step_aside(key) if key else None
            data = obj.data.copy()
            data.name = mesh_name
            if data.shape_keys and key_name:
                data.shape_keys.name = key_name
            copy.data = data
            self._data.append(data)
        # glTF names a light after its light data, and the app finds lights by that name too.
        if own_light and obj.type == "LIGHT":
            light_name = self._step_aside(obj.data)
            data = obj.data.copy()
            data.name = light_name
            copy.data = data
            self._data.append(data)
        if not keep_animation:
            copy.animation_data_clear()
            if own_mesh and copy.type == "MESH" and copy.data.shape_keys:
                copy.data.shape_keys.animation_data_clear()
        self.scene.collection.objects.link(copy)
        self.copies[obj] = copy
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
        for data in self._data:
            if data.users == 0:
                blocks = bpy.data.lights if isinstance(data, bpy.types.Light) else bpy.data.meshes
                blocks.remove(data)
        bpy.data.scenes.remove(self.scene)
        for block, name in reversed(self._renamed):
            block.name = name
        self.copies.clear()


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
