# CharPaper Exporter

A Blender add-on (4.2 to 5.2) that exports a whole CharPaper character suite from one
`.blend`: the model, skins, clips, expressions, environments, cameras, hotspots, and
`suite.toml`. The step-by-step guide for artists is
[Making a character](../../../docs/making-a-character.md); this page is the fine print.

## Install

From the website: drag the add-on's *Latest version* link into Blender. The link names the
website's extensions listing (`blender/index.json`, written by the Pages workflow from the
release's zip with `packaging/blender_listing.py`), which Blender adds as a repository the
first time; it then offers each new release as an update.

From disk: zip this folder's contents, with `blender_manifest.toml` at the zip's root, then
in Blender: *Edit > Preferences > Get Extensions > ⌄ > Install from Disk…* and pick the zip.
Or build the zip with Blender itself:

```
blender --command extension build --source-dir tools/blender/charpaper_exporter
```

The panel is in the 3D Viewport sidebar (N), tab **CharPaper**. Everything you set there is
saved in the `.blend`, except **CharPaper app**, which is an add-on preference and shared by
every .blend on the computer.

## Where it exports

- **Into the app**, by default: `<app>/characters/<folder name>/`. The app's folder is found
  from **CharPaper app**, which may point at the folder with `charpaper.exe` (or
  `charpaper` on other systems), at the executable, at its `characters` folder, or at the
  folder the download was unpacked into (of several apps there, the newest). The folder name
  is **Folder name**, else the Name, else the .blend's name, with characters Windows doesn't
  allow in a file name replaced by `_`. The first export into the app fills in **Folder
  name**, so renaming the character later keeps exporting into the same folder.
- **Custom folder:** any folder, written to as it is. A path starting with `//` is relative
  to the .blend, which must be saved.
- The Suite panel shows the folder, or why there's none; Check reports it as an error and
  export stops. A .blend saved before this setting existed keeps exporting where it did: to its
  picked folder, or to `//charpaper/` next to the .blend if a suite.toml is already there.

## What goes where

| Entry       | Holds                                                   | Written to                   |
|-------------|---------------------------------------------------------|------------------------------|
| Model file  | the armature alone                                      | `<model>.glb` next to suite.toml |
| Skin        | the armature + every mesh in its collection, its sliders as they stand | `skins/<name>.glb`, its drivers in `skins/<name>.drivers.json` |
| Character clip | one action on the armature: the bones it keys, and the shape keys of every slot | `clips/<name>.glb` (bones) and `clips/<name>.json` (timing, shape keys, sounds) |
| Expression  | one shape key action                                    | `expressions/<name>.json` |
| Environment | everything in its collection except cameras, without animation; its World as a panorama | `environment/<name>.glb`, its GIFs in `environment/<name>/animated/`, the panorama in `environment/<name>/` |
| Prop clip   | one action of an environment's object, its shape keys or its light | `environment/<name>/clips/<object>/<clip>.glb` and `.json` |
| Camera      | the camera, the empties it hangs from, its focus object, and its *Moves with* action | `cameras/<name>.glb`, its movement in `cameras/<name>.json` |
| Sound       | an environment's music, in order, and its ambience, converted to Ogg Vorbis | `environment/<name>/music/01 - <title>.ogg`, `environment/<name>/ambience.ogg` |
| Hotspots    | capsules around bones and the clips they play            | `[hotspots]` in suite.toml |
| Physics     | bone chains the app swings, and capsules they keep out of | `[physics]` in suite.toml |
| Wind        | an environment's first Wind force field                  | `wind` in its `[environments.<name>]` |

- **The timeline:** the scene's frame range and frame rate are written to suite.toml as
  `[timeline]`, and everything plays on that one loop. A clip's **Plays** says how: *Timeline*
  shows the timeline's frame, exported over the whole scene range (or its action's *Manual
  Frame Range*); *Once* plays from its first key to its last (or over its Manual Frame
  Range) and hands back; *Hold* is its first frame. Check warns about a Timeline clip whose
  ends don't meet where the timeline loops, and about a clip looping on its own length
  (Cycles or a Manual Frame Range) that doesn't fit the timeline a whole number of times,
  with the end frame that would.
- **Actors:** the character's armature, and in each environment every armature and every
  object with an action on itself, its shape keys or its light. Each has its own clip list
  in the Animation panel. A prop with an empty list plays its active actions on the timeline.
- **Clips** and **expressions** are named after their Blender actions. *Shown as* renames
  them in the app only. *Only for hotspots* plays a clip Once and keeps it out of the app's
  list. The star marks the clip the character starts with.
