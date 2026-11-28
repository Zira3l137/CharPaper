[CharPaper](../README.md) › [Docs](README.md)

# Translating, building and icons

## Translating the panel

English is built in. To add a language:

1. Copy [`crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl`](../crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl)
   to `locales\<tag>\charpaper_ui.ftl` next to `charpaper.exe`, where `<tag>` is a
   language tag such as `de-DE`.
2. Translate the text after each `=`. Leave the names before it as they are.
3. Restart CharPaper and pick the language in **Settings → Interface**.

A line you leave out shows in English, so a half-done translation works. Russian comes
with the app. Translations are welcome as pull requests into [`locales/`](../locales).

## Building from source

You need the current stable [Rust](https://rustup.rs) and, on Windows, the MSVC build
tools that rustup offers to install.

```
cargo run -p charpaper -- --windowed
cargo build --release -p charpaper
```

- The app looks for `characters` next to the executable, so under `cargo run` that is
  `target\debug\characters`.
- A release build takes a long time. It trades build time for speed everywhere it can.
- `--features diagnostics` adds troubleshooting options such as `--inspect`, and
  `--features profile` connects the app to the Tracy profiler.

The code is split so that the Windows-specific part stays small and another system can be
added beside it:

| Crate | What it does |
|-------|--------------|
| `charpaper` | The executable: start-up, command line, saving, frame pacing. |
| `charpaper-scene` | Everything drawn: character, environment, cameras, effects. |
| `charpaper-ui` | The panel and the performance overlay. |
| `charpaper-suite` | Reads a suite folder and suite.toml. No engine inside, so it's quick to test. |
| `charpaper-pack` | Reads and writes the [suite pack](chr-format.md), `suite.chr`. |
| `charpaper-gltf` | Writes a suite's glTF files from what the add-on reads out of Blender, as Blender's exporter would. |
| `charpaper-py` | The Blender add-on's Python module, `charpaper_pack`: packs, sky baking, and the glTF files of a packed export. |
| `charpaper-bake` | Makes the sky files from a panorama. |
| `charpaper-drivers` | Runs Blender's shape key drivers. |
| `charpaper-gaze` | Works out where the character looks, and how neck, head and eyes share the turn. |
| `charpaper-audio` | Sound. |
| `charpaper-settings` | What `state.toml` holds. |
| `charpaper-monitor` | Measurements for the performance overlay. |
| `charpaper-wallpaper` | What any system must provide to be a wallpaper. |
| `charpaper-windows` | How Windows provides it. |

The Blender add-on is in [`tools/blender/charpaper_exporter`](../tools/blender/charpaper_exporter).

### The add-on's Python module

The add-on packs suites and bakes skies through `charpaper_pack`, a Python module written
in Rust, in [`crates/charpaper-py`](../crates/charpaper-py), so it runs the app's own code
for both. A packed export also writes its model, skin, environment, camera and clip files
through it, with [`crates/charpaper-gltf`](../crates/charpaper-gltf), which is much faster
than Blender's glTF exporter. It's built as a wheel with [maturin](https://www.maturin.rs),
1.9.4 or newer:

```
pip install maturin
maturin build -m crates/charpaper-py/Cargo.toml --out dist
pip install dist/charpaper_pack-*.whl
python -m unittest discover crates/charpaper-py/tests
```

- One wheel per system fits every Blender from 4.2 on, since it's built for Python's stable
  ABI from 3.11 up.
- It builds with the `wheel` profile: the release profile, except that a panic becomes a
  Python exception instead of closing Blender.
- [`charpaper_pack.pyi`](../crates/charpaper-py/charpaper_pack.pyi) describes everything
  the module offers, for editors and for reading.
- The Build workflow makes and tests a wheel for each platform Blender runs on, Linux's in a
  manylinux_2_28 container so it runs on every glibc Blender does, and
  `packaging/addon_zips.py` puts each in its own add-on zip.
- On Windows the rest of the workspace builds without Python installed. On Linux,
  `cargo build` needs a Python 3.11 or newer to link this crate against.

### Working on the add-on from a checkout

Link the add-on's folder into Blender's user extensions, so Blender loads it straight from
the checkout and every change shows after a restart. On Windows, from the repository root,
with Blender closed and the add-on not installed from a zip:

```
cmd /c mklink /J "%APPDATA%\Blender Foundation\Blender\4.2\extensions\user_default\charpaper_exporter" tools\blender\charpaper_exporter
```

On Linux the folder is `~/.config/blender/4.2/extensions/user_default/`, on macOS
`~/Library/Application Support/Blender/4.2/extensions/user_default/`, and `ln -s` links it.
Then enable *CharPaper Exporter* in *Edit > Preferences > Add-ons*.

The link brings the Python files only. Blender installs `charpaper_pack` with the add-on only
from a zip that carries its wheel, so install the module into Blender's own Python yourself,
and again after every change to `charpaper-py`, `charpaper-gltf` or `charpaper-pack`:

```
maturin build -m crates/charpaper-py/Cargo.toml --out dist
"<blender>/4.2/python/bin/python" -m pip install --force-reinstall --no-deps dist/charpaper_pack-<version>-*.whl
```

- Close Blender first: on Windows it holds the module's file open, and pip can't replace it.
- Blender in Program Files needs an administrator's terminal for the second line.
- Each Blender version has its own Python and extensions folder: do both for each you test.
- The panel shows *Export unpacked* once Blender finds the module, and says it isn't
  installed when it doesn't.
- A module older than the Python files fails the export with *the charpaper_pack module at …
  doesn't match this add-on*, naming the folder Blender loaded it from. Install it again.

### Checking packed exports against Blender's

A packed export writes the model, skins, environments, cameras and clips with `charpaper_pack`
instead of Blender's glTF exporter, and they should come out the same.
[`tools/blender/compare`](../tools/blender/compare) checks that. It builds a test character
and exports it both ways three times: as it is, with *Deform bones only*, and with every second
frame. Then it compares every model, skin, environment, camera and clip file between the two,
vertex by vertex and frame by frame:

```
blender -b --factory-startup --python tools/blender/compare/run.py -- <work folder>
```

Blender's Python needs `charpaper_pack` for this, installed as
[above](#working-on-the-add-on-from-a-checkout), or run the script with the
[bpy module](https://pypi.org/project/bpy/) from PyPI in a virtual environment that has the
wheel. It takes about 20 seconds and prints each difference it finds, if any.
Run it after changing `charpaper-gltf`, `gltf_write.py`, `clips.py`, `isolate.py` or
`image_cache.py`, and with each Blender version the add-on supports. It also fails when an
export leaves data behind in the .blend.

[RELEASING.md](../RELEASING.md) explains how versions and nightly builds are made.

## The panel's icons

The panel draws its icons from a font of its own,
[`crates/charpaper-ui/assets/icons.ttf`](../crates/charpaper-ui/assets/icons.ttf), made from the
SVGs in [`resources/icons`](../resources/icons). To add or change an icon:

1. Draw it as lines on a 24×24 grid, 2 units thick, as the others are. Copy one of them for
   the `<svg>` line at the top, which sets that up. `fill="currentColor"` fills a shape too,
   and `stroke="none"` leaves its line out. Only `path`, `circle`, `rect`, `line` and
   `polyline` are understood, without transforms.
2. Save it in `resources/icons` under a lower-case name with underscores, such as
   `arrow_left.svg`. In Rust it's then `Icon::ArrowLeft`.
3. Run the script, which needs two Python packages:

   ```
   pip install fonttools skia-pathops
   python tools/icons.py
   ```

   It writes the font and `crates/charpaper-ui/src/icons/glyphs.rs`, the icons' names in
   Rust. Commit both with the SVG.

Show icons at 16 pixels or larger: any smaller and their lines blur.

---

[All docs](README.md)
