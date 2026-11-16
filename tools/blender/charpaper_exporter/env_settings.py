# What an environment's [environments.<name>] table takes from Blender that glTF can't
# carry: fog boxes, read from their Principled Volume, the lights that skip the fog, the
# lights and objects that cast no shadows, and lights' Radius and keyed Strength and Color.
# Export owns these keys; the look's keys are left alone.
#
# A fog's Density may come from a Noise Texture, through nodes that turn its value into
# another: Color Ramp, Map Range, Math with one input from the chain, Float Curve. The app
# draws Blender's fBM noise itself, so export samples what those nodes make of each noise
# value into a curve, and works out where the box's points fall in the noise.

import math

import bpy
import mathutils

from . import anim
from . import lights as light_settings

KEYS = ("fog", "no_volume_scatter", "no_shadows", "lights")

NOISE_POINTS = 32
MAX_NOISE_DETAIL = 3.0

# glTF's Y up: a point (x, y, z) in the app's object space is (x, -z, y) in Blender's.
APP_TO_BLENDER = mathutils.Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))

# How Blender turns a color into a number: its luminance.
LUMINANCE = (0.2126, 0.7152, 0.0722)

FOG_INPUTS = {
    "Color": "color",
    "Density": "density",
    "Anisotropy": "anisotropy",
    "Absorption Color": "absorption_color",
}

def volume_node(obj):
    """What feeds the Volume output of a fog object: a material with a volume and no surface."""
    if obj.type != "MESH":
        return None
    for slot in obj.material_slots:
        material = slot.material
        if material is None or material.node_tree is None:
            continue
        output = material.node_tree.get_output_node("ALL")
        if output is None:
            continue
        if output.inputs["Volume"].is_linked and not output.inputs["Surface"].is_linked:
            return output.inputs["Volume"].links[0].from_node
    return None


def is_principled(node):
    return node.bl_idname == "ShaderNodeVolumePrincipled"


def fog_table(obj, node):
    if not is_principled(node):
        return {}
    noise, _ = density_noise(obj, node)
    table = {}
    for socket, key in FOG_INPUTS.items():
        if key == "density" and noise is not None:
            continue
        value = node.inputs[socket].default_value
        table[key] = [float(v) for v in value[:3]] if key.endswith("color") else float(value)
    if noise is not None:
        table["noise"] = noise
    return table


def density_noise(obj, node):
    """The noise table for a fog whose Density comes from a Noise Texture, and notes on what
    of it the app leaves out. Without one, None and why, if Density is linked at all."""
    chain = _chain(_input(node, "Density"))
    if chain is None:
        return None, [
            "Density comes from nodes other than a Noise Texture through Color Ramp, Map Range, "
            "Math or Float Curve nodes"
        ]
    noise, curve = chain
    if getattr(noise, "noise_type", "FBM") != "FBM":
        return None, ["its Noise Texture isn't fBM"]
    if noise.noise_dimensions not in {"3D", "4D"}:
        return None, [f"its Noise Texture is {noise.noise_dimensions}, not 3D"]
    sockets = _inputs(noise, "Vector", "Scale", "Detail", "Roughness", "Lacunarity", "Distortion")
    if sockets is None:
        return None, ["its Noise Texture lacks inputs the exporter knows"]
    vector, scale, detail, roughness, lacunarity, distortion = sockets
    if any(s.is_linked for s in (scale, detail, roughness, lacunarity)):
        return None, ["its Noise Texture's Scale, Detail, Roughness or Lacunarity come from nodes"]
    coordinates = _coordinates(obj, vector)
    if coordinates is None:
        return None, [
            "its Noise Texture's Vector isn't Generated or Object coordinates, through a Mapping "
            "node at most"
        ]
    noise_from_object, mapping = coordinates

    scale = mathutils.Matrix.Scale(scale.default_value, 4)
    noise_from_box = scale @ noise_from_object @ _object_from_box(obj)
    detail = detail.default_value
    notes = []
    if detail > MAX_NOISE_DETAIL:
        notes.append(f"the app draws Detail up to {MAX_NOISE_DETAIL:g}, not {detail:g}")
    if distortion.default_value != 0.0:
        notes.append("Distortion doesn't carry over")
    if noise.noise_dimensions == "4D":
        notes.append("the app draws the noise in 3D, without W")

    table = {
        "transform": [[float(v) for v in noise_from_box[row]] for row in range(3)],
        "detail": float(min(detail, MAX_NOISE_DETAIL)),
        "roughness": float(roughness.default_value),
        "lacunarity": float(lacunarity.default_value),
        "normalize": bool(noise.normalize),
        "density": [max(float(curve(i / (NOISE_POINTS - 1))), 0.0) for i in range(NOISE_POINTS)],
    }
    drift = _drift(mapping)
    if drift is not None:
        table["drift"] = [float(v) for v in (scale.to_3x3() @ drift)]
    return table, notes