- **Slots (Blender 4.4 and newer):** a clip plays one slot for its object, and every slot of
  its action for shape keys and lights. The object's slot is the one picked in the clip's
  **Slot** field; else the action's only slot for that object type; else the one the object
  is playing; else the one named after the object, with a warning. *Add* remembers the slot
  the object plays. Check warns when the slot is a guess, when a Once clip's other slots are
  keyed past its body's keys (they set its length), and when an action holds the slots of
  another action it was copied from with *New*.
- **Shape keys keyed in a clip** play with it, for what a driver can't do, like a cheek puff in
  frames 40–60 of one clip. In Blender 4.4 and newer, key them in the clip's own action:
  select the mesh, assign that action in the Shape Key Editor, let Blender give it a slot for
  the shape keys, and key them there. In 4.2 and 4.3, put them in a second action and pick
  it in the clip's *Also plays*. The clip's details count the shape keys it keys. A shape key
  that also has a driver follows the driver, as in Blender; Check points it out.
- **What plays at export:** a skin is exported as its sliders stand at the current frame,
  and the app rests every shape key nothing keys at that value. So the add-on reads what
  Blender actually plays on each mesh's shape keys there (the active action and its slot,
  NLA tracks with solo, mute, influence, strip reach and extrapolation) and warns when an
  action sets them, with **Stop it**: it unassigns the action (with a fake user if nothing
  else uses it) and mutes the tracks that play. A skin mesh object's own action is reported
  the same way, with **Unlink it**.
- **Driven shape keys:** a skin's shape keys driven by bones, like correctives a pose reader
  drives, are written next to the skin as `skins/<name>.drivers.json`, and the app runs the
  drivers itself every frame. So they follow every clip, blends between clips and gaze, with no
  keys to export. The app runs what Blender runs without Python: average, sum, min, max and
  simple-expression drivers, whose variables read bones of the suite's armature (Rotational
  Difference; Transform Channel and Distance in world, transform or local space, in any
  rotation mode; a Single Property that is a bone's location, rotation or scale) or another
  shape key of the same skin (a Single Property). It can't read objects other than bones,
  custom properties, context properties or `frame`. A driver may read a bone the export
  leaves out, such as an `MCH-reader` bone, as long as nothing between it and the nearest
  exported bone above it is animated or constrained; such a bone reads as still in local
  space. Transform space and a bone's Single Property read the bone's own channels, which
  Blender reads before its constraints and the app after them, so Check flags them on a
  constrained bone. Check names every driver the app can't run, and the app's log says
  whether its drivers match Blender in the rest pose.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and either
  Focus Distance or a Focus Object. A focus object goes into the camera's file with the
  empties it hangs from, and its animation joins the camera's clip. It may hang from the
  camera itself, animated or not. Focusing on the
  character's armature with a bone picked, or on an object of one of its skins, follows that
  bone or object instead. Animated Focus Distance or F-Stop doesn't carry over; animate a
  focus empty instead.
- **Prop animation:** each actor's clips are exported on their own, into its folder under
  `environment/<name>/clips/`. In the app a prop plays its first Timeline clip (else its
  first Hold clip), and its Once clips only when a hotspot plays them. A lamp's keyed
  *Strength* and *Color* come over; a lamp that both moves and flickers has its light's
  action in its clip's *Also plays*. An armature's bones move by its own clips, so a rigged
  prop works with the armature and its meshes in the collection. NLA tracks of environment
  objects aren't exported, nor is animation of other light, material or camera settings.
  Motion that only comes from constraints or drivers, with no action at all, needs baking
  into one (*Object > Animation > Bake Action*).
