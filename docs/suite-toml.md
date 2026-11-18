[CharPaper](../README.md) › [Docs](README.md)

# suite.toml

**On this page:** [The file is strict](#the-file-is-strict) · [Sections](#sections) · [Things worth knowing](#things-worth-knowing)

`suite.toml` is a plain text file at the top of the suite. The add-on writes it, and you
can edit it in any text editor. Only one line is required:

```toml
schema = 1
```

Everything else is optional, because the folders already say most of it.

- [`schemas/suite.example.toml`](../schemas/suite.example.toml) shows every section in use.
- [`schemas/suite.schema.json`](../schemas/suite.schema.json) describes every key. Keep the
  `#:schema` line at the top of your file and an editor with TOML support (VS Code with
  *Even Better TOML*, for one) completes keys and explains them as you type.

## The file is strict

A misspelt key, a default that doesn't exist, settings for an
environment or camera that isn't there, or a sound that's missing from `sounds/`: any of
these and the character doesn't load at all. `charpaper.log` says what's wrong and on
which line.

## Sections

| Section | What it's for |
|---------|---------------|
| top of the file | `schema = 1`, and `name`, shown in the app. Without a name, the folder's name is used. |
| `[character]` | `default_skin`, `default_animation`, and `model` when more than one `.glb` sits next to suite.toml. |
| `[animations.<name>]` | Names a clip and says how it plays: `file`, `clip` (the action's name), `mode` (`loop`, `once`, `pose`), `gaze`, and `cues` for sounds. |
| `[environment]` | `default`: the environment shown first. |
| `[environments.<name>]` | Per environment: `sky_strength`, `sky_rotation_deg`, `background` and `background_blur`, `shadows` on or off, `sun_follows_sky`, `exposure`, fog boxes, the lights and objects left out of fog and shadows, and `lights`: each light's `radius` and keyed `animation`. |
| `[post]` | The picture: `tonemapping`, `exposure`, `bloom`, `bloom_threshold`, `chromatic_aberration`, and under it `[post.vignette]`, `[post.grain]`, `[post.grading]`, `[post.lut]`. |
| `[camera]` | `default` camera, and where the orbit camera starts: `focus`, `radius`, `yaw_deg`, `pitch_deg`, and `f_stop` for depth of field. |
| `[cameras.<name>]` | An exported camera's depth of field. The add-on fills it in. |
| `[gaze]` | Which bones and shape keys follow the cursor, how far they turn, and how head, neck and eyes share a look. The add-on fills it in. |

## Things worth knowing

- **Without a default**, the first skin and the first environment in name order are
  shown, the orbit camera is used, and no animation plays.
- **Animation files not mentioned** in suite.toml are used whole, every clip under its
  action's name, looping. Once a file is mentioned, only the clips listed for it are used.
- **The orbit camera's `focus` is in the app's axes, where Y is up.** A point at Blender's
  (x, y, z) is written `[x, z, -y]`. One meter above the origin is `[0.0, 1.0, 0.0]`.
- **The model file is optional.** Without one, the armature is taken from the default
  skin, since every skin carries a copy of it.
- **Grading needs tonemapping.** With `tonemapping = "none"` the `[post.grading]` section
  does nothing.
- **What the add-on rewrites on every export:** the name, the *Starts with* defaults,
  every exported clip, each exported camera's depth of field, and each exported
  environment's fog and shadow lists and `lights`, and with a World its `sky_strength` and
  `sky_rotation_deg`. **What it leaves alone:** `[post]`, each environment's `shadows`, `sun_follows_sky`,
  `exposure`, `background` and `background_blur`, the orbit camera, and anything
  about files you didn't export this time. An empty field in the add-on means "don't
  touch that key".

---

[All docs](README.md)
