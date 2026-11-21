[CharPaper](../README.md) › [Docs](README.md)

# Making a character

This part is for 3D artists. You need Blender 4.2 or newer and know your way around it:
collections, armatures, actions, shape keys. You don't need to know anything about game
engines.

**On this page:** [How a character is put together](#how-a-character-is-put-together) · [Before you start: rules of thumb](#before-you-start-rules-of-thumb) · [Your first character, step by step](#your-first-character-step-by-step) · [Testing your character](#testing-your-character) · [Recipes](#recipes) · [Reference](#reference)

## How a character is put together

What the app calls a character is a folder, called a **suite**, inside `characters`. The
Blender add-on writes all of it for you. It looks like this:

```
characters/
└── aki/
    ├── suite.toml            settings: name, what shows first, and what Blender can't export
    ├── model.glb             the armature alone
    ├── skins/
    │   ├── casual.glb        one outfit: every mesh the character wears in it
    │   ├── casual.drivers.json   that outfit's shape key drivers, if it has any
    │   └── pajamas.glb
    ├── animations/
    │   └── basics.glb        one or more animation clips
    ├── environment/
    │   ├── room.glb          a place: its objects and its lights
    │   └── room/             extras for the environment of the same name
    │       ├── sky.hdr       its sky and reflections, from its World
    │       ├── music/        its music, as 01 - Title.ogg, 02 - …
    │       ├── ambience.ogg  background sound that loops under the music
    │       └── animated/     GIFs that play on its materials
    ├── cameras/
    │   └── closeup.glb       a camera, with its animation if it has one
    ├── sounds/
    │   └── step_l.ogg        sounds that animations play
    └── luts/
        └── teal-orange.cube  color grades (you put these here)
```

The ideas behind it:

- **One armature, many outfits.** The character is a skeleton. A **skin** is everything
  drawn on it: body, hair, clothes, all of it. Switching skins takes off everything and
  puts on everything in the new one. So each skin needs its own copy of the body, or of
  the parts of it that show.
- **Files find each other by name.** Skins and animations are matched to the armature by
  bone names. Nothing else ties them together.
- **Folders are the settings.** A file in `skins/` is a skin, named after the file. A file
  in `environment/` is an environment. `suite.toml` only holds what the folders can't say.
- **Only what's on screen is in memory.** The skins, environments and cameras you aren't
  looking at aren't loaded. Switching reads them from disk, which takes a moment, and
  that's on purpose: twenty outfits cost no more than one.
- **No light of its own.** The app adds no sun and no fill light. The character is lit
  only by what the environment brings: its lamps and its sky. No environment, or one
  without lights, gives a black character.

## Before you start: rules of thumb

These save the most trouble.

1. **Real-world size, in meters.** The orbit camera starts 2 m from a point 1 m above the
   ground and can't come closer than 1 m. A character 10 cm or 20 m tall will be awkward to
   look at.
2. **Standing on Z, facing −Y.** That is, facing you in Blender's front view (Numpad 1).
3. **Uniform scale on the armature and everything above it.** A scale like (1, 1, 1.2)
   skews bones in the app. Apply it.
4. **Don't rename bones or the armature object between exports.** Names are how files find
   each other. If you rename something, export everything again.
5. **Names become file names.** Keep skin, environment, camera and material names simple.
   Characters that Windows doesn't allow in a file name are replaced with `_`.
6. **Don't name a camera `orbit` or a color grade `none`.** The app uses those names
   itself, and the character won't load.
7. **Give every action a clear name.** The action's name is what the panel shows.
8. **Materials: Principled BSDF with image textures.** Anything built from procedural
   nodes has to be baked to textures first. PNG and JPEG images work; WebP doesn't.
9. **At most 256 shape keys on a mesh.**
10. **Save your .blend before exporting.** Unless you pick another folder, the suite is
    written next to it.

## Your first character, step by step

**Install the add-on.** On the page you downloaded CharPaper from, also download
`charpaper_exporter-….zip`. In Blender: *Edit → Preferences → Get Extensions → ⌄ (top
right) → Install from Disk…* and pick the zip. Use the add-on from the same release as
your app.

**Open the panel.** In the 3D Viewport press **N** and pick the **CharPaper** tab.
Everything you set there is saved in the .blend.

**Fill it in.**

1. **Suite → Folder:** the character's folder, like `…\CharPaper\characters\aki\`.
   Exporting straight into the app's `characters` folder saves you copying.
2. **Suite → Name:** what the panel shows. **Armature:** your character's armature.
3. **Skins:** press **+**, name the skin (`casual`), and pick the **Collection** that holds
   its meshes.
4. **Animations:** press **+** and name the file (`basics`). Press the refresh button
   next to *Clips* to list your actions, and tick the ones to export. For each, pick a
   **Mode**: *Loop*, *Once*, or *Pose*.
5. **Environments:** press **+**, name it (`room`), and pick the collection that holds the
   place and **its lights**.
6. **Suite → Starts with:** which skin, animation and environment show first.
7. Press **Validate** and read what it says, then **Export All**.

**Look at it.** Start the app with `run-windowed.bat` and pick your character in the Scene
tab. After a change in Blender, export again and restart the app.

Good to know about the add-on:

- **Validation only warns.** It never stops an export. The arrow next to a warning selects
  what the warning is about.
- **Your scene is never changed.** Each file is exported from temporary copies, so your
  NLA tracks, active actions and selection stay as they were.
- **You don't have to export everything every time.** Every entry has its own export
  button, and a checkbox to leave it out of *Export All*.
- **suite.toml is updated, not replaced.** What you typed into it by hand is kept, and the
  previous version is saved as `suite.toml.bak`.
- **glTF Settings** in the panel has the export options in one place. Leave them alone
  unless you know why you're changing one. In particular, keep **Lights** on *Standard*.

The add-on's own [README](../tools/blender/charpaper_exporter/README.md) has the fine print
for every feature in the recipes.

## Testing your character

- Run `run-windowed.bat` while you work. A window is easier to check than the desktop.
- `charpaper.exe --windowed --suite aki` opens straight into the folder `characters\aki`.
- Read `charpaper.log` after a change. It lists the skins, clips, expressions and
  drivers it found, and warns about what it couldn't use.
- **The app remembers what you picked last.** For every character it keeps the skin,
  animation, camera, environment, hidden parts, orbit position and picture edits you last
  had, and they win over your suite.toml defaults. So a changed `default_skin`, orbit
  `radius` or `[post]` value may seem to do nothing. To see the character as a new
  viewer would, close the app and delete `state.toml`. For picture settings alone,
  **Restore defaults** is enough.
- The app reads a suite when it shows it. After exporting, switch to another character
  and back, or restart. A brand-new character folder appears in the list only after a
  restart.

## Recipes

One page for each thing a character can have. Each says what to do in Blender and what
the app makes of it.

| Recipe | What it covers |
|---|---|
| [An outfit (skin)](recipes/outfit.md) | Put meshes in a collection, and each object becomes a part the viewer can switch. |
| [An animation](recipes/animation.md) | Actions on the armature, and how each plays: loop, once or pose. |
| [A facial expression](recipes/expression.md) | Shape key actions that loop on the face. |
| [Shape keys that follow the body](recipes/correctives.md) | Correctives: drivers, or keys inside a clip. |
| [Following the cursor (gaze)](recipes/gaze.md) | Eyes, head and neck that turn towards the mouse. |
| [An environment](recipes/environment.md) | The place around the character: its lights, moving props and screens. |
| [A sky](recipes/sky.md) | The panorama that lights the scene and shows behind it. |
| [A sun that matches the sky](recipes/sun.md) | A Sun light whose shadows follow the sky as it turns. |
| [Fog and light shafts](recipes/fog.md) | Fog boxes, patchy noise, and which lights show in them. |
| [A camera](recipes/camera.md) | Your own cameras, their animation and depth of field. |
| [Sounds](recipes/sounds.md) | Sounds on animations, music and background sound. |
| [A color grade (LUT)](recipes/lut.md) | `.cube` files for the picture. |
| [The picture the viewer sees first](recipes/first-picture.md) | Exposure, view transform and the rest of the look you ship. |

## Reference

- [What comes over from Blender, and what doesn't](blender-support.md)
- [suite.toml](suite-toml.md): the settings file, section by section
- [Keeping it fast](performance.md)
- [Troubleshooting for artists](troubleshooting.md)

---

[All docs](README.md)
