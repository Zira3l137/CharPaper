[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A camera

Add the camera object under **Cameras**. If it's animated, pick its **Action**; the app
loops it.

- A camera may hang from empties (a rig). They are exported with it.
- The app keeps the camera's **vertical** field of view and fits the width to the
  viewer's monitor. A wider monitor shows more at the sides. Set your render resolution to
  the monitor shape you care about most, and leave some room at the sides.
- **Depth of field** comes from the camera's own *Depth of Field* panel: F-Stop, and
  either Focus Distance or a Focus Object. A focus object is followed as it moves, and
  may be animated and parented to the camera. Focus
  on the armature with a bone picked to keep, say, the head sharp.
- Animated F-Stop or Focus Distance doesn't come over. Animate a focus empty instead.
- The viewer can scale how much everything blurs with **Strength** in the Graphics tab.
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
