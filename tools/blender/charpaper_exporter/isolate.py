import uuid
from contextlib import contextmanager

import bpy
from mathutils import Matrix

# Every export runs on copies of the objects it needs, linked into a throwaway scene. The
# copies can have their animation cleared and new NLA tracks added without touching the
# user's scene, and the scene holds exactly what goes into the file.
#
# glTF names nodes after objects, and the app matches files to each other by those names,
# so a copy must carry its original's exact name. The original steps aside under a
# temporary name for the duration of the export.
#
# Copying, renaming and removing an object each walk Blender's name-sorted list of objects, so
# copying thousands, a scattered field of grass, takes minutes. An object the copy would leave
# unchanged can go into the scene itself instead, as an object can be in two scenes.


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
        self._bridges = {}
        self._finished = False
        # How each shared object would have been copied, should finish() find it must be.
        self._shared = {}

    def _step_aside(self, block):
        name = block.name
        block.name = "charpaper~" + uuid.uuid4().hex
        self._renamed.append((block, name))
        return name

    def add(self, obj, own_mesh=False, own_light=False, keep_drivers=False, share=False):
        """Copies the object into the scene, without its actions and NLA tracks. An export that
        wants them adds tracks to the copy. Drivers go too, unless kept: a driven bone or shape
        key then still moves when the exporter samples the animation.

        With `share`, an object with nothing to strip and no data of its own to get goes into
        the scene itself, unless finish() finds it under a copied parent or following a copied
        target. The caller then gets the user's own object, and must not change it."""
        if obj in self.copies:
            return self.copies[obj]
        owns = (own_mesh and obj.type == "MESH") or (own_light and obj.type == "LIGHT")
        if share and not owns and not _moves(obj):
            self.scene.collection.objects.link(obj)
            self.copies[obj] = obj
            self._shared[obj] = (own_mesh, own_light, keep_drivers)
            return obj
        return self._copy(obj, own_mesh, own_light, keep_drivers)

    def _copy(self, obj, own_mesh, own_light, keep_drivers):
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

    def stand_in(self, obj):
        """An empty in the object's place, under its name: a node that animates as the object
        does without carrying its mesh or light into the file."""
        if obj in self.copies:
            return self.copies[obj]
        name = self._step_aside(obj)
        empty = bpy.data.objects.new(name, None)
        empty.rotation_mode = obj.rotation_mode
        empty.matrix_basis = obj.matrix_basis.copy()
        empty.parent_type, empty.parent_bone = obj.parent_type, obj.parent_bone
        self.scene.collection.objects.link(empty)
        self.copies[obj] = empty
        # Parented in finish(), which keeps the original's parent inverse.
        empty.matrix_parent_inverse = obj.matrix_parent_inverse.copy()
        return empty

    def stand_in_with_ancestors(self, obj):
        """The object and every object above it as stand-ins, an armature among them copied
        whole: glTF turns only root nodes to its axes, so a node keeps its parents to come out
        as in the file it animates."""
        chain = []
        current = obj
        while current is not None:
            chain.append(current)
            current = current.parent
        for item in reversed(chain):
            if item.type == "ARMATURE":
                self.add(item)
            else:
                self.stand_in(item)
        return self.copies[obj]

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
        if self._finished:
            return
        self._finished = True
        # glTF keeps the scene's name, and a random one would make every export a new file,
        # which a pack would compress again and the app would take new outfit pictures of.
        self.scene.name = "CharPaper export"
        self._copy_what_must_be()
        for original, copy in self.copies.items():
            if copy is original:
                continue
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
        for original, copy in list(self.copies.items()):
            if copy is original:
                continue
            parent = copy.parent
            if parent is not None and parent.type in {"CAMERA", "LIGHT"} and _animated(copy):
                self._bridge(copy)

    def _bridge(self, copy):
        # glTF cameras and lights look down another axis than Blender's. The exporter turns
        # their nodes and turns their children back, but not the children's animation, so a
        # focus object moving away from its camera moves sideways in the file. A still empty
        # in between takes the turn, and the child under it needs none.
        parent = copy.parent
        bridge = self._bridges.get(parent)
        if bridge is None:
            bridge = bpy.data.objects.new(parent.name + " children", None)
            self.scene.collection.objects.link(bridge)
            bridge.parent = parent
            bridge.matrix_parent_inverse = Matrix.Identity(4)
            self._bridges[parent] = bridge
        inverse = copy.matrix_parent_inverse.copy()
        copy.parent = bridge
        copy.matrix_parent_inverse = inverse

    def _copy_what_must_be(self):
        # Copying one shared object can leave another under a copy, so until none is left.
        while True:
            # obj.children would look through every object in the file for each object.
            children = {}
            for obj in bpy.data.objects:
                if obj.parent in self._shared:
                    children.setdefault(obj.parent, []).append(obj)
            found = [obj for obj in self._shared if self._must_copy(obj, children.get(obj, ()))]
            if not found:
                return
            for obj in found:
                self.scene.collection.objects.unlink(obj)
                del self.copies[obj]
                self._copy(obj, *self._shared.pop(obj))

    def _must_copy(self, obj, children):
        parent = obj.parent
        if parent is not None and self.copies.get(parent) is not parent:
            return True
        # The exporters take an object's children from the whole file, not the scene: a child
        # left out, or stepped aside for its copy, would come along under a shared object.
        if any(self.copies.get(child) is not child for child in children):
            return True
        holders = [*obj.modifiers, *obj.constraints]
        if obj.pose:
            holders += [c for bone in obj.pose.bones for c in bone.constraints]
        return any(self.copies.get(t, t) is not t for h in holders for t in _targets(h))

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
        # In one pass: removing objects one at a time looks through the whole file for each.
        copies = [copy for original, copy in self.copies.items() if copy is not original]
        bpy.data.batch_remove([*copies, *self._bridges.values()])
        # By what uses them, not their count of users: the exporter can leave one behind that
        # no data block holds.
        owned = list(self._data.values())
        held = bpy.data.user_map(subset=owned) if owned else {}
        bpy.data.batch_remove([data for data in owned if not held.get(data)])
        bpy.data.scenes.remove(self.scene)
        for block, name in reversed(self._renamed):
            block.name = name
        self.copies.clear()
        self._data.clear()
        self._bridges.clear()
        self._shared.clear()


