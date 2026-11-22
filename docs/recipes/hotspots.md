[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Hotspots

**On this page:** [What a hotspot is](#what-a-hotspot-is) · [Your first hotspot](#your-first-hotspot) · [The panel, top to bottom](#the-panel-top-to-bottom) · [Plays once](#plays-once) · [While held](#while-held) · [Then](#then) · [Only during](#only-during) · [Props: the ★ and their states](#props-the--and-their-states) · [How it behaves in the app](#how-it-behaves-in-the-app) · [What Check warns about](#what-check-warns-about) · [Good to know](#good-to-know)

## What a hotspot is

A hotspot makes the scene react to the viewer. It goes off on one of four things:

| When | It goes off… | Needs bones? |
|---|---|---|
| **Click** | when the viewer clicks a spot on the character or a prop | Yes |
| **Hover** | when the cursor rests on that spot for a moment | Yes |
| **Mouse keeps still** | when the mouse hasn't moved for a while, and again after as long while it stays still | No |
| **Mouse comes back** | when the mouse moves on the desktop again after a while away | No |

When it goes off, it can do any of these, for the character and for props alike:

- **Plays once:** a reaction that plays through and ends. A giggle, a flinch, a door
  rattling.
- **While held:** an animation that plays on and on for as long as the hotspot is held:
  the button pressed down, the cursor resting on it, or the mouse kept still. A purr while
  the head is stroked, a sleeping pose while nobody is at the computer.
- **Then:** an animation that takes over afterwards and stays. The character standing up
  and staying up, a lamp switched off.

And it can wait: **Only during** lists what must be playing for it to work at all, so a
reaction made for sitting doesn't fire while the character lies down.

That's all there is to it. A hotspot can use any mix of the three, and most use only one.
Some examples, all step by step, are in [Hotspot ideas](hotspot-ideas.md).

The clips themselves are made in Blender like any other: see [A reaction](reaction.md).
Each one must be in its actor's clip list in the **Animation** panel before a hotspot can
use it.

## Your first hotspot

A belly that giggles when clicked:

1. Make a `Giggle` action on the armature and add it in **Animation** as a clip, with
   **Only for hotspots** ticked. See [A reaction](reaction.md#step-by-step).
2. Open the **Hotspots** panel. The eye icon in its header should be open: it draws the
   hotspots in the 3D Viewport.
3. Select the armature, enter **Pose Mode**, and select the belly bone (`spine.002`, say).
4. In the Hotspots panel, press **+** and pick **Click on the selected bones**. A wire
   capsule appears around the bone, in the hotspot's color.
5. Double-click the hotspot's name to call it `Belly`.
6. **Fit the capsule.** In the **Where** list, drag the number next to the bone's name (its
   radius) until the capsule covers the belly. Not wider: it shouldn't catch an arm hanging
   beside it.
7. Under **Plays once**, press **+** and pick **Giggle** under *Character*.
8. **Check**, then **Export all**. Click the belly in the app.

## The panel, top to bottom

Pick a hotspot in the list and its settings show below it, in the order things happen:

| Section | What it's for |
|---|---|
| **When** | The trigger. *Mouse keeps still* and *Mouse comes back* also ask **After, seconds**: how long first. |
| **Where** | Click and hover only: the bones it sits on, each with the radius of its capsule. |
| **Plays once** | Clips that play through once when it goes off. |
| **While held down** / **While hovered** / **While the mouse keeps still** | What plays on and on while it's held. The name follows the trigger; *Mouse comes back* has none. |
| **Then** | What plays on and on afterwards. |
| **Only during** | What must be playing for it to go off. |
| **Expression**, **Cooldown**, **Layer** | The face it shows, how soon it may go off again, and whether it plays over other reactions. |

Every **+** in it opens a menu grouped by actor, the character first, then each prop, and
lists only the clips that fit that list. If what you want isn't there, it's either not in
that actor's clip list in **Animation**, or it's the wrong kind for that list (below).
Each empty list shows a grey line saying what happens without it.

Warnings about the hotspot show at the bottom of its settings.

## Plays once

The reaction. Each clip plays from its first key to its last, then fades back to what
played before.

- **One at random:** with several clips of the same actor, each time it goes off that
  actor plays one of them, never the same one twice in a row. Three giggles keep a poke
  from getting old.
- **Several actors at once:** a clip for the character and one for a prop play together,
  in step. Poke the character, and it flinches while the cat on the shelf startles.
- **From the Nth time in a row:** pick a clip in the list and set **From the time in a
  row**. At 5, it plays only from the fifth time in a row on, *instead of* the clips with
  a lower number. The list shows "from ×5" on it. That's how a character gets annoyed
  after being poked five times. *In a row* means the hotspot going off again within 3
  seconds of when it could; for *Mouse keeps still*, without the mouse moving in between.
  Wait longer, and it counts from 1 again.

Clips here should be **Once** clips. A *Timeline* clip would last the whole timeline, and a
*Hold* clip shows for a moment only. Check warns about both.

## While held

What plays on and on while the hotspot is **held**:

| When | Held means |
|---|---|
| Click | the mouse button stays pressed. The hotspot goes off as soon as it's **pressed**, not when it's let go. |
| Hover | the cursor stays on it. |
| Mouse keeps still | the mouse stays still. It lets go the moment the mouse moves. |
| Mouse comes back | — (the mouse coming back is a moment; the list isn't shown) |

Put a **Timeline** clip here for a loop (purring, swaying), or a **Hold** clip for a pose
(shy, covering its eyes). When the hotspot lets go, it fades out over a quarter of a second.

- **One per actor:** picking a second clip for the same actor replaces the first.
- **With Plays once too:** the once-only clip plays first, over the loop, and fades into it
  as it ends. A flinch that settles into a purr.
- **Plays once can stay empty:** a hotspot with only a loop under While is fine.
- **A loop has to repeat on its own length.** A Timeline clip is exported over the whole
  scene range. A 48-frame purr in a 240-frame scene plays once, then holds its last pose,
  unless its curves have a **Cycles** modifier or its action a **Manual Frame Range** of its
  own 48 frames. Its length should fit the scene's a whole number of times (48 in 240 does),
  or it jumps where the timeline starts over. Check says when it doesn't fit.
- **A loop joins mid-cycle.** A Timeline clip shows the timeline's frame, so it starts
  wherever the scene is in its loop. Even loops don't show it; a loop with a clear start
  (a wind-up into a sway) would. Hold poses aren't affected.

## Then

What each actor plays **on and on after** the hotspot is done: when it lets go, or, without
anything under While, when its once-only clips end.

- **A Timeline or Hold clip**, one per actor. For the character, one the viewer could pick
  too: not *Only for hotspots*, since it becomes the viewer's pick.
- **It takes over smoothly.** The new animation starts underneath while the reaction fades
  out on top, so the reaction's last pose fades straight into it. Make the reaction end
  close to the new animation's pose: a stand-up should end standing.
- **The character's Then is remembered.** It becomes the animation picked in the app's
  Scene tab, as if the viewer had picked it, and is still playing after a restart.
- **A prop's Then lasts until its environment is shown again,** or the app restarts. Then
  the prop is back to its ★ clip.
- **Nothing interrupts it.** Once a hotspot with a Then has gone off, no other hotspot goes
  off until it has switched. A click can't cut a stand-up short.
- **The viewer wins.** If the viewer picks another animation in the panel mid-reaction, the
  reaction stops and its Then is dropped.

Then plus Only during is how things get **states**: sitting → standing, a lamp on → off.
See [Hotspot ideas](hotspot-ideas.md).

## Only during

What has to be playing for the hotspot to work. For each actor listed, what that actor
plays on and on must be one of the clips listed for it.

- **For the character**, that's the animation the viewer picked in the panel, or that a
  Then switched it to. A *Once* clip the viewer picks plays over that one and counts as it.
- **For a prop**, that's its ★ clip, or the one a Then switched it to.
- **Empty:** it works during anything.

While its actors play something else, the hotspot isn't there at all: a click goes through
to a hotspot behind it. When the viewer switches to an animation it doesn't list, a
reaction of it still playing fades out right away.

Two hotspots can share bones, each listing different animations: a knee poke for sitting,
another for lying down, each with a reaction made for that pose.

## Props: the ★ and their states

A prop plays one clip by itself, all the time: the one with the **★** in its clip list in
the **Animation** panel. Click the ☆ on another Timeline or Hold clip to move it. With none
starred, it's the first Timeline clip in the list (else the first Hold one).

Its other clips wait for hotspots:

- **Once** clips play over the ★ clip and hand back to it: a door rattling.
- **Timeline** and **Hold** clips are **other states** of the prop: hotspots switch to them
  with **Then**, play them while held with **While**, and wait for them with **Only
  during**. A lamp's *On* and *Off*, a door's *Closed* and *Open*.

The viewer can't pick a prop's clips; only hotspots do.

A hotspot can also **sit on a prop**: select its armature's bones in Pose Mode and add them
like the character's. Even both at once: enter Pose Mode with both armatures selected. A
prop that should be clicked needs an armature, even a one-bone one. Its areas only work
while its environment is shown.

## How it behaves in the app

- **Click:** pressed and let go on the same spot without moving more than a few pixels, so
  dragging to turn the view never sets it off. With something under *While held down*, it
  goes off on the press instead.
- **Hover:** after the cursor rests on it for about a third of a second. It doesn't go off
  again until the cursor has left it and come back.
- **Mouse keeps still:** after **After** seconds without the mouse moving over the desktop,
  and again every **After** seconds while it stays still. In wallpaper mode, the mouse
  being in another app's window counts as still: the character may doze off while the
  viewer works, and wake when they come back to the desktop.
- **Mouse comes back:** the mouse moving on the desktop after keeping still, or being in
  other windows, for **After** seconds. Defaults: 60 seconds for *still*, 300 for *comes
  back*.
- **It can't go off again while its clips still play**, nor before its **Cooldown** has
  passed since it last went off.
- **Overlapping spots:** the one nearest the viewer under the cursor wins, among those of
  the same kind. A click spot and a hover spot on the same place both work.
- **The settings panel blocks them:** clicks on the panel never reach the scene behind it.
- **Desktop icons don't:** a click on an icon over a hotspot sets the hotspot off too.
  Icons usually line the left edge of the screen, so keep click spots toward the middle.
- **The viewer can turn reactions off** with **Reactions** under **Cursor** in the Scene
  tab. That stops every hotspot, still and comes-back ones included.

## What Check warns about

Warnings show in the hotspot's settings and in the **Warnings** panel, with its name.

**Where:**
- *Has no bones, so nothing sets it off* (click and hover), or an area names a bone the
  armature doesn't have.
- *Sits on a bone Deform bones only leaves out*: use a deform bone near it, or turn
  *Deform bones only* off in Export settings.
- *Sits on an armature that isn't the character's or in an exported environment.*
- *Has bones, but reacts to the mouse anywhere* (a tip, for still and comes-back ones): its
  bones are left out.

**What plays:**
- *Plays nothing*: give it a clip under Plays once, While or Then.
- *Isn't one of that actor's clips*: add it in **Animation** first.
- *Plays a Timeline clip once*: it would last the whole timeline. Make it Once, or move it
  to While.
- *Plays a Hold clip once*: its pose shows for a moment only. Move it to While to hold it.
- *Plays nothing on X the first N times in a row* (a tip): every clip of X waits for a
  later time.

**While, Then and Only during:**
- *A Once clip under While or Then*: those play on and on, so they need Timeline or Hold.
- *A clip only for hotspots under the character's Then or Only during*: the viewer can't
  pick it, so the character never plays it on and on.
- *Two While or Then animations for one actor*: only one can play.
- *Waits for a prop to play a clip that only a Then reaches, and no hotspot's Then does*:
  the prop never plays it, so the hotspot never goes off.
- *Can never go off*: none of its Only during animations can ever be playing.
- *Isn't held* (comes-back hotspots): its While animations are left out.

**Other:**
- *Shows an expression that isn't one of the character's.*
- *Sets an expression but plays nothing on the character* (a tip).
- *Two hotspots with the same name*: only the last is exported.

When you export only part of the character, a hotspot whose bones, clips, While or Then
animations aren't exported yet is left out of suite.toml, and the report says so; so is
one none of whose Only during animations is exported. *Export all* writes everything.

## Good to know

- **The add-on owns the hotspots in suite.toml** once you've made one in Blender. If you
  wrote some by hand, press **Load suite.toml** first, or exporting replaces them.
- **Areas follow bones, not meshes.** A capsule can't follow cloth or a shape key: put it
  on the bone that moves that part.
- **Big, simple spots work best.** People click quickly and roughly.
- **Hidden parts don't hide hotspots.** A spot on a jacket's bone still works when the
  viewer has taken the jacket off.
- **Colors:** each hotspot's tag in the list is the color its capsules are drawn in. The
  picked hotspot is drawn solid, its picked area thicker, the others faint, through the
  mesh, in the pose of the current frame.

---

← [Following the cursor (gaze)](gaze.md) · [All recipes](../making-a-character.md#recipes) · [A reaction](reaction.md) →
