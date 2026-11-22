[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A facial expression

An expression is an action that keys **only shape keys**. The viewer picks it in the app's
Scene tab and the face holds it, over whatever the body is doing. It can be a still face
(one keyframe) or a living one (a smile that blinks now and then): it loops on its own
length for as long as it's picked.

## Making one

1. Select the face mesh. Open the **Dope Sheet** and switch it to **Shape Key Editor**.
2. **Unlink the action that's there, if any** (the **X** next to its name), then press
   **New**. In Blender 4.4 and newer, *New* with an action assigned copies it, keys and all
   (see [Start every action empty](animation.md#start-every-action-empty)).
3. Name the action. The name is what the app shows (`Happy`, `Angry`).
4. Key the shape keys. For a still face, key them all on one frame. For a living face,
   key the loop's first frame again at its end, so it loops without a pop.
5. **Unlink the action again** when you're done (the **X**). Otherwise its values stay on
   the sliders and get saved as the skin's resting face. If you forget, Check warns and
   **Stop it** does it for you.

A face made of several meshes (a head, eyebrows, a tongue)? In Blender 4.4 and newer one
action can key them all: select each mesh in turn, pick the same action in its Shape Key
Editor, and key it there. Each mesh gets a slot in the action.

## Adding it in the add-on

In **Character → Expressions**, press **+** and pick the action. The box below shows which
shape keys it touches, and which meshes of each skin have them.

- **Expressions belong to the character, not to a skin.** The app finds the shape keys by
  name on whichever skin is worn. One `Happy` works for every outfit whose meshes have the
  same shape key names, and a skin that has none of them just doesn't change.
- **Shown as** renames it in the app.
- **No expression** in the app means every shape key rests at the value its slider had when
  the skin was exported (see [Its sliders are its resting face](outfit.md#its-sliders-are-its-resting-face)).

## How it mixes with everything else

From bottom to top, each one wins over the one below it, only for the shape keys it keys:

1. The skin's resting sliders.
2. Shape keys keyed in the body's clip.
3. The expression the viewer picked.
4. A reaction's clip, and the expression a [hotspot](hotspots.md) shows, while they play.
5. **Drivers.** A shape key with a driver always follows its driver.

So an expression that keys `Blink` takes blinking away from an idle that also keys it. Key
in an expression only what makes the face.

---

← [An animation](animation.md) · [All recipes](../making-a-character.md#recipes) · [Shape keys that follow the body](correctives.md) →
