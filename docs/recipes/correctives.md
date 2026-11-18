[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Shape keys that follow the body

**On this page:** [Drivers, for anything a bone can explain](#drivers-for-anything-a-bone-can-explain) · [Keys inside a clip, for what a driver can't do](#keys-inside-a-clip-for-what-a-driver-cant-do) · [When a shape key has both](#when-a-shape-key-has-both)

Two ways, for two jobs.

## Drivers, for anything a bone can explain

A knee fix driven by the knee's bend, a
skirt driven by the thigh. Set the driver up in Blender as usual. The add-on writes the
skin's drivers next to it and the app runs them every frame, so they follow every clip,
the blends between clips, and the head turning to the cursor. Nothing to tick.

The app runs drivers that read **bones of the character's armature** or other shape keys
of the same skin, with the types *Averaged Value*, *Sum Values*, *Minimum Value*,
*Maximum Value*, and *Scripted Expression* as long as the expression is simple math. It
can't run Python functions, read other objects or custom properties, or use the current
frame. Validation names every driver that won't come over.

## Keys inside a clip, for what a driver can't do

A cheek puff at frames 40 to 60 of one
clip. In Blender 4.4 and newer, key the shape keys in the clip's own action: select the
mesh, assign that action in the Shape Key Editor, and key there. The clip then shows
*Keyed in this clip* in the add-on. In older Blender, make a separate shape key action and
pick it in the clip's **Shape keys** field.

## When a shape key has both

The driver wins, as in Blender.

---

← [A facial expression](expression.md) · [All recipes](../making-a-character.md#recipes) · [Following the cursor (gaze)](gaze.md) →
