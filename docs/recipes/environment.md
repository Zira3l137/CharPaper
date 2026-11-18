[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An environment

**On this page:** [Lights](#lights) · [Moving props](#moving-props) · [Props that move with the character](#props-that-move-with-the-character) · [A screen that plays a GIF](#a-screen-that-plays-a-gif) · [Sky and fog](#sky-and-fog)

An environment is a collection: the place, its props, and **its lights**. Cameras in the
collection are left out; they go under Cameras.

## Lights

Point, spot and sun lights come over. **Area lights don't**; replace them with
spots or points.

- A point or spot light's *Radius* comes over as the size of its bulb: its reflections
  on shiny surfaces widen and soften as in Blender. Its shadows stay sharp.
- A light's keyed *Strength* and *Color* come over and loop: a flickering candle, a TV's
  glow. Key them on the light itself (*Object Data Properties*), not through its nodes.
  F-Curve modifiers count, so a *Noise* modifier on *Strength* makes a flicker. The loop
  runs from the action's first key to its last, or over the scene's frame range when the
  keys don't span a frame, as with one key and a Noise modifier: there, set the scene's
  range, or the action's *Manual Frame Range*, to the loop you want. Like a prop's clip, a
  loop exactly as long as the character's playing clip stays in step with it. The light
  reaches as far as its brightest moment.
- A point or spot light reaches as far as it does in EEVEE: its *Custom Distance* when
  ticked, otherwise where it fades below *Render Properties → Sampling → Advanced → Light
  Threshold*.
- **A Sun light turns to match the sky**, so its shadows fall the right way however the
  sky is turned. How, and what to do in Blender for it, is in
  [A sun that matches the sky](sun.md).
- **Shadows** follow each light's *Shadow* checkbox. Untick it on lamps whose shadows
  nobody would miss. This is the biggest thing you can do for speed: every point light
  with shadows draws the scene six more times a frame.
- An object casts shadows unless its *Object → Visibility → Ray Visibility → Shadow* is
  unticked. Untick it on small clutter.

## Moving props

Each object's *active* action becomes a clip that loops on its own, as
long as its own keys. A 25-frame fan and a 420-frame curtain both keep their length.

## Props that move with the character

Props like a chair it rocks or a door it opens play in step with it on one condition: the
prop's action is exactly as long as the character's clip. Then the app keeps the two at
the same frame, the way Blender's timeline does, and starts the prop over whenever that
clip starts over. Props of any other length loop on their own, so a fan never jumps when
the animation changes.

- **Length is counted from an action's own first key to its own last key**, not from the
  scene's frame range. Give the prop's action and the character's action keys on the same
  first and last frame, even if the prop holds still there. A prop keyed from frame 30 would
  otherwise come out 29 frames short and start early.
- Only the clip the character plays right now counts. While another clip plays, the prop
  loops on its own until the matching one comes back.

- NLA tracks of environment objects aren't exported; only the active action is.
- Animation of material or camera *settings*, and of a light's settings other than its
  *Strength* and *Color*, isn't exported. Only objects, bones, shape keys and those two
  light settings move.
- Motion that comes only from constraints or drivers needs baking into an action
  (*Object → Animation → Bake Action*).
- A rigged prop works: put its armature and meshes in the collection.
- For repeated props use linked duplicates (*Alt+D*). Objects sharing a mesh share it in
  the file, and the app keeps it in memory once.

## A screen that plays a GIF

Open the GIF in an Image Texture node (Blender opens it as
a movie) and plug it into the material. The add-on copies it to
`environment/<name>/animated/<material>.gif`, and the app plays it on that material's
base color at the speed saved in the GIF.

- Only GIFs play. Videos and image sequences show as a still.
- One GIF per material, and the material's name must work as a file name.
- For a screen that glows on its own and ignores the lights, plug the image's *Color*
  straight into the Material Output's *Surface*.

## Sky and fog

An environment's sky and its fog have pages of their own: [A sky](sky.md) and
[Fog and light shafts](fog.md).

---

← [Following the cursor (gaze)](gaze.md) · [All recipes](../making-a-character.md#recipes) · [A sky](sky.md) →
