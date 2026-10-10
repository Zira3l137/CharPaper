# English, compiled into CharPaper and always complete. To translate, copy this file to
# `locales/<language>/charpaper_ui.ftl` next to the executable, where <language> is a tag
# such as `de-DE` or `ru-RU`, and translate the text after each `=`. Keep the names before
# it as they are. A string left out shows in English.

# How the language is named in the panel's language list: in the language itself.
language-name = English

## Panel

tab-character = Character
tab-scene = Scene
tab-render = Render
tab-system = System
quit = Quit

## Character tab

section-suite = CHARACTER
label-suite = Character
section-outfit = OUTFIT
section-parts = Parts
section-animation = ANIMATION
label-animation = Animation
section-expression = EXPRESSION
label-expression = Expression
section-gaze = GAZE
label-follow-cursor = Follow cursor
expression-neutral = Neutral

## Scene tab

section-camera = CAMERA
label-camera = Camera
section-environment = ENVIRONMENT
label-environment = Environment
label-brightness = Brightness
label-shadows = Shadows
section-image = IMAGE
label-tonemapping = Tonemapping
label-exposure = Exposure
label-bloom = Bloom
label-chromatic-aberration = Aberration
section-color = COLOR
label-warmth = Warmth
label-tint = Tint
label-saturation = Saturation
label-contrast = Contrast
label-lut = LUT
label-lut-strength = LUT strength
section-film = FILM
label-vignette = Vignette
label-vignette-size = Vignette size
label-grain = Grain
label-grain-size = Grain size
restore-look = Restore defaults
camera-orbit = Orbit

## Render tab

section-frame-rate = FRAME RATE
label-fps-limit = FPS limit
section-quality = QUALITY
label-render-scale = Render scale
label-anti-aliasing = Anti-aliasing
label-depth-of-field = Depth of field
section-fog = FOG
label-fog = Fog
label-fog-quality = Quality
label-fog-dithering = Dithering
section-pause = PAUSE WHEN
label-pause-fullscreen = App fullscreen
label-pause-covered = Desktop covered
label-pause-battery = On battery
value-max = Max
value-blur = Blur
value-bokeh = Bokeh
value-very-low = Very low
value-low = Low
value-medium = Medium
value-high = High
value-ultra = Ultra

## System tab

section-sound = SOUND
label-master-volume = Volume
label-music-volume = Music
label-ambience-volume = Ambience
label-character-volume = Character
section-interface = INTERFACE
label-language = Language
# The choice that follows the system's languages instead of naming one.
language-system = System

## Values shown in controls

value-on = On
value-off = Off

## Help line at the bottom of the panel: one short sentence or two about the setting under
## the pointer. Kept short so it fits in two lines beside the Quit button.

help-panel = Point at a setting to see what it does. The mouse wheel over a value changes it.
help-suite = Which character to show, from the folders in characters.
help-animation = What the character is doing.
help-expression = The look on the character's face.
help-follow-cursor = The character's head and eyes follow the mouse pointer.
help-camera = Where the scene is seen from. Orbit moves with the mouse over the scene.
help-environment = The surroundings, with their own light and sky.
help-brightness = How bright the environment's sky is, and the light it gives.
help-shadows = Whether the environment's lights cast shadows. Off is faster.
help-tonemapping = How bright colors are fitted to the screen. Each one sets a different mood.
help-exposure = Brightens or darkens the whole picture, as a camera would.
help-bloom = A glow around the brightest parts of the picture.
help-chromatic-aberration = Colored fringes toward the edges, like a cheap lens.
help-vignette = Darkens the corners of the picture.
help-vignette-size = How far the darkened corners reach in.
help-grain = Fine noise over the picture, like film.
help-grain-size = How coarse the grain is.
help-warmth = Shifts colors toward orange or blue.
help-tint = Shifts colors toward magenta or green.
help-saturation = How vivid colors are. At 0% the picture is black and white.
help-contrast = How far darks and lights are pushed apart.
help-lut = A ready-made color grade from the character's luts folder.
help-lut-strength = How much of the color grade to apply.
help-fps-limit = How many frames a second to draw. Fewer save power.
help-render-scale = Draws the scene at a lower resolution to save power. The panel stays sharp.
help-anti-aliasing = Smooths jagged edges. TAA is smoothest but can trail behind movement.
help-depth-of-field = Blurs what's out of focus, like a camera lens. Bokeh looks best, Blur is faster.
help-fog = Fog and light shafts, where the environment has them.
help-fog-quality = How finely the fog is drawn. Higher costs more.
help-fog-dithering = Hides banding in the fog with fine noise.
help-pause-fullscreen = Stops the picture and sound while an app, such as a game, runs fullscreen.
help-pause-covered = Stops the picture and sound while windows cover the whole desktop.
help-pause-battery = Stops the picture and sound while the computer runs on battery.
help-master-volume = How loud all of CharPaper's sound is.
help-music-volume = The environment's music.
help-ambience-volume = The environment's background sounds, such as rain or wind.
help-character-volume = Sounds the character's animations make, such as footsteps.
help-language = The panel's language. System follows the system's language.
