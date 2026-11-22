[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An animation

**On this page:** [Adding clips](#adding-clips) · [How each clip plays](#how-each-clip-plays) · [A clip moves only what it keys](#a-clip-moves-only-what-it-keys) · [Loops that don't pop](#loops-that-dont-pop) · [Start every action empty](#start-every-action-empty) · [Slots (Blender 4.4 and newer)](#slots-blender-44-and-newer) · [Rigs, controls and IK](#rigs-controls-and-ik) · [What Check warns about](#what-check-warns-about)

An animation is a Blender action on the armature. In the app it's a **clip**: something
the viewer can pick in the Scene tab, or that a [hotspot](hotspots.md) plays.

## Adding clips

1. Open the **Animation** panel. The character is listed first; click it if another
   object is shown.
2. Press **Add** and pick an action. The list only offers actions that move this
   armature. **Add all that fit** adds every one of them.
3. Click a clip to see its settings below the list:
   - **Action:** the Blender action it plays.
   - **Shown as:** the name the app shows. Empty: the action's name.
   - **Plays:** *Timeline*, *Once* or *Hold* (see below).
   - **Gaze:** whether the character may follow the cursor during it. Untick it where it
     must look somewhere on purpose: a dance, sleeping.
   - **Only for hotspots:** keeps the clip out of the app's animation list, for clips only
     [hotspots](hotspots.md) play. Ticking it sets **Plays** to *Once*, what most of them
     are; set *Timeline* for a loop a hotspot plays while held, or *Hold* for a pose.
4. Click the **☆** next to the clip the character starts with. It turns into **★**.
   A *Once* clip can't be the start: it would play once and leave nothing under it.

The line under the settings says what the clip will export: its frames, how many bones
and shape keys it moves, and its sounds. Check it whenever something looks off.

Without a starred clip, the character stands in its rest pose until the viewer picks one.

## How each clip plays

Everything plays on one loop, the scene's frame range (see
[How things move](../making-a-character.md#how-things-move-one-timeline)).

- **Timeline** (🟦 blue): shows the timeline's current frame, loops with it, and stays in
  step with every other Timeline clip and moving prop. It's exported over the **whole
  scene range**, whatever frames its keys are on. Before its first key and after its last,
  it holds those keys, unless the curves have a Cycles modifier.
- **Once** (🟧 orange): plays from **its own first key to its last**, then fades back to what
  played before. Picked in the panel, it plays once and the previous animation comes back.
  Its frame numbers don't matter: a clip keyed on frames 300–340 is a 41-frame clip.
- **Hold** (🟪 violet): holds the first frame. For still poses.

Clips fade into each other over a quarter of a second. Two different clips don't have to
start or end in the same pose.

## A clip moves only what it keys

A clip takes over exactly the bones it has keys on. Everything else keeps doing what it
did. That's usually what you want, but it has two sides:

- **Key every bone that should hold still.** A bone that no playing clip keys goes back to
  its rest pose. If your idle doesn't key the arms, they snap into the T-pose (or whatever
  your rest pose is) as soon as nothing else holds them. Key the whole body in idles.
- **Key only what should move in partial clips.** A wave that keys only the right arm lets
  the idle go on breathing underneath. A keying set that keys every bone of the rig would
  freeze the idle everywhere for the wave's length.

## Loops that don't pop

A Timeline clip goes from the last frame of the scene straight to the first. If the pose
there differs, it pops once per loop. Two ways to make a seamless loop:

- **Key the first frame again one frame after the end.** With a range of 1–240, copy the
  keys of frame 1 to frame 241. Frame 241 never shows, but Blender's curves arrive at the
  first pose smoothly.
- **Use a Cycles modifier** on the curves, and make the cycle's length divide the scene's
  range: a 60-frame cycle in a 240-frame range, not in 250.

A clip that loops on its own length, by Cycles or by the action's *Manual Frame Range*,
must fit the scene's range a whole number of times. Check says how far off it is and which
end frame would fix it.

## Start every action empty

In **Blender 4.4 and newer**, the **New** button in the Action Editor doesn't make an empty
action while another is assigned: it makes a **copy**, with all its keys and slots. A
reaction made that way still carries the idle inside it. The app may then play the idle's
keys, play the idle's face during the reaction, or make the reaction as long as the idle.

**Always do it this way:**

1. Select the armature. In the Action Editor, click the **X** next to the action's name to
   unlink it. (It isn't deleted; it's still in the file.)
2. Now press **New**. You get an empty action.

If you already made one by copying: Check warns about it, and names the action it was
copied from. Make a fresh action as above, then copy the keys you want from the old one
(select them, Ctrl+C, then Ctrl+V in the new one).

## Slots (Blender 4.4 and newer)

Since Blender 4.4, one action can animate several things at once: the armature, and the
shape keys of the body and of the skirt. Each gets a **slot** in the action. That's how a
clip keys shape keys along with bones (see
[Keys inside a clip](correctives.md#keys-inside-a-clip-for-what-a-driver-cant-do)).

An action can also hold **several slots for the armature**. Then the add-on needs to know
which one the clip plays:

- When you **Add** a clip, it remembers the slot itself if the action has only one for
  the armature, or the slot the armature is playing right then.
- Otherwise the clip's settings show a **Slot** field in red, and Check warns. Pick the
  slot there.

Simplest of all: one action per clip, made empty as above. Then there's never anything to
pick.

## Rigs, controls and IK

- **Key your controls, as you always do.** Export follows constraints, so deform bones that
  follow a keyed IK control or a *Copy Rotation* come out moving, even with *Deform bones
  only* on in Export settings.
- **Don't key the armature object itself**, only its bones. A character whose object moves
  jumps back to where it started every time the timeline loops. Check notes it.
- **Motion that only comes from drivers or constraints without any keys** on the rig needs
  baking into an action: *Pose → Animation → Bake Action*.

## What Check warns about

- An action that keys nothing this armature has, or bones it lacks (names must match
  exactly).
- A clip that only moves bones *Deform bones only* leaves out.
- A Timeline clip that pops where the timeline loops, ends early and holds still, or has
  keys past the end of the scene's range.
- A clip looping on a length that doesn't fit the timeline.
- Keyed properties the app can't play: materials, visibility, modifiers. Only bones, object
  transforms, shape keys and a lamp's Strength and Color move.
- Two slots for the armature and no Slot picked; a slot keyed far past the body's keys.
- Pose markers outside the clip's frames, and sounds that can't be found.

---

← [An outfit (skin)](outfit.md) · [All recipes](../making-a-character.md#recipes) · [A facial expression](expression.md) →
