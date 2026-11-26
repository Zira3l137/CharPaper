[CharPaper](../README.md) › [Docs](README.md)

# Translating, building and icons

## Translating the panel

English is built in. To add a language:

1. Copy [`crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl`](../crates/charpaper-ui/i18n/en-US/charpaper_ui.ftl)
   to `locales\<tag>\charpaper_ui.ftl` next to `charpaper.exe`, where `<tag>` is a
   language tag such as `de-DE`.
2. Translate the text after each `=`. Leave the names before it as they are.
3. Restart CharPaper and pick the language in **System → Interface**.

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
| `charpaper-bake` | Makes the sky files from a panorama. |
| `charpaper-drivers` | Runs Blender's shape key drivers. |
| `charpaper-gaze` | Works out where the character looks, and how neck, head and eyes share the turn. |
| `charpaper-audio` | Sound. |
| `charpaper-settings` | What `state.toml` holds. |
| `charpaper-monitor` | Measurements for the performance overlay. |
| `charpaper-wallpaper` | What any system must provide to be a wallpaper. |
| `charpaper-windows` | How Windows provides it. |

The Blender add-on is in [`tools/blender/charpaper_exporter`](../tools/blender/charpaper_exporter).
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
