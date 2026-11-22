[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Sounds

**On this page:** [Sounds an animation makes](#sounds-an-animation-makes) · [Music and background sound](#music-and-background-sound) · [The format](#the-format)

## Sounds an animation makes

Footsteps, a giggle: they are *pose markers* on the action.

1. **Load the sound into the .blend.** The easiest way: open a *Video Sequencer* editor,
   *Add → Sound*, and pick the file. That makes a Sound in the .blend named after the file.
   You can delete the strip afterwards, but then give the Sound a fake user (the shield
   icon in *Blender File* view of the Outliner) or it's gone when you save.
2. In the Action Editor, with the clip's action shown, turn on *Marker → Show Pose
   Markers*.
3. Put a marker on the frame where the sound should start (*Marker → Add Marker*, or **M**).
4. Rename the marker (*Marker → Rename Marker*, or **F2**) after the sound: `step_l`
   matches the Sound `step_l.wav`, with or without the extension.

The clip's line in the Animation panel lists the sounds it found and when they play. A
marker that names no Sound, or sits outside the clip's frames, plays nothing, and Check
says so.

The add-on converts each sound to `sounds/<name>.ogg`. The app plays it every time the
clip passes that frame: on every loop of a Timeline clip, once for a Once clip or a
[reaction](reaction.md). Sounds of the viewer's animation stop when another animation is
picked. Sounds work on the clips of props too: a door's creak, keyed on the door's clip.

## Music and background sound

Each environment has its own music and one ambience, set in the add-on: *Environments*,
pick the environment, then its *Sound* panel.

1. Tick **Sound**. Adding a track or an ambience ticks it for you.
2. Under *Music*, press **+** and pick the files. Several at once are added in file-name order.
   The arrows change the order; the name in the list is what the app's panel shows.
   A file in the .blend's folder or below is kept relative to the .blend, so the two can move
   together; one anywhere else is kept by its full path.
3. Under *Ambience*, pick one sound that loops underneath: rain, wind, a room's hum. It
   loops for as long as the environment shows, so its end should flow into its start.
4. Export all, or **Export sound** to write just these.

Any format Blender plays works: the add-on converts each file to Ogg Vorbis and numbers the
tracks to keep their order, as `music/01 - Morning.ogg`, `music/02 - Rain.ogg`, and writes
`ambience.ogg`. A track that hasn't changed since the last export isn't converted again.

With *Sound* ticked, the folder mirrors the panel: export removes the `.ogg` files in
`music/` that aren't in the list, and `ambience.ogg` when no ambience is picked. Check
warns before that happens. With *Sound* off, the add-on leaves both alone, so music you put
there by hand stays: `environment/<name>/music/` plays in file-name order, and
`environment/<name>/ambience.ogg` loops underneath.

## The format

**Every sound the app plays is Ogg Vorbis** (`.ogg`). The add-on converts what it exports,
including an Opus file named `.ogg`. Files you put in the suite by hand must already be Ogg
Vorbis: MP3 and WAV don't play, and neither does Opus. Audacity and most converters can save
Ogg Vorbis.

---

← [A camera](camera.md) · [All recipes](../making-a-character.md#recipes) · [A color grade (LUT)](lut.md) →
