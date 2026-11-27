[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Materials and textures

**On this page:** [What comes over](#what-comes-over) · [Transparency](#transparency) · [Dithered or Blended](#dithered-or-blended) · [Hair, lashes and lace](#hair-lashes-and-lace) · [Glass](#glass) · [One side or both](#one-side-or-both) · [Glow](#glow) · [Normal maps](#normal-maps) · [Textures](#textures) · [Vertex colors](#vertex-colors) · [Warnings about materials](#warnings-about-materials)

The same for the character and the environment: every material goes out as glTF, so the app
sees only what glTF can carry. Build materials from a **Principled BSDF** and **Image
Texture** nodes. Anything procedural (Noise, Voronoi, Bump, color ramps feeding colors) has
to be baked to an image first.

## What comes over

| Principled BSDF input | In the app |
|---|---|
| *Base Color*, *Metallic*, *Roughness*, *Alpha*, *Normal* (through a Normal Map node), *Emission Color* and *Strength* | Yes, values and textures. |
| *Transmission Weight*, *Coat Weight*, *Coat Roughness*, *Coat Normal*, *Specular IOR Level*, *Specular Tint*, *Anisotropic*, *Anisotropic Rotation* | Yes, values and textures. |
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
| **See-through**: partly shows what's behind | Blend | Anything else: a texture's Alpha plugged straight in, or the Alpha slider below 1. |

The same goes for a Mix Shader with a Transparent BSDF in its first slot, as for a
see-through screen: its *Fac* counts as the alpha.

### Dithered or Blended

A see-through material is drawn the way its *Material → Settings → Render Method* says,
as EEVEE draws it:

- **Dithered**, Blender's default: solid pixels with gaps between them, more gaps the more
  see-through it is. Every pixel is in front or behind on its own, so overlapping hair cards
  and panes of glass are always in the right order; its shadow is as see-through as it is;
  and depth of field, fog and ambient occlusion treat it as the surface it is. Ink strokes
  leave it alone. With **TAA** anti-aliasing the pattern changes every frame and blends into
  smooth transparency. With **4× MSAA** each pixel is split four ways, so the pattern is four
  times finer: smooth from a little way off, faintly grainy close up. With anti-aliasing
  off it looks grainy, like a fine screen.
- **Blended**: mixed over what's behind, smooth with any anti-aliasing. But:
  - **It's sorted per object, not per face.** Of two blended objects the farther is drawn
    first, but the faces inside one object are drawn in whatever order they're stored: a
    mesh of overlapping hair cards shows inner cards over outer ones.
  - **It casts a solid shadow** wherever its alpha is above about 5%. Untick *Object →
    Visibility → Ray Visibility → Shadow* on it to cast none.
  - **The effects that need the scene's depth don't see it.** Depth of field blurs it as
    much as whatever is behind it, fog in front of it is the fog in front of the wall behind,
    strokes of what's behind show through, and with TAA a moving blended part smears.

Leave it on **Dithered** unless the grain bothers you on a surface that's alone in its
spot: a single window or a bottle. Outfit pictures (the tiles in the Character tab) draw
every see-through material blended, since a still picture can't smooth the grain.
*Transmission* (glass that bends light) always has the Blended depth limits.

## Hair, lashes and lace

Both work:

- **Dithered see-through** keeps soft strand edges, and sorts and shadows properly. Under
  TAA it looks like Blender's viewport, under 4× MSAA nearly so; with anti-aliasing off it's
  grainy.
- **A cutout** has hard edges, but no grain with any anti-aliasing:
  1. Plug the image's *Color* into *Base Color*.
  2. Add a **Math** node set to **Round**, plug the image's *Alpha* into it, and its result
     into the Principled BSDF's *Alpha*. For thinner or thicker strands, use **Greater Than**
     instead and set the cutoff as its second value.

Leave **Backface Culling** unticked for cards seen from both sides. A cutout's edges
stair-step a little; TAA smooths them best, 4× MSAA only the mesh's own edges. The app
makes smaller copies of every texture for distant views (mipmaps) and keeps a cutout's
strands as thick at a distance as close up, so hair doesn't thin out or flicker away.

## Glass

Dithered glass needs nothing special. For Blended glass, keep each piece its own object so
they sort against each other, tick **Backface Culling** on closed shapes, and untick the
object's *Shadow* visibility unless it should cast a full shadow.

## One side or both

Blender shows both sides of a face unless *Material → Settings → Backface Culling →
Camera* is ticked, and the app follows that checkbox. A new material has it unticked, so
everything shows both sides by default.

- Tick it on **closed meshes**: bodies, heads, props. It's a little faster.
- Always tick it on a **closed Blended mesh**. Otherwise its back faces can be drawn over
  its front ones. Dithered ones sort their faces themselves.
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

- **"…is see-through with Render Method Blended"** on a skin: see
  [Dithered or Blended](#dithered-or-blended). Set it back to Dithered unless you want the
  blending and its limits.
- **"…is Blended and shows both sides"**: tick *Backface Culling* unless both sides must
  show. A blue tip.
- **"…takes alpha from a JPEG"**: save that image as a PNG and load the PNG.
- **"…Sheen / Subsurface doesn't come over"**: a blue tip; the app shows the material without.
- **"…placed by different Mapping nodes"**: give all the material's textures one Mapping
  node, or bake the placement into the images.
- **"…glows through an Emission shader"** on a GIF's material: see [Glow](#glow).
- **"…painted color attributes no material uses"**: see [Vertex colors](#vertex-colors).

---

← [An outfit (skin)](outfit.md) · [All recipes](../making-a-character.md#recipes) · [An animation](animation.md) →
