# English, compiled into CharPaper and always complete. To translate, copy this file to
# `locales/<language>/charpaper_ui.ftl` next to the executable, where <language> is a tag
# such as `de-DE` or `ru-RU`, and translate the text after each `=`. Keep the names before
# it as they are. A string left out shows in English.

# How the language is named in the panel's language list: in the language itself.
language-name = English

## Panel

tab-scene = Scene
tab-graphics = Graphics
tab-sound = Sound
tab-system = System
quit = Quit

## Scene tab

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
section-camera = CAMERA
label-camera = Camera
camera-orbit = Orbit
section-environment = ENVIRONMENT
label-environment = Environment
label-shadows = Shadows
label-lock-orbit = Lock orbit

## Graphics tab

section-lighting = LIGHTING
label-brightness = Sky Brightness
section-quality = QUALITY
label-render-scale = Render scale
label-anti-aliasing = Anti-aliasing
label-depth-of-field = Depth of field
label-shadow-quality = Shadow quality
label-shadow-updates = Animated shadows
label-ambient-occlusion = Ambient occlusion
label-ambient-occlusion-quality = Occlusion quality
label-ambient-occlusion-softness = Occlusion softness
section-fog = FOG
label-fog = Fog
label-fog-quality = Quality
label-fog-dithering = Dithering
label-fog-resolution = Resolution
section-strokes = STROKES
label-strokes = Strokes
label-stroke-width = Width
label-stroke-strength = Strength
label-stroke-creases = Creases
label-stroke-hand-drawn = Hand-drawn
section-post-processing = POST PROCESSING
section-image = IMAGE
label-tonemapping = Tonemapping
label-exposure = Exposure
label-bloom = Bloom
label-bloom-threshold = Bloom threshold
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
value-blur = Blur
value-bokeh = Bokeh
value-very-low = Very low
value-low = Low
value-medium = Medium
value-high = High
value-ultra = Ultra
value-full = Full
value-half = Half
value-quarter = Quarter
value-every-frame = Every frame
value-every-other-frame = Every other frame
value-sharp = Sharp
value-soft = Soft
value-thin = Thin
value-thick = Thick
value-light = Light
value-strong = Strong

## Sound tab

section-sound = SOUND
label-master-volume = Volume
label-music-volume = Music
label-ambience-volume = Ambience
label-character-volume = Character
section-music = MUSIC
label-track = Track
label-playback = Playback
label-order = Order
value-play = Play
value-pause = Pause
value-in-order = In order
value-shuffle = Shuffle
value-repeat-one = Repeat one

## System tab

section-app = APP
label-mode = Show as
mode-wallpaper = Wallpaper
mode-window = Window
label-start-at-login = Start at sign-in
section-display = DISPLAY
# Above the panel, with several stages: which one the panel shows.
stage-name = Stage { $letter }
# After the name of a chosen monitor that is unplugged.
monitor-missing = not connected
label-layout = Layout
layout-mirror = Mirror
layout-span = Span
section-frame-rate = FRAME RATE
label-fps-limit = FPS limit
label-monitor = Performance monitor
value-max = Max
section-pause = PAUSE WHEN
label-pause-fullscreen = App fullscreen
label-pause-covered = Desktop covered
label-pause-battery = On battery
section-interface = INTERFACE
label-language = Language
# The choice that follows the system's languages instead of naming one.
language-system = System
label-theme = Theme
theme-slate = Slate
theme-midnight = Midnight
theme-light = Light
label-accent = Accent
label-accent-hue = Hue
label-accent-saturation = Saturation
label-accent-brightness = Brightness
label-opacity = Opacity
label-size = Size
# Which edge of the screen the panel keeps to.
label-side = Side
side-left = Left
side-right = Right

## Performance monitor, shown at the top of the screen. Units after numbers.

monitor-title = PERFORMANCE
monitor-ms = { $value } ms
monitor-mb = { $value } MB
monitor-frame-rate = Frame rate
monitor-fps = { $value } fps
monitor-frame-time = { $time } a frame
monitor-cpu = CPU
# How many of the computer's cores CharPaper keeps busy, on average.
monitor-cores = { $busy } of { $total } cores busy
monitor-main-thread = Main thread
monitor-render-thread = Render thread
monitor-recording = { $time } writing GPU commands
monitor-stage-assets = Assets
monitor-stage-specialize = Shader variants
monitor-stage-views = Views, screen
monitor-stage-views-note = waits here while the GPU or vsync is slower
monitor-stage-queue = Sorting draws
monitor-stage-prepare = GPU buffers
monitor-stage-render = Commands
monitor-stage-cleanup = Cleanup
# The memory section. "Memory" is all the RAM the process touches, as Task Manager counts it.
monitor-memory = Memory
# How much a kind of memory grew over the last minute, with its sign.
monitor-growth = { $mb } MB a minute
# Memory the process asked Windows for, touched or not.
monitor-committed = Committed
# What the app's own code holds, and in how many separate pieces.
monitor-heap = Heap
monitor-heap-blocks = { $count } blocks
monitor-gpu-memory = GPU memory
monitor-gpu-reserved = { $mb } MB reserved
monitor-gpu-objects = GPU objects
monitor-gpu-object-details = buffers { $buffers }, textures { $textures }, views { $views }, bind groups { $groups }, samplers { $samplers }, pipelines { $pipelines }, shaders { $shaders }
monitor-entities = Entities
monitor-render-entities = render world: { $count }
monitor-assets = Assets
monitor-asset-details = images { $images }, meshes { $meshes }, materials { $materials }, clips { $clips }, glTF { $gltfs }
monitor-graphics = Graphics
monitor-discrete = discrete
monitor-integrated = integrated
monitor-software = no GPU, drawn by the CPU
monitor-other-gpu = other
monitor-gpu = GPU
monitor-busy = { $percent } of the time
monitor-waiting = measuring…
monitor-unsupported = this GPU can't time its work
monitor-light-shadows = Light shadows
# Of the shadow maps there are, how many were drawn again this frame; the rest show what
# they showed before, since nothing in them changed.
monitor-shadow-maps = shadow maps: { $drawn } of { $count } drawn
monitor-prepass = Prepass, sun shadows
monitor-main-pass = Main pass
monitor-fog = Fog
monitor-effects = Effects
monitor-display = Screen, panel
monitor-scene = Scene
monitor-lights = { $count } lights
monitor-light-details = with shadows: { $shadowed }, in fog: { $fog }, fog boxes: { $boxes }
monitor-meshes = Meshes
# How many times a frame meshes are drawn: by the cameras, and into the lights' shadow maps.
monitor-mesh-details = drawn by cameras: { $drawn }, into shadow maps: { $shadows }

