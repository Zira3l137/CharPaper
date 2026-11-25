[CharPaper](../README.md) › [Docs](README.md)

# Making a character

This part is for 3D artists. You need Blender 4.2 or newer and know your way around it:
collections, armatures, actions, shape keys. You don't need to know anything about game
engines or code.

**On this page:** [How a character is put together](#how-a-character-is-put-together) · [How things move: one timeline](#how-things-move-one-timeline) · [Before you start: rules of thumb](#before-you-start-rules-of-thumb) · [The add-on](#the-add-on) · [Your first character, step by step](#your-first-character-step-by-step) · [Testing your character](#testing-your-character) · [Recipes](#recipes) · [Reference](#reference)

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
    ├── clips/
    │   ├── Idle.json         one clip of the character: its timing and shape keys
    │   └── Idle.glb          the bones it moves
    ├── expressions/
    │   └── Happy.json        one facial expression
    ├── environment/
    │   ├── room.glb          a place: its objects and its lights
    │   └── room/             extras for the environment of the same name
    │       ├── clips/Door/   clips of the room's moving objects, one folder each
    │       ├── sky.hdr       its sky and reflections, from its World
    │       ├── music/        its music, as 01 - Title.ogg, 02 - …
    │       ├── ambience.ogg  background sound that loops under the music
    │       └── animated/     GIFs that play on its materials
    ├── cameras/
    │   ├── closeup.glb       a camera
    │   └── closeup.json      its movement, if it moves
    ├── sounds/
    │   └── step_l.ogg        sounds that clips play
    └── luts/
        └── teal-orange.cube  color grades (you put these here)
```

The ideas behind it:

- **One armature, many outfits.** The character is a skeleton. A **skin** is everything
  drawn on it: body, hair, clothes, all of it. Switching skins takes off everything and
  puts on everything in the new one. So each skin needs its own copy of the body, or of
  the parts of it that show.
- **Files find each other by name.** Clips find bones by their names, and shape keys by
  theirs, on whichever skin is worn. Nothing else ties the files together, so **don't
  rename bones or shape keys** without exporting everything again.
- **Every action is a clip.** A *clip* is one action the app can play: on the character, on
  a moving object of an environment (a door, a fan, a lamp), or on a camera. Each object
  keeps its own list of clips in the add-on.
- **Folders are the settings.** A file in `skins/` is a skin, named after the file.
  `suite.toml` only holds what the folders can't say.
- **Only what's on screen is in memory.** The skins, environments and cameras you aren't
  looking at aren't loaded. Switching reads them from disk, which takes a moment, and
  that's on purpose: twenty outfits cost no more than one.
- **No light of its own.** The app adds no sun and no fill light. The character is lit
  only by what the environment brings: its lamps and its sky. No environment, or one
  without lights, gives a black character.

## How things move: one timeline

**The app plays what Blender's timeline shows.** Your scene's frame range and frame rate
(*Output Properties*) become one timeline that loops for ever, and everything plays on it
at once: the character, the props, the cameras. A door keyed over frames 1–120 next to a
character keyed over frames 1–120 moves in step with it in the app, exactly as when you
press play in Blender.

Each clip says how it plays, with its **Plays** setting:

| Plays | What it does | Use it for |
|---|---|---|
| **Timeline** | Shows the timeline's frame, so it stays in step with everything else, and loops with it. | Idles, loops, everything that runs all the time. |
| **Once** | Plays from its first key to its last when started, then hands back to what played before. | A wave picked in the panel, reactions to clicks. |
| **Hold** | Holds its first frame. | Still poses. |

A clip moves **only what it keys**. A clip that keys an arm leaves the legs to whatever else
plays. Clips fade into each other over a quarter of a second, so switching never pops.

A clip that loops on its own length (with a *Cycles* modifier, or the action's *Manual
Frame Range*) should fit the timeline a whole number of times, or it restarts early when
the timeline does. The add-on's Check tells you when it doesn't, and which end frame would
fix it.

## Before you start: rules of thumb

These save the most trouble.

1. **Real-world size, in meters.** The orbit camera starts 2 m from a point 1 m above the
   ground and can't come closer than 1 m. A character 10 cm or 20 m tall will be awkward to
   look at.
2. **Standing on Z, facing −Y.** That is, facing you in Blender's front view (Numpad 1).
3. **Uniform scale on the armature and everything above it.** A scale like (1, 1, 1.2)
   skews bones in the app. Apply it.
4. **Don't rename bones, shape keys or the armature object between exports.** Names are how
   files find each other. If you rename something, export everything again.
5. **Names become file names.** Keep action, skin, environment, camera and material names
   simple. Characters that Windows doesn't allow in a file name are replaced with `_`.
6. **Don't name a camera `orbit` or a color grade `none`.** The app uses those names
   itself, and the character won't load.
7. **Give every action a clear name.** The action's name is what the panel shows, unless
   you give the clip a *Shown as* name.
8. **Make each new action from nothing.** In Blender 4.4 and newer, pressing **New** in the
   Action Editor while an action is assigned makes a **copy** of it, keys and all. Unlink
   the current action first (the **X**), then press **New**. See
   [An animation](recipes/animation.md#start-every-action-empty).
9. **Leave shape key sliders at 0 when you export a skin**, unless that's the face you want
   it to rest with. A slider no clip keys stays in the app where it was in Blender.
10. **Materials: Principled BSDF with image textures.** Anything built from procedural
    nodes has to be baked to textures first. PNG and JPEG images work; WebP doesn't.
11. **At most 256 shape keys on a mesh.**
12. **Save your .blend often.** The add-on's settings live in it.

## The add-on

**Install it** (Blender 4.2 or newer). On the CharPaper website, open **Download Blender
Add-on** and **drag Latest version into the Blender window**, as you would from Blender's
own extensions site.

**The first time, it takes two drags.** Blender only installs from sites you've said you
trust, and asks once:

1. **Drag the link into Blender.** Blender says *"The dropped extension comes from an
   unknown repository"*. That's expected: it's asking whether you trust the CharPaper site.
   Press **Add Repository...**.
2. **In the Add Repository window,** tick **Check for Updates on Startup**, so new versions
   show up by themselves, and press **OK**. If Blender asks to allow online access, allow
   it: it needs it to download the add-on.
3. **Drag the link in again.** Blender now knows the site and asks whether to install the
   add-on. Press **OK**.

From then on, one drag installs it, and new versions show up as updates under *Edit →
Preferences → Get Extensions*. Keep the add-on at the same version as your app.

**Or install it from disk**, for the experimental version or without online access: click
the link to download the zip, then in Blender *Edit → Preferences → Get Extensions → ⌄ (top
right) → Install from Disk…* and pick it. An add-on installed from disk gets no updates: to
update it, uninstall it, install the new zip, and restart Blender. To switch to the dragged
kind, uninstall the one from disk first.

**Open it.** In the 3D Viewport press **N** and pick the **CharPaper** tab. Everything you
set there is saved in the .blend, except where CharPaper is (below).

**Tell it where CharPaper is.** Once per computer: in the **Suite** panel, click the folder
icon of **CharPaper app** and pick the folder you unpacked CharPaper into, the one with
`charpaper.exe` in it. Picking `charpaper.exe` itself, its `characters` folder, or the
folder you unpacked the download into works too; the add-on finds the app from there. The
add-on then exports every character straight into the app's `characters` folder, so
there's nothing to copy. The line under it shows where exactly, with a ✓, or in red what's
wrong. It's kept with Blender's preferences, not in the .blend, so every .blend on this
computer uses it. (It's also under *Edit → Preferences → Add-ons → CharPaper Exporter*.)
If you move or update CharPaper to another folder, pick the new one.

**Or export somewhere else.** Tick **Custom folder** and pick any folder; suite.toml goes
straight into it. For a character you're sending to someone, or keeping out of the app's
list while it's unfinished. To see it in the app, copy that folder into the app's
`characters` folder.

The tab has these panels, top to bottom:

| Panel | What's in it |
|---|---|
| **Suite** | The name, where CharPaper is and where the character goes, the timeline (read from your scene), what the app starts with, and the **Check** and **Export all** buttons. |
| **Character** | The armature. Under it: **Skins** (outfits), **Expressions** (faces) and **Gaze** (following the cursor). |
| **Animation** | Everything that moves, grouped: the character first, then each environment's objects. Pick one to see and edit its clips. |
| **Environments** | The places, their sky, and under each, **Sound**: its music and ambience. |
| **Cameras** | Your own cameras, and the action each moves with. |
| **Hotspots** | What reacts to the viewer: clicks and the cursor on the character or a prop, the mouse keeping still or coming back. |
| **Physics** | Bones the app swings by itself, like hair, skirts and tails, and the body they keep off. |
| **Warnings** | What Check found, worst first. |
| **Export settings** | The glTF options. Leave them alone unless you know why you're changing one. |

**Colors tell you what needs attention.** The tags are Blender's own collection colors,
so they follow your theme:

| You see | It means |
|---|---|
| 🟥 Red text and tag | **An error.** An export step failed, or a file came out wrong. Fix it and export again. |
| 🟨 Yellow tag | **A warning.** Something will most likely look wrong in the app. |
| 🟦 Blue tag, dimmed text | **A tip.** Worth knowing, often harmless. |
| A row that turns red | Something it needs is missing: an action, a bone, a sound file. |
| 🟦 / 🟧 / 🟪 tag on a clip | Its mode: **Timeline** blue, **Once** orange, **Hold** violet. |

Good to know about the add-on:

- **Check only warns.** It never stops an export. Many warnings come with a button that
  fixes them (**Stop it**, **Unlink it**), and the arrow next to a warning selects what it's
  about. Export all runs Check first.
- **Your scene is never changed by an export.** Each file is exported from temporary
  copies, so your NLA tracks, active actions and selection stay as they were.
- **You don't have to export everything every time.** Skins, environments and cameras have
  their own export button and a checkbox to leave them out of *Export all*; an object's
  clips export from the Animation panel.
- **suite.toml is updated, not replaced.** What you typed into it by hand is kept, and the
  previous version is saved as `suite.toml.bak`. **Load suite.toml** fills the panel from
  an existing file.

The add-on's own [README](../tools/blender/charpaper_exporter/README.md) has the fine print
for every feature.

## Your first character, step by step

1. **Suite → Name:** what the app's list shows, like `Aki`. **CharPaper app:** where the
   app is, if you haven't picked it yet (see [The add-on](#the-add-on)). The character's
   folder is named after the Name, `characters\Aki`, unless you type another **Folder
   name**. After the first export the folder name stays as it is, so renaming the
   character later doesn't make a second copy next to the first.
2. **Set the timeline.** In *Output Properties*, set the frame rate and the frame range your
   idle loops over, like 1–240 at 24 fps. The Suite panel shows the result: "a 10.0 s loop".
3. **Character → Armature:** your character's armature.
4. **Character → Skins:** press **+**, name the skin (`casual`), and pick the
   **Collection** that holds its meshes.
5. **Animation:** the character is listed first. Press **Add** and pick your idle action,
   or **Add all that fit** for every action that moves this armature. Click the **☆** next
   to the idle so the character starts with it. Set each clip's **Plays**: *Timeline*,
   *Once* or *Hold*.
6. **Environments:** press **+**, name it (`room`), and pick the collection that holds the
   place and **its lights**.
7. **Suite → Starts with:** which skin, clip, environment and camera show first.
8. Press **Check**, read the Warnings, then press **Export all**.

**Look at it.** Start the app with `run-windowed.bat` and pick your character in the Scene
tab. If the app was already running during the first export, restart it: a new character
shows in the list only after a restart. After a change in Blender, export again, then
switch to another character and back in the app, or restart it.

## Testing your character

- Run `run-windowed.bat` while you work. A window is easier to check than the desktop.
- `charpaper.exe --windowed --suite Aki` opens straight into the folder `characters\Aki`.
- Read `charpaper.log` after a change. It lists the skins, clips, expressions and
  drivers it found, and warns about what it couldn't use. If the character doesn't load
  at all, the log says which line of suite.toml is wrong.
- **The app remembers what you picked last.** For every character it keeps the skin,
  animation, camera, environment, hidden parts, orbit position and picture edits you last
  had, and they win over your *Starts with* defaults. So a changed default may seem to do
  nothing. To see the character as a new viewer would, close the app and delete
  `state.toml`. For picture settings alone, **Restore defaults** is enough.
- The app reads a suite when it shows it. After exporting, switch to another character
  and back, or restart. A brand-new character folder appears in the list only after a
  restart.

## Recipes

One page for each thing a character can have. Each says what to do in Blender and what
the app makes of it.

| Recipe | What it covers |
|---|---|
| [An outfit (skin)](recipes/outfit.md) | Put meshes in a collection, and each object becomes a part the viewer can switch. |
| [An animation](recipes/animation.md) | Clips on the timeline, once or held; what a clip moves; slots and other Blender 4.4 traps. |
| [A facial expression](recipes/expression.md) | Shape key actions the viewer can pick for the face. |
| [Shape keys that follow the body](recipes/correctives.md) | Correctives: drivers, or keys inside a clip. |
| [Following the cursor (gaze)](recipes/gaze.md) | Eyes, head and neck that turn towards the mouse. |
| [Hotspots](recipes/hotspots.md) | Reactions to clicks, the cursor resting on a spot, the mouse keeping still or coming back: clips played once, loops while held, and switching what plays after. |
| [A reaction](recipes/reaction.md) | Making the clip a hotspot plays: body, face, correctives and sound. |
| [Hotspot ideas](recipes/hotspot-ideas.md) | Step by step: escalating pokes, stroking, sitting and standing, dozing off, a lamp, a door. |
| [Hair, clothes and tails that swing (physics)](recipes/physics.md) | Bone chains the app swings by itself, the body they keep off, and wind. |
| [An environment](recipes/environment.md) | The place around the character: its lights, moving props and screens. |
| [A sky](recipes/sky.md) | The panorama that lights the scene and shows behind it. |
| [A sun that matches the sky](recipes/sun.md) | A Sun light whose shadows follow the sky as it turns. |
| [Fog and light shafts](recipes/fog.md) | Fog boxes, patchy noise, and which lights show in them. |
| [A camera](recipes/camera.md) | Your own cameras, their movement and depth of field. |
| [Sounds](recipes/sounds.md) | Sounds on clips, music and background sound. |
| [A color grade (LUT)](recipes/lut.md) | `.cube` files for the picture. |
| [The picture the viewer sees first](recipes/first-picture.md) | Exposure, view transform and the rest of the look you ship. |

## Reference

- [What comes over from Blender, and what doesn't](blender-support.md)
- [suite.toml](suite-toml.md): the settings file, section by section
- [Keeping it fast](performance.md)
- [Troubleshooting for artists](troubleshooting.md)

---

[All docs](README.md)
