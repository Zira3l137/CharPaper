[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A reaction

**On this page:** [What a reaction is](#what-a-reaction-is) · [Step by step](#step-by-step) · [Face and correctives in the same clip](#face-and-correctives-in-the-same-clip) · [How long it lasts](#how-long-it-lasts) · [How it mixes with what's playing](#how-it-mixes-with-whats-playing) · [Pitfalls](#pitfalls) · [When it looks wrong](#when-it-looks-wrong)

## What a reaction is

A reaction is a clip a [hotspot](hotspots.md) plays: the character giggles when the belly is
poked, waves when the cursor rests on its hand. It's an ordinary action, made like any
other, with three differences:

- It plays **Once**: from its first key to its last, then it hands back to what played
  before.
- It plays **over** the animation the viewer picked, and takes over only what it keys.
- It's usually kept out of the app's animation list with **Only for hotspots**.

This page is about that kind, the clips under a hotspot's **Plays once**. Loops and poses
for **While held**, and animations a hotspot switches to with **Then**, are made the same
way; only their **Plays** differs (*Timeline* or *Hold*). See
[Hotspots](hotspots.md#while-held) and [Hotspot ideas](hotspot-ideas.md).

A reaction can move the body, change the face, move correctives, and play sounds, all in
one clip. In Blender 4.4 and newer, all of it can live in **one action**.

## Step by step

This example makes `Giggle`: the body shakes, the face laughs (shape keys on the body
mesh), and the skirt bounces (shape keys on the skirt mesh).

**1. Make an empty action on the armature.**

1. Select the armature and enter Pose Mode. Open the **Dope Sheet → Action Editor**.
2. If an action is shown (your idle, say), click the **X** next to its name to unlink it.
   It isn't deleted.
3. Press **New** and name the action `Giggle`.

> ⚠️ **Don't skip the X.** In Blender 4.4 and newer, *New* with an action assigned makes a
> **copy** of that action, keys and slots and all. A reaction copied from the idle carries
> the idle inside it, and in the app it plays the idle's keys, or the idle's face, or
> lasts as long as the idle. Check warns when an action was copied, but it's easiest never
> to do it.

**2. Key the body.**

- Use any frames you like. A clip that plays Once starts at its own first key, so keys on
  frames 1–40 or 500–540 make the same 40-frame reaction. Working on frames after the end
  of your scene keeps it clear of the idle in the timeline.
- **Key only the bones that react.** For a giggle, the spine, head and shoulders; the legs
  go on doing what the idle does. Key a bone and it's the reaction's for as long as the
  reaction plays.
- Start and end near the idle's pose. The app fades into the reaction and out of it over a
  quarter of a second, so it doesn't need to match exactly, but a reaction that starts in a
  wildly different pose looks like a jump.

**3. Key the face** (optional, see [Face and correctives](#face-and-correctives-in-the-same-clip)
below). Or skip this, and give the hotspot an **Expression** instead.

**4. Add sounds** (optional). In the Action Editor, turn on *Marker → Show Pose Markers*,
put a marker where the giggle should be heard, and name it after a sound in the .blend.
See [Sounds](sounds.md#sounds-an-animation-makes).

**5. Add it as a clip.**

1. In the **Animation** panel, with the character picked, press **Add** and pick `Giggle`.
2. Tick **Only for hotspots**. That keeps it out of the app's animation list, and sets
   **Plays** to *Once*. (Leave it unticked and set **Plays** to *Once* yourself if the
   viewer should also be able to pick it.)
3. If the reaction turns the head or the eyes on purpose, untick **Gaze**. Otherwise gaze
   keeps pulling the head towards the cursor during it.
4. Read the line under the settings: "Frames 500–540 · keys 12 nodes · 9 shape keys (…) ·
   1 sounds: giggle 0.4 s". The frames should be your reaction's frames, and the counts
   what you keyed. If the frames are the idle's, or the whole scene's, see
   [When it looks wrong](#when-it-looks-wrong).

**6. Give it a hotspot.** Select the belly bone in Pose Mode. In the **Hotspots** panel,
press **+** and pick **Click on the selected bones**, fit the capsule, and add `Giggle`
under **Plays once**. If the giggle only fits some of the character's animations (it was
made sitting, say), add those under **Only during these animations**. See
[Your first hotspot](hotspots.md#your-first-hotspot).

**7. Put the idle back on the armature** (optional, for your own sake). Pick it again in
the Action Editor, so the viewport shows the idle while you work. The add-on exports each
clip from its own action whatever is assigned.

**8. Check and Export all.** Then click the belly in the app.

## Face and correctives in the same clip

In **Blender 4.4 and newer**, one action holds slots for several things. The reaction's
action can have a slot for the armature, one for the body mesh's shape keys, and one for
the skirt mesh's shape keys. The app plays them all together.

1. Select the **body mesh**. In the Dope Sheet, switch to **Shape Key Editor**.
2. Unlink whatever action is shown there (the **X**). Then open the action list and pick
   **`Giggle`**, the same action the armature has. Blender adds a slot to it for this
   mesh's shape keys.
3. Key the face shape keys, **within the same frames as the body**.
4. Select the **skirt mesh**, and do the same: unlink, pick `Giggle`, key its shape keys.
5. **Unlink `Giggle` from both meshes' shape keys when you're done** (the **X** in the Shape
   Key Editor). Left assigned, it writes its values into the sliders, and the skins get
   exported with the giggle's face as their resting face. Check warns about it and
   **Stop it** unlinks it for you.

Things to know:

- **The app finds shape keys by name** on whichever skin is worn. Face keys keyed on the
  body mesh play on every skin whose body has keys by those names.
- **Keep every slot's keys inside the body's first and last key.** The reaction lasts until
  the last key of *any* slot. A face slot keyed to frame 200 makes a 40-frame giggle last
  200 frames, holding still at the end, and the hotspot can't go off again until then. The
  clip's settings show it in red: "Slot …, keyed …, sets its length".
- A shape key with a **driver** ignores keys: the driver wins. Correctives that a driver
  already moves don't need keys in the reaction.
- In **Blender 4.2 and 4.3**, an action can't mix bones and shape keys. Make a second action
  in the Shape Key Editor for the face, and pick it in the clip's **Also plays** field.

**Expression or face keys?** The hotspot's **Expression** setting shows one of the
character's expressions for as long as the reaction plays. It's quicker, and the same
`Happy` can serve several hotspots. Face keys in the clip can change during the reaction:
a wince that turns into a smile. When both key the same shape key, the hotspot's
expression wins.

## How long it lasts

| The clip is | It lasts |
|---|---|
| **Once** (or *Only for hotspots*) | From its first key to its last, of all its slots together. |
| **Once**, with the action's **Manual Frame Range** on | That range, whatever the keys. Check warns when the range runs past the keys. |
| **Timeline** | **The whole timeline.** Don't use Timeline under Plays once; Check warns. A loop goes under [While held](hotspots.md#while-held). |
| **Hold** | One frame: it's over as it starts. Check warns. A pose goes under [While held](hotspots.md#while-held). |

When it ends it fades back to what played before, over a quarter of a second: into the
hotspot's *While* loop, if it has one, or its *Then* animation. The hotspot can go off
again as soon as its clips have ended **and** its **Cooldown** has passed since it went
off, whichever is later.

A *Cycles* modifier on a reaction's curves repeats nothing: a Once clip plays once. Make it
longer by keying it longer.

## How it mixes with what's playing

- **The viewer's animation goes on underneath.** The reaction takes over only the bones
  and shape keys it keys, while it plays. A reaction that keys the arms leaves the idle's
  breathing in the chest.
- **A new reaction replaces the last** on the same actor, unless its hotspot has **Layer**
  on. Then it plays over the others, for small additions like a blush or an ear twitch.
  A layered reaction that keys the same parts as one already layered takes its place.
- **Over a While loop:** a hotspot's own once-only clips play above its loop, and fade
  into it as they end.
- **Into a Then animation:** the new animation starts underneath as the reaction fades
  out, so end the reaction close to its pose. A sit-down should end seated.
- **Gaze** lets go while a reaction with *Gaze* unticked plays, and comes back after.
- **Expressions:** a reaction's face keys, and its hotspot's Expression, win over the
  expression the viewer picked, while the reaction plays.
- **Sounds** from pose markers play when the reaction passes them, and stop when the
  viewer picks another animation.

## Pitfalls

These are the ones that actually happen:

1. **The action was made with New while another was assigned** (Blender 4.4 and newer). It
   then holds the idle's slots too. Symptoms: the body plays the idle instead of the
   reaction, or the idle's face shows during the reaction, or the reaction is as long as
   the idle. Fix: make a fresh empty action and copy the keys you want into it.
2. **Two slots for the armature in one action.** The clip's settings then show a red
   **Slot** field: pick the reaction's slot there.
3. **The clip plays Timeline.** It lasts the whole loop. Set **Plays** to *Once*.
4. **A face or skirt slot keyed further than the body.** The reaction lasts until that
   slot's last key.
5. **A Manual Frame Range left on the action** (from copying, or from an old setup). The
   reaction lasts that range. Untick *Manual Frame Range* in the Action Editor's sidebar
   (*Action* tab), or set it to the reaction's frames.
6. **The action is still assigned to a mesh's shape keys** when the skins are exported. The
   character rests with the reaction's face. *Stop it*, then export the skin again.
7. **The reaction keys the whole rig** because a keying set keys every bone. Then it also
   freezes everything it didn't mean to move, for its whole length. Delete the keys on
   bones that don't react.

## When it looks wrong

Look at the clip's line in the Animation panel first. It says what the add-on will export.

| You see in the app | Look at | Likely cause |
|---|---|---|
| The body does the idle, not the reaction | The clip's frames and node count | It was copied from the idle (pitfall 1), or plays the wrong slot (2). |
| The face of the idle shows during the reaction | The shape keys listed | Copied from the idle: the idle's face slot came along (1). |
| It plays much longer than its keys, and can't be set off again meanwhile | The frames; a red *Slot … sets its length* line | Timeline mode (3), a long slot (4), or a Manual Frame Range (5). |
| Nothing plays when clicked | The hotspot's warnings | It plays nothing, its bone isn't exported, or the click is on a desktop icon or the panel. |
| The character's resting face changed | Warnings about skins | The action stayed on the mesh's shape keys (6). |
| The rest of the body freezes during it | The node count | It keys bones it doesn't move (7). |
| The head snaps back to the cursor during it | The clip's *Gaze* | Untick *Gaze* on the clip. |

---

← [Hotspots](hotspots.md) · [All recipes](../making-a-character.md#recipes) · [Hotspot ideas](hotspot-ideas.md) →
