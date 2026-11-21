import math

# EEVEE fades a point or spot light out where it drops below the scene's Light Threshold, or
# at its Custom Distance when that is ticked. glTF only carries that distance, as the light's
# range, when Custom Distance is ticked; without it the app lets every light reach 20 m, so it
# is shaded and shadowed far past where Blender stops it. Each exported light gets EEVEE's,
# at the Strength the file gives it: for a lamp its clips light, its brightest.


def eevee_range(light, threshold):
    """Where EEVEE stops a point or spot light, or None when the glTF exporter already writes
    it or there is nothing to write."""
    if light.type not in {"POINT", "SPOT"} or light.use_custom_distance or threshold <= 0.0:
        return None
    tint = [1.0, 1.0, 1.0]
    # Newer Blenders can tint a light by its Temperature as well.
    if getattr(light, "use_temperature", False):
        tint = list(light.temperature_color)
    brightest = max(c * t for c, t in zip(light.color, tint)) * abs(light.energy)
    brightest *= 2.0 ** getattr(light, "exposure", 0.0) / 100.0
    factor = max(
        light.diffuse_factor, light.specular_factor, light.transmission_factor, light.volume_factor
    )
    distance = math.sqrt(factor * brightest / threshold)
    return distance if distance > 0.0 else None


def give_range(light, threshold):
    distance = eevee_range(light, threshold)
    if distance is not None:
        light.use_custom_distance = True
        light.cutoff_distance = distance
