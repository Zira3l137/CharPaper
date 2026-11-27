[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Materials and textures

**On this page:** [What comes over](#what-comes-over) · [Transparency](#transparency) · [Hair, lashes and lace](#hair-lashes-and-lace) · [Glass](#glass) · [One side or both](#one-side-or-both) · [Glow](#glow) · [Normal maps](#normal-maps) · [Textures](#textures) · [Vertex colors](#vertex-colors) · [Warnings about materials](#warnings-about-materials)

The same for the character and the environment: every material goes out as glTF, so the app
sees only what glTF can carry. Build materials from a **Principled BSDF** and **Image
Texture** nodes. Anything procedural (Noise, Voronoi, Bump, color ramps feeding colors) has
to be baked to an image first.

## What comes over

| Principled BSDF input | In the app |
|---|---|
| *Base Color*, *Metallic*, *Roughness*, *Alpha*, *Normal* (through a Normal Map node), *Emission Color* and *Strength* | Yes, values and textures. |
| *Transmission Weight*, *Coat Weight*, *Coat Roughness*, *Coat Normal*, *Specular IOR Level*, *Specular Tint*, *Anisotropic* | The slider's value only. A texture plugged in is dropped. |
| *IOR* | Only along with transmission or specular. |
| *Sheen*, *Subsurface*, *Thin Film*, *Coat Tint* | No. |

- **One Mapping node per material.** The app places every texture of a material the way
  its base color texture is placed. A normal map scaled by its own Mapping node lines up
  wrong.
- **Animating a material doesn't come over.** For a screen, use a GIF: see
  [A screen that plays a GIF](environment.md#a-screen-that-plays-a-gif).

## Transparency

Blender 4.2 dropped the material's *Blend Mode* setting, and the glTF exporter doesn't
read one anyway: it works out the kind of transparency from what feeds the **Alpha** input.

| Kind | In glTF | How to get it |
|---|---|---|
| **Solid** | Opaque | Nothing plugged into Alpha, slider at 1. |
| **Cutout**: each pixel shown or not, nothing in between | Mask | Alpha through a **Math** node set to **Round** (cut at 0.5), or to **Greater Than** with your cutoff as its second value. Or `1 − (alpha < cutoff)`: a *Less Than* node, then a *Subtract* node with 1 on top. |
| **See-through**: blended over what's behind | Blend | Anything else: a texture's Alpha plugged straight in, or the Alpha slider below 1. |

The same goes for a Mix Shader with a Transparent BSDF in its first slot, as for a
see-through screen: its *Fac* counts as the alpha.

**Use a cutout whenever you can.** It costs no more than a solid surface and behaves like
one. A see-through surface has limits in the app that it doesn't have in Blender:

- **It's sorted per object, not per face.** Of two see-through objects the farther one is
  drawn first, but the faces inside one object are drawn in whatever order they're stored.
  A mesh of overlapping see-through hair cards shows inner cards over outer ones.
- **It casts a solid shadow** wherever its alpha is above about 5%. A window at 30% casts a
  shadow as dark as a wall. Untick *Object → Visibility → Ray Visibility → Shadow* on it to
  cast none.
- **The effects that need the scene's depth don't see it.** Depth of field blurs it as much
  as whatever is behind it. Fog in front of it is the fog in front of the wall behind.
  Ink strokes skip it, and strokes of what's behind show through. Ambient occlusion
  ignores it, and with TAA anti-aliasing a moving see-through part smears.

*Transmission* (glass that bends light) has the same depth limits.

## Hair, lashes and lace

Make them cutouts:

1. Plug the image's *Color* into *Base Color*.
2. Add a **Math** node set to **Round**, plug the image's *Alpha* into it, and its result
   into the Principled BSDF's *Alpha*. For thinner or thicker strands, use **Greater Than**
   instead and set the cutoff as its second value.
3. Leave **Backface Culling** unticked for cards seen from both sides.

A cutout's edges are hard: they stair-step a little where Blender shows soft ones. **TAA**
anti-aliasing in the Graphics tab smooths them best; 4× MSAA smooths only the mesh's own
edges. The app makes smaller copies of every texture for distant views (mipmaps) and keeps a
cutout's strands as thick at a distance as close up, so hair doesn't thin out or flicker away.

## Glass

See-through is right for glass, a window or a bottle on its own. Keep each piece its own
object so they sort against each other, tick **Backface Culling** on closed shapes, and untick
the object's *Shadow* visibility unless it should cast a full shadow.

## One side or both

Blender shows both sides of a face unless *Material → Settings → Backface Culling →
Camera* is ticked, and the app follows that checkbox. A new material has it unticked, so
everything shows both sides by default.

- Tick it on **closed meshes**: bodies, heads, props. It's a little faster.
- Always tick it on a **closed see-through mesh**. Otherwise its back faces can be drawn
  over its front ones.
- Leave it unticked on **cards and single planes** seen from both sides: hair cards,
  leaves, a flag.

## Glow

- **Emission** on the Principled BSDF glows and lights nothing else: an emission of
  *Strength* 1 shows as bright as in Blender, and darkens or brightens with the exposure like
  everything else. Bright enough, it blooms.
- **Unlit**: plug the image's *Color* straight into the Material Output's *Surface* (or use a
  *Background* shader). The app shows the color as it is, ignoring the lights **and the
  exposure**. Right for a screen or a sign that should look the same in any light.
- An **Emission shader** on its own exports as a black base color with the image as its
  glow. That glows fine, but a GIF never shows on it: the app plays GIFs on the base color.
  Use the unlit setup for a GIF screen.

## Normal maps

Image Texture set to **Non-Color**, into a **Normal Map** node left on **Tangent Space**,
into *Normal*. Other spaces don't come over, and neither does a Bump node: bake it to a
normal map.

Blender and glTF both use the OpenGL convention (green up). A map made for DirectX (green
down) looks lit from the wrong side in both; invert its green channel. Keep **Tangents**
ticked in the add-on's export settings.

## Textures

- **PNG for anything with transparency.** JPEG has no alpha, so alpha read from a JPEG is
  solid everywhere. The add-on's JPEG image setting is safe: textures that carry alpha stay
  PNG.
- **No WebP.** The app can't load it, so leave the image setting on *Automatic* or *JPEG*.
- **2K is plenty** for a character seen on a desktop.
- **Mipmaps are made for you.** The app makes each texture's smaller copies when the
  character loads: about a third more video memory, and a moment of work (a 4K texture
  takes about a quarter of a second, a 2K one a fifteenth).

## Vertex colors

The app multiplies the base color by a mesh's color attribute whenever the mesh comes
with one. With *Vertex colors* on *Used by materials*, the exporter can still ship a color
attribute no material uses (seen in Blender 4.2 and 5.2), so a mask you painted for baking
tints the mesh in the app. Delete color attributes you don't need, or set *Vertex colors* to
*None* in the add-on's export settings.

## Warnings about materials

- **"…is see-through (Blend)"** on a skin: see [Transparency](#transparency). For hair,
  lashes and lace, make a cutout. For a skin's glass (glasses, a visor), it's fine: keep it a
  separate object.
- **"…shows both sides"**: a see-through material without *Backface Culling*. Tick it unless
  both sides must show. A blue tip.
- **"…takes alpha from a JPEG"**: save that image as a PNG and load the PNG.
- **"…the app drops textures on…"**: only those inputs' slider values come over. Bake the
  look into *Base Color* or *Roughness* if it matters.
- **"…Sheen / Subsurface doesn't come over"**: a blue tip; the app shows the material without.
- **"…placed by different Mapping nodes"**: give all the material's textures one Mapping
  node, or bake the placement into the images.
- **"…glows through an Emission shader"** on a GIF's material: see [Glow](#glow).
- **"…painted color attributes no material uses"**: see [Vertex colors](#vertex-colors).

---

← [An outfit (skin)](outfit.md) · [All recipes](../making-a-character.md#recipes) · [An animation](animation.md) →
