<h1 align="center">CharPaper</h1>

<p align="center">
  A live 3D character for your Windows desktop, behind your icons.
</p>

<br>

<p align="center">
  <a href="https://github.com/Zira3l137/CharPaper/releases/latest">
    <img alt="Download CharPaper" height="96"
      src="https://img.shields.io/github/v/release/Zira3l137/CharPaper?style=for-the-badge&logo=windows&logoColor=white&label=Download&color=2ea44f">
  </a>
</p>
<p align="center">
  <b>Recommended.</b> The latest tested version.<br>
  <sub>On the page that opens, click <b>charpaper-…-windows-x64.zip</b> under <b>Assets</b>.</sub>
</p>

<br>

<p align="center">
  <a href="https://github.com/Zira3l137/CharPaper/releases/download/nightly/charpaper-nightly-windows-x64.zip">
    <img alt="Download the nightly build" height="72"
      src="https://img.shields.io/badge/Download-Nightly-orange?style=for-the-badge&logo=windows&logoColor=white">
  </a>
</p>
<p align="center">
  <b>For the curious.</b> Built automatically from every change, so it has the newest features<br>
  and may have new bugs. The download starts right away.
</p>

<br>

CharPaper draws a 3D scene with a character where your wallpaper is: behind the desktop
icons, in real time. You can change the character's outfit, animation and expression, look
around with the mouse, switch cameras and surroundings, and tune the picture, all from a
panel on the desktop.

