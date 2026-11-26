from . import front
from . import gaze
from . import hotspots
from . import migrate
from . import multi
from . import operators
from . import paths
from . import physics
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
    paths.register()
    migrate.register()
    operators.register()
    gaze.register()
    front.register()
    multi.register()
    hotspots.register()
    physics.register()
    ui.register()


def unregister():
    ui.unregister()
    physics.unregister()
    hotspots.unregister()
    multi.unregister()
    front.unregister()
    gaze.unregister()
    operators.unregister()
    migrate.unregister()
    paths.unregister()
    props.unregister()
