from . import front
from . import operators
from . import props
from . import ui

bl_info = {
    "name": "CharPaper Exporter",
    "author": "Zirael",
    "version": (0, 1, 0),
    "blender": (4, 2, 0),
    "location": "3D Viewport > Sidebar > CharPaper",
    "description": "Exports a CharPaper character suite: model, skins, animations, environments, cameras and suite.toml",
    "category": "Import-Export",
}


def register():
    props.register()
    operators.register()
    front.register()
    ui.register()


def unregister():
    ui.unregister()
    front.unregister()
    operators.unregister()
    props.unregister()
