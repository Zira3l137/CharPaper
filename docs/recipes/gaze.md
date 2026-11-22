[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# Following the cursor (gaze)

**On this page:** [Setting it up](#setting-it-up) · [What the fields mean](#what-the-fields-mean) · [Good to know](#good-to-know)

The character looks towards the mouse pointer, on top of whatever animation plays. With
the pointer on its face it looks at you. The further from the face the pointer is, the
further aside it looks: the head and neck turn part of the way and the eyes cover the
rest. It works the same through any camera, near or far.

The character goes back to what the animation has it do once the pointer has been still
for a few seconds, and lets go of a pointer it can't turn to, such as one behind its back.

## Setting it up

1. Open **Character → Gaze** in the add-on and set it to **Write**. (*Leave as is* keeps
   whatever suite.toml already says about gaze; *Remove* turns following off for good.)
2. Click **Detect bones**. It fills in the eye, head and neck bones, and whether the eyes
   are eyeballs. Check what it found, and fix by hand what it got wrong.
3. Turn on **Preview** and drag the two sliders. The character is posed the way the app
   will pose it. At −1 and 1 everything is at its limit, so tune the limits until that
   looks right.
4. Optionally name shape keys that should follow the eyes, such as eyelids.

## What the fields mean

- **Eyes are** says what the eye bones turn. *Eyeballs* are round balls in their sockets,
  and are aimed exactly. *Drawn* is anything else, like irises that slide over a flat or
  shallow eye: aimed, those would shoot past, so they move in proportion to how far aside
  the character looks. Drawn eyes usually want small limits, a few degrees.
- **Eye yaw** and **Eye pitch** are how far the eye bones turn, sideways and up or down.
- **Head** and **Neck** yaw and pitch add up to how far the head may point away from the
  chest. That holds for the animation and gaze together, so a clip that already turns the
  head leaves gaze less to add, and the head can't end up on backwards.
- **Head turn** is how much of the way to the pointer the head and neck go. At 0 the head
  only moves for what the eyes can't reach; at 1 it points straight there.
- **Head share** splits that turn between head and neck.
- **Front** is which way the character faces in the rest pose. *Automatic* works it out
  from where the eye bones sit. The arrow button shows the rest pose with an arrow from
  the head: green when the front is known, orange when it's a guess. If it points wrong,
  look the character in the face in the viewport and click **Front from view**, the eye
  icon next to the arrow.

## Good to know

- **Eye bones are optional.** A head alone follows, and so do eyes moved only by shape
  keys: name the four shape keys and leave the eye list empty.
- **Gaze moves the eye bones itself** while the character follows, so a clip's own eye
  animation doesn't show. Untick **Gaze** on that clip to see it, and on any clip where
  the character must look somewhere on purpose: a dance, sleeping.
- Eye bones must be children of the head bone, and an eyeball's bone should sit at the
  center of the ball. Otherwise the eye swings instead of spinning.
- **The preview is temporary.** The character's own pose comes back when you turn it off, change the
  frame, save or export.
- Detect bones reads bone names, so it can miss a bone with an unusual name. On Rigify's
  deform bones it finds no neck; pick it by hand.
- **During a reaction**, gaze lets go if the reaction's clip has **Gaze** unticked, and
  comes back when it ends. A reaction that turns the head towards something should have
  it unticked, or gaze pulls the head back to the cursor. See [A reaction](reaction.md).
- The viewer can switch following off, or change its **Strength**, under **Cursor** in the
  app's Scene tab.
- Without a gaze section the character never follows the cursor, and the app hides
  those rows.

---

← [Shape keys that follow the body](correctives.md) · [All recipes](../making-a-character.md#recipes) · [Hotspots](hotspots.md) →
