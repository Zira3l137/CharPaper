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
| Animation   | the armature + ticked clips, plus corrective meshes if turned on | `animations/<name>.glb` |
| Environment | everything in its collection except cameras, with active actions | `environment/<name>.glb` |
| Camera      | the camera, the empties it hangs from, its focus object, and one action | `cameras/<name>.glb` |

- **Clips** and **expressions** are named after their Blender actions. *Shown as* renames a
  clip in the app only.
- **Correctives:** tick *Correctives*, pick the collection of meshes they key, then give each
  clip that needs them a shape key action. It plays together with the clip under the clip's
  name.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and either
  Focus Distance or a Focus Object. A focus object goes into the camera's file with the
  empties it hangs from, and its animation joins the camera's clip. Focusing on the
  character's armature with a bone picked, or on an object of one of its skins, follows that
  bone or object instead. Animated Focus Distance or F-Stop doesn't carry over; animate a
  focus empty instead.
- **Fog** is any mesh in an environment whose material has a Volume and no Surface. The
  app fills its bounding box with fog set up like its Principled Volume: Color, Density,
  Anisotropy and Absorption Color. Only lights light fog, not the sky: a light whose
  *Influence > Volume Scatter* is 0 doesn't, and a sun only does while it casts shadows.
- **Shadows** follow each light's *Shadow* checkbox and each object's *Visibility > Ray
  Visibility > Shadow*. Turn the latter off on a lamp's housing to keep its pattern out of
  the fog and off the walls.
- Your scene is never changed. Each file is exported from temporary copies in a throwaway
  scene, so your NLA tracks, active actions and selection stay as they were.

## suite.toml

Export updates `suite.toml` in place, and keeps the previous one as `suite.toml.bak`:

- It writes the name, the model file, the *Starts with* defaults, every exported clip (file,
  action, mode, gaze), and `[gaze]` when Gaze is set to *Write*.
- An empty field leaves the matching key in the file alone.
- Each exported camera's `[cameras.<name>]` is rewritten from its Depth of Field panel, and
  removed when depth of field is off.
- Each exported environment's fog, `no_volume_scatter`, `no_shadow` and `no_shadow_casting`
  are rewritten from the scene; its brightness, shadows and exposure are kept.
- Clips of files that weren't exported this time, `[post]`, environments that weren't
  exported, and the orbit camera settings are kept as they are.

*Load suite.toml* fills the panel from an existing file.

## Validation

Validation runs before every export and shows warnings in the *Validation* section. It never
stops an export. Click the arrow next to a warning to select what it is about.
