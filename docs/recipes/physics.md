[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Hair, clothes and tails that swing (physics)

**On this page:** [What it does](#what-it-does) · [Your first chain](#your-first-chain) · [Chains](#chains) · [The four settings](#the-four-settings) · [The cursor](#the-cursor) · [Colliders](#colliders) · [Wind](#wind) · [What the viewer can change](#what-the-viewer-can-change) · [What Check warns about](#what-check-warns-about) · [Good to know](#good-to-know)

## What it does

The app can swing bones by itself: a ponytail that sways when the head turns, a skirt
that trails behind a spin, a tail that bounces as the character walks. There's nothing to
bake. You pick the bones in Blender, and the app moves them while it runs.

Two things make it work:

- **Chains**: the bones that swing. A strand of hair, a strip of skirt, a tail. Each is a
  line of bones, each the child of the one before.
- **Colliders**: the body the chains keep out of, so hair rests on the shoulders instead of
  sinking into them. Each is a capsule around one bone, like a hotspot's.

**The animation stays in charge.** A chain's bones are pulled towards wherever your clips
put them, and the swinging is added on top. So you don't need to animate hair at all: left
in its rest pose, it hangs from the head and follows it. If a clip does key a hair bone,
say a flick of the ponytail, the flick plays and the ponytail follows through after it.

It all happens in the app. Blender only shows where the chains and capsules are; it
doesn't preview the swinging.

## Your first chain

A ponytail that sways:

1. Open the **Physics** panel. The eye icon in its header should be open: it draws the
   chains and colliders in the 3D Viewport.
2. Select the armature, enter **Pose Mode**, and select the **first** bone of the
   ponytail, the one hanging from the head.
3. In the Physics panel, under **Chains**, press **+** and pick **Hair**. A blue line runs
   down the ponytail, all the way to its last bone.
4. **Colliders next**, so the ponytail doesn't swing through the neck. Select the head,
   neck and upper spine bones, and under **Colliders** press **+**. Orange capsules appear,
   each about as thick as the body around its bone.
5. Look at the capsules from a few sides. Each should sit just inside the skin. Drag the
   number next to a bone's name (its radius) to fix one that doesn't.
6. **Check**, then **Export all**. Turn the character's head in the app (an animation that
   moves it, or the cursor with gaze on) and watch the ponytail.

## Chains

**Making them.** Select bones in Pose Mode and press **+** under Chains, picking the kind
(below). What you select decides what the chain is:

| You select | The chain is |
|---|---|
| **Its first bone only** | That bone and everything below it, down to the end. Where a bone has more than one child, the chain stops. |
| **Its first bone and more of it below** | Exactly the bones you selected. Use this to stop a chain before the end, or to choose which way to go where it forks. |
| **The first bones of several strands** | One chain for each, in one go: every strip of a skirt, every lock of bangs. |

A chain is named after its first bone. Bones already in a chain are skipped, since a bone
can only swing one way.

**Each bone has to be the child of the one before.** They don't have to be *Connected*,
but there can't be a gap. If you select a bone and its grandchild without the child in
between, you get two chains.

**How many bones?** Three to eight per strand is plenty. More bones bend more smoothly but
take longer to settle. A long braid with twelve bones works; it just moves more like rope.

**In the viewport**, a chain is a blue line along its bones, with a small ring at each
bone's tail showing its radius. The chain picked in the list is drawn solid, the others
faint. The arrow next to a chain selects its bones in Pose Mode.

## The four settings

Pick a chain in the list to see its settings. The kind you picked fills them in:

| Kind | Stiffness | Drag | Gravity | Radius | For |
|---|---|---|---|---|---|
| **Hair** | 0.1 | 0.15 | 1 | 0.02 | Ponytails, bangs, braids |
| **Cloth** | 0.05 | 0.2 | 1 | 0.03 | Skirts, capes, ribbons |
| **Soft** | 0.4 | 0.3 | 0.2 | 0 | Bouncy accessories, ears |
| **Tail** | 0.2 | 0.1 | 0.5 | 0.03 | Tails, long earrings |

Changing any value turns its **Preset** to *Custom*. Picking a preset again puts its values
back.

- **Stiffness**: how strongly it springs back to the animated pose. At 0 it hangs loose,
  pulled only by gravity; at 1 it follows the animation exactly and doesn't swing at all.
  Most things look right between 0.05 and 0.3.
- **Drag**: how quickly it settles. At 0 it swings on and on, like a pendulum; at 1 it
  stops dead. Higher drag looks heavier and wetter.
- **Gravity**: how much it's pulled down, as a share of Earth's gravity. Lower it for
  things that hold their shape, like a tail that sticks out.
- **Radius**: how far the bones' tails stay from the colliders, in meters. About how thick
  the hair or cloth is at each tail. Too small and it sinks into the body; too big and it
  floats off it.

Stiffness and drag are steps of the app's own clock, 60 a second, so a chain swings the
same at 30 and at 144 frames a second.

## The cursor

In the app, the pointer moving over a chain can push it aside (if the viewer leaves
**Cursor pushes** on). Each chain's **Cursor** setting says what it does to that chain:

| Cursor | The chain |
|---|---|
| **Pushes it** (the default) | Swings on its own, and the pointer pushes it aside too. |
| **Ignores it** | Swings on its own, and the pointer goes through it. For things too heavy to brush aside, or parts that look wrong when pushed. |
| **Only the cursor moves it** | Follows the animation exactly, as if it had no physics, until the pointer pushes it; then it springs back. Gravity, wind and colliders leave it alone. |

**Only the cursor moves it** is for things that should stay put but react to a touch:
grass or flowers in the character's hair, fur, a fringe of beads, a curtain on its
costume. The animation carries the chain along without any swing, so it doesn't sway when
the character moves. Only the pointer puts it out of place, and **Stiffness** and **Drag**
then decide how it springs back: high Stiffness snaps back, low Stiffness drifts back
slowly; high Drag settles at once, low Drag wobbles. **Gravity** does nothing for it, and is
greyed out. **Radius** is how far the pointer keeps its tails away.

The list marks chains that aren't the default (*no cursor*, *cursor only*), and the viewport
draws cursor-only chains green instead of blue.

## Colliders

**Making them. Select bones of the body in Pose Mode and press **+** under Colliders.
Each gets a capsule from the bone's head to its tail, and its radius is **fitted to the
mesh**. The add-on looks at every mesh the armature deforms and takes the vertices weighted
mostly to that bone. It uses the thinnest such mesh, so the body counts and a coat over it
doesn't. A bone no mesh is weighted to gets a guess, and the report says how many did.

**Fixing one.** Drag the radius in the list, or pick it and press **Fit to the mesh** to
measure it again after you've changed the weights. A capsule should sit just inside the
skin: the chains' own radius keeps them the rest of the way off.

**Which bones?** The ones the chains could reach: the head and neck for hair, the chest and
upper arms for long hair and capes, the hips, thighs and shins for skirts. A bone in a chain
can't be a collider too.

**Every chain keeps out of every collider.** There's nothing to pair up. In the viewport,
colliders are orange capsules, the picked one solid.

## Wind

An environment can have wind that blows the character's chains while it's shown.

1. In the environment's collection, add a Wind force field: **Add › Force Field › Wind**.
2. **Point it.** The wind blows along the field's arrow, its Z axis. Rotate the empty until
   the arrow points the way the wind should blow. Where it sits doesn't matter; it blows the
   same everywhere.
3. **Set its Strength**, in the force field's settings. It's how hard the wind pushes,
   measured the same way as gravity (which is 9.81): about **2** stirs hair, **10** blows
   it out sideways as hard as gravity pulls it down. A negative Strength blows backwards.
4. **Set its Noise** for gusts. At **0** the wind blows steadily; at **1** it drops to calm
   and rises to twice as strong. More than 1 does nothing more.
5. Export the environment. The **Environments** panel shows what it found, as in *Wind from
   Breeze: 3 m/s², gusts 40%*.

Gusts roll along the way the wind blows, so bits close together sway together and bits far
apart along the wind sway out of step. Only the first Wind field by name in a collection
counts. Other kinds of force field, and Blender's own wind on cloth or particles, don't come
over.

## What the viewer can change

The app's **Scene** tab has a **Physics** section with four switches, saved for each
monitor's scene:

| Setting | What it does | Shows when |
|---|---|---|
| **Physics** | On by default. Off, the chains move only as the animation moves them. | The character has chains. |
| **High quality** | Off by default. Chains keep off the character's actual mesh instead of the capsules. | It also has colliders. |
| **Wind** | On by default. The environment's wind blows the chains. | The environment shown has wind. |
| **Cursor pushes** | On by default. Moving the pointer over hair and clothes pushes them aside, like a hand brushing through them, except chains set to ignore it. Off, chains only the cursor moves just follow the animation. | The character has chains. |

**High quality** reads the worn outfit from its file each time it's put on, in the
background, so the capsules hold for a moment after a switch. Around each collider's bone,
the chains then keep off the mesh itself: the thinnest object of the outfit there, as
for fitting, and only objects the viewer hasn't hidden. Your colliders still decide which
parts of the body count. A bone that no mesh is weighted to keeps its capsule.

**Cursor pushes** works along the line from the camera through the pointer, up to the
first collider under it, so hair behind the head is out of reach. It lets go a moment after the mouse
stops, so a resting pointer doesn't hold hair aside, and it's off while a button is held,
since dragging turns the camera.

## What Check warns about

Warnings show under the chain or collider they're about, and the list marks its row.

**Chains:**
- *Has no bones*: select bones in Pose Mode and make a chain from them instead.
- *Names bones the armature doesn't have*: a bone was renamed or deleted. It's left out.
- *Shares a bone with another chain*: it's left out. A bone can only swing one way.
- *Has bones that Deform bones only leaves out of the export*: the chain is left out. Tick
  **Deform** on those bones, or untick *Deform bones only* in Export settings.
- *Has bones that aren't each the child of the one before*: it's left out.
- *Starts inside a collider*: at rest, one of its tails is closer to a capsule than the
  chain's radius, so it jumps out the moment the character loads. Make the capsule or the
  chain's radius smaller. Not for chains only the cursor moves, which keep out of nothing
  else.

**Colliders:**
- *Names a bone the armature doesn't have*, or *a bone Deform bones only leaves out*: it's
  left out.
- *Is there twice* (a tip): only the first counts.
- *No colliders* (a tip): with chains but no colliders, the chains swing straight through
  the body. Chains only the cursor moves don't need any.

**Wind**, in the environment's warnings:
- *Only the first wind blows* (a tip), with several Wind fields in one collection.
- *A Strength of 0*: it doesn't blow.
- *Noise above 1* (a tip): the gusts are already at their strongest.

## Good to know

- **The add-on owns the physics in suite.toml** once you've made a chain or collider in
  Blender. If you wrote some by hand, press **Load suite.toml** first, or exporting replaces
  them.
- **A chain can hang from one outfit's own bones.** A skirt's bones that only the dress
  has: the chain swings while the dress is worn, and is skipped while another outfit is.
- **Keep chains off the bones gaze turns.** A chain can hang from the head, but if a chain's
  own bone is one gaze turns, the two fight over it.
- **Switching outfits** starts that outfit's chains again from the animated pose, so they
  settle for a moment.
- **Only the character has physics.** Props in an environment don't swing.
- **The capsules follow bones, not meshes.** Like hotspots, a capsule can't follow cloth or
  a shape key. High quality uses the mesh as it's exported, without shape keys, and moves
  each part of it with the bone it's mostly weighted to, so it's a little off right at a
  bending joint.
- **A tail pulled deep into a limb** comes out of its nearest side, which may be the far
  side. Give limbs colliders, and keep chain radii sensible.
- **It costs very little.** Two dozen chains of five bones take a fraction of a millisecond
  a frame. High quality costs more, mostly when many tails rest against the body.

---

← [Hotspot ideas](hotspot-ideas.md) · [All recipes](../making-a-character.md#recipes) · [An environment](environment.md) →
