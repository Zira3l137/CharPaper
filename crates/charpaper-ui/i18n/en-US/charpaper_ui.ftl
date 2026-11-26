# English, compiled into CharPaper and always complete. To translate, copy this file to
# `locales/<language>/charpaper_ui.ftl` next to the executable, where <language> is a tag
# such as `de-DE` or `ru-RU`, and translate the text after each `=`. Keep the names before
# it as they are. A string left out shows in English.
#
# How the text is written, so the panel reads the same everywhere:
# - Section titles (section-*) and the performance overlay's title are in capitals.
# - Tab names, setting names (label-*) and choices (value-* and the like) start with a
#   capital and go on in lower case, as in "Sky strength" or "Every other frame". Names
#   keep their own spelling: CharPaper, LUT, TAA, AgX, Blender.
# - Notes that follow something on the same line start in lower case, as in "not connected"
#   or "with shadows: 2".
# - Help lines (help-*) are whole sentences that end in a full stop. Where Blender has the
#   same setting, they name it, since most people who make characters know it from there.
# - A choice translates on its own, so it can agree with the setting it belongs to.

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
section-parts = PARTS
section-animation = ANIMATION
label-animation = Animation
section-expression = EXPRESSION
label-expression = Expression
section-gaze = CURSOR
label-follow-cursor = Follow cursor
label-gaze-strength = Strength
label-reactions = Reactions
section-physics = PHYSICS
label-physics = Physics
label-physics-quality = High quality
label-wind = Wind
label-physics-cursor = Cursor pushes
expression-neutral = Neutral
section-camera = CAMERA
label-camera = Camera
camera-orbit = Orbit
section-environment = ENVIRONMENT
label-environment = Environment
label-shadows = Shadows
label-lock-orbit = Lock orbit
label-parallax = Parallax
label-parallax-strength = Strength

## Graphics tab

section-lighting = LIGHTING
label-sky-strength = Sky strength
label-sky-rotation = Sky rotation
label-sun-follows-sky = Sun follows sky
label-background = Background
label-background-blur = Background blur
section-quality = QUALITY
label-render-scale = Render scale
label-anti-aliasing = Anti-aliasing
label-depth-of-field = Depth of field
label-shadow-quality = Shadow quality
label-shadow-updates = Animated shadows
label-ambient-occlusion = Ambient occlusion
label-ambient-occlusion-quality = Occlusion quality
label-ambient-occlusion-softness = Occlusion distance
section-fog = FOG
label-fog = Fog
label-fog-density = Density
label-fog-quality = Quality
label-fog-dithering = Jitter
label-fog-resolution = Resolution
section-depth-of-field = DEPTH OF FIELD
label-blur-shape = Shape
label-blur-strength = Strength
label-max-blur = Max size
label-blur-quality = Quality
label-blur-foreground = Foreground blur
label-bokeh-highlights = Highlights
section-strokes = STROKES
label-strokes = Strokes
label-stroke-width = Thickness
label-stroke-strength = Strength
label-stroke-creases = Creases
label-stroke-hand-drawn = Hand-drawn
section-post-processing = POST PROCESSING
section-image = IMAGE
label-tonemapping = View transform
label-exposure = Exposure
label-bloom = Bloom
label-bloom-threshold = Bloom threshold
label-chromatic-aberration = Aberration
section-color = COLOR
label-warmth = Temperature
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
# Occlusion distance.
value-distance-short = Short
value-distance-medium = Medium
value-distance-long = Long
# Stroke thickness.
value-width-thin = Thin
value-width-medium = Medium
value-width-thick = Thick
# Stroke strength.
value-strength-light = Light
value-strength-medium = Medium
value-strength-strong = Strong

## Sound tab

section-sound = SOUND
label-master-volume = All sound
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
label-start-at-login = Start with Windows
label-check-updates = Check for updates
label-update-version = New version
label-update = Update
# The version on offer while it downloads, after downloading it failed, and while CharPaper
# restarts to install it.
update-downloading = { $version } · { $percent }%
update-failed = { $version } · failed
update-restarting = { $version } · restarting
update-install = Install
update-retry = Try again
update-skip = Skip
update-open-page = Open on GitHub
# Kinds of change in the list of what changed in a newer version.
changelog-features = New
changelog-fixes = Fixes
changelog-performance = Performance
changelog-refactoring = Code changes
changelog-documentation = Documentation
changelog-tests = Tests
changelog-build = Build
changelog-reverts = Undone
changelog-other = Other
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
label-monitor = Performance overlay
value-max = Max
section-pause = PAUSE WHEN
label-pause-fullscreen = Fullscreen app
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
label-auto-hide = Auto-hide button

