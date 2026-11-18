[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A sky

**On this page:** [Exporting the sky](#exporting-the-sky) · [When the sky isn't exported](#when-the-sky-isnt-exported) · [Good to know](#good-to-know) · [In the app](#in-the-app)

An environment's sky is a panorama, a picture of everything around. It shows behind the
scene, lights it, and gives shiny surfaces something to reflect. For a Sun light that
follows it, see [A sun that matches the sky](sun.md).

## Exporting the sky

Each environment in the add-on has a **Sky** row: a tick box and a World. Ticked, the
export puts that World into the environment's own folder as the panorama the app lights
the scene with: `environment/room/sky.hdr` for `environment/room.glb`. Leave the World
empty to use the scene's.

- A World that is just an *Environment Texture* into a *Background* goes over at full
  size. Its *Strength* becomes `sky_strength`, and a *Mapping* node's Z rotation becomes
  `sky_rotation_deg`. An `.hdr` or `.exr` in Linear Rec.709 is copied as it is; anything
  else Blender reads (another format, color space or EXR layout, a packed image) is
  written out as Blender sees it, into `sky.exr`.
- Any other World, such as a *Sky Texture*, a plain color, or mixed textures, is rendered
  with Cycles into a 2048×1024 `sky.exr`. That adds a few seconds to every export. A *Light
  Path* trick (a sky that looks different from how it lights) comes over as the camera
  sees it; use the app's **Background** setting for that instead.
- You can also put a panorama in the folder by hand. Its name doesn't matter as long as it
  has one; the folder name must match the environment's. Keep only one panorama in a
  folder.

## When the sky isn't exported

Only in these cases. The export reports the last four,
and **Validate** warns about a missing image or World beforehand:

| Case | What happens |
|---|---|
| *Sky* is unticked | The folder is left as it is. |
| The World field is empty and the folder holds a panorama you put there (any name but `sky.hdr` or `sky.exr`) | It's kept, in case it isn't the scene's World. Pick the World to replace it. |
| The environment isn't exported | Its *Export* box is unticked, or the export was for something else. |
| The World's image file is missing and not packed (Blender shows it pink) | The export stops with an error. Find it (*File > External Data > Find Missing Files*) or pack it. |
| The World field is empty and the scene has no World | No sky. The environment is lit by its own lights. |
| CharPaper is baking that sky at the moment (Windows) | The old files are in use, so the export stops with an error. Export again when the sky shows. |

## Good to know

- The first time the environment shows, the app makes three files from the panorama
  (`skybox.ktx2`, `specular.ktx2`, `diffuse.ktx2`). That takes a few seconds, and the sky
  appears when it's done.
- **Those files are never remade on their own.** The export deletes them when the panorama
  changed. After replacing a panorama by hand, delete the three `.ktx2` files yourself.
  Ship them with your character so nobody else has to wait.
- Only a rotation about Z (up) comes over from a Mapping node.
- A folder with a panorama and no `.glb` is an environment too: just a sky, lighting the
  character on its own. In the add-on, that's an environment with a World and no collection.
- **Sky strength** means the same as *Strength* on the World's Background node: 1 shows
  the panorama as it is, as Blender does at an exposure of 0.
## In the app

- In the app's **Graphics** tab, while the environment shows: **Sky strength**, **Sky
  rotation**, **Sun follows sky**, **Background** (how bright the sky looks behind the scene, apart from the
  light it gives; Off hides it) and **Background blur** (0% sharp to 100%, about what a
  Principled BSDF of Roughness 0.5 reflects; it needs the environment's reflection map).
  Write the values you like into suite.toml as `sky_strength`, `sky_rotation_deg`,
  `sun_follows_sky`, `background` and `background_blur` (from 0 to 1). An older suite's `brightness` still
  works: 1000 is a strength of 1, and an older `background_blur` of "low", "medium" or
  "high" reads as 0.71, 0.87 or 1.

---

← [An environment](environment.md) · [All recipes](../making-a-character.md#recipes) · [A sun that matches the sky](sun.md) →
