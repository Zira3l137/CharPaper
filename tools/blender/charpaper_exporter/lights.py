import math
from collections import namedtuple

from . import anim

# EEVEE fades a point or spot light out where it drops below the scene's Light Threshold, or
# at its Custom Distance when that is ticked. glTF only carries that distance, as the light's
# range, when Custom Distance is ticked; without it the app lets every light reach 20 m, so it
# is shaded and shadowed far past where Blender stops it. Each exported light gets EEVEE's.
#
# glTF carries no keyed Strength or Color either. They are sampled once a frame and played in
# the app as a loop, as multiples of what the file holds: the light's peak Strength, and white
# when its Color is keyed. The copy that is exported is set to those and loses its keys, which
# would otherwise put back what they hold at the current frame.

# A light's keyed Strength and Color, one sample a frame. Each samples list holds every
# frame's value, keyed or not; the flags say which change.
Keyed = namedtuple("Keyed", "duration energies colors keyed_strength keyed_color")

# Few enough digits to keep suite.toml readable, more than the eye tells apart.
DIGITS = 4


def keyed(light, scene):
    """A light's keyed Strength and Color over its action's frame range, or the scene's when
    the action's keys don't span a frame; None when neither is keyed or neither changes."""
    data = light.animation_data
    action = data.action if data else None
    if action is None:
        return None
    curves = {}
    for curve in anim.fcurves(action, getattr(data, "action_slot", None)):
        if curve.mute:
            continue
        if curve.data_path == "energy":
            curves["energy"] = curve
        elif curve.data_path == "color" and curve.array_index < 3:
            curves[curve.array_index] = curve
    if not curves:
        return None

    # A clip runs from its first key to its last, which plays as the first again. The scene's
    # range plays its last frame too.
    start, end = action.frame_range
    count = round(end - start)
    if count < 1:
        start, count = scene.frame_start, scene.frame_end - scene.frame_start + 1
    if count < 1:
        return None

    def value(key, still, frame):
        return curves[key].evaluate(frame) if key in curves else still

    frames = [start + i for i in range(count)]
    energies = [value("energy", light.energy, f) for f in frames]
    colors = [[value(i, light.color[i], f) for i in range(3)] for f in frames]
    changes = lambda values: max(values) - min(values) > 1e-6
    keyed_strength = changes(energies)
    keyed_color = any(changes([c[i] for c in colors]) for i in range(3))
    if not keyed_strength and not keyed_color:
        return None
    fps = scene.render.fps / scene.render.fps_base
    return Keyed(count / fps, energies, colors, keyed_strength, keyed_color)


def bake(light, keys):
    """Sets the light copy to what its samples are multiples of, without its keys."""
    if keys is None:
        return
    if keys.keyed_strength:
        light.energy = max(max(keys.energies), 0.0)
    if keys.keyed_color:
        light.color = (1.0, 1.0, 1.0)
    light.animation_data_clear()


def animation_table(keys):
    """The [lights.<name>.animation] table for suite.toml."""
    table = {"duration": round(keys.duration, 6)}
    peak = max(keys.energies)
    if keys.keyed_strength and peak > 0.0:
        table["strength"] = [round(max(e, 0.0) / peak, DIGITS) for e in keys.energies]
    if keys.keyed_color:
        table["color"] = [[round(max(c, 0.0), DIGITS) for c in color] for color in keys.colors]
    return table


def eevee_range(light, threshold, keys=None):
    """Where EEVEE stops a point or spot light at its brightest, or None when the glTF
    exporter already writes it or there is nothing to write."""
    if light.type not in {"POINT", "SPOT"} or light.use_custom_distance or threshold <= 0.0:
        return None
    tint = [1.0, 1.0, 1.0]
    # Newer Blenders can tint a light by its Temperature as well.
    if getattr(light, "use_temperature", False):
        tint = list(light.temperature_color)
    if keys is None:
        moments = [(light.energy, light.color)]
    else:
        moments = zip(keys.energies, keys.colors)
    brightest = max(max(c * t for c, t in zip(color, tint)) * abs(energy) for energy, color in moments)
    brightest *= 2.0 ** getattr(light, "exposure", 0.0) / 100.0
    factor = max(
        light.diffuse_factor, light.specular_factor, light.transmission_factor, light.volume_factor
    )
    distance = math.sqrt(factor * brightest / threshold)
    return distance if distance > 0.0 else None


def give_range(light, threshold, keys=None):
    distance = eevee_range(light, threshold, keys)
    if distance is not None:
        light.use_custom_distance = True
        light.cutoff_distance = distance