## Performance overlay, shown at the top of the screen. Units after numbers.

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
monitor-stage-assets = Loading
monitor-stage-specialize = Preparing shaders
monitor-stage-views = Cameras, screen
monitor-stage-views-note = waits here while the GPU or vsync is slower
monitor-stage-queue = Sorting draws
monitor-stage-prepare = Uploading to GPU
monitor-stage-render = Recording draws
monitor-stage-cleanup = Cleanup
# The memory section. "Memory" is all the RAM the process touches, as Task Manager counts it.
monitor-memory = Memory
# How much a kind of memory grew over the last minute, with its sign.
monitor-growth = { $mb } MB a minute
# Memory the process asked Windows for, touched or not.
monitor-committed = Committed
# What the app's own code holds, and in how many separate pieces.
monitor-heap = App memory
monitor-heap-blocks = { $count } blocks
monitor-gpu-memory = GPU memory
monitor-gpu-reserved = { $mb } MB reserved
monitor-gpu-objects = GPU objects
monitor-gpu-object-details = buffers { $buffers }, textures { $textures }, views { $views }, bind groups { $groups }, samplers { $samplers }, pipelines { $pipelines }, shaders { $shaders }
monitor-entities = Objects
monitor-render-entities = copied for drawing: { $count }
monitor-assets = Assets
monitor-asset-details = images { $images }, meshes { $meshes }, materials { $materials }, animations { $clips }, glTF { $gltfs }
monitor-graphics = Graphics
monitor-discrete = discrete
monitor-integrated = integrated
monitor-software = no GPU, drawn by the CPU
monitor-other-gpu = other
monitor-gpu = GPU
monitor-busy = { $percent } of the time
monitor-waiting = measuring…
monitor-unsupported = this GPU can't time its work
monitor-light-shadows = Lamp shadows
# Of the shadow maps there are, how many were drawn again this frame; the rest show what
# they showed before, since nothing in them changed.
monitor-shadow-maps = shadow maps: { $drawn } of { $count } drawn
monitor-prepass = Depth prepass, sun shadows
monitor-main-pass = Main pass
monitor-fog = Fog
monitor-effects = Post processing
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
help-parallax = The camera leans a little towards the pointer, so the scene shows its depth. It drifts back to the middle once the mouse keeps still.
help-parallax-strength = How far the camera leans. Higher shows more depth, and more of the scene's edges.
help-panel = Point at a setting to see what it does. The mouse wheel over a value changes it.
help-suite = Which character to show: one for each folder in the characters folder next to CharPaper.
help-animation = What the character is doing.
help-expression = The look on the character's face.
help-follow-cursor = The character's head and eyes follow the mouse pointer.
help-reactions = The character and the scene react when you click them or rest the pointer on them.
help-gaze-strength = How far the character turns toward the pointer. Lower keeps it calmer, higher makes it turn sooner.
help-physics = Hair, clothes and tails swing on their own as the character moves. Off, they move only as animated.
help-wind = The environment's wind blows hair and clothes, in gusts.
help-physics-cursor = Moving the pointer over hair and clothes pushes them aside, as a hand brushing through them. Dragging to turn the camera doesn't.
help-physics-quality = Keeps them off the character's actual body instead of rounded shapes around its bones, so they lie closer to it. A little slower, and the outfit's file is read again each time it's put on.
help-camera = Where the scene is seen from. Orbit turns with the mouse over the scene; the others are the cameras the character came with.
help-environment = The place the character is in, with its own lights, sky, props and music.
help-sky-strength = How bright the environment's sky is, and the light it gives. The same as Strength on Blender's World: 1 is the panorama as it is.
help-sky-rotation = Turns the sky around the scene, and the light it gives with it, like the Z rotation of a Mapping node on the World.
help-background = How bright the sky looks behind the scene, without changing the light it gives. Off hides it.
help-background-blur = Blurs the sky behind the scene, as if a lens were focused on the character. The light it gives stays the same.
help-shadows = Whether the environment's lights cast shadows. Off is faster.
help-sun-follows-sky = Turns the environment's sun to shine from the sun in the sky, and with it as the sky turns, so light and shadows match the background. A sky with no clear sun only takes the sun along as it turns. Off keeps the sun as it was made.
help-tonemapping = How the scene's light is fitted to the screen, like View Transform in Blender's Color Management. AgX and Filmic match Blender's.
help-exposure = Brightens or darkens the whole picture, as a camera would.
help-bloom = A glow around the brightest parts of the picture.
help-bloom-threshold = How bright a part must be to glow: 1.0 is white, lower lets dimmer lights glow too.
help-chromatic-aberration = Chromatic aberration: colored fringes toward the edges of the picture, like a cheap lens.
help-vignette = Darkens the corners of the picture.
help-vignette-size = How far the darkened corners reach in.
help-grain = Fine noise over the picture, like film.
help-grain-size = How coarse the grain is.
help-warmth = White balance: shifts colors toward orange or blue, like Temperature in Blender's Color Management.
help-tint = White balance: shifts colors toward magenta or green, like Tint in Blender's Color Management.
help-saturation = How vivid colors are. At 0% the picture is black and white.
help-contrast = How far darks and lights are pushed away from middle gray. Black and middle gray stay where they are.
help-lut = A ready-made color grade from the character's luts folder, much like a Look in Blender.
help-lut-strength = How much of the color grade to apply.
help-fps-limit = How many frames a second to draw. Fewer save power.
help-monitor = Shows at the top of the screen what drawing each frame costs the CPU and GPU, step by step, and what the scene holds.
help-render-scale = Draws the scene at a lower resolution to save power, like Resolution % in Blender. The panel stays sharp.
help-anti-aliasing = Smooths jagged edges. TAA is the smoothest but can smear what moves. Ambient occlusion can't run with 4x, so it hides while 4x is on.
help-shadow-quality = How detailed shadows are, on every stage. Low is the fastest, with blocky edges. High takes much more video memory.
help-shadow-updates = How often lamps redraw shadows that only animation moves. Every other frame halves their cost, but they lag a frame behind the motion.
help-ambient-occlusion = Darkens creases and corners the sky's light can't reach, like where the character meets the floor, as Ambient Occlusion does in Blender.
help-ambient-occlusion-quality = How many samples shade each pixel. Lower is faster and grainier; TAA hides most of the grain.
help-ambient-occlusion-softness = How far the darkening reaches from creases, like Distance in Blender's ambient occlusion. Short keeps it in corners, Long spreads it wide.
help-fog = Fog and light shafts, in environments that have fog.
help-fog-density = How thick the fog is. 100% is as it was made in Blender. Lower lets you see further through it; higher hides more and makes light shafts stronger.
help-fog-quality = How many steps the fog is drawn in, like Steps in Blender's volumes. Higher is smoother and costs more.
help-fog-dithering = Turns banding in the fog into fine noise, like Jitter in Blender's volumes. The noise changes every frame and the fog averages it away while the view is still, so even a low Quality looks smooth.
help-fog-resolution = Like Resolution in Blender's volumes. Half draws the fog four times faster and rarely shows, since fog is soft. Quarter is faster again but blurs light shafts.
help-depth-of-field = Blurs what's out of focus, from the camera's F-Stop and focus in Blender. Only for cameras that have depth of field.
help-blur-shape = Bokeh turns bright spots into round discs, like a real lens. Blur is a softer, plain blur.
help-blur-strength = How much everything out of focus blurs. 100% is the camera's own F-Stop; 200% blurs as if it were halved.
help-max-blur = The widest a blur may grow, in pixels, like Max Size in Blender. Larger lets near and far things melt away more, but needs higher quality.
help-blur-quality = How many samples each blurred pixel takes. Lower is faster, but wide blurs show rings or grain.
help-blur-foreground = Whether things in front of the focus blur too, or only what's behind it.
help-bokeh-highlights = How much brighter spots outweigh the rest, so lights stand out as discs. Off blurs everything alike.
help-strokes = Draws ink lines over the scene for a hand-drawn look, a little like Freestyle: outlines around shapes, and lines where surfaces fold.
help-stroke-width = How thick the lines are. A lower render scale makes them thicker on screen.
help-stroke-strength = How much the lines darken what's under them. They keep its color, like ink.
help-stroke-creases = Also draws lines where a surface bends sharply, like folds and corners, as Crease does in Freestyle.
help-stroke-hand-drawn = Lets the lines wander and swell a little and redraws them a few times a second, like hand-drawn animation.
help-pause-fullscreen = Stops the picture and sound while an app, such as a game, runs fullscreen.
help-pause-covered = Stops the picture and sound while windows cover the whole desktop.
help-pause-battery = Stops the picture and sound while the computer runs on battery.
help-mode = Wallpaper puts the scene behind the desktop icons, Window in an ordinary window. CharPaper restarts to switch.
help-start-at-login = Starts CharPaper whenever you sign in to Windows on this computer.
help-check-updates = Asks GitHub for a newer version each time CharPaper starts. Nothing is downloaded until you click Install.
help-update-version = A newer CharPaper is out. Click it to see what changed since your version.
help-update = Install downloads it and restarts CharPaper with it, keeping your characters and settings. Skip hides this version.
help-layout = For this monitor's stage. Mirror shows the whole scene on each of its monitors. Span spreads one wide view across them, as if they were a single screen.
help-master-volume = How loud all of CharPaper's sound is.
help-music-volume = How loud the environment's music is.
help-ambience-volume = How loud the environment's background sounds are, such as rain or wind.
help-character-volume = How loud the sounds of the character's animations are, such as footsteps.
help-language = The panel's language. System follows the language Windows is set to.
help-theme = The panel's colors. Light is easier to read over a bright scene.
help-accent = The color of what is picked or open. Click to set its hue, saturation and brightness.
help-accent-hue = Which color the accent is, around the color wheel: red, then yellow, green, cyan, blue, magenta.
help-accent-saturation = How vivid the accent is. At 0% it is gray.
help-accent-brightness = How light the accent is. At 0% it is black.
help-opacity = How much of the scene shows through the panel and the performance overlay.
help-size = How big the panel and the performance overlay are drawn.
help-side = The edge of the screen the panel keeps to.
help-auto-hide = Hides the button that brings the panel back a few seconds after the pointer leaves it. Move the pointer to that spot on the edge and it shows again.
