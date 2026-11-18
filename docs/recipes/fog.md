[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Fog and light shafts

Add a cube where the fog should be and give it a material with
a **Principled Volume** plugged into *Volume*, and nothing in *Surface*.

- The fog fills the object's bounding box, whatever its shape.
- Color, Density, Anisotropy and Absorption Color come over, as typed into the node.
  Values fed from other nodes don't, and neither do Emission and Blackbody.
- Patchy fog: feed *Density* from a **Noise Texture** (fBM, 3D) through any of Color
  Ramp, Map Range, Math and Float Curve nodes. Its *Vector* may be Generated or Object
  coordinates, through a Mapping node. The app draws Blender's own noise, up to *Detail* 3,
  without *Distortion* or *W*. Animate the Mapping node's *Location* and the fog drifts, at
  that animation's average speed, for as long as the app runs.
- The app's noise repeats every 16 units of the noise's own space: past a Scale of about
  16 across the box, the pattern repeats. Within that, it follows Blender's own pattern
  where the noise's coordinates are positive; with Object coordinates, the side of the box
  below the object's origin shows a different, alike pattern. For the closest match to a
  Cycles render, use Generated coordinates.
- The sky lights fog evenly, as it lights surfaces, even indoors where walls would keep it
  out in Cycles. Lamps add light shafts. Fog with neither a sky nor a lamp is invisible.
- A sun lights fog only while its *Shadow* is ticked.
- To keep a light out of the fog, set its *Light → Influence → Volume Scatter* to 0. Any
  value above 0 counts as fully on.

---

← [A sun that matches the sky](sun.md) · [All recipes](../making-a-character.md#recipes) · [A camera](camera.md) →