## Values shown in controls

value-on = On
value-off = Off

## Help line at the bottom of the panel: one short sentence or two about the setting under
## the pointer. Kept short so it fits in two lines beside the Quit button.
help-track = The environment's music playing now. Pick a track to play it.
help-playback = Pauses the music, and plays it on from where it stopped.
help-order = Which track comes next: the following one, any other, or the same again.
help-lock-orbit = Keeps the orbit camera where it is: dragging and scrolling over the scene no longer move it.
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
help-bloom-threshold = How bright a part must be to glow: 1.0 is white, lower lets dimmer lights glow too.
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
help-monitor = Shows at the top of the screen what drawing each frame costs the CPU and GPU, stage by stage.
help-render-scale = Draws the scene at a lower resolution to save power. The panel stays sharp.
help-anti-aliasing = Smooths jagged edges. TAA is smoothest but can trail behind movement.
help-depth-of-field = Blurs what's out of focus, like a camera lens. Bokeh looks best, Blur is faster.
help-shadow-quality = How sharp shadows are, on every stage. Low is fastest, with hard edges. High takes much more video memory.
help-shadow-updates = How often lamps redraw shadows that only animation moves. Every other frame halves their cost, and they trail the motion by a frame.
help-ambient-occlusion = Darkens creases and corners the sky's light can't reach, like where the character meets the floor. Needs anti-aliasing off or TAA, and looks smoothest with TAA.
help-ambient-occlusion-quality = How many samples shade each pixel. Lower is faster and grainier; TAA hides most of the grain.
help-ambient-occlusion-softness = How far the darkening spreads from creases and contacts. Sharp keeps it tight in the corners, Soft fades it over a wide area.
help-fog = Fog and light shafts, where the environment has them.
help-fog-quality = How finely the fog is drawn. Higher costs more.
help-fog-dithering = Hides banding in the fog with fine noise.
help-fog-resolution = Half draws the fog four times faster, and it is soft, so this rarely shows. Quarter is faster again but softens light shafts.
help-strokes = Inks the edges of what's on screen for a drawn look: outlines around shapes, and lines where surfaces fold.
help-stroke-width = How wide the lines are. A lower render scale makes them wider on screen.
help-stroke-strength = How much the lines darken what's under them. They keep its color, like ink.
help-stroke-creases = Also draws lines where a surface bends sharply, like folds and corners, not just around outlines.
help-stroke-hand-drawn = Lets the lines wander and swell a little and redraws them a few times a second, like hand-drawn animation.
help-pause-fullscreen = Stops the picture and sound while an app, such as a game, runs fullscreen.
help-pause-covered = Stops the picture and sound while windows cover the whole desktop.
help-pause-battery = Stops the picture and sound while the computer runs on battery.
help-mode = Wallpaper puts the scene behind the desktop icons, Window in an ordinary window. CharPaper restarts to switch.
help-start-at-login = Starts CharPaper whenever you sign in to this computer.
help-layout = For this monitor's stage. Mirror shows the whole scene on each of its monitors. Span spreads one wide view across them, as if they were a single screen.
help-master-volume = How loud all of CharPaper's sound is.
help-music-volume = The environment's music.
help-ambience-volume = The environment's background sounds, such as rain or wind.
help-character-volume = Sounds the character's animations make, such as footsteps.
help-language = The panel's language. System follows the system's language.
help-theme = The panel's colors. Light is easier to read over a bright scene.
help-accent = The color of what is picked or open. Click to set its hue, saturation and brightness.
help-accent-hue = Which color the accent is, around the color wheel: red, then yellow, green, cyan, blue, magenta.
help-accent-saturation = How vivid the accent is. At 0% it is gray.
help-accent-brightness = How light the accent is. At 0% it is black.
help-opacity = How much of the scene shows through the panel and the performance monitor.
help-size = How big the panel and the performance monitor are drawn.
help-side = The edge of the screen the panel keeps to.
