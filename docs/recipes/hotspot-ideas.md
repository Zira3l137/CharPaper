[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Hotspot ideas, step by step

Worked examples of what [hotspots](hotspots.md) can do. Each lists the clips to make, then
the hotspots to set up. Read [Hotspots](hotspots.md) first if a word here is new.

**On this page:** [Before you start](#before-you-start) · [Poke, and it gets annoyed](#poke-and-it-gets-annoyed) · [Stroke its head while hovering](#stroke-its-head-while-hovering) · [Hold its hand](#hold-its-hand) · [Sit down, stand up](#sit-down-stand-up) · [Dozes off when nobody's there](#dozes-off-when-nobodys-there) · [Says hello when you come back](#says-hello-when-you-come-back) · [A lamp that switches on and off](#a-lamp-that-switches-on-and-off) · [A door that opens and stays open](#a-door-that-opens-and-stays-open) · [The room goes dark while you're away](#the-room-goes-dark-while-youre-away)

## Before you start

The same few moves come up in every example:

- **Making a clip:** an empty action on the armature (unlink the current one with **X**
  first, then **New**), keyed and named. See [A reaction](reaction.md#step-by-step).
- **Adding it:** in **Animation**, pick the actor, press **Add**, pick the action, then set
  its **Plays**. Tick **Only for hotspots** on clips the viewer shouldn't pick in the app.
- **Its kind:**
  - **Once** for anything that plays through and ends: reactions, transitions.
  - **Timeline** for loops. Give a loop shorter than the scene a **Cycles** modifier (or a
    **Manual Frame Range** of its own frames) so it repeats; see
    [While held](hotspots.md#while-held).
  - **Hold** for still poses.
- **Making a hotspot:** **+** in the Hotspots panel. For a click or hover, select its bones
  in Pose Mode first.
- **Filling its lists:** **+** next to each list, then pick the clip under its actor. Below,
  `Lamp · Off` means the clip `Off` listed under `Lamp` in that menu; a clip without an
  actor's name is the character's.

## Poke, and it gets annoyed

The first pokes giggle; from the fifth in a row on, it's annoyed.

**Clips** (character, all *Once*, *Only for hotspots*): `Giggle_A`, `Giggle_B`, `Annoyed`.

**Hotspot** *Click* on the belly bone:
- **Plays once:** `Giggle_A`, `Giggle_B`, `Annoyed`. Pick `Annoyed` in the list and set
  **From the time in a row** to **5**.
- **Expression:** `Happy`, if the character has one.

The first four pokes in a row play one of the giggles at random. From the fifth on, only
`Annoyed`. Stop poking for a few seconds and it starts over.

## Stroke its head while hovering

It leans into the cursor and keeps purring while the cursor rests on its head.

**Clips** (character):
- `Lean_In` (*Once*, *Only for hotspots*): a short lean into the touch.
- `Purr` (*Timeline*, *Only for hotspots*): a loop of a few seconds, with a Cycles modifier.
  Untick **Gaze** on it, so the head doesn't turn back to the cursor.

**Hotspot** *Hover* on the head bone:
- **Plays once:** `Lean_In`.
- **While hovered:** `Purr`.
- **Expression:** `Content`.

`Lean_In` plays, then fades into `Purr`. When the cursor leaves, `Purr` fades back to
whatever played before.

## Hold its hand

It holds a pose for as long as the button is held on its hand.

**Clips** (character): `Hand_Held` (*Hold*, *Only for hotspots*): one keyframe, the
pose with its hand closed around nothing.

**Hotspot** *Click* on the hand bones:
- **While held down:** `Hand_Held`.

It goes off as the button is pressed and lets go when it's released. A quick click shows a
brief squeeze.

## Sit down, stand up

Click a standing character and it sits; click it again and it stands.

**Clips** (character):
- `Stand_Idle` and `Sit_Idle` (*Timeline*): the two idles. Not *Only for hotspots*: they're
  what the character plays on and on.
- `Sit_Down` and `Stand_Up` (*Once*, *Only for hotspots*): the transitions. Each keys the
  whole body and ends in its idle's pose: `Sit_Down` ends seated as `Sit_Idle` starts.

**Hotspot** *Sit* — *Click* on the hips bone:
- **Plays once:** `Sit_Down`.
- **Then:** `Sit_Idle`.
- **Only during:** `Stand_Idle`.

**Hotspot** *Stand* — *Click* on the same bone:
- **Plays once:** `Stand_Up`.
- **Then:** `Stand_Idle`.
- **Only during:** `Sit_Idle`.

Only one of them works at a time, so each click does the right one. The character keeps
sitting after a restart, since Then becomes the viewer's pick. The viewer can still pick
either idle in the Scene tab.

**Other hotspots that only fit one pose:** add the idle to their **Only during**: a knee
poke made sitting, only during `Sit_Idle`.

## Dozes off when nobody's there

After two minutes without the mouse moving, it lies down and sleeps until the mouse moves.

**Clips** (character):
- `Lie_Down` (*Once*, *Only for hotspots*): from standing to lying, ending in `Sleeping`'s
  pose.
- `Sleeping` (*Timeline*, *Only for hotspots*): slow breathing, looping. Untick **Gaze**.

**Hotspot** *Mouse keeps still*:
- **After, seconds:** 120.
- **Plays once:** `Lie_Down`.
- **While the mouse keeps still:** `Sleeping`.
- **Only during:** `Stand_Idle` (so it doesn't lie down from another pose).
- **Expression:** `Asleep`, for closed eyes.

When the mouse moves, `Sleeping` fades out and it's back to `Stand_Idle`. Don't put
`Sleeping` under **Then**: Then keeps playing after the mouse moves, so it would never wake.
To make it wave as it wakes, add the next example.

## Says hello when you come back

**Clips** (character): `Wave` (*Once*, *Only for hotspots*).

**Hotspot** *Mouse comes back*:
- **After, seconds:** 300.
- **Plays once:** `Wave`.

It waves when the mouse moves on the desktop after five minutes away, including after
working in other windows. With the dozing example above, set this one's **After** to the
same 120 seconds to make it wake up and wave.

## A lamp that switches on and off

**In the environment:**
- The lamp's light, called `Lamp`. Its clips key its own *Strength* (in *Object Data
  Properties*), so the light is an actor in **Animation**.
- Something to click: hotspots sit on bones, so add a one-bone armature, `Lamp_Switch`, over
  the lamp's shade, in the environment's collection.

**Clips** (on `Lamp`):
- `On` (*Timeline*): the light at full strength, or a slight flicker. Click its **☆** so
  it's the lamp's **★** clip: lit to start with.
- `Off` (*Hold*): *Strength* keyed at 0.

**Hotspot** *Switch off* — *Click* on `Lamp_Switch`'s bone:
- **Then:** `Lamp · Off`.
- **Only during:** `Lamp · On`.

**Hotspot** *Switch on* — *Click* on the same bone:
- **Then:** `Lamp · On`.
- **Only during:** `Lamp · Off`.

The lamp goes back to *On* when its environment is shown again, or the app restarts.

## A door that opens and stays open

**Clips** (the door's armature):
- `Closed` (*Hold*, **★**) and `Open` (*Hold*): the two resting states.
- `Opening` and `Closing` (*Once*): the swings, each ending in the other state's pose.

**Hotspot** *Open* — *Click* on the door's bone:
- **Plays once:** `Door · Opening`.
- **Then:** `Door · Open`.
- **Only during:** `Door · Closed`.

**Hotspot** *Close*: the same with `Closing`, `Closed` and `Open`.

The swing ends, and the door settles into `Open` without a jump: Then takes over underneath
as the swing fades out.

## The room goes dark while you're away

Hotspots can play on several actors at once. With the lamp above:

**Hotspot** *Away* — *Mouse keeps still*, **After** 300:
- **Plays once:** the character's `Yawn`.
- **Then:** `Lamp · Off`.
- **Only during:** `Lamp · On`.

**Hotspot** *Back* — *Mouse comes back*, **After** 300:
- **Plays once:** the character's `Wave`.
- **Then:** `Lamp · On`.
- **Only during:** `Lamp · Off`.

The viewer can still click the lamp in between: the two click hotspots and these two all
agree on the lamp's state through Only during.

---

← [A reaction](reaction.md) · [All recipes](../making-a-character.md#recipes) · [An environment](environment.md) →
