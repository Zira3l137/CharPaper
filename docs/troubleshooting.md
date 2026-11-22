[CharPaper](../README.md) › [Docs](README.md)

# Troubleshooting for artists

Problems running the app itself are under
[When something goes wrong](using.md#when-something-goes-wrong). In every case, `charpaper.log`, next to
`charpaper.exe`, usually says what happened.

**On this page:** [Loading](#loading) · [Outfits, animations and expressions](#outfits-animations-and-expressions) · [Reactions and hotspots](#reactions-and-hotspots) · [Gaze](#gaze) · [Lights and exposure](#lights-and-exposure) · [The sky and the sun](#the-sky-and-the-sun) · [Fog](#fog) · [Camera](#camera) · [Sound and GIFs](#sound-and-gifs)

## Loading

See also: [Testing your character](making-a-character.md#testing-your-character) · [suite.toml](suite-toml.md)

| What you see | Why, and what to do |
|---|---|
| The character doesn't appear in the list | The app was running when it was first exported: restart it. Or the add-on exports it somewhere else: the Suite panel's **Exports to** line says where. With **Custom folder** ticked, copy the folder into `characters` yourself. |
| Export says there's nowhere to export to | **CharPaper app** isn't picked, or no `charpaper.exe` is where it points (the app was moved or updated into another folder). Pick the folder CharPaper is in. |
| It's in the list but nothing shows | suite.toml has an error. Read `charpaper.log`. |
| The log says the schema version isn't supported | The suite was exported by an older add-on. Install the add-on from the same release as the app, open the .blend, and **Export all**. |
| A hotspot names a clip or expression that doesn't exist | A clip was renamed or removed after the hotspot was made. Fix the hotspot in the add-on and export again. |
| My new default in suite.toml is ignored | The app remembers your last pick. Delete `state.toml`. |
| The character stopped loading after I changed the export format | Files of the old format are still there, and a `.glb` and a `.gltf` of the same name clash. Delete the old ones. |
| The character stays in its rest pose, or parts of the outfit stay behind | Bone names, or the armature object's name, differ between files. Export the model, skins and animations again from the same .blend. |

## Outfits, animations and expressions

See also: [An outfit (skin)](recipes/outfit.md) · [An animation](recipes/animation.md) · [A facial expression](recipes/expression.md) · [Shape keys that follow the body](recipes/correctives.md)

| What you see | Why, and what to do |
|---|---|
| An animation isn't in the app's list | It's ticked **Only for hotspots**, or it wasn't exported: it isn't in the character's clip list in the Animation panel, or has no action. |
| The character starts in its rest pose | No clip is starred, the starred clip plays *Once* or is *Only for hotspots*, or the app remembers the viewer's pick (delete `state.toml`). |
| An accessory doesn't follow the character | It's neither deformed by the armature nor parented to it or one of its bones. |
| Part of the body snaps to the rest pose during a clip | No clip that plays keys those bones. Key the whole body in the clip that plays underneath. |
| A clip plays another action's keys | Its action holds several slots for the armature and the wrong one is picked. Pick the **Slot** in the clip's settings. If the action was copied with **New**, make a fresh one. See [Slots](recipes/animation.md#slots-blender-44-and-newer). |
| A loop pops once per round | The pose at the end of the scene's range differs from the start, or the clip's own loop doesn't fit the range. Check says which, and which end frame fits. |
| A prop's loop stops partway through the timeline | It's shorter than the timeline and doesn't repeat. Give it a Cycles modifier or a Manual Frame Range that fits. |
| Shape keys vanished from a mesh | It has modifiers and *Apply modifiers* is on. Apply them by hand. |
| The character rests with a face it shouldn't have | An action was still playing on the mesh's shape keys when the skin was exported, or a slider wasn't at 0. **Stop it**, set sliders, export the skin again. |
| An expression shows nothing on one outfit | That skin's meshes have no shape keys by those names. The Expressions panel lists which meshes of each skin it touches. |
| A clip's shape keys do nothing | They have drivers: the driver wins. Or the keys are in another action (4.4 and newer: in a slot of the clip's action; 4.2: in **Also plays**). |

## Reactions and hotspots

See also: [Hotspots](recipes/hotspots.md) · [A reaction](recipes/reaction.md)

| What you see | Why, and what to do |
|---|---|
| Clicking does nothing | The viewer turned **Reactions** off in the Scene tab; the hotspot isn't in suite.toml (export all); the click was on the settings panel; or the hotspot's capsule is smaller than it looks. Check for the hotspot's warnings. |
| A reaction lasts much longer than its keys, and can't be set off again meanwhile | It plays *Timeline*, a face or corrective slot is keyed past the body, or the action's *Manual Frame Range* is on. See [How long it lasts](recipes/reaction.md#how-long-it-lasts). |
| The reaction plays the idle, or the idle's face | The action was made with **New** while the idle was assigned, so it's a copy. Make a fresh empty action. |
| A reaction freezes the rest of the body | It keys bones it doesn't move. Delete those keys. |
| A prop's hotspot does nothing | Its environment isn't the one shown, or the prop's bone isn't exported (*Deform bones only*). |
| Clicking a desktop icon sets off a hotspot | Clicks on desktop icons count. Keep click hotspots away from where icons sit. |
| A hotspot goes off when I only wanted to orbit | It doesn't for drags. A click that barely moves counts as a click. |

## Gaze

See also: [Following the cursor (gaze)](recipes/gaze.md)

| What you see | Why, and what to do |
|---|---|
| The eyes swing out of the head with gaze on | The eye bones don't sit at the eyeballs' centers, or the eyes aren't eyeballs: set **Eyes are** to *Drawn*. |
| The irises slide too far, or out of sight | The eye limits are too big for the eyes. Turn on **Preview**, drag a slider to 1 and lower *Eye yaw* and *Eye pitch*. |
| Only the eyes follow, not the head | No head bone is named in Gaze, or *Head turn* is 0. |
| The character turns away from the cursor | Its front is wrong. Set **Front** back to *Automatic*; if the arrow still points wrong, set it by hand. |
| A clip's eye animation doesn't show | Gaze moves the eyes while the character follows. Untick the clip's **Gaze**. |

## Lights and exposure

See also: [Lights](recipes/environment.md#lights) · [The picture the viewer sees first](recipes/first-picture.md)

| What you see | Why, and what to do |
|---|---|
| The character is black | The environment has no lights and no panorama. Add lights to its collection, or a `.hdr` to its folder. |
| The character is far too bright or dark | *glTF Settings → Lights* isn't on *Standard*, or the scene needs its own `exposure`. |
| A light does nothing | It's an area light. Use a spot or point light. |
| A light's flicker or color change doesn't play | It's keyed in the light's nodes, not on the light's *Strength* or *Color*. Or the lamp's object also has an action, which became its clip: pick the light's action in that clip's **Also plays**. Export again. |

## The sky and the sun

See also: [A sky](recipes/sky.md) · [A sun that matches the sky](recipes/sun.md)

| What you see | Why, and what to do |
|---|---|
| The sun's shadows ignore how I turned the sun in Blender | That's right when the sky's sun is found: the sun shines from it instead. Turn **Sun follows sky** off to keep yours. See [A sun that matches the sky](recipes/sun.md). |
| The shadows don't move when the sky turns | **Sun follows sky** is off, or the Sun light isn't in the environment's collection. See [A sun that matches the sky](recipes/sun.md). |
| The shadows move with the sky but point the wrong way | The sky's sun wasn't found (`charpaper.log` says why), so the app trusts your Blender scene, where the Sun light doesn't point at the sky's sun. Point it there (step 4 in [A sun that matches the sky](recipes/sun.md)), or use an HDRI with an unclipped sun. |
| The sky didn't change after replacing the `.hdr` by hand | Delete `skybox.ktx2`, `specular.ktx2` and `diffuse.ktx2` next to it. The export does this itself when the World's panorama changed. |

## Fog

See also: [Fog and light shafts](recipes/fog.md)

| What you see | Why, and what to do |
|---|---|
| The fog doesn't show | Nothing lights it: the environment has no sky, and no lamp lights the fog. A sun only does with *Shadow* on. Or **Fog** is off in the Graphics tab. |
| Patchy fog shows as even fog | Its Density setup isn't one the app follows. **Check** says why. Only a Noise Texture's *Fac*, fBM, through Color Ramp, Map Range, Math and Float Curve nodes, works. |
| The fog's noise repeats | The noise's Scale is beyond about 16 across the box. Lower it, or scale the Mapping node down. |

## Camera

See also: [A camera](recipes/camera.md)

| What you see | Why, and what to do |
|---|---|
| Nothing blurs, and Graphics has no Depth of field | The camera in use has no F-Stop: tick *Depth of Field* on the camera and export again. For the orbit camera, set `f_stop` under `[camera]`. |

## Sound and GIFs

See also: [Sounds](recipes/sounds.md) · [A screen that plays a GIF](recipes/environment.md#a-screen-that-plays-a-gif)

| What you see | Why, and what to do |
|---|---|
| A sound or the music doesn't play | It isn't Ogg Vorbis. |
| A GIF shows as a still picture | It isn't a GIF, or its file isn't named after the material. |

---

[All docs](README.md)
