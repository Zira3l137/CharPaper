[CharPaper](../README.md) › [Docs](README.md)

# What comes over from Blender, and what doesn't

At a glance. The [recipes](making-a-character.md#recipes) have the details.

**Blender versions:** 4.2 to 5.2. In 4.4 and newer, one action can key bones and shape keys
together (slots); in 4.2 and 4.3, a clip's shape keys go in a second action, its
**Also plays**.

| Comes over | Doesn't |
|------------|---------|
| Meshes, UVs, vertex colors, armature deformation | Modifiers on a mesh that has shape keys (apply them yourself) |
| Principled BSDF with image textures; PNG and JPEG | Procedural shader nodes and Bump (bake them); WebP images |
| Cutouts (alpha rounded with a Math node) and see-through alpha, dithered or blended by the material's *Render Method*; Backface Culling; Transmission, Coat, Specular and Anisotropic, with their textures | Sheen, Subsurface, Thin Film |
| Emission, following the exposure; unlit materials | A Mapping node per texture (the base color's places them all) |
| Tangent-space normal maps | Object- and world-space normal maps |
| Up to 4 bones on a vertex | More (the 4 strongest are kept) |
| Sizes in meters: one Blender unit is one meter | *Unit Scale* (ignored) |
| Shape keys, up to 256 a mesh | Draco or meshopt compressed files |
| Shape key drivers that read bones or other shape keys | Drivers using Python, other objects, custom properties or the frame |
| Actions on the armature, as clips on the timeline, played once or held | Animation of material or camera settings, visibility, modifiers |
| Shape keys keyed in a clip, on any mesh of the skin | Keys on a shape key that also has a driver (the driver wins) |
| Several slots in one action (Blender 4.4+) | More than one slot for the same armature, unless you pick which |
| Point, spot and sun lights, with color, strength, Radius and keyed Strength and Color | Area lights; soft shadows from a light's Radius |
| Per-light *Shadow* and per-object shadow visibility | Light probes and reflection planes |
| Principled Volume fog boxes, with Noise Texture density | Volume emission, blackbody, other textures on density |
| Camera position, lens, animation, depth of field | Animated F-Stop or Focus Distance; aperture shape |
| GIFs on materials | Videos and image sequences |
| Sounds on pose markers | A World's Light Path tricks (use **Background**) |
| Actions of each environment object, as clips on the timeline or for hotspots | NLA tracks; constraint-only motion (bake it); a moving armature object |
| Bones as hotspot areas | Meshes as hotspot areas |
| Bone chains the app swings by itself, kept off capsules around bones or the body's mesh | Cloth, soft body, rigid body and hair particle simulations (bake them into an action, or use physics chains) |
| A Wind force field, as the environment's wind on those chains | Other force fields; wind on cloth or particles |

---

[All docs](README.md)
