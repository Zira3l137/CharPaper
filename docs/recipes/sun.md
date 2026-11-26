[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A sun that matches the sky

**On this page:** [How it works](#how-it-works) · [Will my sky's sun be found?](#will-my-skys-sun-be-found) · [Clipped and unclipped](#clipped-and-unclipped) · [Setting it up, step by step](#setting-it-up-step-by-step) · [What turns the sun, and what doesn't](#what-turns-the-sun-and-what-doesnt) · [Turning it off](#turning-it-off)

An outdoor scene has two suns. One is **in the sky**: a bright spot in the picture. The
other is your **Sun light**: it lights the scene and casts the shadows. The scene looks
right only when both point the same way, so that shadows fall away from the bright spot.

In the app, anyone can turn the sky with **Look → Lighting → Sky rotation**. Left
alone, the sun in the picture would move and the shadows would stay where they were. So
CharPaper turns your Sun light too. That's **Sun follows sky**, and it's on unless you
turn it off.

## How it works

When the environment first shows, CharPaper searches the sky for its
sun: a spot far brighter than the rest. Then one of two things happens:

| | The sky's sun is found | The sky's sun isn't found |
|---|---|---|
| Your Sun light shines | from the sun in the sky, wherever you pointed it in Blender | the way you pointed it in Blender |
| When the sky turns | it keeps shining from the sun in the sky | it turns by the same angle as the sky |
| What you need to do | nothing | point it at the sky's sun in Blender, once |

When the sun is found, it's all automatic. When it isn't, CharPaper can't know where the
sun in the picture is, so it trusts your Blender scene: line the two up there, and the app
keeps them together from then on.

## Will my sky's sun be found?

Only if its brightest spot is at least 25 times brighter
than the sky's average, and isn't below the horizon.

| Your sky | Found? |
|---|---|
| A clear-sky HDRI with an unclipped sun (see below) | Yes |
| A *Sky Texture* World with *Sun Disc* on and the sun above the horizon | Yes |
| An HDRI with a clipped sun | No |
| Overcast, or the sun behind clouds | No |
| The sun has set: it's below the horizon | No |
| An indoor HDRI: windows and lamps, no sun | No |

`charpaper.log` says which it was: `the sky's sun is towards ...` when found, or `the sky
has no sun to find: ...` followed by the reason.

## Clipped and unclipped

An HDRI is a photo of the whole sky that keeps how bright
things really are, and a real sun is thousands of times brighter than the sky around it.
Many HDRIs, free ones especially, cut the sun down, or *clip* it, to a few times the
brightness of the clouds. To CharPaper, such a sun looks like one more bright cloud. To
check yours:

1. Open the HDRI in Blender's Image Editor.
2. Hold the mouse button on the sun (left or right, depending on your keymap). The bar at
   the bottom shows that pixel's R, G and B.
3. Hundreds or more: unclipped. About as bright as the brightest clouds, say 1 to 50:
   clipped.

Sites that sell or share HDRIs often say "unclipped" in the description. Poly Haven's are.

## Setting it up, step by step

1. **Add a Sun light** (*Add → Light → Sun*) to the **environment's collection**. A sun in
   the character's collection isn't turned, and point and spot lights never are. With
   several Sun lights, only the strongest turns; the others stay as they are.
2. **Tick its *Shadow*** if it should cast shadows.
3. **Make the World**: an *Environment Texture* into a *Background*, as in
   [An environment](environment.md). Leave the sky unturned in Blender, with no *Mapping*
   node or its Z rotation at 0, and turn it in the app instead. If you do turn it in
   Blender, do step 4 after.
4. **Point the Sun light at the sky's sun.** Only needed when the sun won't be found, but
   always worth doing, since Blender's own renders then match too. Either:
   - **In one click, with Sun Position.** Turn on Blender's free *Sun Position* add-on: in
     *Edit → Preferences → Get Extensions* in Blender 4.2 and later, or *Add-ons →
     Lighting* before. In *Properties → World → Sun Position*, switch to *Sun + HDRI
     Texture*, pick your Sun light and the Environment Texture node, press *Sync Sun to
     Texture* and click the sun in the 3D view. Keep the add-on's *Rotation* and the
     Mapping node's rotation at 0: the add-on doesn't see a Mapping node, and the export
     doesn't carry the add-on's Rotation.
   - **By eye.** Set *Viewport Shading* to *Rendered*, so the sky shows. Select the Sun
     light: Blender draws a line out of it that shows which way it shines. Rotate it (*R*)
     until that line runs from the sun in the sky into the scene.
5. **Export**, and open the environment in the app.
6. **Check it.** Turn **Sky rotation** in Look → Lighting. The shadows should swing
   round with the sky. `charpaper.log` says whether the sun was found.

## What turns the sun, and what doesn't

- **Sky rotation** in the app turns it. **Restore defaults** turns the sky, and the sun
  with it, back to how the suite came.
- A **Mapping node** in Blender is part of where things start. After turning the sky
  there, point the sun at it again (step 4) before exporting.
- **Typing `sky_rotation_deg` into suite.toml** by hand also changes where things start:
  a sun that isn't found stays put while the sky turns. Turn the sky in Blender or in the
  app instead.
- The sky only turns around the vertical, and so does a sun that isn't found: how high it
  stands stays as you set it in Blender. A found sun stands as high as the one in the sky.

## Turning it off

Untick **Sun follows sky** in Look → Lighting, or write
`sun_follows_sky = false` under the environment in suite.toml. The sun then stays
exactly as Blender has it, however the sky turns.

---

← [A sky](sky.md) · [All recipes](../making-a-character.md#recipes) · [Fog and light shafts](fog.md) →
