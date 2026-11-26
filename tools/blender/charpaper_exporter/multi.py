"""Several rows of a list picked at once, to change their settings together.

Clicking a row's name picks it alone; Shift-clicking adds it to the pick or takes it out. A
setting changed on a picked row is set on every picked row of its list, so the settings a list
shares are wrapped in `spread`. Clicking elsewhere on a row picks it alone, through the list
index's update, `follow`.

Blender's lists can't tell a Shift-click on a row, so the name is a button that can.
"""

from contextlib import contextmanager

import bpy
from bpy.props import IntProperty
from bpy.props import StringProperty
from bpy.types import Operator

# Set while one row's change is copied to the others, and while the pick button moves the
# active row, so neither starts over from the rows it touches.
_spreading = False
_picking = False


def siblings(item):
    """The collection `item` is in."""
    path = item.path_from_id()
    return item.id_data.path_resolve(path.rsplit("[", 1)[0])


def picked(items, index):
    """The picked rows' indices: those marked, and the active one."""
    marked = {i for i, item in enumerate(items) if item.selected}
    if 0 <= index < len(items):
        marked.add(index)
    return sorted(marked)


def spread(name, then=None):
    """An update for the setting `name` that also sets it on the other picked rows."""

    def update(item, context):
        global _spreading
        if then is not None:
            then(item, context)
        if _spreading or not item.selected:
            return
        value = getattr(item, name)
        _spreading = True
        try:
            for other in siblings(item):
                if other.selected and other.as_pointer() != item.as_pointer() and getattr(other, name) != value:
                    setattr(other, name, value)
        finally:
            _spreading = False

    return update


@contextmanager
def alone():
    """Changes made inside go to the row they're made on only."""
    global _spreading
    was, _spreading = _spreading, True
    try:
        yield
    finally:
        _spreading = was


def follow(collection, index_name, then=None):
    """An update for a list's index: the row made active by clicking it is picked alone."""

    def update(owner, context):
        if not _picking:
            index = getattr(owner, index_name)
            for i, item in enumerate(getattr(owner, collection)):
                if item.selected != (i == index):
                    item.selected = i == index
        if then is not None:
            then(owner, context)

    return update


def remove_picked(owner, collection, index_name):
    """Removes the picked rows, and makes the row before the first of them active."""
    items = getattr(owner, collection)
    gone = picked(items, getattr(owner, index_name))
    for i in reversed(gone):
        items.remove(i)
    if gone:
        setattr(owner, index_name, max(0, min(gone[0] - 1, len(items) - 1)))
    return len(gone)


def count(items):
    return sum(1 for item in items if item.selected)


def button(layout, data, item, index, index_name, text, icon="NONE"):
    """The row's name as its pick button. A row picked with others shows a tick."""
    collection = item.path_from_id().rsplit("[", 1)[0].rsplit(".", 1)[-1]
    several = item.selected and count(getattr(data, collection)) > 1
    row = layout.row(align=True)
    row.alignment = "LEFT"
    op = row.operator("charpaper.pick", text=text, icon="CHECKMARK" if several else icon, emboss=False)
    op.id_kind = "OBJECT" if isinstance(item.id_data, bpy.types.Object) else "SCENE"
    op.id_name = item.id_data.name
    op.owner = data.path_from_id()
    op.collection = collection
    op.index_name = index_name
    op.index = index


def several_note(layout, items, noun):
    """A line over the settings saying they go to all the picked rows."""
    n = count(items)
    if n > 1:
        note = layout.row()
        note.active = False
        note.label(text=f"{n} {noun} picked: changes go to all of them", icon="CHECKMARK")


class CHARPAPER_OT_pick(Operator):
    bl_idname = "charpaper.pick"
    bl_label = "Pick"
    bl_description = "Pick this one. Shift-click to pick several, and change their settings together"
    bl_options = {"INTERNAL", "UNDO"}

    id_kind: StringProperty()
    id_name: StringProperty()
    owner: StringProperty()
    collection: StringProperty()
    index_name: StringProperty()
    index: IntProperty()

    def invoke(self, context, event):
        return self._pick(context, event.shift)

    def execute(self, context):
        return self._pick(context, False)

    def _pick(self, context, adding):
        global _picking
        ids = bpy.data.objects if self.id_kind == "OBJECT" else bpy.data.scenes
        owner = ids.get(self.id_name)
        if owner is not None and self.owner:
            owner = owner.path_resolve(self.owner)
        items = getattr(owner, self.collection, None) if owner is not None else None
        if items is None or not 0 <= self.index < len(items):
            return {"CANCELLED"}
        active = getattr(owner, self.index_name)
        target = items[self.index]
        if adding:
            if 0 <= active < len(items):
                items[active].selected = True
            target.selected = not target.selected
            if not target.selected:
                left = [i for i, item in enumerate(items) if item.selected]
                active = left[0] if left else self.index
                items[active].selected = True
            else:
                active = self.index
        else:
            for i, item in enumerate(items):
                item.selected = i == self.index
            active = self.index
        _picking = True
        try:
            setattr(owner, self.index_name, active)
        finally:
            _picking = False
        for area in context.screen.areas if context.screen else ():
            area.tag_redraw()
        return {"FINISHED"}


def register():
    bpy.utils.register_class(CHARPAPER_OT_pick)


def unregister():
    bpy.utils.unregister_class(CHARPAPER_OT_pick)
