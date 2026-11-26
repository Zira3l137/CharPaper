[CharPaper](../README.md) › [Docs](README.md)

# suite.toml

**On this page:** [The file is strict](#the-file-is-strict) · [Sections](#sections) · [A hotspot](#a-hotspot) · [Physics](#physics) · [Things worth knowing](#things-worth-knowing)

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
| `[environments.<name>]` | Per environment: `sky_strength`, `sky_rotation_deg`, `background` and `background_blur`, `shadows` on or off, `sun_follows_sky`, `exposure`, `[…fog.<object>]` fog boxes, `no_volume_scatter` and `no_shadows`, the lights and objects left out of fog and shadows, and `wind`. |
| `[environments.<name>.actors.<object>]` | A prop's `default_clip`: the `timeline` or `hold` clip it plays by itself. Left out: its first `timeline` clip by name, else its first `hold` one. The add-on writes it from the ★. |
| `[environments.<name>.actors.<object>.clips.<clip>]` | How a prop's clip plays: `mode` and `cues`, as for the character. The clips are the files in `environment/<name>/clips/<object>/`. |
| `[hotspots.<name>]` | Something that reacts: to a click or the cursor resting on a place, or to the mouse keeping still or coming back. `trigger`, `after`, `areas`, `clips`, `while`, `then`, `during`, `layer`, `expression` and `cooldown`. See below. |
| `[physics.chains.<name>]` | Bones that swing by themselves: `bones`, `length`, `radius`, `stiffness`, `drag`, `gravity` and `cursor`. See below. |
| `[physics.colliders.<name>]` | A capsule around a bone that chains keep out of: `bone`, `length` and `radius`. |
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
during = [{ clip = "sit_idle" }, { clip = "sit_reading" }]
areas = [{ bone = "spine.002", length = 0.15, radius = 0.12 }]
clips = [
    { clip = "giggle" },
    { clip = "giggle_2" },
    { clip = "annoyed", from = 5 },
    { actor = "Door", clip = "slam" },
]

[hotspots.Pet]
trigger = "hover"
areas = [{ bone = "head", length = 0.2, radius = 0.1 }]
clips = [{ clip = "lean_in" }]
while = [{ clip = "purr" }]
then = [{ clip = "content_idle" }]

[hotspots.Nap]
trigger = "idle"
after = 120
clips = [{ clip = "lie_down" }]
while = [{ clip = "sleeping" }, { actor = "Lamp", clip = "off" }]
```

- `trigger`: `click`, `hover`, `idle` (the mouse keeps still for `after` seconds, 60 if
  left out, and again every `after` seconds while it does) or `welcome` (the mouse moves
  on the desktop after keeping still, or being elsewhere, for `after` seconds, 300 if left
  out). Only `click` and `hover` have `areas`; only `idle` and `welcome` have `after`.
- `areas`: each a capsule around a bone: `bone`, its `length` (from the bone's head along
  the bone) and `radius`, in meters. Add `actor = "<armature object>"` for a bone of an
  environment's armature; without it, the bone is the character's.
- `clips`: what plays. Without `actor` it's a clip of the character; with it, a clip of
  that prop in the environment shown. Several clips of one actor: one of them at random.
  `from = 5` plays that clip only from the 5th time in a row on, instead of those with a
  lower `from`. In a row means going off again within 3 seconds of being able to, or for
  `idle`, without the mouse moving in between.
- `layer = true` plays over other reactions instead of replacing them.
- `while`: what each actor named plays on and on for as long as the hotspot is held: its
  button down for `click`, the cursor on it for `hover`, the mouse still for `idle`. A
  `timeline` or `hold` clip, at most one per actor, and none for `welcome`. `clips` play
  once over them, fading into them as they end. A `click` hotspot with `while` goes off as
  the button is pressed, rather than when it's let go.
- `during`: animations it waits for. For each actor named in it, what that actor plays on
  and on must be one of its clips listed: the character's animation the viewer picked
  (a `once` clip picked over it counts as that one), or a prop's own clip, or the one
  `then` switched it to. Left out: any. When an animation changes to one not listed, the
  hotspot's clips stop.
- `then`: what each actor named plays on and on afterwards: once the hotspot lets go, or
  if it has no `while`, once its `clips` end. At most one per actor, a `timeline` or `hold`
  clip, and for the character, not one only for hotspots. It switches at once under the
  reaction fading out, so the reaction's last pose fades straight into it. The character's
  becomes the viewer's pick, as if picked in the panel, and is remembered. A prop's goes
  back to its own when its environment is shown again. Until a hotspot has switched, no
  other goes off.
- Every clip named, in `clips`, `while`, `during` and `then`, and `expression`, must exist, or the
  character doesn't load. Bones are looked up when they're shown; one that isn't there is
  skipped.

### Physics

```toml
[physics.chains.Ponytail]
bones = ["hair.001", "hair.002", "hair.003", "hair.004"]
length = 0.08
radius = 0.02
stiffness = 0.1
drag = 0.15
gravity = 1.0

[physics.colliders.head]
bone = "head"
length = 0.2
radius = 0.09

[environments.garden.wind]
direction = [1.0, 0.0, 0.0]
strength = 3.0
gusts = 0.4
```

- A chain's `bones` go from its first bone down, each the child of the one before. `length`
  is the last bone's, from its head to its tail, in meters: the others end where the next
  begins. `radius` is how far the bones' tails keep from colliders, 0 if left out.
- `stiffness` (0.2 if left out) and `drag` (0.1) go from 0 to 1. `gravity` (1) is a share of
  Earth's. What they do is in [The four settings](recipes/physics.md#the-four-settings).
- `cursor` is what the pointer does to the chain: `"pushes"` (if left out), `"ignores"`, or
  `"only"`, for a chain that follows the animation exactly until the pointer pushes it,
  which gravity, wind and colliders leave alone. See [The cursor](recipes/physics.md#the-cursor).
- A bone can be in only one chain, and a chain needs at least one bone, or the character
  doesn't load. A chain whose bones aren't on the outfit worn is skipped; one with only some
  of them is skipped too, and the log says so.
- A collider is a capsule from the bone's head along the bone for `length`, grown by
  `radius`, in meters. Every chain keeps out of every collider.
- `wind` sits in an environment's table. `direction` is the way it blows, in the app's axes
  (below), of any length but not zero. `strength` is in m/s², as gravity's 9.81. `gusts` go
  from 0, steady, to 1, which drops to calm and rises to twice as strong; 0 if left out.

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
- **The orbit camera's `focus`, and the wind's `direction`, are in the app's axes, where Y is up.** A point at Blender's
  (x, y, z) is written `[x, z, -y]`. One meter above the origin is `[0.0, 1.0, 0.0]`.
- **The model file is optional.** Without one, the armature is taken from the default
  skin, since every skin carries a copy of it.
- **Grading needs tonemapping.** With `tonemapping = "none"` the `[post.grading]` section
  does nothing.
- **What the add-on rewrites on every export:** the name, `[timeline]`, the *Starts with*
  defaults, the clips tables of every actor it exported, the expressions' labels, the
  hotspots and the physics (once you've made one in the add-on), each exported camera's
  depth of field, and each exported environment's fog and shadow lists and wind, and with a World its `sky_strength` and
  `sky_rotation_deg`. Before writing, it saves the old file as `suite.toml.bak`. **What it leaves alone:** `[post]`, each environment's `shadows`, `sun_follows_sky`,
  `exposure`, `background` and `background_blur`, the orbit camera, and anything
  about files you didn't export this time. An empty field in the add-on means "don't
  touch that key".

---

[All docs](README.md)
