[CharPaper](../README.md) › [Docs](README.md)

# Troubleshooting for artists

Problems running the app itself are under
[When something goes wrong](using.md#when-something-goes-wrong). In every case, `charpaper.log`, next to
`charpaper.exe`, usually says what happened.

**On this page:** [Loading](#loading) · [Outfits, animations and expressions](#outfits-animations-and-expressions) · [Gaze](#gaze) · [Lights and exposure](#lights-and-exposure) · [The sky and the sun](#the-sky-and-the-sun) · [Fog](#fog) · [Camera](#camera) · [Sound and GIFs](#sound-and-gifs)

## Loading

See also: [Testing your character](making-a-character.md#testing-your-character) · [suite.toml](suite-toml.md)

| What you see | Why, and what to do |
|---|---|
| The character doesn't appear in the list | Its folder has no `suite.toml`, or isn't directly inside `characters`. |
| It's in the list but nothing shows | suite.toml has an error. Read `charpaper.log`. |
| My new default in suite.toml is ignored | The app remembers your last pick. Delete `state.toml`. |
| The character stopped loading after I changed the export format | Files of the old format are still there, and a `.glb` and a `.gltf` of the same name clash. Delete the old ones. |
| The character stays in its rest pose, or parts of the outfit stay behind | Bone names, or the armature object's name, differ between files. Export the model, skins and animations again from the same .blend. |

## Outfits, animations and expressions

See also: [An outfit (skin)](recipes/outfit.md) · [An animation](recipes/animation.md) · [A facial expression](recipes/expression.md) · [Shape keys that follow the body](recipes/correctives.md)

| What you see | Why, and what to do |
|---|---|
| An animation is in the file but not in the list | suite.toml mentions that file, so only the clips listed for it are used. Export through the add-on, or list the clip. |
| An accessory doesn't follow the character | It's neither deformed by the armature nor parented to it or one of its bones. |
| Shape keys vanished from a mesh | It has modifiers and *Apply modifiers* is on. Apply them by hand. |
| An expression isn't listed | Its action also keys bones, or it wasn't ticked for that skin. An expression keys only shape keys. |
| A clip's shape keys do nothing on the face | That mesh is used by expressions. Use a driver, or put the keys on another mesh. |

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
| A light's flicker or color change doesn't play | It's keyed on the object or in the light's nodes, not on the light's *Strength* or *Color*. Or its keys hold one value: a Noise modifier with no range to loop over needs the scene's or the action's frame range set. Export again. |

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
| Patchy fog shows as even fog | Its Density setup isn't one the app follows. **Validate** says why. Only a Noise Texture's *Fac*, fBM, through Color Ramp, Map Range, Math and Float Curve nodes, works. |
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
