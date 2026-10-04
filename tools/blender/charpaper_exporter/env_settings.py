# What an environment's [environments.<name>] table takes from Blender that glTF can't
# carry: fog boxes, read from their Principled Volume, and the lights that skip the fog.
# Export owns these keys; the look's keys are left alone.

KEYS = ("fog", "no_volume_scatter")

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


def fog_table(node):
    if not is_principled(node):
        return {}
    table = {}
    for socket, key in FOG_INPUTS.items():
        value = node.inputs[socket].default_value
        table[key] = [float(v) for v in value[:3]] if key.endswith("color") else float(value)
    return table


def fog_objects(objects):
    return {obj: node for obj in objects if (node := volume_node(obj)) is not None}


def table(collection):
    objects = [o for o in collection.all_objects if o.type != "CAMERA"]
    fog = fog_objects(objects)
    lights = [o for o in objects if o.type == "LIGHT"]
    return {
        "fog": {obj.name: fog_table(node) for obj, node in sorted(fog.items(), key=lambda i: i[0].name)},
        "no_volume_scatter": sorted(o.name for o in lights if o.data.volume_factor <= 0.0),
    }
