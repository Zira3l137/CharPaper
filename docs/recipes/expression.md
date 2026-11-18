[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A facial expression

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

---

← [An animation](animation.md) · [All recipes](../making-a-character.md#recipes) · [Shape keys that follow the body](correctives.md) →
