# CharPaper Exporter

A Blender add-on (4.2 and newer) that exports a whole CharPaper character suite from one
`.blend`: the model, skins, animations, environments, cameras, and `suite.toml`.

## Install

Zip this folder (the zip must contain `charpaper_exporter/` with the files inside), then in
Blender: *Edit > Preferences > Get Extensions > ⌄ > Install from Disk…* and pick the zip.
Or build the zip with Blender itself:

```
blender --command extension build --source-dir tools/blender/charpaper_exporter
```

The panel is in the 3D Viewport sidebar (N), tab **CharPaper**. Everything you set there is
saved in the `.blend`.

## What goes where

| Entry       | Holds                                                   | Written to                   |
|-------------|---------------------------------------------------------|------------------------------|
| Model file  | the armature alone                                      | `<model>.glb` next to suite.toml |
| Skin        | the armature + every mesh in its collection, plus ticked expressions | `skins/<name>.glb` |
| Animation   | the armature + ticked clips, plus corrective meshes if turned on, without materials or textures | `animations/<name>.glb` |
| Environment | everything in its collection except cameras, with active actions | `environment/<name>.glb`, its GIFs in `environment/<name>/animated/` |
| Camera      | the camera, the empties it hangs from, its focus object, and one action | `cameras/<name>.glb` |

- **Clips** and **expressions** are named after their Blender actions. *Shown as* renames a
  clip in the app only.
- **Correctives:** tick *Correctives*, pick the collection of meshes they key, then give each
  clip that needs them a shape key action. It plays together with the clip under the clip's
  name.
- **Driven shape keys:** a skin's shape keys driven by bones, like correctives a pose reader
  drives, are written next to the skin as `skins/<name>.drivers.json`, and the app runs the
  drivers itself every frame. So they follow every clip, blends between clips and gaze, with no
  keys to export. The app runs what Blender runs without Python: average, sum, min, max and
  simple-expression drivers, reading Rotational Difference variables. A driver may read a bone
  the export leaves out, such as an `MCH-reader` bone, as long as nothing between it and the
  nearest exported bone above it is animated or constrained. Validation names every driver the
  app can't run, and the app's log says whether its drivers match Blender in the rest pose.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and either
  Focus Distance or a Focus Object. A focus object goes into the camera's file with the
  empties it hangs from, and its animation joins the camera's clip. Focusing on the
  character's armature with a bone picked, or on an object of one of its skins, follows that
  bone or object instead. Animated Focus Distance or F-Stop doesn't carry over; animate a
  focus empty instead.
- **Environment animation:** each object's active action, and the active action of its
  shape keys, becomes a clip of its own, as long as the action's frame range, and the app
  loops each clip on its own. A 25-frame loop and a 420-frame one both keep their length.
  Objects playing the same action share its clip and stay in step. To leave a pause at the
  end of a loop, give the action a *Manual Frame Range* past its last key. An armature's
  bones move by its own action, so a rigged prop works the same way with the armature and its
  meshes in the collection. NLA tracks of environment objects aren't exported, nor is
  animation of light, material or camera settings. Motion that only comes from constraints
  or drivers, with no action at all, needs baking into one (*Object > Animation > Bake
  Action*).
- **Shared data:** objects sharing a mesh (*Alt+D*, or *Link/Transfer Data*) share it in the
  file too, and the app keeps it in memory once. An object with modifiers gets a mesh of its
  own when *Apply Modifiers* is on, since its result is its own. Lights sharing light data
  share it the same way. A collection instance exports as the instanced collection's objects
  under its empty, sharing their meshes.
- **Fog** is any mesh in an environment whose material has a Volume and no Surface. The
  app fills its bounding box with fog set up like its Principled Volume: Color, Density,
  Anisotropy and Absorption Color. Only lights light fog, not the sky: a light whose
  *Influence > Volume Scatter* is 0 doesn't, and a sun only does while it casts shadows.
