[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An animation

An animation is a Blender action on the armature. Add a file under **Animations**, press
refresh, tick the actions.

- **One file, many clips.** Group them however you like. All clips of all files end up in
  one list in the app, so names must be unique across files.
- **Mode** decides how a clip plays:
  - *Loop* repeats forever.
  - *Once* plays through, then goes back to the *Starts with* animation.
  - *Pose* holds the action's first frame. Use it for still poses.
- **Shown as** gives a clip a nicer name in the app than its action has.
- Clips blend into each other over a quarter of a second, so the first and last frame of
  a loop should match, but two different clips don't have to line up.
- Without a *Starts with* animation, the character stands in its rest pose.

---

← [An outfit (skin)](outfit.md) · [All recipes](../making-a-character.md#recipes) · [A facial expression](expression.md) →
