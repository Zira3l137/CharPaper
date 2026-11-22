[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Hotspots

**On this page:** [What a hotspot is](#what-a-hotspot-is) · [Making one](#making-one) · [The settings](#the-settings) · [How it behaves in the app](#how-it-behaves-in-the-app) · [Hotspots on props](#hotspots-on-props) · [What Check warns about](#what-check-warns-about) · [Good to know](#good-to-know)

## What a hotspot is

A hotspot is a place on the character, or on a prop of an environment, that reacts to the
viewer: a **click** on it, or the cursor **resting** on it. When it goes off, it plays
clips (a giggle when the belly is poked, a door that swings open) and can change the face
while they play.

A hotspot is made of:

- **Where:** one or more **capsules around bones**. A capsule runs along a bone from its
  head to its tail, with a radius you set. It follows the bone as the character moves, so
  a hotspot on the hand stays on the hand while it waves.
- **What plays:** clips, of the character and of props. Each one is a clip you already
  added in the Animation panel.
- **When:** on a click or on hover, and how soon it may go off again.

Making the clips themselves is the next recipe, [A reaction](reaction.md). Make at least one
first, or the hotspot has nothing to play.

## Making one

1. Open the **Hotspots** panel. Make sure the eye icon in its header is open: that draws
   the hotspots in the viewport.
2. Select the armature, go to **Pose Mode** and select the bones the hotspot covers: say
   `spine.002` for the belly, or both hands.
3. Press **+** in the Hotspots panel. The hotspot is made with those bones as its areas, and
   drawn in the viewport as wire capsules in its color. The color tag in the list is the
   same color.
4. Rename it by double-clicking its name. The name only matters to you, and appears in
   warnings.
5. **Fit the capsules.** Each area in the **Where** list has its radius next to the bone's
   name. Drag it until the capsule covers what the viewer would click: wide enough for the
   belly, but not so wide it catches the arm hanging next to it. The selected hotspot is
   drawn solid, the selected area thicker; the others are faint. Click the bone icon next
   to an area to select that bone.
6. More bones later? Select them in Pose Mode and press **Add selected bones**.
7. **What plays:** press **+** under the clip list. The menu lists each actor (the
   character, then the props) with its clips. Pick one. Add several for variety (below).
8. Set the **Trigger**, and the rest of the settings if you need them.
9. **Check**, then **Export all**. The hotspot is written into suite.toml.

The capsules are drawn in the pose of the current frame, through the mesh. Scrub the
timeline to see them follow the bones. They're sized in the armature's units, so on a
correctly scaled character a radius of 0.05 is 5 cm.

## The settings

| Setting | What it does |
|---|---|
| **Trigger** | **Click**: a left click on it, pressed and let go without dragging. **Hover**: the cursor resting on it for about a third of a second. |
| **Layer** | Off (the default): it **stops** reactions other hotspots set off on the same actor and takes their place. On: it plays **over** them, moving only what its clips key. Use it for small things that go with anything: a blush, an ear twitch. |
| **Expression** | One of the character's [expressions](expression.md), shown while its clips play. Empty: the face stays as it is, apart from what the clips key. |
| **Cooldown** | Seconds after it goes off before it can go off again. 0: as soon as its clips end. |

**Several clips for one actor:** each time the hotspot goes off, each actor plays **one** of
its clips in the list, picked at random, and never the same one twice in a row. Three
giggles keep a poke from getting old.

**Clips of several actors:** they all play at once. A hotspot on the character's hand can
play a wave on the character and a lamp flicker on a prop together.

## How it behaves in the app

- **A click** goes off when the mouse button is pressed and let go on the same hotspot
  without moving more than a few pixels, so dragging to orbit the camera never sets it off.
- **Hover** goes off once the cursor has rested on it for about a third of a second, and
  doesn't go off again until the cursor has left it and come back.
- **It can't go off while its clips still play**, nor before its cooldown has passed since
  it last went off. Whichever comes later.
- **When hotspots overlap**, the one nearest to the viewer under the cursor wins. A click
  hotspot and a hover hotspot on the same place both work: one for clicks, one for hovering.
- **The settings panel blocks them.** Clicks on the panel never reach a hotspot behind it.
- **A reaction ends with its keys:** each clip plays from its first key to its last, then
  fades back over a quarter of a second to what played before.
- **The viewer can switch reactions off** with **Reactions** under **Cursor** in the Scene
  tab. That row only shows for characters that have hotspots.
- **On the desktop, clicks on desktop icons count.** A click on an icon that sits over a
  hotspot sets the hotspot off too. Icons usually line the left edge of the screen, so
  the character's middle is the safest place for click hotspots.

## Hotspots on props

A hotspot can sit on a bone of **an armature in one of your environments**: a door, a
chest, a pet with a rig. Select that armature's bones in Pose Mode and add them the same
way. You can even select bones of the character and of a prop together: enter Pose Mode
with both armatures selected.

- The hotspot only works while that environment is shown. In another environment its
  areas on that prop simply aren't there.
- A prop's **Once** clips don't play by themselves: they wait for a hotspot. That's how a
  door opens only when clicked. See [An environment](environment.md#props-that-move).
- Props have no expressions; a hotspot's **Expression** is always the character's.

## What Check warns about

Each warning shows in the hotspot's own box and in Warnings, tagged with the hotspot's name.

- **Has no bones, so nothing sets it off**, or an area has no bone or names a bone the
  armature doesn't have (renamed, or another armature).
- **Sits on a bone *Deform bones only* leaves out of the file**: the app wouldn't find it.
  Use a deform bone near it, or turn *Deform bones only* off in Export settings.
- **Sits on an armature that isn't the character's or in an exported environment.**
- **Plays nothing**, or plays something that isn't one of that actor's clips: add the clip
  in the Animation panel first.
- **Plays a Timeline clip:** a Timeline clip is exported over the whole timeline, so as a
  reaction it would last the whole loop and block the hotspot until then. Make it *Once*,
  or tick *Only for hotspots*.
- **Plays a Hold clip:** a single frame, so the reaction is over as it starts.
- **Shows an expression that isn't one of the character's.**
- **Sets an expression but plays nothing on the character** (a tip): the face changes only
  while the props' clips play.
- **Two hotspots with the same name:** only the last one is exported.

When you export only part of the character, a hotspot whose bones or clips aren't exported
yet is left out of suite.toml, and the export report says so. *Export all* writes them all.

## Good to know

- **The add-on owns the hotspots in suite.toml** once you've made one in Blender. If you
  wrote hotspots by hand in suite.toml, press **Load suite.toml** first, or exporting
  replaces them with the panel's.
- **Areas follow bones, not meshes.** A capsule can't follow a skirt's cloth or a shape
  key. Put the area on the bone that moves the part.
- **Big, simple capsules work best.** The viewer clicks fast and roughly. A hotspot that's
  hard to find is a hotspot nobody finds.
- **Hidden parts don't hide hotspots.** A hotspot on a jacket's bone still works when the
  viewer has taken the jacket off.

---

← [Following the cursor (gaze)](gaze.md) · [All recipes](../making-a-character.md#recipes) · [A reaction](reaction.md) →
