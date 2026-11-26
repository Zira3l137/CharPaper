[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A camera

1. Select the camera object and press **+** under **Cameras**. It takes the camera and its
   name, which is what the app's Character tab shows.
2. If it moves, pick its action in **Moves with**. A camera selected when you press **+**
   gets its current action filled in.
3. Export it with its own button, or with *Export all*. A camera is written to
   `cameras/<name>.glb`, and its movement next to it as `<name>.json`.

**A moving camera plays on the timeline,** like everything else: a dolly keyed over the
scene's frame range moves in step with the character's idle, and starts over with it.

- Don't name a camera `orbit`: that's the app's built-in camera, and the name is taken.
- The **Starts with → Camera** setting in the Suite panel can be one of yours or `orbit`.

- A camera may hang from empties (a rig). They are exported with it.
- The app keeps the camera's **vertical** field of view and fits the width to the
  viewer's monitor. A wider monitor shows more at the sides. Set your render resolution to
  the monitor shape you care about most, and leave some room at the sides.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and
  either Focus Distance or a Focus Object. A focus object is followed as it moves, and
  may be animated and parented to the camera. Focus
  on the armature with a bone picked to keep, say, the head sharp.
- Animated F-Stop or Focus Distance doesn't come over. Animate a focus empty instead.
- The viewer can scale how much everything blurs with **Strength** under Depth of field in the Look tab.
  100% is your F-Stop exactly; 200% is as if you'd halved it. They can also keep what's in
  front of the focus sharp with **Foreground blur**.
- No blur grows wider than the viewer's **Max size**, 64 pixels across on screen by
  default. A very shallow focus in Blender reaches that limit sooner, and things far out
  of focus look less blurred than in your render.
- Aperture blades, rotation and ratio have no match in the app.
- The orbit camera is always there in addition to yours. Where it starts is set in
  [suite.toml](../suite-toml.md).

---

← [Fog and light shafts](fog.md) · [All recipes](../making-a-character.md#recipes) · [Sounds](sounds.md) →
