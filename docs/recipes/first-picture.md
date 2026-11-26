[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# The picture the viewer sees first

View transform, exposure, bloom, vignette, grain and color settings are yours to set as the
author. The easy way: tune them in the app's **Look** tab until the scene looks
right, then write the same numbers into the `[post]` section of suite.toml. The app never
writes to suite.toml itself; what a viewer changes is kept in their own `state.toml`, and
**Restore defaults** brings yours back.

To get close to your Blender viewport, set `tonemapping` (**View transform** in the app) to
match your *View Transform*:
`agx` for AgX, `blender_filmic` for Filmic. Without one the app uses `tony_mc_mapface`.

A sunlit scene and a lamp-lit room need very different exposure. Give each environment
its own with `exposure` under `[environments.<name>]`.

---

← [A color grade (LUT)](lut.md) · [All recipes](../making-a-character.md#recipes)
