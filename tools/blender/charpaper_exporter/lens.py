import math

# A camera's Depth of Field panel as charpaper's [cameras.<name>] table, since glTF carries
# none of it. The app looks for the focus object in the camera's file first, then on the
# character, so a focus object on the character stays out of the camera's file.


def _chain(obj):
    while obj is not None:
        yield obj
        obj = obj.parent


def focus_object(camera):
    dof = camera.data.dof
    return dof.focus_object if dof.use_dof else None


def on_character(suite, obj):
    return suite.armature is not None and suite.armature in _chain(obj)


def exported_with(suite, camera):
    """The focus object that goes into the camera's file, if any."""
    target = focus_object(camera)
    if target is None or target.type == "ARMATURE" or on_character(suite, target):
        return None
    return target


def table(camera, render):
    data = camera.data
    dof = data.dof
    if not dof.use_dof or data.type != "PERSP":
        return None
    out = {"f_stop": float(dof.aperture_fstop)}
    target = dof.focus_object
    if target is None:
        out["focus_distance"] = float(dof.focus_distance)
    elif target.type == "ARMATURE" and dof.focus_subtarget:
        out["focus_object"] = dof.focus_subtarget
    else:
        out["focus_object"] = target.name
    out["sensor_height_mm"] = round(sensor_height_mm(data, render), 3)
    return out


def sensor_height_mm(data, render):
    # The height that, with the vertical field of view the glTF exporter writes, gives the
    # app the same focal length as Blender's lens, and so the same blur.
    width = render.resolution_x * render.pixel_aspect_x
    height = render.resolution_y * render.pixel_aspect_y
    if width >= height:
        fits_height = data.sensor_fit == "VERTICAL"
    else:
        fits_height = data.sensor_fit != "HORIZONTAL"
    half = data.angle / 2 if fits_height else math.atan(math.tan(data.angle / 2) * height / width)
    return 2 * data.lens * math.tan(half)