def _input(node, identifier):
    """A node's input by identifier, which stays as it was when Blender renames a socket, as
    newer versions show Fac as Factor."""
    return next((s for s in node.inputs if s.identifier == identifier and s.enabled), None)


def _inputs(node, *identifiers):
    found = [_input(node, identifier) for identifier in identifiers]
    return None if None in found else found


def _linked(socket):
    """The node and output feeding a socket through any reroutes, or None."""
    while socket.is_linked:
        link = socket.links[0]
        if not link.is_valid or link.is_muted or link.from_node.mute:
            return None
        if link.from_node.bl_idname != "NodeReroute":
            return link.from_node, link.from_socket
        socket = link.from_node.inputs[0]
    return None


def _chain(socket):
    """The Noise Texture feeding a socket, and what the nodes in between make of its value."""
    if socket is None:
        return None
    fed = _linked(socket)
    if fed is None:
        return None
    node, output = fed
    kind = node.bl_idname
    if kind == "ShaderNodeTexNoise":
        return (node, lambda x: x) if output.identifier == "Fac" else None

    if kind == "ShaderNodeValToRGB":
        chain = _chain(_input(node, "Fac"))
        if chain is None:
            return None
        noise, inner = chain
        ramp = node.color_ramp
        if output.identifier == "Alpha":
            return noise, lambda x: ramp.evaluate(_unit(inner(x)))[3]
        return noise, lambda x: _gray(ramp.evaluate(_unit(inner(x))))

    if kind == "ShaderNodeMapRange" and node.data_type == "FLOAT":
        ends = _inputs(node, "From Min", "From Max", "To Min", "To Max", "Steps")
        if ends is None or any(s.is_linked for s in ends):
            return None
        chain = _chain(_input(node, "Value"))
        if chain is None:
            return None
        noise, inner = chain
        ends = [s.default_value for s in ends]
        return noise, lambda x: _map_range(node, inner(x), *ends)

    if kind == "ShaderNodeMath":
        linked = [i for i, s in enumerate(node.inputs) if s.enabled and s.is_linked]
        if len(linked) != 1 or node.operation not in MATH:
            return None
        chain = _chain(node.inputs[linked[0]])
        if chain is None:
            return None
        noise, inner = chain
        values = [s.default_value for s in node.inputs]

        def math_node(x):
            args = list(values)
            args[linked[0]] = inner(x)
            result = MATH[node.operation](*args)
            return _unit(result) if node.use_clamp else result

        return noise, math_node

    sockets = _inputs(node, "Factor", "Value") if kind == "ShaderNodeFloatCurve" else None
    if sockets is not None and not sockets[0].is_linked:
        chain = _chain(sockets[1])
        if chain is None:
            return None
        noise, inner = chain
        mapping = node.mapping
        mapping.initialize()
        factor = sockets[0].default_value

        def curve_node(x):
            value = inner(x)
            return value + (mapping.evaluate(mapping.curves[0], value) - value) * factor

        return noise, curve_node
    return None


def _unit(x):
    return min(max(x, 0.0), 1.0)