- **Sky:** with *Sky* ticked, an environment's World, or the scene's while the field is
  empty, becomes the panorama its sky and reflections are baked from. A World that is one
  Environment Texture into a Background, optionally through a Mapping node turned only
  about Z, goes over at full size: Strength and the Z rotation go to suite.toml as
  `sky_strength` and `sky_rotation_deg`. Its file is copied as `sky.hdr` or `sky.exr` when
  it's an `.hdr` or plain `.exr` in Linear Rec.709, and otherwise written out as Blender
  sees it into `sky.exr`. Any other World (Sky Texture, a plain color, mixed textures) is
  rendered with Cycles into a 2048×1024 `sky.exr`, which takes a few seconds on every
  export; Cycles is switched on for it if it's off. The panorama replaces any other in the
  folder, and when it changed the baked `.ktx2` maps are deleted so the app bakes them
  again. The cases where no sky is exported are listed under
  [When the sky isn't exported](../../../docs/recipes/sky.md#when-the-sky-isnt-exported). An environment with a World and no collection is just a sky.
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
  BSDF* in its first shader slot and the image's *Color* in its second. An Emission shader
  instead exports a black base color, so the GIF wouldn't show; Check warns.
- **Materials:** the exporter decides each material's transparency from what feeds its
  Alpha (or a Transparent BSDF mix's *Fac*): rounded with a Round or Greater Than node, or as
  `1 − (alpha < cutoff)`, it's a cutout; anything else below 1 is see-through. A see-through
  material whose *Render Method* is Blended is written to suite.toml's `[materials]`, so the
  app blends it; the others it dithers. Check works it out the same way and warns about a
  Blended skin material, alpha from a JPEG,
  sheen and subsurface, differing Mapping nodes, painted
  color attributes no material uses, vertices on more than 4 bones and a Unit Scale other
  than 1. See [Materials and textures](../../../docs/recipes/materials.md).
- **Sounds** are pose markers on a clip's action. In the Action Editor, turn on *Marker >
  Show Pose Markers*, put a marker where the sound should start and name it after a sound
  in the .blend: `step_l` matches the Sound `step_l.wav` or `step_l`. The app plays it each
  time the clip passes that frame, so again on every loop, and stops it when another
  animation starts. Export writes each named sound to `sounds/<name>.ogg`: an .ogg is
  copied, anything else is converted to Ogg Vorbis. A marker may also name an .ogg you put
  in `sounds/` yourself. Markers outside the action's frame range play nothing. The clip
  list shows how many markers a clip has.
- **Gaze** has the character look towards the mouse cursor, on top of its animation. Where
  it looks comes from where the cursor is on screen: on its eyes it looks at the viewer,
  and the further from them the cursor is, the further aside it looks, whatever the
  camera's lens or distance. The head and neck go *Head turn* of the way, split between
  them by *Head share*, and the eyes cover the rest.
  - *Detect bones* reads bone names word by word (`mixamorig:LeftEye` is mixamorig, left,
    eye), skips controls and helpers, and where a rig has several bones per eye takes the
    ones that move the mesh. The head is the bone named so above the eyes, or else the
    nearest deforming bone above them; the neck is the bone named so above the head. It
    finds no neck on Rigify's deform bones, which are named `DEF-spine`; pick it by hand.
  - *Eyes are* is set by Detect bones from the eyes' shape. An eye counts as an eyeball
    when the vertices its bone moves lie on a ball, the bone sits near the ball's center
    and the vertices cover a good part of it. *Eyeballs* are aimed exactly, up to their
    limits. *Drawn* eyes turn in proportion instead, reaching their limits for a look
    about 35° aside, because aiming a bone that isn't at the center of a ball shoots past.
  - The head's and neck's limits add up to how far the head may point from the bone above
    the neck, with the clip's own turn counted in. The eyes' limits are measured from the
    head. A cursor further round than the character can turn, it lets go of.
  - Gaze sets the eye bones outright while the character follows, so the eyes can't leave their
    sockets and a clip's own keys on them don't show; Check notes such clips. Untick
    the clip's *Gaze* to see them.
  - Eye bones are optional. A head alone follows, and so do eyes moved by the four shape
    keys alone, which also move in proportion.
  - *Front* is which way the character faces in the rest pose, which is where gaze turns
    from; a clip may turn it around entirely. *Automatic* works it out as the app does:
    level, square to the line between the eye bones, on the side of the head bone they are
    on. The arrow button shows the rest pose with an arrow from the head, green when the
    front is known and orange when nothing says and it's taken to face -Y. Where it points
    wrong, look the character in the face in the viewport and click *Front from view*:
    that front is kept as one of the armature's axes, so turning the character or the
    scene later keeps it right, and only a front set this way is written to suite.toml.
  - *Preview* poses the character as the app would, on top of its current pose, with the
    same maths. The two sliders are the look: at -1 and 1 every part is at its limit.
    Changing a field updates the pose, and named shape keys are set too. The character's
    own pose and shape key values are put back when it is turned off, on a frame change, before a save and at
    export.
- **Hotspots** go off on a click or hover on capsules around bones (of the character's
  armature or an exported environment's), or when the mouse keeps still or comes back for
  *After* seconds; the panel's **+** makes each kind. A capsule runs from the bone's head
  to its tail, grown by a radius that starts at 30% of the bone's length; *Add selected
  bones* takes those selected in Pose Mode, several armatures at once. The eye icon draws
  them in each hotspot's color tag, in the current pose. A hotspot plays clips once (one
  at random per actor, never the same twice in a row, each from its *From the time in a
  row* on), plays a Timeline or Hold clip per actor on and on while held (*While*), and
  switches actors to a Timeline or Hold clip after (*Then*). *Only during* lists what its
  actors must be playing: the character's pick, or a prop's ★ clip or Then state. Each list's
  **+** offers only clips that fit it. A hotspot whose bones, clips, While or Then clips
  weren't exported this time is left out of suite.toml, as is one none of whose *Only
  during* clips is, and the report says so. The add-on owns `[hotspots]` once one is made
  in it; *Load suite.toml* reads them back.
- **Physics**: *+* under Chains makes a chain from each bone selected in Pose Mode whose
  parent isn't selected. With only its first bone selected, it runs down through only
  children to the end; with more of it selected, it takes exactly those. A chain is named
  after its first bone and takes its kind's values (Hair, Cloth, Soft or Tail: stiffness,
  drag, gravity, radius); changing one sets the preset to Custom. Its *Cursor* is written as
  `cursor` when not *Pushes it*: `"ignores"`, or `"only"` for a chain only the pointer moves,
  drawn green, which Check doesn't warn about starting inside a collider. *+* under Colliders puts a
  capsule on each selected bone not in a chain, from its head to its tail, its radius the
  median distance from the bone of the vertices weighted at least half to it, in the
  thinnest mesh the armature deforms, or 30% of its length without one; *Fit to the mesh*
  measures it again. The eye icon draws chains blue and colliders orange in the current
  pose. A chain with a bone that isn't exported, is in an earlier chain, or isn't the child
  of the one before is left out of suite.toml, and so is a collider whose bone isn't
  exported; the report says so. The add-on owns `[physics]` once a chain or collider is
  made in it; *Load suite.toml* reads it back.
- **Wind** is an environment's first Wind force field by name: it blows along the field's Z
  axis, written in the app's axes, as hard as its Strength (in m/s², negative blowing
  backwards), with its Noise up to 1 as `gusts`.
