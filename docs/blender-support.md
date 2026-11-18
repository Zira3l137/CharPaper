[CharPaper](../README.md) › [Docs](README.md)

# What comes over from Blender, and what doesn't

At a glance. The [recipes](making-a-character.md#recipes) have the details.

| Comes over | Doesn't |
|------------|---------|
| Meshes, UVs, vertex colors, armature deformation | Modifiers on a mesh that has shape keys (apply them yourself) |
| Principled BSDF with image textures; PNG and JPEG | Procedural shader nodes (bake them); WebP images |
| Shape keys, up to 256 a mesh | Draco or meshopt compressed files |
| Shape key drivers that read bones | Drivers using Python, other objects, custom properties or the frame |
| Actions on the armature | Animation of material or camera settings |
| Point, spot and sun lights, with color, strength, Radius and keyed Strength and Color | Area lights; soft shadows from a light's Radius |
| Per-light *Shadow* and per-object shadow visibility | Light probes and reflection planes |
| Principled Volume fog boxes, with Noise Texture density | Volume emission, blackbody, other textures on density |
| Camera position, lens, animation, depth of field | Animated F-Stop or Focus Distance; aperture shape |
| GIFs on materials | Videos and image sequences |
| Sounds on pose markers | A World's Light Path tricks (use **Background**) |
| The active action of each environment object | Environment NLA tracks; constraint-only motion (bake it) |

---

[All docs](README.md)