def _gray(color):
    return sum(c * w for c, w in zip(color[:3], LUMINANCE))


def _divide(a, b):
    return a / b if b != 0.0 else 0.0


def _power(a, b):
    if a < 0.0 and b != math.floor(b):
        return 0.0
    try:
        return math.pow(a, b)
    except (OverflowError, ValueError, ZeroDivisionError):
        return 0.0


# Blender's Math node, safe where Blender is: dividing by zero gives 0, and so do the square
# root of a negative number and a negative number to a fractional power.
MATH = {
    "ADD": lambda a, b, c: a + b,
    "SUBTRACT": lambda a, b, c: a - b,
    "MULTIPLY": lambda a, b, c: a * b,
    "DIVIDE": lambda a, b, c: _divide(a, b),
    "MULTIPLY_ADD": lambda a, b, c: a * b + c,
    "POWER": lambda a, b, c: _power(a, b),
    "SQRT": lambda a, b, c: math.sqrt(a) if a > 0.0 else 0.0,
    "ABSOLUTE": lambda a, b, c: abs(a),
    "MINIMUM": lambda a, b, c: min(a, b),
    "MAXIMUM": lambda a, b, c: max(a, b),
    "LESS_THAN": lambda a, b, c: 1.0 if a < b else 0.0,
    "GREATER_THAN": lambda a, b, c: 1.0 if a > b else 0.0,
}


def _map_range(node, value, from_min, from_max, to_min, to_max, steps):
    kind = node.interpolation_type
    if kind in {"SMOOTHSTEP", "SMOOTHERSTEP"}:
        low, high, flip = (from_max, from_min, True) if from_min > from_max else (from_min, from_max, False)
        t = _unit(_divide(value - low, high - low))
        t = t * t * (3.0 - 2.0 * t) if kind == "SMOOTHSTEP" else t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
        factor = 1.0 - t if flip else t
    else:
        factor = _divide(value - from_min, from_max - from_min)
        if kind == "STEPPED":
            factor = math.floor(factor * (steps + 1.0)) / steps if steps > 0.0 else 0.0
    result = to_min + factor * (to_max - to_min)
    if node.clamp and kind in {"LINEAR", "STEPPED"}:
        result = min(max(result, min(to_min, to_max)), max(to_min, to_max))
    return result


def _coordinates(obj, socket):
    """The matrix from Blender's object space to what reaches a Noise Texture's Vector, before
    its Scale, and the Mapping node in between, if any."""
    fed = _linked(socket)
    if fed is None:
        return _generated(obj), None
    node, output = fed
    mapping = None
    if node.bl_idname == "ShaderNodeMapping":
        if node.vector_type not in {"POINT", "TEXTURE"}:
            return None
        sockets = _inputs(node, "Vector", "Location", "Rotation", "Scale")
        if sockets is None or any(s.is_linked for s in sockets[1:]):
            return None
        mapping = node
        fed = _linked(sockets[0])
        if fed is None:
            return None
        node, output = fed
    if node.bl_idname != "ShaderNodeTexCoord" or node.object not in (None, obj):
        return None
    if output.identifier == "Generated":
        coordinates = _generated(obj)
    elif output.identifier == "Object":
        coordinates = mathutils.Matrix.Identity(4)
    else:
        return None
    return (_mapped(mapping) if mapping else mathutils.Matrix.Identity(4)) @ coordinates, mapping


def _generated(obj):
    """Blender's Generated coordinates: the mesh's texture space, from 0 to 1."""
    mesh = obj.data
    size = [max(abs(s), 1e-6) for s in mesh.texspace_size]
    to_corner = mathutils.Matrix.Translation(-mathutils.Vector(mesh.texspace_location) + mathutils.Vector(size))
    return mathutils.Matrix.Diagonal([0.5 / s for s in size] + [1.0]) @ to_corner