Characters are made in Blender and exported with the add-on that comes with CharPaper.
If you are a 3D artist, [Making a character](#making-a-character) is written for you.

## Contents

- **Using CharPaper**
  - [Getting started](#getting-started)
  - [The panel](#the-panel)
  - [The mouse on the desktop](#the-mouse-on-the-desktop)
  - [Several monitors](#several-monitors)
  - [When it pauses](#when-it-pauses)
  - [Where your settings live](#where-your-settings-live)
  - [When something goes wrong](#when-something-goes-wrong)
- **Making a character**
  - [How a character is put together](#how-a-character-is-put-together)
  - [Before you start: rules of thumb](#before-you-start-rules-of-thumb)
  - [Your first character, step by step](#your-first-character-step-by-step)
  - [Recipes](#recipes)
  - [What comes over from Blender, and what doesn't](#what-comes-over-from-blender-and-what-doesnt)
  - [suite.toml](#suitetoml)
  - [Testing your character](#testing-your-character)
  - [Keeping it fast](#keeping-it-fast)
  - [Troubleshooting for artists](#troubleshooting-for-artists)
- **Everything else**
  - [Command line](#command-line)
  - [Translating the panel](#translating-the-panel)
  - [Building from source](#building-from-source)

# Using CharPaper

## Getting started

You need Windows 10 or 11 (64-bit) and a processor with AVX2. Most made since 2013 have
it; some budget Pentium, Celeron and Atom chips don't, and on those CharPaper won't start
at all.

1. Download one of the two above. You get a `.zip` file.
2. Right-click the file, choose **Extract All…**, then **Extract**. Extract it somewhere
   in your own folders, like Documents, not into Program Files: CharPaper saves its
   settings next to itself and can't write there.
3. Put a character into the `characters` folder. **CharPaper comes with none.** A
   character is a folder, like `characters\aki`, that you got from its author or
   [made yourself](#making-a-character).
4. Double-click **charpaper.exe**.
5. If Windows shows "Windows protected your PC", click **More info**, then **Run anyway**.
   CharPaper isn't signed, which is why Windows doesn't recognize it yet.

To quit, click **Quit** at the bottom of the panel.

To update, download and extract the new version, then move the `characters` folder and
`state.toml`, where your settings are, from the old folder into the new one.

## The panel

The panel sits at the right edge of the screen. The small arrow button on that edge hides
it and brings it back.

- **Point at a setting** and the line at the bottom of the panel says what it does.
- **Turn the mouse wheel over a value** to change it. Anywhere else on the panel, the
  wheel scrolls.
- **Click a section's title** to fold it away.

| Tab | What's there |
|-----|--------------|
| **Scene** | Which character, its outfit and the outfit's parts, animation, expression, whether it follows the cursor, camera, environment and shadows. |
| **Graphics** | Sky brightness, quality settings (render scale, anti-aliasing, depth of field, shadows, ambient occlusion, fog), ink strokes, and post processing (tonemapping, exposure, bloom, color, vignette, grain). |
| **Sound** | Volume for everything, and for music, ambience and the character's sounds each. The music track, play and pause, and playing order. |
| **System** | Wallpaper or window, start at sign-in, monitors, frame rate limit, performance monitor, when to pause, and the panel's language, colors, size and side. |

A row only shows when it has something to offer. No camera row means the character has no
extra cameras; no Gaze section means its author didn't set gaze up.

**Restore defaults**, under Post processing, takes the picture back to how the
character's author set it.

If your computer struggles, lower **Render scale**, set **Shadow quality** to Low, turn
**Fog** off, and lower the **FPS limit**. The **Performance monitor** shows what each
frame costs.

## The mouse on the desktop

With the **Orbit** camera chosen (it's the one every character has):

| Do this over the desktop | To |
|--------------------------|----|
| Hold the **right** button and drag | Turn around the character |
| Hold the **middle** button and drag | Slide the view sideways and up or down |
| Turn the **wheel** | Move closer or farther |

**Lock orbit** in the Scene tab switches all three off, so you can't nudge the view by
accident. Cameras made by the character's author can't be moved.

Your desktop icons work as always. CharPaper only listens to the mouse over the desktop
and never takes a click away from anything.

## Several monitors

By default the wallpaper is on your main monitor only. In **System → Display** every
monitor has a button. Click it to step through **Stage A**, **Stage B**, …, and **Off**.

- A **stage** is one scene: a character with its own outfit, camera, environment and
  picture settings. Monitors on the same stage show the same scene.
- Give two monitors different stages to have a different character, or the same one set
  up differently, on each.
- When monitors share a stage, **Layout** picks between **Mirror** (the whole picture on
  each) and **Span** (one wide picture across them).
- The panel always shows and changes the stage under the mouse pointer.
- You hear the stage the pointer is on. Move to a monitor with another stage and the sound
  follows after a moment.

## When it pauses

CharPaper stops drawing and playing sound, to leave the computer alone, when:

- the screen is locked or the screen saver runs (always);
- an app runs fullscreen, such as a game (**App fullscreen**, on by default);
- windows cover the whole desktop (**Desktop covered**, on by default);
- the computer runs on battery (**On battery**, off by default).

## Where your settings live

Everything is in the folder you extracted. Nothing is installed, so deleting the folder
removes CharPaper. The one thing kept outside it is **Start at sign-in**, which is an
entry in Windows' own list of startup apps: turn it off before deleting the folder, and
turn it on again after moving the folder.

| File or folder | What it is |
|----------------|------------|
| `characters\` | Your characters, one folder each. |
| `state.toml` | Every choice you made in the panel. Delete it to start fresh. |
| `charpaper.log` | What happened during the last run. `charpaper.previous.log` is the run before. |
| `locales\` | Translations of the panel. |
| `schemas\` | Reference files for people making characters. |
| `run-windowed.bat` | Starts CharPaper in an ordinary window. |

## When something goes wrong

- **The wallpaper doesn't show.** Double-click `run-windowed.bat`. CharPaper opens in an
  ordinary window and stays that way until you set **System → App → Show as** back to
  Wallpaper.
- **The screen is empty or dark blue.** There is no character in `characters`, or the one
  chosen failed to load. `charpaper.log` says which and why.
- **The character is black.** Its environment has no lights. Pick another environment, or
  tell its author.
- **A frozen picture stays on the desktop after a crash.** On Windows versions where that
  can happen, CharPaper leaves a small helper running to clean it up, which is why Task
  Manager may show a second `charpaper.exe`. If the picture stays anyway, start CharPaper
  and quit it with the Quit button.
- **Settings went back to defaults.** `state.toml` couldn't be read. CharPaper sets a
  broken one aside as `state.toml.bad` instead of deleting it.

When reporting a problem, attach `charpaper.log`.

# Making a character

This part is for 3D artists. You need Blender 4.2 or newer and know your way around it:
collections, armatures, actions, shape keys. You don't need to know anything about game
engines.

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
    │       ├── studio.hdr    its sky and reflections (you put this here)
    │       ├── music/        .ogg tracks (you put these here)
    │       ├── ambience.ogg  background sound (you put this here)
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

The add-on's own [README](tools/blender/charpaper_exporter/README.md) has the fine print
for every feature below.

## Recipes

### An outfit (skin)

Put every mesh of the outfit into one collection and add it under **Skins**. That means
the body too, or at least what shows of it.

- Meshes must be deformed by the character's armature, or parented to it or to one of its
  bones. A hat parented to the head bone works. A mesh that is neither stays where it is
  and doesn't follow the character.
- In a skin with more than one mesh object, each becomes a switch under **Outfit → Parts**
  in the app, named after the object. So a jacket as its own object can be taken off by
  the viewer. Name objects the way you'd want them listed, and join what should never be
  hidden on its own.
- **Modifiers and shape keys don't mix.** With *Apply modifiers* on, a mesh that has both
  may lose its shape keys. Apply the modifiers yourself on such meshes.

### An animation

An animation is a Blender action on the armature. Add a file under **Animations**, press
refresh, tick the actions.

- **One file, many clips.** Group them however you like. All clips of all files end up in
  one list in the app, so names must be unique across files.
- **Mode** decides how a clip plays:
  - *Loop* repeats forever.
  - *Once* plays through, then goes back to the *Starts with* animation.
  - *Pose* holds the action's first frame. Use it for still poses.
- **Shown as** gives a clip a nicer name in the app than its action has.
- Clips blend into each other over a quarter of a second, so the first and last frame of
  a loop should match, but two different clips don't have to line up.
- Without a *Starts with* animation, the character stands in its rest pose.

### A facial expression

An expression is an action that keys **only shape keys**, on meshes of a skin. It loops
while chosen, so it can be a still face (one keyframe) or a living one (blinking,
breathing).

1. Select the face mesh. In the Dope Sheet's **Shape Key Editor**, make a new action and
   key the shape keys.
2. In the add-on, select the skin, press refresh next to **Expressions**, and tick it.

- Expressions belong to the skin and travel in its file, so tick them for every skin that
  should have them. One action can serve several skins whose shape keys have the same
  names.
- **Neutral** in the app is every shape key of that mesh at 0, whatever the sliders showed
  in Blender when you exported.
- A mesh used by any expression belongs to expressions: body animations can't key shape
  keys on it. Drivers still work on it.

### Shape keys that follow the body (correctives)

Two ways, for two jobs.

**Drivers, for anything a bone can explain.** A knee fix driven by the knee's bend, a
skirt driven by the thigh. Set the driver up in Blender as usual. The add-on writes the
skin's drivers next to it and the app runs them every frame, so they follow every clip,
the blends between clips, and the head turning to the cursor. Nothing to tick.

The app runs drivers that read **bones of the character's armature** or other shape keys
of the same skin, with the types *Averaged Value*, *Sum Values*, *Minimum Value*,
*Maximum Value*, and *Scripted Expression* as long as the expression is simple math. It
can't run Python functions, read other objects or custom properties, or use the current
frame. Validation names every driver that won't come over.

**Keys inside a clip, for what a driver can't do.** A cheek puff at frames 40 to 60 of one
clip. In Blender 4.4 and newer, key the shape keys in the clip's own action: select the
mesh, assign that action in the Shape Key Editor, and key there. The clip then shows
*Keyed in this clip* in the add-on. In older Blender, make a separate shape key action and
pick it in the clip's **Shape keys** field.

If a shape key has both, the driver wins, as in Blender.

### Following the cursor (gaze)

The character's eyes, then head, then neck, turn towards the mouse pointer on top of
whatever animation plays. After the pointer has been still for a few seconds she goes back
to what the animation has her do.

1. Open **Gaze** in the add-on and set it to **Write**.
2. Add the eye bones, and pick the head and neck bones. Head and neck are optional.
3. Turn on the arrow button. It shows the rest pose. Look the character in the face in
   the viewport and click **She faces me**. The arrow turns green.
4. Optionally name shape keys that should follow the eyes, such as eyelids.

- The eye bones must be children of the head bone, and each eye bone should sit at the
  center of its eyeball. Otherwise the eye swings instead of spinning.
- Untick **Gaze** on clips where she must look somewhere on purpose: a dance, sleeping.
- A clip that animates the eye bones leaves the eyes less room to follow.
- Through a long lens the whole effect gets subtle.
- Without a gaze section the character never follows the cursor, and the app hides the
  switch.

### An environment

An environment is a collection: the place, its props, and **its lights**. Cameras in the
collection are left out; they go under Cameras.

**Lights.** Point, spot and sun lights come over. **Area lights don't**; replace them with
spots or points.

- Lights are points in the app. A light's *Radius*, which softens shadows in Blender,
  doesn't come over.
- A point or spot light reaches as far as it does in EEVEE: its *Custom Distance* when
  ticked, otherwise where it fades below *Render Properties → Sampling → Advanced → Light
  Threshold*.
- **Shadows** follow each light's *Shadow* checkbox. Untick it on lamps whose shadows
  nobody would miss. This is the biggest thing you can do for speed: every point light
  with shadows draws the scene six more times a frame.
- An object casts shadows unless its *Object → Visibility → Ray Visibility → Shadow* is
  unticked. Untick it on small clutter.

**Sky and reflections.** Blender's World doesn't come over. Instead, put the panorama you
lit the scene with, as an `.hdr` or `.exr`, into the environment's own folder:
`environment/room/studio.hdr` for `environment/room.glb`. The file name doesn't matter;
the folder name must match the environment's. Keep only one panorama in a folder.

- The first time the environment shows, the app makes three files from the panorama
  (`skybox.ktx2`, `specular.ktx2`, `diffuse.ktx2`). That takes a few seconds, and the sky
  appears when it's done.
- **Those files are never remade on their own.** After replacing the panorama, delete the
  three `.ktx2` files. Ship them with your character so nobody else has to wait.
- The sky is laid out as Blender's *Environment Texture* shows it without any rotation.
  A rotation from a Mapping node doesn't come over: rotate your scene instead, or the
  image itself.
- A folder with a panorama and no `.glb` is an environment too: just a sky, lighting the
  character on its own.
- Set how bright the sky is with **Graphics → Sky Brightness** while the environment
  shows, and write the number you like into suite.toml as `brightness`.

**Moving props.** Each object's *active* action becomes a clip that loops on its own, as
long as its own keys. A 25-frame fan and a 420-frame curtain both keep their length.

- NLA tracks of environment objects aren't exported; only the active action is.
- Animation of light, material or camera *settings* (a flickering strength, a changing
  color) isn't exported. Only objects, bones and shape keys move.
- Motion that comes only from constraints or drivers needs baking into an action
  (*Object → Animation → Bake Action*).
- A rigged prop works: put its armature and meshes in the collection.
- For repeated props use linked duplicates (*Alt+D*). Objects sharing a mesh share it in
  the file, and the app keeps it in memory once.

**A screen that plays a GIF.** Open the GIF in an Image Texture node (Blender opens it as
a movie) and plug it into the material. The add-on copies it to
`environment/<name>/animated/<material>.gif`, and the app plays it on that material's
base color at the speed saved in the GIF.

- Only GIFs play. Videos and image sequences show as a still.
- One GIF per material, and the material's name must work as a file name.
- For a screen that glows on its own and ignores the lights, plug the image's *Color*
  straight into the Material Output's *Surface*.

**Fog and light shafts.** Add a cube where the fog should be and give it a material with
a **Principled Volume** plugged into *Volume*, and nothing in *Surface*.

- The fog fills the object's bounding box, whatever its shape.
- Color, Density, Anisotropy and Absorption Color come over, as typed into the node.
  Values fed from other nodes don't, and neither do Emission and Blackbody.
- **Only lamps light fog. The sky doesn't.** Fog with no light in it is invisible.
- A sun lights fog only while its *Shadow* is ticked.
- To keep a light out of the fog, set its *Light → Influence → Volume Scatter* to 0. Any
  value above 0 counts as fully on.

### A camera

Add the camera object under **Cameras**. If it's animated, pick its **Action**; the app
loops it.

- A camera may hang from empties (a rig). They are exported with it.
- The app keeps the camera's **vertical** field of view and fits the width to the
  viewer's monitor. A wider monitor shows more at the sides. Set your render resolution to
  the monitor shape you care about most, and leave some room at the sides.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and
  either Focus Distance or a Focus Object. A focus object is followed as it moves. Focus
  on the armature with a bone picked to keep, say, the head sharp.
- Animated F-Stop or Focus Distance doesn't come over. Animate a focus empty instead.
- Aperture blades, rotation and ratio have no match in the app.
- The orbit camera is always there in addition to yours. Where it starts is set in
  [suite.toml](#suitetoml).

### Sounds

**Sounds an animation makes** (footsteps, a giggle) are *pose markers* on its action.

1. In the Action Editor, turn on *Marker → Show Pose Markers*.
2. Put a marker on the frame where the sound should start.
3. Name the marker after a sound loaded in the .blend: `step_l` matches `step_l.wav`.

The add-on converts each sound to `sounds/<name>.ogg`. The app plays it every time the
clip passes that frame, so on every loop, and stops it when another animation starts.

**Music and background sound** aren't exported. Put them in by hand:

- `environment/<name>/music/` holds the tracks. They play in file-name order, so number
  them: `01 morning.ogg`, `02 rain.ogg`.
- `environment/<name>/ambience.ogg` loops underneath: rain, wind, a room's hum.

**Every sound must be Ogg Vorbis** (`.ogg`). MP3 and WAV don't play, and neither does an
Opus file named `.ogg`. Audacity and most converters can save Ogg Vorbis.

### A color grade (LUT)

Put `.cube` files, as Resolve, Photoshop and most grading apps export them, into `luts/`.
They appear under **Graphics → Color → LUT**. To have one on from the start, name it in
suite.toml:

```toml
[post.lut]
name = "teal-orange"
strength = 0.8
```

The grade is applied to the finished picture, after everything except film grain.

### The picture the viewer sees first

Tonemapping, exposure, bloom, vignette, grain and color settings are yours to set as the
author. The easy way: tune them in the app's **Graphics** tab until the scene looks
right, then write the same numbers into the `[post]` section of suite.toml. The app never
writes to suite.toml itself; what a viewer changes is kept in their own `state.toml`, and
**Restore defaults** brings yours back.

To get close to your Blender viewport, set `tonemapping` to match your *View Transform*:
`agx` for AgX, `blender_filmic` for Filmic. Without one the app uses `tony_mc_mapface`.

A sunlit scene and a lamp-lit room need very different exposure. Give each environment
its own with `exposure` under `[environments.<name>]`.

## What comes over from Blender, and what doesn't

| Comes over | Doesn't |
|------------|---------|
| Meshes, UVs, vertex colors, armature deformation | Modifiers on a mesh that has shape keys (apply them yourself) |
| Principled BSDF with image textures; PNG and JPEG | Procedural shader nodes (bake them); WebP images |
| Shape keys, up to 256 a mesh | Draco or meshopt compressed files |
| Shape key drivers that read bones | Drivers using Python, other objects, custom properties or the frame |
| Actions on the armature | Animation of light, material or camera settings |
| Point, spot and sun lights, with color and strength | Area lights; a light's Radius |
| Per-light *Shadow* and per-object shadow visibility | Light probes and reflection planes |
| Principled Volume fog boxes | Volume emission, blackbody, textured density |
| Camera position, lens, animation, depth of field | Animated F-Stop or Focus Distance; aperture shape |
| GIFs on materials | Videos and image sequences |
| Sounds on pose markers | The Blender World (ship a `.hdr` instead) |
| The active action of each environment object | Environment NLA tracks; constraint-only motion (bake it) |

## suite.toml

`suite.toml` is a plain text file at the top of the suite. The add-on writes it, and you
can edit it in any text editor. Only one line is required:

```toml
schema = 1
```

Everything else is optional, because the folders already say most of it.

- [`schemas/suite.example.toml`](schemas/suite.example.toml) shows every section in use.
- [`schemas/suite.schema.json`](schemas/suite.schema.json) describes every key. Keep the
  `#:schema` line at the top of your file and an editor with TOML support (VS Code with
  *Even Better TOML*, for one) completes keys and explains them as you type.

**The file is strict.** A misspelt key, a default that doesn't exist, settings for an
environment or camera that isn't there, or a sound that's missing from `sounds/`: any of
these and the character doesn't load at all. `charpaper.log` says what's wrong and on
which line.

| Section | What it's for |
|---------|---------------|
| top of the file | `schema = 1`, and `name`, shown in the app. Without a name, the folder's name is used. |
| `[character]` | `default_skin`, `default_animation`, and `model` when more than one `.glb` sits next to suite.toml. |
| `[animations.<name>]` | Names a clip and says how it plays: `file`, `clip` (the action's name), `mode` (`loop`, `once`, `pose`), `gaze`, and `cues` for sounds. |
| `[environment]` | `default`: the environment shown first. |
| `[environments.<name>]` | Per environment: `brightness` of the sky, `shadows` on or off, `exposure`, fog boxes, and the lights and objects left out of fog and shadows. |
| `[post]` | The picture: `tonemapping`, `exposure`, `bloom`, `bloom_threshold`, `chromatic_aberration`, and under it `[post.vignette]`, `[post.grain]`, `[post.grading]`, `[post.lut]`. |
| `[camera]` | `default` camera, and where the orbit camera starts: `focus`, `radius`, `yaw_deg`, `pitch_deg`, and `f_stop` for depth of field. |
| `[cameras.<name>]` | An exported camera's depth of field. The add-on fills it in. |
| `[gaze]` | Which bones follow the cursor and how far they turn. The add-on fills it in. |

Things worth knowing:

- **Without a default**, the first skin and the first environment in name order are
  shown, the orbit camera is used, and no animation plays.
- **Animation files not mentioned** in suite.toml are used whole, every clip under its
  action's name, looping. Once a file is mentioned, only the clips listed for it are used.
- **The orbit camera's `focus` is in the app's axes, where Y is up.** A point at Blender's
  (x, y, z) is written `[x, z, -y]`. One meter above the origin is `[0.0, 1.0, 0.0]`.
- **The model file is optional.** Without one, the armature is taken from the default
  skin, since every skin carries a copy of it.
- **Grading needs tonemapping.** With `tonemapping = "none"` the `[post.grading]` section
  does nothing.
- **What the add-on rewrites on every export:** the name, the *Starts with* defaults,
  every exported clip, each exported camera's depth of field, and each exported
  environment's fog and shadow lists. **What it leaves alone:** `[post]`, each
  environment's `brightness`, `shadows` and `exposure`, the orbit camera, and anything
  about files you didn't export this time. An empty field in the add-on means "don't
  touch that key".

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

## Keeping it fast

A wallpaper runs all day next to everything else, so a light scene matters more than in a
render.

- **Shadows from point lights are the main cost.** Keep *Shadow* on for one or two key
  lights and untick the rest.
- **Animated props that cast shadows** make lamps redraw their shadows every frame. Untick
  shadow visibility on the ones that don't need it.
- **Every light that lights fog adds to the fog's cost.** Set *Volume Scatter* to 0 on
  lights that don't need to show in it.
- **Big GIFs take memory.** A GIF's frames are kept unpacked, up to 200 MB of them; a
  bigger one is read again on every loop instead. Keep screens small and loops short.
- **Textures:** 2K is plenty for a character seen on a desktop.
- **Ship the baked `.ktx2` sky files** so first launch isn't spent making them.
- `charpaper.exe --benchmark` measures what each graphics setting costs with your
  character and writes `benchmark.txt`.

## Troubleshooting for artists

| What you see | Why, and what to do |
|--------------|---------------------|
| The character doesn't appear in the list | Its folder has no `suite.toml`, or isn't directly inside `characters`. |
| It's in the list but nothing shows | suite.toml has an error. Read `charpaper.log`. |
| The character is black | The environment has no lights and no panorama. Add lights to its collection, or a `.hdr` to its folder. |
| The character is far too bright or dark | *glTF Settings → Lights* isn't on *Standard*, or the scene needs its own `exposure`. |
| The character stays in its rest pose, or parts of the outfit stay behind | Bone names, or the armature object's name, differ between files. Export the model, skins and animations again from the same .blend. |
| An animation is in the file but not in the list | suite.toml mentions that file, so only the clips listed for it are used. Export through the add-on, or list the clip. |
| An accessory doesn't follow the character | It's neither deformed by the armature nor parented to it or one of its bones. |
| Shape keys vanished from a mesh | It has modifiers and *Apply modifiers* is on. Apply them by hand. |
| An expression isn't listed | Its action also keys bones, or it wasn't ticked for that skin. An expression keys only shape keys. |
| A clip's shape keys do nothing on the face | That mesh is used by expressions. Use a driver, or put the keys on another mesh. |
| The eyes swing out of the head with gaze on | The eye bones don't sit at the eyeballs' centers. |
| She turns away from the cursor | Her front is set wrong. Click **She faces me** again, with the arrow on. |
| The fog doesn't show | No lamp lights it. The sky doesn't light fog, and a sun only does with *Shadow* on. |
| A light does nothing | It's an area light. Use a spot or point light. |
| The sky didn't change after replacing the `.hdr` | Delete `skybox.ktx2`, `specular.ktx2` and `diffuse.ktx2` next to it. |
| A sound or the music doesn't play | It isn't Ogg Vorbis. |
| A GIF shows as a still picture | It isn't a GIF, or its file isn't named after the material. |
| My new default in suite.toml is ignored | The app remembers your last pick. Delete `state.toml`. |
| The character stopped loading after I changed the export format | Files of the old format are still there, and a `.glb` and a `.gltf` of the same name clash. Delete the old ones. |

# Everything else

## Command line

`charpaper.exe --help` lists everything. The useful ones:

| Option | What it does |
|--------|--------------|
| `--windowed` | Runs in an ordinary window. Remembered until you switch back in the System tab. |
| `--suite <name>` | Shows the character in `characters\<name>`. |
| `--language <tag>` | Sets the panel's language, like `ru-RU`. Remembered. |
| `--bake-environments <folder>` | Makes the missing sky files for every environment of the suite in that folder, then exits. With `--dry-run` it only lists them. |
| `--benchmark` | Measures what each graphics setting costs for about two minutes and writes `benchmark.txt`. Your settings stay as they are. |
| `--no-input-forwarding` | The wallpaper stops reacting to the mouse. |
| `--log-level <level>` | How much goes into the log: `off`, `error`, `warn`, `info`, `debug`, `trace`. |

`run-windowed.bat` passes its arguments on, so `run-windowed.bat --suite aki` works too.

## Translating the panel

English is built in. To add a language:

1. Copy [`crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl`](crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl)
   to `locales\<tag>\charpaper_ui.ftl` next to `charpaper.exe`, where `<tag>` is a
   language tag such as `de-DE`.
2. Translate the text after each `=`. Leave the names before it as they are.
3. Restart CharPaper and pick the language in **System → Interface**.

A line you leave out shows in English, so a half-done translation works. Russian comes
with the app. Translations are welcome as pull requests into [`locales/`](locales).

## Building from source

You need the current stable [Rust](https://rustup.rs) and, on Windows, the MSVC build
tools that rustup offers to install.

```
cargo run -p charpaper -- --windowed
cargo build --release -p charpaper
```

- The app looks for `characters` next to the executable, so under `cargo run` that is
  `target\debug\characters`.
- A release build takes a long time. It trades build time for speed everywhere it can.
- `--features diagnostics` adds troubleshooting options such as `--inspect`, and
  `--features profile` connects the app to the Tracy profiler.

The code is split so that the Windows-specific part stays small and another system can be
added beside it:

| Crate | What it does |
|-------|--------------|
| `charpaper` | The executable: start-up, command line, saving, frame pacing. |
| `charpaper-scene` | Everything drawn: character, environment, cameras, effects. |
| `charpaper-ui` | The panel and the performance monitor. |
| `charpaper-suite` | Reads a suite folder and suite.toml. No engine inside, so it's quick to test. |
| `charpaper-bake` | Makes the sky files from a panorama. |
| `charpaper-drivers` | Runs Blender's shape key drivers. |
| `charpaper-audio` | Sound. |
| `charpaper-settings` | What `state.toml` holds. |
| `charpaper-monitor` | Measurements for the performance monitor. |
| `charpaper-wallpaper` | What any system must provide to be a wallpaper. |
| `charpaper-windows` | How Windows provides it. |

The Blender add-on is in [`tools/blender/charpaper_exporter`](tools/blender/charpaper_exporter).
[RELEASING.md](RELEASING.md) explains how versions and nightly builds are made.