- **Shadows** are cast by every light whose *Light > Shadow* is ticked. Untick it on lamps
  whose shadows nobody would miss: each point light with shadows draws the scene six more
  times a frame, and a few of them are usually what holds the frame rate down. A light
  without shadows still lights the scene and the fog, just evenly, with no light shafts.
  Objects cast shadows unless their *Object > Visibility > Ray Visibility > Shadow* is
  unticked. Untick it on small clutter and on animated props whose shadows nobody would miss:
  a lamp that sees an animated object draws its shadows again every frame, and one that sees
  nothing move only now and then.
- **Lights** reach as far as they do in EEVEE: their *Light > Custom Distance* when it's
  ticked, otherwise the distance where they fade below *Render Properties > Sampling >
  Advanced > Light Threshold*. Every exported point and spot light carries that distance, so
  the app stops shading and shadowing it there too. Without one it would reach 20 m.
- **Animated materials:** an Image Texture node showing a GIF (Blender opens a GIF as a
  movie) plays in the app on its material's base color. Export copies the GIF to
  `environment/<name>/animated/<material>.gif`, and the app finds it by that name. The app
  plays the GIF at the speed saved in it, while the viewport shows one GIF frame per scene
  frame, so the speed there can differ. Only GIFs play: image sequences and other movies
  export as a still. To make a material unlit, like a screen or a sign that ignores the
  scene's lights, plug the image's *Color* straight into the Material Output's *Surface*. For
  transparency, put a *Mix Shader* in between: *Fac* from the image's *Alpha*, a *Transparent
  BSDF* in its first shader slot and the image's *Color* in its second.
- **Sounds** are pose markers on a clip's action. In the Action Editor, turn on *Marker >
  Show Pose Markers*, put a marker where the sound should start and name it after a sound
  in the .blend: `step_l` matches the Sound `step_l.wav` or `step_l`. The app plays it each
  time the clip passes that frame, so again on every loop, and stops it when another
  animation starts. Export writes each named sound to `sounds/<name>.ogg`: an .ogg is
  copied, anything else is converted to Ogg Vorbis. A marker may also name an .ogg you put
  in `sounds/` yourself. Markers outside the action's frame range play nothing. The clip
  list shows how many markers a clip has.
- **Gaze** turns her eyes, head and neck towards the mouse cursor, on top of her animation.
  The app has to know which way her face points in her rest pose, which a clip can turn her
  away from: in the Gaze section, turn on the arrow, which shows her rest pose, look her in
  the face in the viewport and click *She faces me*. It's kept as one of the armature's axes,
  so turning her or the scene later keeps it right. The arrow is green once set, orange while
  she's taken to face -Y; turning it off shows her pose again. A clip that animates the eye bones
  leaves the eyes less room to follow the cursor, and a long camera lens makes the whole
  effect subtle; validation points out both.
- Your scene is never changed. Each file is exported from temporary copies in a throwaway
  scene, so your NLA tracks, active actions and selection stay as they were.

## suite.toml

Export updates `suite.toml` in place, and keeps the previous one as `suite.toml.bak`:

- It writes the name, the model file, the *Starts with* defaults, every exported clip (file,
  action, mode, gaze, cues), and `[gaze]` when Gaze is set to *Write*.
- A clip's cues come from its pose markers. An action without any keeps the cues already
  in suite.toml, so cues typed by hand survive an export.
- An empty field leaves the matching key in the file alone.
- Each exported camera's `[cameras.<name>]` is rewritten from its Depth of Field panel, and
  removed when depth of field is off.
- Each exported environment's fog, `no_volume_scatter` and `no_shadows` are rewritten from
  the scene; its brightness, shadows and exposure are kept.
- Clips of files that weren't exported this time, `[post]`, environments that weren't
  exported, and the orbit camera settings are kept as they are.

*Load suite.toml* fills the panel from an existing file.

## Validation

Validation runs before every export and shows warnings in the *Validation* section. It never
stops an export. Click the arrow next to a warning to select what it is about.
