[CharPaper](../README.md) › [Docs](README.md)

# suite.toml

**On this page:** [The file is strict](#the-file-is-strict) · [Sections](#sections) · [Things worth knowing](#things-worth-knowing)

`suite.toml` is a plain text file at the top of the suite. The add-on writes it, and you
can edit it in any text editor. Only one line is required:

```toml
schema = 2
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
| top of the file | `schema = 2`, and `name`, shown in the app. Without a name, the folder's name is used. |
| `[timeline]` | The loop everything plays on: `start` and `end` frame, and `fps`. Defaults: 1, 250 and 24. The add-on copies it from Blender's *Output Properties*. |
| `[character]` | `default_skin`, `default_clip`, and `model` when more than one `.glb` sits next to suite.toml. |
| `[character.clips.<name>]` | How one of the character's clips plays. `<name>` is a file name in `clips/` without its extension. `mode` (`timeline`, the default, `once` or `hold`), `label` (shown in the app), `gaze` (false to stop following the cursor during it), `reaction_only` (true keeps it out of the app's list, for clips only hotspots play), and `cues` for sounds: `[{ at = 0.4, sound = "giggle" }]`, `at` in seconds from the clip's start. |
| `[character.expressions.<name>]` | An expression's `label`. `<name>` is a file name in `expressions/`. |
| `[environment]` | `default`: the environment shown first. |
| `[environments.<name>]` | Per environment: `sky_strength`, `sky_rotation_deg`, `background` and `background_blur`, `shadows` on or off, `sun_follows_sky`, `exposure`, `[…fog.<object>]` fog boxes, and `no_volume_scatter` and `no_shadows`, the lights and objects left out of fog and shadows. |
| `[environments.<name>.actors.<object>.clips.<clip>]` | How a prop's clip plays: `mode` and `cues`, as for the character. The clips are the files in `environment/<name>/clips/<object>/`. |
| `[hotspots.<name>]` | A place that reacts to the cursor: `trigger` (`click` or `hover`), `areas`, `clips`, `layer`, `expression`, `cooldown` and `during`. See below. |
| `[post]` | The picture: `tonemapping`, `exposure`, `bloom`, `bloom_threshold`, `chromatic_aberration`, and under it `[post.vignette]`, `[post.grain]`, `[post.grading]`, `[post.lut]`. |
| `[camera]` | `default` camera, and where the orbit camera starts: `focus`, `radius`, `yaw_deg`, `pitch_deg`, and `f_stop` for depth of field. |
| `[cameras.<name>]` | An exported camera's depth of field. The add-on fills it in. |
| `[gaze]` | Which bones and shape keys follow the cursor, how far they turn, and how head, neck and eyes share a look. The add-on fills it in. |

### A hotspot

```toml
[hotspots.Belly]
trigger = "click"
expression = "happy"
cooldown = 2.0
during = ["sit_idle", "sit_reading"]
areas = [{ bone = "spine.002", length = 0.15, radius = 0.12 }]
clips = [{ clip = "giggle" }, { clip = "giggle_2" }, { actor = "Door", clip = "slam" }]
```

- `areas`: each a capsule around a bone: `bone`, its `length` (from the bone's head along
  the bone) and `radius`, in meters. Add `actor = "<armature object>"` for a bone of an
  environment's armature; without it, the bone is the character's.
- `clips`: what plays. Without `actor` it's a clip of the character; with it, a clip of
  that prop in the environment shown. Several clips of one actor: one of them at random.
- `layer = true` plays over other reactions instead of replacing them.
- `during`: the character's clips during which it can go off, of those the viewer picks to
  play on and on (`timeline` or `hold`). A `once` clip picked over one of them counts as
  that one. Left out: any. When the animation changes to one not listed, its clips stop.
- `expression` must name one of the character's expressions, and each clip of the
  character, in `clips` and in `during`, must exist, or the character doesn't load. A prop's clips and bones are
  looked up when its environment shows; one that isn't there is skipped.

## Things worth knowing

- **Without a default**, the first skin and the first environment in name order are
  shown, the orbit camera is used, and no animation plays.
- **Every clip in `clips/` is used**, whether suite.toml mentions it or not. One that isn't
  mentioned plays on the timeline, under its file's name. The same goes for
  `expressions/`.
- **Older files don't load.** A suite.toml with `schema = 1` (with `[animations.…]`
  sections) is from an older add-on: the log says so, and exporting again with the
  current add-on fixes it. Open the .blend with the new add-on installed first: it moves
  your old settings over by itself.
- **The orbit camera's `focus` is in the app's axes, where Y is up.** A point at Blender's
  (x, y, z) is written `[x, z, -y]`. One meter above the origin is `[0.0, 1.0, 0.0]`.
- **The model file is optional.** Without one, the armature is taken from the default
  skin, since every skin carries a copy of it.
- **Grading needs tonemapping.** With `tonemapping = "none"` the `[post.grading]` section
  does nothing.
- **What the add-on rewrites on every export:** the name, `[timeline]`, the *Starts with*
  defaults, the clips tables of every actor it exported, the expressions' labels, the
  hotspots (once you've made one in the add-on), each exported camera's depth of field, and
  each exported environment's fog and shadow lists, and with a World its `sky_strength` and
  `sky_rotation_deg`. Before writing, it saves the old file as `suite.toml.bak`. **What it leaves alone:** `[post]`, each environment's `shadows`, `sun_follows_sky`,
  `exposure`, `background` and `background_blur`, the orbit camera, and anything
  about files you didn't export this time. An empty field in the add-on means "don't
  touch that key".

---

[All docs](README.md)
