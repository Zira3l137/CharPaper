[CharPaper](../README.md) › [Docs](README.md)

# Keeping it fast

A wallpaper runs all day next to everything else, so a light scene matters more than in a
render.

- **Shadows from point lights are the main cost.** Keep *Shadow* on for one or two key
  lights and untick the rest.
- **Animated props that cast shadows** make lamps redraw their shadows every frame. Untick
  shadow visibility on the ones that don't need it.
- **Every light that lights fog adds to the fog's cost.** Set *Volume Scatter* to 0 on
  lights that don't need to show in it.
- **Noise on fog costs a texture read per *Detail* octave at every step**, though gaps
  the noise thins to nothing cost almost nothing. Keep *Detail* low; soft fog rarely needs
  more than 2.
- **Big GIFs take memory.** A GIF's frames are kept unpacked, up to 200 MB of them; a
  bigger one is read again on every loop instead. Keep screens small and loops short.
- **Textures:** 2K is plenty for a character seen on a desktop. The app makes each one's
  mipmaps as the character loads, which takes a third more video memory and, for a 4K
  texture, about a quarter of a second.
- **Big Blended surfaces cost more than solid ones:** what's behind them is drawn first and
  they're blended over it. Dithered ones and cutouts cost about what solid surfaces do. See
  [Materials and textures](recipes/materials.md#transparency).
- **Ship the baked `.ktx2` sky files** so first launch isn't spent making them.
- **Physics costs very little.** Two dozen chains of five bones take a fraction of a
  millisecond a frame. Physics **High quality** costs more, most when many tails rest
  against the body, and reads the outfit's file each time it's put on.
- `charpaper.exe --benchmark` measures what each graphics setting costs with your
  character and writes `benchmark.txt`.

---

[All docs](README.md)
