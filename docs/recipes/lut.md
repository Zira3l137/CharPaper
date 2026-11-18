[CharPaper](../../README.md) › [Docs](../README.md) › [Making a character](../making-a-character.md) › [Recipes](../making-a-character.md#recipes)

# A color grade (LUT)

Put `.cube` files, as Resolve, Photoshop and most grading apps export them, into `luts/`.
They appear under **Graphics → Color → LUT**. To have one on from the start, name it in
suite.toml:

```toml
[post.lut]
name = "teal-orange"
strength = 0.8
```

The grade is applied to the finished picture, after everything except film grain.

---

← [Sounds](sounds.md) · [All recipes](../making-a-character.md#recipes) · [The picture the viewer sees first](first-picture.md) →
