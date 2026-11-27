import os
from array import array

from . import animated

# Inputs that don't come over at all.
DROPPED = ("Sheen Weight", "Subsurface Weight")
INFLUENCES = 4


def _examples(names, limit=3):
    names = sorted(names)
    shown = ", ".join(names[:limit])
    return shown + (f" and {len(names) - limit} more" if len(names) > limit else "")


def _source(socket):
    """The node feeding a socket, past reroutes, or None."""
    while socket is not None and socket.is_linked:
        node = socket.links[0].from_node
        if node.type != "REROUTE":
            return node
        socket = node.inputs[0]
    return None


def _surface(material):
    tree = material.node_tree if material.use_nodes else None
    output = tree.get_output_node("ALL") if tree is not None else None
    return _source(output.inputs["Surface"]) if output is not None else None


def _principled(material):
    node = _surface(material)
    return node if node is not None and node.type == "BSDF_PRINCIPLED" else None


def _is_math(node, *operations):
    return node is not None and node.type == "MATH" and node.operation in operations


def _alpha(material):
    """The socket the exporter reads alpha from: the Principled BSDF's Alpha, or the Fac of a
    Mix Shader mixing a Transparent BSDF in, as for an unlit screen."""
    surface = _surface(material)
    if surface is None:
        return None
    if surface.type == "BSDF_PRINCIPLED":
        return surface.inputs["Alpha"]
    if surface.type == "MIX_SHADER":
        transparent = _source(surface.inputs[1])
        fac = _source(surface.inputs[0])
        # Mixing by Is Camera Ray is the unlit trick, not alpha.
        if transparent is not None and transparent.type == "BSDF_TRANSPARENT" and (fac is None or fac.type != "LIGHT_PATH"):
            return surface.inputs[0]
    return None


def alpha_mode(material):
    """What the glTF exporter makes of a material's alpha: OPAQUE, MASK (a cutout) or BLEND.

    It writes MASK only for alpha rounded to 0 or 1: a Round or Greater Than node, or
    1 - (alpha < cutoff). Anything else less than 1 becomes BLEND."""
    socket = _alpha(material)
    if socket is None:
        return "OPAQUE"
    if not socket.is_linked:
        return "BLEND" if socket.default_value < 1.0 else "OPAQUE"
    node = _source(socket)
    if _is_math(node, "ROUND", "GREATER_THAN"):
        return "MASK"
    if (
        _is_math(node, "SUBTRACT")
        and not node.inputs[0].is_linked
        and abs(node.inputs[0].default_value - 1.0) < 1e-6
        and _is_math(_source(node.inputs[1]), "LESS_THAN")
    ):
        return "MASK"
    return "BLEND"


def _users(objects):
    users = {}
    for obj in objects:
        for slot in getattr(obj, "material_slots", ()):
            if slot.material is not None:
                users.setdefault(slot.material, obj)
    return users


def _is_jpeg(image):
    return os.path.splitext(image.filepath)[1].lower() in {".jpg", ".jpeg"}


def _mapping(node):
    """The Mapping node placing an Image Texture, or None. One left as it was places nothing,
    and the exporter writes nothing for it."""
    source = _source(node.inputs["Vector"])
    if source is None or source.type != "MAPPING":
        return None
    moved = any(
        source.inputs[name].is_linked or any(abs(v - rest) > 1e-6 for v in source.inputs[name].default_value)
        for name, rest in (("Location", 0.0), ("Rotation", 0.0), ("Scale", 1.0))
        if name in source.inputs
    )
    return source if moved else None


