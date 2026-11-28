[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An environment

**On this page:** [Adding one](#adding-one) · [Lights](#lights) · [Props that move](#props-that-move) · [A screen that plays a GIF](#a-screen-that-plays-a-gif) · [Sky and fog](#sky-and-fog)

An environment is a collection: the place, its props, and **its lights**. It's the only
light the character gets, so an environment without lights shows a black character.
Cameras in the collection are left out; they go under [Cameras](camera.md).

## Adding one

1. Put the place, its props and its lights in one collection. Collections inside it count.
2. Click the collection in the Outliner, then press **+** under **Environments**. It takes
   the collection and its name.
3. Pick its **Sky**: a World. Empty: the scene's World. See [A sky](sky.md).
4. Its music and background sound go under **Sound**, inside it. See [Sounds](sounds.md).

Like skins, each environment has a checkbox (untick it to leave it out of *Export all*) and
its own export button. The viewer switches environments in the app's Character tab.

## Lights

Point, spot and sun lights come over. **Area lights don't**; replace them with
spots or points.

- A point or spot light's *Radius* comes over as the size of its bulb: its reflections
  on shiny surfaces widen and soften as in Blender. Its shadows stay sharp.
- A light's keyed *Strength* and *Color* come over: a flickering candle, a TV's glow. Key
  them on the light itself (*Object Data Properties*), not through its nodes. F-Curve
  modifiers count, so a *Noise* modifier on *Strength* makes a flicker. The lamp then shows
  up in the Animation panel like any moving prop, and plays on the timeline (below). The
  light reaches as far as its brightest moment.
- A lamp that both moves and flickers has two actions: one on the object, one on its light
  data. Add the object's as the clip, and pick the light's in the clip's **Also plays**.
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

## Props that move

Anything in the environment that moves (a fan, a door, a curtain with shape keys, a rigged
cat, a flickering lamp) is an **actor**, like the character. The add-on finds them by
itself: every armature, and every object with an action on it, its shape keys or its light.
They're listed in the **Animation** panel under the environment's name.

**Without doing anything,** a prop plays its active action **on the timeline**: in step with
the character and everything else, looping with the scene's frame range. The panel says
"Plays its active action … on the timeline". For most props, that's all.

**For more than that,** press **Edit as a list** (or **Add**). The prop then gets a list of
clips like the character's, each with its **Plays** setting:

| Plays | On a prop |
|---|---|
| **Timeline** | A loop. The ★ one plays all the time; the others are other states hotspots switch to. |
| **Hold** | A still pose. The ★ one is held all the time; the others are states hotspots switch to. |
| **Once** | Doesn't play by itself. A [hotspot](hotspots.md) plays it over the ★ clip, which comes back after: a door that rattles when clicked. |

**The ★** marks the clip the prop plays by itself. Click the ☆ on a Timeline or Hold clip
to move it there. With none starred, it's the first Timeline clip in the list, else the
first Hold one.

**Its other Timeline and Hold clips are states.** Hotspots switch the prop to them with
**Then**, play them while held with **While**, and wait for them with **Only during**: a
lamp's *On* and *Off*, a door's *Closed* and *Open*. A prop switched by a hotspot goes back
to its ★ clip when its environment is shown again, or the app restarts. See
[Hotspot ideas](hotspot-ideas.md) for a lamp and a door, step by step.

The viewer can't pick a prop's clips in the app; only the timeline and hotspots play them.
Each prop's clips are exported into `environment/<name>/clips/<prop>/`, and its export
button in the Animation panel exports just those.

**In step with the character:** a door the character pushes open, keyed over frames 80–120
of the same timeline as the character's idle, opens exactly when the hand reaches it.
Both play the same frame of the same loop, as in Blender.

- **A short loop on a long timeline** (a fan keyed over 24 frames, in a 240-frame scene)
  stops at its last key unless it repeats. Give its curves a **Cycles** modifier, or set
  the action's **Manual Frame Range** to its 24 frames. Either way its length must fit the
  timeline a whole number of times (24 in 240 does, 25 doesn't), or it jumps where the
  timeline starts over. Check says when it doesn't fit.
- NLA tracks of environment objects aren't exported; only actions are.
- Only objects, bones, shape keys and a light's *Strength* and *Color* move. Animation of
  material or camera settings, and of other light settings, isn't exported.
- Motion that comes only from constraints or drivers needs baking into an action
  (*Object → Animation → Bake Action*).
- A rigged prop works: put its armature and meshes in the collection. Its armature can also
  hold hotspots.
- Give each prop a unique name: the clip folder is named after it, and two props whose
  names make the same folder name overwrite each other (Check warns).
- For repeated props use linked duplicates (*Alt+D*) or collection instances (*Add →
  Collection Instance*). Objects sharing a mesh share it in the file, and the app keeps it
  in memory once.

## A screen that plays a GIF

Open the GIF in an Image Texture node (Blender opens it as
a movie) and plug it into the material. The add-on copies it to
`environment/<name>/animated/<material>.gif`, and the app plays it on that material's
base color at the speed saved in the GIF.

- Only GIFs play. Videos and image sequences show as a still.
- One GIF per material, and the material's name must work as a file name.
- For a screen that glows on its own and ignores the lights, plug the image's *Color*
  straight into the Material Output's *Surface*. Not through an Emission shader: that
  exports a black base color, and the GIF plays on the base color. See
  [Glow](materials.md#glow).
- A material a GIF plays on is always blended, never dithered, when it's see-through.
- Materials of the environment follow the same rules as the character's: see
  [Materials and textures](materials.md).

## Sky and fog

An environment's sky and its fog have pages of their own: [A sky](sky.md) and
[Fog and light shafts](fog.md).

## Wind

A Wind force field in the environment's collection blows the character's hair and clothes
while the environment is shown. See [Wind](physics.md#wind).

---

← [Hair, clothes and tails that swing](physics.md) · [All recipes](../making-a-character.md#recipes) · [A sky](sky.md) →
