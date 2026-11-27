[CharPaper](../README.md) › [Docs](README.md)

# Troubleshooting for artists

Problems running the app itself are under
[When something goes wrong](using.md#when-something-goes-wrong). In every case, `charpaper.log`, next to
`charpaper.exe`, usually says what happened.

**On this page:** [Loading](#loading) · [Outfits, animations and expressions](#outfits-animations-and-expressions) · [Materials and textures](#materials-and-textures) · [Reactions and hotspots](#reactions-and-hotspots) · [Physics](#physics) · [Gaze](#gaze) · [Lights and exposure](#lights-and-exposure) · [The sky and the sun](#the-sky-and-the-sun) · [Fog](#fog) · [Camera](#camera) · [Sound and GIFs](#sound-and-gifs)

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
| An animation is missing, and the log says a `.glb` *has no .json beside it* | A clip's two files were copied by hand and one was left behind, or an old export left a stray file. Export the clip again, or delete the lone `.glb`. |
| Everything is far too big or small, lights look too weak, hair swings in slow motion | The scene isn't at real size, or its *Unit Scale* isn't 1, which the exporter ignores. Build at real size in meters with Unit Scale 1. |

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
| Shape keys vanished from a mesh | It has modifiers and *Apply modifiers* is on. Apply them by hand. *Shade Auto Smooth* (Blender 4.1+) adds one too, *Smooth by Angle*: use *Shade Smooth by Angle* instead, which needs no modifier. |
| Part of an outfit stays at the origin, and the log says a bone *has nowhere to attach* | The skin's armature has a bone the model's doesn't, and no parent of it does either: the bone was added after the model was exported. Export the model again. |
| A joint bends a little differently than in Blender | Its vertices are weighted to more than 4 bones; the app keeps the 4 strongest. *Weights → Limit Total* in Weight Paint mode. Check counts them. |
| A corrective doesn't follow, and the log says the driver of it *is left out* | It uses something the app can't run. Check names it and says why; see [Shape keys that follow the body](recipes/correctives.md). If the log says it *reads … at rest, where Blender reads …*, it runs but disagrees with Blender: look at what Check says about that driver. |
| The character rests with a face it shouldn't have | An action was still playing on the mesh's shape keys when the skin was exported, or a slider wasn't at 0. **Stop it**, set sliders, export the skin again. |
| An expression shows nothing on one outfit | That skin's meshes have no shape keys by those names. The Expressions panel lists which meshes of each skin it touches. |
| A clip's shape keys do nothing | They have drivers: the driver wins. Or the keys are in another action (4.4 and newer: in a slot of the clip's action; 4.2: in **Also plays**). |

## Materials and textures

See also: [Materials and textures](recipes/materials.md)

| What you see | Why, and what to do |
|---|---|
| Hair or layered cards overlap in the wrong order, flickering as the camera turns | They're see-through (Blend), which the app sorts per object, not per face. Make them [cutouts](recipes/materials.md#hair-lashes-and-lace). |
| Glass or a curtain casts a shadow as dark as a wall | A see-through surface casts a solid shadow. Untick its *Ray Visibility → Shadow*. |
| What's behind glass is blurred, fogged or outlined through it | Depth of field, fog and ink strokes don't see see-through surfaces. Keep glass small, or out of focus edges and fog. |
| Something is see-through that shouldn't be | Its Alpha slider is below 1, or something is plugged into Alpha. Leave Alpha at 1 for solid materials. |
| A cutout is solid everywhere | Its alpha comes from a JPEG, or from an image saved without alpha. Use a PNG with alpha. |
| A cutout's edges look jagged | Cutouts have hard edges. TAA anti-aliasing smooths them best. |
| The inside of a closed object shows through it | A see-through material shows both sides. Tick *Backface Culling*. |
| A mesh is tinted, or darker than in Blender | A painted color attribute came along, and the app multiplies base color by it. Delete it, or set *Vertex colors* to *None*. Check warns. |
| Textures are missing | They're WebP, or made with procedural nodes (bake them), or the image file was missing when exporting. |
| A normal map looks lit from the wrong side | It's a DirectX map (green down): invert its green channel. Or its Image Texture isn't *Non-Color*. |
| A coat, specular or transmission texture does nothing | Only those inputs' slider values come over. |
| A textured detail is out of place | It has a Mapping node of its own; the app places all of a material's textures like its base color. |
| A screen stays as bright as ever when the exposure changes | It's unlit, which ignores lights and exposure. That's what unlit is for; use Emission to follow the exposure. |

## Reactions and hotspots

See also: [Hotspots](recipes/hotspots.md) · [A reaction](recipes/reaction.md) · [Hotspot ideas](recipes/hotspot-ideas.md)

| What you see | Why, and what to do |
|---|---|
| Clicking does nothing | The hotspot works only during other animations (its **Only during** list); the viewer turned **Reactions** off in the Character tab; the hotspot isn't in suite.toml (export all); the click was on the settings panel; or the hotspot's capsule is smaller than it looks. Check for the hotspot's warnings. |
| A reaction lasts much longer than its keys, and can't be set off again meanwhile | It plays *Timeline*, a face or corrective slot is keyed past the body, or the action's *Manual Frame Range* is on. See [How long it lasts](recipes/reaction.md#how-long-it-lasts). |
| The reaction plays the idle, or the idle's face | The action was made with **New** while the idle was assigned, so it's a copy. Make a fresh empty action. |
| A reaction freezes the rest of the body | It keys bones it doesn't move. Delete those keys. |
| A prop's hotspot does nothing | Its environment isn't the one shown, or the prop's bone isn't exported (*Deform bones only*). |
| Clicking a desktop icon sets off a hotspot | Clicks on desktop icons count. Keep click hotspots away from where icons sit. |
| A hotspot goes off when I only wanted to orbit | It doesn't for drags. A click that barely moves counts as a click. |
| The While loop plays once, then freezes | It's shorter than the scene range and doesn't repeat. Give its curves a **Cycles** modifier, or its action a **Manual Frame Range** of its own frames. |
| The While loop starts mid-way through | A Timeline clip shows the timeline's frame. Make the loop even, so any point is a good start, or use a Hold pose. |
| A click hotspot goes off on press, not on release | It has something under **While held down**: those go off as the button goes down, so the loop can start. |
| After a hotspot with **Then**, the character stays in the new animation | That's what Then does, and the app remembers it as the viewer's pick. Give another hotspot a Then back, or pick the animation in the Character tab. |
| The character pops when Then takes over | The reaction ends far from the new animation's pose. Make its last frames match the new animation's first. |
| Nothing else reacts while one hotspot plays | A hotspot with **Then** holds the others back until it has switched, so states can't be left half-done. |
| A prop forgot its state | A prop switched by Then goes back to its ★ clip when its environment is shown again, or the app restarts. |
| The prop plays the wrong clip by itself | It plays its ★ clip: star the right one in the **Animation** panel, and export the environment again. |
| A *Mouse keeps still* hotspot goes off while I'm working | In wallpaper mode, the mouse in another app's window counts as still. Make **After** longer, or use **Only during** to limit it. |
| A *Mouse comes back* hotspot doesn't go off | The mouse wasn't away for **After** seconds, or its **Only during** doesn't match what plays. |
| The 5th poke still plays the first clip | *In a row* means going off again within 3 seconds of when it could. Waiting longer than that between pokes starts over at 1. |

## Physics

See also: [Hair, clothes and tails that swing (physics)](recipes/physics.md)

| What you see | Why, and what to do |
|---|---|
| Nothing swings | The viewer turned **Physics** off in the Character tab; the chain isn't in suite.toml (export all); or its bones aren't on the outfit worn. Check for the chain's warnings, and look in `charpaper.log` for lines starting with `physics`. |
| A chain swings with one outfit and not another | Its bones are only in one outfit. That's expected: it's skipped while the other is worn. |
| Hair sinks into the body | No collider there, or the capsule is too thin, or the chain's **Radius** is too small. Add colliders on the bones it reaches, and look at their capsules from every side. |
| Hair floats off the body, or jumps when the character loads | A capsule is too fat, or the chain's Radius too big. Check warns when a chain *starts inside a collider*. |
| It swings too much, or never settles | Raise **Drag** to settle sooner, **Stiffness** to swing less. |
| It's stiff, or lags behind the head like jelly | Lower **Stiffness** for more swing; raise it if a long chain trails too far behind. |
| A ponytail pokes through the back of the head | The head's capsule ends short of the back of the skull. Raise its radius, or add one on the neck. With **High quality** on, the mesh itself is used. |
| The pointer goes through a chain | Its **Cursor** is *Ignores it*, the viewer turned **Cursor pushes** off, or a mouse button is held (dragging turns the camera). |
| A chain set to *Only the cursor moves it* never moves | **Cursor pushes** is off in the Character tab, or the pointer crosses its bones between their tails: only the tails are pushed. Give it more, shorter bones, or a bigger **Radius**. |
| A chain fights the cursor or shakes | One of its bones is one gaze turns. Keep chains off gaze's bones. |
| No wind in the app | **Wind** is off in the Character tab; the force field isn't in the environment's collection, isn't a Wind field, or has a Strength of 0; or the environment wasn't exported again. |
| High quality looks the same as without | It reads the outfit's file after it's put on, so give it a moment. Bones that no mesh is weighted to keep their capsule, and so do colliders the chains never touch. |

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
| The fog doesn't show | Nothing lights it: the environment has no sky, and no lamp lights the fog. A sun only does with *Shadow* on. Or **Fog** is off in the Look tab. |
| Patchy fog shows as even fog | Its Density setup isn't one the app follows. **Check** says why. Only a Noise Texture's *Fac*, fBM, through Color Ramp, Map Range, Math and Float Curve nodes, works. |
| The fog's noise repeats | The noise's Scale is beyond about 16 across the box. Lower it, or scale the Mapping node down. |

## Camera

See also: [A camera](recipes/camera.md)

| What you see | Why, and what to do |
|---|---|
| Nothing blurs, and the Look tab has no Depth of field | The camera in use has no F-Stop: tick *Depth of Field* on the camera and export again. For the orbit camera, set `f_stop` under `[camera]`. |

## Sound and GIFs

See also: [Sounds](recipes/sounds.md) · [A screen that plays a GIF](recipes/environment.md#a-screen-that-plays-a-gif)

| What you see | Why, and what to do |
|---|---|
| A sound or the music doesn't play | It isn't Ogg Vorbis. |
| A GIF shows as a still picture | It isn't a GIF, or its file isn't named after the material. |
| A GIF screen stays black | Its material glows through an Emission shader, which exports a black base color. Plug the image straight into *Surface*. Check warns. |
| The log says *no mesh wears the material* for a GIF | The material was renamed or taken off its mesh after the GIF was exported. Export the environment again. |

---

[All docs](README.md)
