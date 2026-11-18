[CharPaper](../README.md) › [Docs](README.md)

# Translating and building

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

---

[All docs](README.md)