def check(where, objects, add, Finding, character):
    """Warnings for the materials of a skin (character=True) or an environment."""
    for material, obj in _users(objects).items():
        name = f"{where}: material {material.name!r}"
        if not material.use_nodes or material.node_tree is None:
            continue
        mode = alpha_mode(material)
        if mode == "BLEND" and character:
            add(Finding(
                f"{name} is see-through (Blend): the app sorts it per object, so hair cards in one "
                "mesh overlap wrongly, and it casts solid shadows. For hair, lashes or lace, round "
                "its alpha to make a cutout",
                obj,
            ))
        if mode == "BLEND" and not material.use_backface_culling:
            add(Finding(
                f"{name} is see-through and shows both sides, so its back faces can draw over its "
                "front; tick Backface Culling unless both sides must show",
                obj,
                severity="INFO",
            ))

        nodes = animated.image_nodes(material)
        jpeg_alpha = sorted({n.image.name for n in nodes if _is_jpeg(n.image) and n.outputs["Alpha"].is_linked})
        if jpeg_alpha:
            add(Finding(
                f"{name} takes alpha from a JPEG, which has none, so it is solid everywhere: "
                f"{_examples(jpeg_alpha)}. Save it as a PNG",
                obj,
            ))

        bsdf = _principled(material)
        if bsdf is not None:
            dropped = [s.split()[0] for s in DROPPED if bsdf.inputs[s].is_linked or bsdf.inputs[s].default_value > 0.0]
            if dropped:
                verb = "doesn't" if len(dropped) == 1 else "don't"
                add(Finding(f"{name}: {' and '.join(dropped)} {verb} come over to the app", obj, severity="INFO"))

        placed = {_mapping(n) for n in nodes if any(o.is_linked for o in n.outputs)}
        if len(placed) > 1:
            add(Finding(
                f"{name}: its textures are placed by different Mapping nodes, but the app places "
                "them all as the base color's is; give them one Mapping node",
                obj,
            ))


def check_gif_surface(material, add, Finding):
    """A GIF plays on the base color, which an Emission shader exports as black."""
    surface = _surface(material)
    if surface is not None and surface.type == "EMISSION":
        add(Finding(
            f"material {material.name!r} glows through an Emission shader, which exports a black "
            "base color, and the app plays the GIF on the base color. Plug the image straight into "
            "the Material Output's Surface instead",
        ))


def check_influences(where, meshes, armature, add, Finding):
    """The app moves each vertex by its four strongest bones."""
    bones = set(armature.data.bones.keys())
    for obj in meshes:
        deform = {g.index for g in obj.vertex_groups if g.name in bones}
        if not deform:
            continue
        over = sum(1 for v in obj.data.vertices if sum(1 for g in v.groups if g.group in deform and g.weight > 0.0) > INFLUENCES)
        if over:
            add(Finding(
                f"{where}: {over} vertices of {obj.name} are weighted to more than {INFLUENCES} "
                f"bones; the app keeps the {INFLUENCES} strongest, so they bend a little "
                "differently than in Blender. Limit Total in Weight Paint fixes it",
                obj,
            ))


def check_color_attributes(where, objects, vertex_color, add, Finding):
    """Bevy multiplies base color by a mesh's first color attribute, and the exporter can ship
    one no material uses."""
    if vertex_color == "NONE":
        return
    for obj in objects:
        if obj.type != "MESH":
            continue
        used = set()
        for slot in obj.material_slots:
            material = slot.material
            if material is None or material.node_tree is None:
                continue
            for node in material.node_tree.nodes:
                if node.type == "VERTEX_COLOR":
                    used.add(node.layer_name)
                elif node.type == "ATTRIBUTE":
                    used.add(node.attribute_name)
        unused = [a for a in obj.data.color_attributes if a.name not in used and not _all_white(a)]
        if unused and not used:
            add(Finding(
                f"{where}: {obj.name} has painted color attributes no material uses "
                f"({_examples([a.name for a in unused])}); the exporter can ship them anyway and "
                "the app tints the mesh by them. Remove them, or set Vertex colors to None",
                obj,
            ))


def _all_white(attribute):
    values = array("f", bytes(4 * 4 * len(attribute.data)))
    attribute.data.foreach_get("color", values)
    return min(values, default=1.0) >= 0.999