def _targets(holder):
    """The objects a modifier or constraint follows."""
    for attr in ("object", "target"):
        target = getattr(holder, attr, None)
        if isinstance(target, bpy.types.Object):
            yield target
    for target in getattr(holder, "targets", ()):
        if isinstance(getattr(target, "target", None), bpy.types.Object):
            yield target.target


def _moves(obj):
    # Anything a copy would have stripped: actions, NLA tracks or drivers.
    data = obj.animation_data
    return data is not None and (
        data.action is not None or len(data.nla_tracks) > 0 or len(data.drivers) > 0
    )


def _animated(obj):
    data = obj.animation_data
    return data is not None and (data.action is not None or len(data.nla_tracks) > 0)


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
        with context.temp_override(scene=scene, view_layer=scene.view_layers[0]):
            yield
    finally:
        window.scene = previous
        if _away:
            _away[-1].come_back()


# The run of exports away() is in, if any.
_away = []


class _Away:
    def __init__(self, context, home, layer):
        self.context = context
        self.home = home
        self.layer = layer
        self.overrides = []

    def come_back(self):
        # Setting window.scene makes it the context's scene too, past any override: so the
        # user's scene is overridden in again, each time, and all let go of at the end.
        override = self.context.temp_override(scene=self.home, view_layer=self.layer)
        override.__enter__()
        self.overrides.append(override)

    def close(self):
        while self.overrides:
            self.overrides.pop().__exit__(None, None, None)


@contextmanager
def away(context):
    """For a run of exports: the window shows an empty scene between them, while the code
    still finds the user's scene as context.scene. Each export makes and removes objects, and
    Blender then rebuilds the whole of the scene the window comes back to, which in a scene of
    thousands of objects takes a good part of a second. This way it's rebuilt once, at the end."""
    window = context.window
    if window is None or _away:
        yield
        return
    home, layer = window.scene, context.view_layer
    lobby = bpy.data.scenes.new("charpaper~lobby")
    window.scene = lobby
    run = _Away(context, home, layer)
    _away.append(run)
    run.come_back()
    try:
        yield
    finally:
        _away.pop()
        run.close()
        window.scene = home
        bpy.data.scenes.remove(lobby)


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
