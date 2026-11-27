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

The add-on never writes `[post]`, so these are typed in by hand. The
[schema](../suite-toml.md) explains each as you type.

```toml
[post]
tonemapping = "agx"
exposure = 0.0              # EV stops, + is brighter
bloom = 0.1                 # 0 is off
bloom_threshold = 1.0       # how bright light must be to glow, as the Glare node's
chromatic_aberration = 0.0  # 0.005 is subtle

[post.vignette]
intensity = 0.3             # 0 is off
size = 1.0
falloff = 2.0
roundness = 1.0
color = [0.0, 0.0, 0.0]

[post.grain]
intensity = 0.04            # 0 is off, 0.1 heavy
size = 1.5                  # in screen pixels
colored = false

[post.grading]
warmth = 0.0                # -1 blue to 1 orange
tint = 0.0                  # -1 green to 1 magenta
hue_deg = 0.0
saturation = 1.0
contrast = 1.0
midtones_range = [0.2, 0.7]
# [post.grading.shadows], [post.grading.midtones] and [post.grading.highlights] each take
# saturation, contrast, gamma, gain and lift.

[post.lut]
name = "none"               # a file in luts/ without its extension
strength = 1.0
```

---

← [A color grade (LUT)](lut.md) · [All recipes](../making-a-character.md#recipes)
