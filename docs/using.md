[CharPaper](../README.md) › [Docs](README.md)

# Using CharPaper

**On this page:** [Getting started](#getting-started) · [The panel](#the-panel) · [The mouse on the desktop](#the-mouse-on-the-desktop) · [Several monitors](#several-monitors) · [When it pauses](#when-it-pauses) · [Where your settings live](#where-your-settings-live) · [When something goes wrong](#when-something-goes-wrong) · [Command line](#command-line)

## Getting started

You need Windows 10 or 11 (64-bit) and a processor with AVX2. Most made since 2013 have
it; some budget Pentium, Celeron and Atom chips don't, and on those CharPaper won't start
at all.

1. Download one of the two above. You get a `.zip` file.
2. Right-click the file, choose **Extract All…**, then **Extract**. Extract it somewhere
   in your own folders, like Documents, not into Program Files: CharPaper saves its
   settings next to itself and can't write there.
3. Put a character into the `characters` folder. **CharPaper comes with none.** A
   character is a folder, like `characters\aki`, that you got from its author or
   [made yourself](making-a-character.md).
4. Double-click **charpaper.exe**.
5. If Windows shows "Windows protected your PC", click **More info**, then **Run anyway**.
   CharPaper isn't signed, which is why Windows doesn't recognize it yet.

To quit, click **Quit** at the bottom of the panel.

To update, download and extract the new version, then move the `characters` folder and
`state.toml`, where your settings are, from the old folder into the new one.

## The panel

The panel sits at the right edge of the screen. The small arrow button on that edge hides
it and brings it back. With **Auto-hide button** on in the System tab, that button
disappears a few seconds after you move the pointer away, and comes back as soon as you
point at its spot on the edge.

- **Point at a setting** and the line at the bottom of the panel says what it does.
- **Turn the mouse wheel over a value** to change it. Anywhere else on the panel, the
  wheel scrolls.
- **Click a section's title** to fold it away.

| Tab | What's there |
|-----|--------------|
| **Scene** | Which character, its outfit and the outfit's parts, animation, expression, whether it follows the cursor and how strongly, camera, environment and shadows. |
| **Graphics** | Sky strength, rotation, whether the sun follows the sky, background and its blur, quality settings (render scale, anti-aliasing, shadows, ambient occlusion), fog, depth of field (shape, strength, max size, quality, foreground blur, bokeh highlights), ink strokes, and post processing (tonemapping, exposure, bloom, color, vignette, grain). |
| **Sound** | Volume for everything, and for music, ambience and the character's sounds each. The music track, play and pause, and playing order. |
| **System** | Wallpaper or window, start with Windows, monitors, frame rate limit, performance overlay, when to pause, and the panel's language, colors, size, side and auto-hiding button. |

A row only shows when it has something to offer. No camera row means the character has no
extra cameras; no Gaze section means its author didn't set gaze up. In **Graphics**, Fog
shows only in an environment with fog, Depth of field only through a camera that asks for
it, and Ambient occlusion only with a sky and anti-aliasing other than 4x, which it can't
run with. A setting's finer rows hide while it's off.

**Restore defaults**, under Post processing, takes the picture back to how the
character's author set it.

If your computer struggles, lower **Render scale**, set **Shadow quality** to Low, turn
on fog **Jitter** and lower fog **Quality** (or turn **Fog** off), lower depth of field
**Quality** or **Max size**, and lower the **FPS limit**. The **Performance overlay** shows what each
frame costs.

## The mouse on the desktop

With the **Orbit** camera chosen (it's the one every character has):

| Do this over the desktop | To |
|--------------------------|----|
| Hold the **right** button and drag | Turn around the character |
| Hold the **middle** button and drag | Slide the view sideways and up or down |
| Turn the **wheel** | Move closer or farther |

**Lock orbit** in the Scene tab switches all three off, so you can't nudge the view by
accident. Cameras made by the character's author can't be moved.

Your desktop icons work as always. CharPaper only listens to the mouse over the desktop
and never takes a click away from anything.

## Several monitors

By default the wallpaper is on your main monitor only. In **System → Display** every
monitor has a button. Click it to step through **Stage A**, **Stage B**, …, and **Off**.

- A **stage** is one scene: a character with its own outfit, camera, environment and
  picture settings. Monitors on the same stage show the same scene.
- Give two monitors different stages to have a different character, or the same one set
  up differently, on each.
- When monitors share a stage, **Layout** picks between **Mirror** (the whole picture on
  each) and **Span** (one wide picture across them).
- The panel always shows and changes the stage under the mouse pointer.
- You hear the stage the pointer is on. Move to a monitor with another stage and the sound
  follows after a moment.

## When it pauses

CharPaper stops drawing and playing sound, to leave the computer alone, when:

- the screen is locked or the screen saver runs (always);
- an app runs fullscreen, such as a game (**Fullscreen app**, on by default);
- windows cover the whole desktop (**Desktop covered**, on by default);
- the computer runs on battery (**On battery**, off by default).

## Where your settings live

Everything is in the folder you extracted. Nothing is installed, so deleting the folder
removes CharPaper. The one thing kept outside it is **Start with Windows**, which is an
entry in Windows' own list of startup apps: turn it off before deleting the folder, and
turn it on again after moving the folder.

| File or folder | What it is |
|----------------|------------|
| `characters\` | Your characters, one folder each. |
| `state.toml` | Every choice you made in the panel. Delete it to start fresh. |
| `charpaper.log` | What happened during the last run. `charpaper.previous.log` is the run before. |
| `locales\` | Translations of the panel. |
| `schemas\` | Reference files for people making characters. |
| `run-windowed.bat` | Starts CharPaper in an ordinary window. |
| `LICENSE-MIT`, `LICENSE-APACHE` | CharPaper's license: either one, your choice. |
| `THIRD-PARTY-LICENSES.html` | Every library inside CharPaper, and its license. |

## When something goes wrong

- **The wallpaper doesn't show.** Double-click `run-windowed.bat`. CharPaper opens in an
  ordinary window and stays that way until you set **System → App → Show as** back to
  Wallpaper.
- **The screen is empty or dark blue.** There is no character in `characters`, or the one
  chosen failed to load. `charpaper.log` says which and why.
- **The character is black.** Its environment has no lights. Pick another environment, or
  tell its author.
- **A frozen picture stays on the desktop after a crash.** On Windows versions where that
  can happen, CharPaper leaves a small helper running to clean it up, which is why Task
  Manager may show a second `charpaper.exe`. If the picture stays anyway, start CharPaper
  and quit it with the Quit button.
- **Settings went back to defaults.** `state.toml` couldn't be read. CharPaper sets a
  broken one aside as `state.toml.bad` instead of deleting it.

When reporting a problem, attach `charpaper.log`.

## Command line

`charpaper.exe --help` lists everything. The useful ones:

| Option | What it does |
|--------|--------------|
| `--windowed` | Runs in an ordinary window. Remembered until you switch back in the System tab. |
| `--suite <name>` | Shows the character in `characters\<name>`. |
| `--language <tag>` | Sets the panel's language, like `ru-RU`. Remembered. |
| `--bake-environments <folder>` | Makes the missing sky files for every environment of the suite in that folder, then exits. With `--dry-run` it only lists them. |
| `--benchmark` | Measures what each graphics setting costs for about two minutes and writes `benchmark.txt`. Your settings stay as they are. |
| `--no-input-forwarding` | The wallpaper stops reacting to the mouse. |
| `--log-level <level>` | How much goes into the log: `off`, `error`, `warn`, `info`, `debug`, `trace`. |

`run-windowed.bat` passes its arguments on, so `run-windowed.bat --suite aki` works too.

---

[All docs](README.md)
