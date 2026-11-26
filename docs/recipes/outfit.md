[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# An outfit (skin)

**On this page:** [Adding a skin](#adding-a-skin) · [What a skin can hold](#what-a-skin-can-hold) · [Its sliders are its resting face](#its-sliders-are-its-resting-face) · [Warnings about skins](#warnings-about-skins)

A skin is everything the viewer sees of the character in one outfit: the body (or what
shows of it), hair, eyes, clothes. Switching skins in the app swaps all of it at once.

## Adding a skin

1. Put every mesh of the outfit into one collection. Collections inside it count too.
2. Click that collection in the Outliner, so it's the active one.
3. In **Character → Skins**, press **+**. The skin takes the collection and its name. Change
   the name if you like: it's what the app shows and the file's name.

The app shows each outfit on a tile with a picture of it, which it takes itself the first
time the character shows: the outfit in the **rest pose** of its file, from a three-quarter
view of the character's front. So export with the armature in a clean rest pose, a T-pose
or A-pose, rather than mid-gesture. The pictures are kept in the character's `thumbnails`
folder and taken again whenever a skin is exported again. The character's own tile shows a
close-up of its head and shoulders in the outfit it starts in, centered on the head bone
when **Gaze** names one.

Each skin in the list has:

- a **checkbox**: unticked, *Export all* leaves it out. Handy when you only changed one
  outfit;
- an **export button** on the right, which exports that skin alone.

## What a skin can hold

- **Meshes deformed by the character's armature** (an *Armature* modifier pointing at it),
  or **parented to the armature or one of its bones**. A hat parented to the head bone
  works. A mesh that's neither stays put in the app while the character moves.
- **Several mesh objects become parts.** In the app each object of the skin is a switch
  under **Outfit → Parts**, named after the object. A jacket as its own object can be
  taken off by the viewer. So name objects the way you'd want them listed, and join what
  should never be hidden on its own.
- **Shape keys**, up to 256 per mesh, and their **drivers** (see
  [Shape keys that follow the body](correctives.md)).
- **Not** lights, cameras or other armatures. The add-on leaves them out and says so.

**Modifiers and shape keys:** with *Apply modifiers* on in Export settings, glTF can't apply
modifiers to a mesh that has shape keys without losing them. Apply such modifiers yourself
(a Subdivision you can do without, a Mirror, a Solidify), or keep them off those meshes.

**Shape key names are shared across meshes.** The app moves a shape key by its name, on
every mesh of the worn skin that has one by that name. Two meshes both with a key called
`Smile` smile together, which is what you want for a face split into several objects, and
not what you want for a skirt whose key happens to be called `Key 1` like the body's. Give
shape keys names that say what they do.

## Its sliders are its resting face

A skin is exported **as it stands**: every shape key slider is saved at the value it shows
right then. In the app, a shape key that nothing keys (no clip, no expression, no driver)
rests at that saved value.

So before exporting a skin:

- **Set sliders to how the character should look at rest.** Usually all at 0. A slider
  left at 1 on `Smile` makes the character smile whenever no expression plays.
- **Stop anything animating the sliders.** If an action plays on the mesh's shape keys, its
  values are what gets saved. This happens easily: keying a face in the Shape Key Editor
  leaves that action assigned. The add-on warns about it and offers **Stop it**, which
  unassigns the action (it stays in the file) and mutes NLA tracks that play on the shape
  keys. Then export the skin again.

## Warnings about skins

- **"…has an action of its own"** (with **Unlink it**): the mesh object itself, not its
  shape keys, has an action. A skin is exported still and moves only with the armature,
  so whatever that action does (moving, rotating the mesh) won't play in the app. Often an
  importer left an empty action behind; then it's a blue tip and harmless. To move a part
  with the character, parent it to a bone or weight it to the armature and key the bones.
- **"…is deformed by X, not the suite's armature"**: its Armature modifier points at another
  armature. It won't follow the character.
- **"…has a non-uniform scale"**: apply scale (Ctrl+A → Scale) on the armature and on
  meshes parented to bones.
- **"left out (not meshes)"**: lights, cameras and the like in the skin's collection.
  Move them to an environment.

---

[All recipes](../making-a-character.md#recipes) · [An animation](animation.md) →
