[CharPaper](../README.md) › Docs

# CharPaper docs

Everything about CharPaper, split by who it's for. Each page starts with a list of what's
on it, so you can jump straight to the part you need.

## Using CharPaper

For everyone who runs it.

- **[Using CharPaper](using.md)**: installing it, the panel, the mouse, several monitors, when it pauses, where settings live, fixing problems, and the command line.
  <br><sub>[Getting started](using.md#getting-started) · [The panel](using.md#the-panel) · [The mouse on the desktop](using.md#the-mouse-on-the-desktop) · [Several monitors](using.md#several-monitors) · [When it pauses](using.md#when-it-pauses) · [Where your settings live](using.md#where-your-settings-live) · [When something goes wrong](using.md#when-something-goes-wrong) · [Command line](using.md#command-line)</sub>

## Making a character

For 3D artists working in Blender.

- **[Making a character](making-a-character.md)**: how a character is put together, the timeline, rules of thumb, the add-on and its colors, your first one step by step, and testing it.
  <br><sub>[How a character is put together](making-a-character.md#how-a-character-is-put-together) · [How things move: one timeline](making-a-character.md#how-things-move-one-timeline) · [Before you start: rules of thumb](making-a-character.md#before-you-start-rules-of-thumb) · [The add-on](making-a-character.md#the-add-on) · [Your first character, step by step](making-a-character.md#your-first-character-step-by-step) · [Testing your character](making-a-character.md#testing-your-character) · [Recipes](making-a-character.md#recipes) · [Reference](making-a-character.md#reference)</sub>

**Recipes**, one page for each thing a character can have:

- [An outfit (skin)](recipes/outfit.md): put meshes in a collection, and each object becomes a part the viewer can switch.
- [An animation](recipes/animation.md): clips on the timeline, once or held; what a clip moves; slots and other Blender 4.4 traps.
- [A facial expression](recipes/expression.md): shape key actions the viewer can pick for the face.
- [Shape keys that follow the body](recipes/correctives.md): correctives: drivers, or keys inside a clip.
- [Following the cursor (gaze)](recipes/gaze.md): eyes, head and neck that turn towards the mouse.
- [Hotspots](recipes/hotspots.md): reactions to clicks, the cursor, the mouse keeping still or coming back: clips played once, loops while held, and switching what plays after.
- [A reaction](recipes/reaction.md): making the clip a hotspot plays: body, face, correctives and sound, and the pitfalls.
- [Hotspot ideas](recipes/hotspot-ideas.md): step by step: escalating pokes, stroking, sitting and standing, dozing off, saying hello, a lamp, a door.
- [An environment](recipes/environment.md): the place around the character: its lights, moving props and screens.
- [A sky](recipes/sky.md): the panorama that lights the scene and shows behind it.
- [A sun that matches the sky](recipes/sun.md): a Sun light whose shadows follow the sky as it turns.
- [Fog and light shafts](recipes/fog.md): fog boxes, patchy noise, and which lights show in them.
- [A camera](recipes/camera.md): your own cameras, their movement and depth of field.
- [Sounds](recipes/sounds.md): sounds on animations, music and background sound.
- [A color grade (LUT)](recipes/lut.md): `.cube` files for the picture.
- [The picture the viewer sees first](recipes/first-picture.md): exposure, view transform and the rest of the look you ship.

## Reference

- **[suite.toml](suite-toml.md)**: the settings file next to the character, section by section.
  <br><sub>[The file is strict](suite-toml.md#the-file-is-strict) · [Sections](suite-toml.md#sections) · [A hotspot](suite-toml.md#a-hotspot) · [Things worth knowing](suite-toml.md#things-worth-knowing)</sub>
- **[What comes over from Blender, and what doesn't](blender-support.md)**: one table, at a glance.
- **[Keeping it fast](performance.md)**: what costs the most in a scene, and how to measure it.
- **[Troubleshooting for artists](troubleshooting.md)**: what you see, why, and what to do, by topic.
  <br><sub>[Loading](troubleshooting.md#loading) · [Outfits, animations and expressions](troubleshooting.md#outfits-animations-and-expressions) · [Reactions and hotspots](troubleshooting.md#reactions-and-hotspots) · [Gaze](troubleshooting.md#gaze) · [Lights and exposure](troubleshooting.md#lights-and-exposure) · [The sky and the sun](troubleshooting.md#the-sky-and-the-sun) · [Fog](troubleshooting.md#fog) · [Camera](troubleshooting.md#camera) · [Sound and GIFs](troubleshooting.md#sound-and-gifs)</sub>

## For developers

- **[Translating and building](development.md)**: adding a language to the panel, building from source, and how the code is split.
  <br><sub>[Translating the panel](development.md#translating-the-panel) · [Building from source](development.md#building-from-source)</sub>
- **[The Blender add-on's README](../tools/blender/charpaper_exporter/README.md)**: the fine
  print for every export feature.
- **[RELEASING.md](../RELEASING.md)**: how versions and nightly builds are made.
