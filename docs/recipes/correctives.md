[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Shape keys that follow the body

**On this page:** [Drivers, for anything a bone can explain](#drivers-for-anything-a-bone-can-explain) · [Keys inside a clip, for what a driver can't do](#keys-inside-a-clip-for-what-a-driver-cant-do) · [When a shape key has both](#when-a-shape-key-has-both)

Correctives are shape keys that fix the mesh as the body moves: an elbow that keeps its
volume, a skirt that lifts with the thigh. There are two ways to drive them, for two jobs.

## Drivers, for anything a bone can explain

A knee fix driven by the knee's bend, a skirt driven by the thigh. Set the driver up in
Blender as usual. The add-on writes each skin's drivers into a file next to it
(`skins/casual.drivers.json`) and the app runs them every frame. They follow every clip,
the fades between clips, and the head turning to the cursor. There's nothing to tick.

What the app can run:

| | Works | Doesn't |
|---|---|---|
| **What's driven** | A shape key's **Value** | Anything else (modifiers, materials, bone properties) |
| **Driver type** | *Averaged Value*, *Sum Values*, *Minimum Value*, *Maximum Value*, *Scripted Expression* with plain math (`var * 2 - 0.5`, `max(a, b)`, `clamp(…)`) | Expressions that need Python functions, or that read `frame` |
| **Variables** | *Transform Channel* in World Space or Local Space; *Rotational Difference*; *Distance*; *Single Property* that reads a bone's location, rotation or scale, or another shape key of the same skin | Pose Space; reading other objects, custom properties, or meshes outside the skin |
| **Bones read** | Bones of the character's armature | Bones of other armatures |
| **Driver curve** | Its keyframes and modifiers, as Blender shows them | |

Check names every driver that won't come over and says why. Test a driver in Blender by
posing the bones: if it works there and Check says nothing, it works in the app.

## Keys inside a clip, for what a driver can't do

A cheek puff at frames 40 to 60 of one clip, a skirt that flutters during a spin. Here the
shape keys are keyed by hand, in the clip itself.

**In Blender 4.4 and newer,** one action can hold the bones and the shape keys together, in
separate slots:

1. Make the clip's action on the armature and key the bones (see
   [An animation](animation.md)).
2. Select the mesh whose shape keys you'll key, say the skirt. In the Dope Sheet's
   **Shape Key Editor**, unlink whatever action is there (the **X**), then pick **the
   clip's action** from the action list. Blender gives the mesh's shape keys a slot of
   their own in it.
3. Key the shape keys.
4. More meshes? Repeat for each. Every mesh gets its own slot in the same action.
5. **Unlink the action from the shape keys when you're done** (the **X** in the Shape Key
   Editor). Left assigned, it writes its values into the sliders and the skin is exported
   with them as its resting state. If you forget, Check warns and **Stop it** does it for
   you.

The clip's settings in the add-on then say what it keys, with the shape keys counted:
"keys 52 nodes · 3 shape keys (Puff_L, Puff_R, Skirt_Lift)". If no shape keys are listed,
they aren't in the action.

The app plays **every** shape key slot of the clip's action, finding the keys by name on
whichever skin is worn. So a face keyed on the body mesh and correctives keyed on the
skirt mesh both play, from one clip.

**In Blender 4.2 and 4.3,** an action can't hold bones and shape keys together. Make a
second action in the Shape Key Editor for the shape keys, and pick it in the clip's
**Also plays** field. It plays alongside the clip, over the same frames.

## When a shape key has both

The driver wins, as in Blender. A shape key with a driver ignores every key on it, in
clips and in expressions alike. If you want to key one by hand in some clip, it can't have
a driver.

---

← [A facial expression](expression.md) · [All recipes](../making-a-character.md#recipes) · [Following the cursor (gaze)](gaze.md) →
