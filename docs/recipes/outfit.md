[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An outfit (skin)

Put every mesh of the outfit into one collection and add it under **Skins**. That means
the body too, or at least what shows of it.

- Meshes must be deformed by the character's armature, or parented to it or to one of its
  bones. A hat parented to the head bone works. A mesh that is neither stays where it is
  and doesn't follow the character.
- In a skin with more than one mesh object, each becomes a switch under **Outfit → Parts**
  in the app, named after the object. So a jacket as its own object can be taken off by
  the viewer. Name objects the way you'd want them listed, and join what should never be
  hidden on its own.
- **Modifiers and shape keys don't mix.** With *Apply modifiers* on, a mesh that has both
  may lose its shape keys. Apply the modifiers yourself on such meshes.

---

[All recipes](../making-a-character.md#recipes) · [An animation](animation.md) →