- **A prop's ★** is the clip it plays by itself, written as its `default_clip`; with none
  starred, its first Timeline clip in the list, else its first Hold one. *Only for hotspots*
  no longer fixes a clip's mode: ticking it sets Once, and Timeline or Hold make loops and
  poses for *While*.
- Your scene is never changed. Each file is exported from temporary copies in a throwaway
  scene, so your NLA tracks, active actions and selection stay as they were. Only the gaze
  preview poses the character, and it puts the pose back.

## suite.toml

Export updates `suite.toml` in place, and keeps the previous one as `suite.toml.bak`:

- It writes `schema = 2`, the name, `[timeline]`, the model file, the *Starts with*
  defaults, the clip tables of every actor it exported (only what differs from the
  defaults: mode, label, gaze, reaction_only, cues), the expressions' labels, `[hotspots]`
  once the add-on has hotspots, `[physics]` once it has chains or colliders, and `[gaze]` when Gaze is set to *Write*. In `[gaze]`,
  `eyes` is written only when there are eye bones, `eye_kind` only for Drawn eyes, and
  `front` only when it was set by hand.
- A clip's cues come from its pose markers. An action without any keeps the cues already
  in suite.toml, so cues typed by hand survive an export.
- An empty field leaves the matching key in the file alone.
- Each exported camera's `[cameras.<name>]` is rewritten from its Depth of Field panel, and
  removed when depth of field is off.
- Each exported environment's fog, `no_volume_scatter`, `no_shadows` and `wind` are
  rewritten from the scene. With a World, its `sky_strength` and `sky_rotation_deg` are too. Its shadows,
  exposure and background settings are kept, and so are strength and rotation without a
  World.
- Actors that weren't exported this time, `[post]`, environments that weren't exported, and
  the orbit camera settings are kept as they are.

*Load suite.toml* fills the panel from an existing file. Opening a .blend saved with an
older add-on moves its settings to the current ones by itself.

## Sync

**Sync** writes suite.toml as an export would, without exporting a file: it takes the
clips, environments and cameras already in the suite folder as exported. A clip whose
**Plays** differs from suite.toml's keeps its old entry, the timeline keeps its old
`[timeline]`, and cues naming sounds not yet in `sounds/` are left out, since each needs
files exported again; the report names what to export. It runs Check first, writes even with
*Write suite.toml* off, and needs a suite.toml there already.

## Picking several

The lists of clips, hotspots, a hotspot's areas and clips, chains and colliders pick several
rows by Shift-clicking their names. A setting changed on a picked row is set on every picked
row of the list, except what names a row (its action, bone, label or name); **−** removes all
of them, and *Fit to the mesh* fits each picked collider to its own bone. Clicking a row, or
its name without Shift, picks it alone.

## Check

**Check** runs before every export and lists what it finds in the *Warnings* panel, worst
first, and next to what each is about (a clip, a hotspot, a skin). It never stops an export.
The arrow next to a warning selects what it's about, and some warnings have a button that
fixes them: **Stop it**, **Unlink it**.

Colors are Blender's collection color tags, so they follow the theme: red for errors (an
export step failed or wrote something wrong), yellow for warnings (it will most likely look
wrong in the app), blue for tips. A clip's tag is its mode: blue Timeline, orange Once,
violet Hold. A row turns red when something it needs is missing.