def _mapped(mapping, location=None):
    """A Mapping node as a matrix: Point scales, turns and then moves; Texture undoes that."""
    _, at, turn, size = _inputs(mapping, "Vector", "Location", "Rotation", "Scale")
    if location is None:
        location = at.default_value
    rotation = mathutils.Euler(turn.default_value, "XYZ").to_matrix().to_4x4()
    scale = mathutils.Matrix.Diagonal(list(size.default_value) + [1.0])
    point = mathutils.Matrix.Translation(location) @ rotation @ scale
    if mapping.vector_type == "POINT":
        return point
    # Blender divides by zero scale safely, to 0; so does a pseudo-inverse here.
    safe = mathutils.Matrix.Diagonal([_divide(1.0, s) for s in size.default_value] + [1.0])
    return safe @ rotation.transposed() @ mathutils.Matrix.Translation(-mathutils.Vector(location))


def _object_from_box(obj):
    """From the app's fog box, the cube from -0.5 to 0.5 at the mesh's bounds, to Blender's
    object space. The app's object space is glTF's, with Y up."""
    corners = [mathutils.Vector(c) for c in obj.bound_box]
    low = mathutils.Vector([min(c[i] for c in corners) for i in range(3)])
    high = mathutils.Vector([max(c[i] for c in corners) for i in range(3)])
    size = [max(v, 0.001) for v in high - low]
    app_size = [size[0], size[2], size[1]]
    return (
        mathutils.Matrix.Translation((low + high) / 2.0)
        @ APP_TO_BLENDER
        @ mathutils.Matrix.Diagonal(app_size + [1.0])
    )


def _drift(mapping):
    """How far an animated Mapping Location moves the noise's coordinates each second, on
    average over its keys; None when it doesn't move."""
    if mapping is None:
        return None
    tree = mapping.id_data
    data = tree.animation_data
    if data is None or data.action is None:
        return None
    location_socket = _input(mapping, "Location")
    path = location_socket.path_from_id("default_value")
    render = bpy.context.scene.render
    fps = render.fps / render.fps_base
    speed = [0.0, 0.0, 0.0]
    for curve in anim.fcurves(data.action, getattr(data, "action_slot", None)):
        if curve.data_path != path or curve.mute or len(curve.keyframe_points) < 2:
            continue
        first, last = curve.keyframe_points[0].co[0], curve.keyframe_points[-1].co[0]
        if last > first:
            speed[curve.array_index] = (curve.evaluate(last) - curve.evaluate(first)) * fps / (last - first)
    if not any(speed):
        return None
    location = mathutils.Vector(location_socket.default_value)
    moved = _mapped(mapping, location + mathutils.Vector(speed)).translation
    return moved - _mapped(mapping, location).translation


def fog_objects(objects):
    return {obj: node for obj in objects if (node := volume_node(obj)) is not None}


def table(collection):
    objects = [o for o in collection.all_objects if o.type != "CAMERA"]
    fog = fog_objects(objects)
    lights = [o for o in objects if o.type == "LIGHT"]
    unshadowed = [o for o in lights if not o.data.use_shadow]
    unshadowed += [o for o in objects if o.type != "LIGHT" and not o.visible_shadow]
    return {
        "fog": {obj.name: fog_table(obj, node) for obj, node in sorted(fog.items(), key=lambda i: i[0].name)},
        "no_volume_scatter": sorted(o.name for o in lights if o.data.volume_factor <= 0.0),
        "no_shadows": sorted(o.name for o in unshadowed),
        "lights": {o.name: light for o in sorted(lights, key=lambda o: o.name) if (light := light_table(o))},
    }


def light_table(obj):
    """What glTF can't carry of a light: a point or spot light's Radius, and its keyed
    Strength and Color."""
    table = {}
    light = obj.data
    if light.type in {"POINT", "SPOT"} and light.shadow_soft_size > 0.0:
        table["radius"] = float(light.shadow_soft_size)
    keys = light_settings.keyed(light, bpy.context.scene)
    if keys is not None:
        table["animation"] = light_settings.animation_table(keys)
    return table
