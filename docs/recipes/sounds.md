[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Sounds

**On this page:** [Sounds an animation makes](#sounds-an-animation-makes) · [Music and background sound](#music-and-background-sound) · [The format](#the-format)

## Sounds an animation makes

Footsteps, a giggle: they are *pose markers* on the action.

1. In the Action Editor, turn on *Marker → Show Pose Markers*.
2. Put a marker on the frame where the sound should start.
3. Name the marker after a sound loaded in the .blend: `step_l` matches `step_l.wav`.

The add-on converts each sound to `sounds/<name>.ogg`. The app plays it every time the
clip passes that frame, so on every loop, and stops it when another animation starts.

## Music and background sound

These aren't exported. Put them in by hand:

- `environment/<name>/music/` holds the tracks. They play in file-name order, so number
  them: `01 morning.ogg`, `02 rain.ogg`.
- `environment/<name>/ambience.ogg` loops underneath: rain, wind, a room's hum.

## The format

**Every sound must be Ogg Vorbis** (`.ogg`). MP3 and WAV don't play, and neither does an
Opus file named `.ogg`. Audacity and most converters can save Ogg Vorbis.

---

← [A camera](camera.md) · [All recipes](../making-a-character.md#recipes) · [A color grade (LUT)](lut.md) →
