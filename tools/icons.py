"""Builds the panel's icon font from the SVGs in resources/icons.

    pip install fonttools skia-pathops
    python tools/icons.py

Writes crates/charpaper-ui/assets/icons.ttf and crates/charpaper-ui/src/icons/glyphs.rs, the
Rust names of the icons in it. Run it again after adding or changing an icon, and commit all
three together.

An icon is drawn on a 24×24 grid, as lines: the root <svg> sets fill="none",
stroke="currentColor", stroke-width="2" and round caps and joins, which any element may
override. fill="currentColor" fills a shape as well, stroke="none" leaves its line out. The
elements understood are path, circle, rect (with rx), line and polyline; nothing else, no
transforms. A font can't draw lines, only filled shapes, so every line is turned into the
outline of its stroke here and everything in an icon merged into one shape.
"""

import re
import sys
import xml.etree.ElementTree as ElementTree
from pathlib import Path

import pathops
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import parse_path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "resources" / "icons"
FONT = ROOT / "crates" / "charpaper-ui" / "assets" / "icons.ttf"
NAMES = ROOT / "crates" / "charpaper-ui" / "src" / "icons" / "glyphs.rs"

GRID = 24
# 100 font units to a grid unit, so the grid's halves and tenths land on whole units.
UNITS_PER_EM = GRID * 100
SCALE = UNITS_PER_EM / GRID
# The icon fills the whole em, from the descender to the ascender, with no gap between lines.
# Text centers the em in its line, so an icon sits centered in whatever line height it gets.
ASCENT = UNITS_PER_EM * 4 // 5
DESCENT = UNITS_PER_EM - ASCENT
# The Private Use Area, where no real character is.
FIRST_CODE_POINT = 0xE000

INHERITED = ("fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin")
CAPS = {"round": pathops.LineCap.ROUND_CAP, "butt": pathops.LineCap.BUTT_CAP,
        "square": pathops.LineCap.SQUARE_CAP}
JOINS = {"round": pathops.LineJoin.ROUND_JOIN, "miter": pathops.LineJoin.MITER_JOIN,
         "bevel": pathops.LineJoin.BEVEL_JOIN}


def main():
    files = sorted(ICONS.glob("*.svg"))
    if not files:
        sys.exit(f"no icons in {ICONS}")
    names = [file.stem for file in files]
    for name in names:
        if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            sys.exit(f"{name}.svg: name icons in lower case with underscores, as in arrow_left")

    glyphs = {".notdef": TTGlyphPen(None).glyph(), "space": TTGlyphPen(None).glyph()}
    metrics = {".notdef": (UNITS_PER_EM, 0), "space": (UNITS_PER_EM, 0)}
    for file, name in zip(files, names):
        shape = icon_shape(file)
        pen = TTGlyphPen(None)
        shape.draw(Cu2QuPen(pen, max_err=1.0, reverse_direction=False))
        glyphs[name] = pen.glyph()
        bounds = BoundsPen(None)
        shape.draw(bounds)
        left = round(bounds.bounds[0]) if bounds.bounds else 0
        metrics[name] = (UNITS_PER_EM, left)

    code_points = {FIRST_CODE_POINT + i: name for i, name in enumerate(names)}
    builder = FontBuilder(UNITS_PER_EM, isTTF=True)
    builder.setupGlyphOrder(list(glyphs))
    builder.setupCharacterMap({0x20: "space", **code_points})
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics(metrics)
    builder.setupHorizontalHeader(ascent=ASCENT, descent=-DESCENT, lineGap=0)
    builder.setupNameTable({"familyName": "CharPaper Icons", "styleName": "Regular"})
    builder.setupOS2(
        version=4,
        sTypoAscender=ASCENT,
        sTypoDescender=-DESCENT,
        sTypoLineGap=0,
        usWinAscent=ASCENT,
        usWinDescent=DESCENT,
        fsSelection=0x80,  # Use the typo metrics above.
    )
    builder.setupPost()
    FONT.parent.mkdir(parents=True, exist_ok=True)
    builder.save(FONT)

    NAMES.parent.mkdir(parents=True, exist_ok=True)
    NAMES.write_text(rust_names(code_points), encoding="utf-8", newline="\n")
    print(f"{len(names)} icons: {FONT.relative_to(ROOT)}, {NAMES.relative_to(ROOT)}")


def icon_shape(file):
    root = ElementTree.parse(file).getroot()
    style = {key: root.get(key) for key in INHERITED if root.get(key) is not None}
    shape = pathops.Path()
    for element in root.iter():
        tag = element.tag.rsplit("}", 1)[-1]
        if tag == "svg":
            continue
        d = path_data(tag, element)
        if d is None:
            sys.exit(f"{file.name}: <{tag}> isn't understood")
        own = {**style, **{key: element.get(key) for key in INHERITED if element.get(key)}}
        for part in parts(d, own, file):
            shape = pathops.op(shape, part, pathops.PathOp.UNION)
    shape.simplify(fix_winding=True, clockwise=True)
    return shape


# The filled shape and the outline of the line an element draws, in font units, y up.
def parts(d, style, file):
    outline = pathops.Path()
    parse_path(d, TransformPen(outline.getPen(), (SCALE, 0, 0, -SCALE, 0, ASCENT)))
    found = []
    if style.get("fill", "black") != "none":
        found.append(outline)
    if style.get("stroke", "none") != "none":
        width = float(style.get("stroke-width", "1")) * SCALE
        cap = CAPS[style.get("stroke-linecap", "butt")]
        join = JOINS[style.get("stroke-linejoin", "miter")]
        line = pathops.Path()
        outline.draw(line.getPen())
        line.stroke(width, cap, join, 4.0)
        # Round caps and joins come out as conic curves, which neither fonts nor the merging
        # below can take.
        line.convertConicsToQuads()
        found.append(line)
    if not found:
        sys.exit(f"{file.name}: an element with neither fill nor stroke draws nothing")
    return found


def path_data(tag, e):
    number = lambda key, default=0.0: float(e.get(key, default))
    if tag == "path":
        return e.get("d")
    if tag == "circle":
        cx, cy, r = number("cx"), number("cy"), number("r")
        return f"M{cx - r} {cy}a{r} {r} 0 1 0 {2 * r} 0a{r} {r} 0 1 0 {-2 * r} 0z"
    if tag == "rect":
        x, y, w, h = number("x"), number("y"), number("width"), number("height")
        r = min(number("rx", e.get("ry", 0)), w / 2, h / 2)
        if r == 0:
            return f"M{x} {y}h{w}v{h}h{-w}z"
        arc = f"a{r} {r} 0 0 1"
        return (f"M{x + r} {y}h{w - 2 * r}{arc} {r} {r}v{h - 2 * r}{arc} {-r} {r}"
                f"h{-(w - 2 * r)}{arc} {-r} {-r}v{-(h - 2 * r)}{arc} {r} {-r}z")
    if tag == "line":
        return f"M{number('x1')} {number('y1')}L{number('x2')} {number('y2')}"
    if tag == "polyline":
        return "M" + e.get("points", "")
    return None


def rust_names(code_points):
    variants = [(pascal(name), code_point) for code_point, name in code_points.items()]
    lines = [
        "// Made by tools/icons.py from resources/icons. Change an icon's SVG and run it again,",
        "// rather than editing this.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        '#[allow(dead_code, reason = "the set can hold icons the panel does not use yet")]',
        "pub(crate) enum Icon {",
        *(f"    {variant}," for variant, _ in variants),
        "}",
        "",
        "impl Icon {",
        "    // The character the icon font draws it for.",
        "    pub(crate) fn glyph(self) -> char {",
        "        match self {",
        *(f"            Icon::{variant} => '\\u{{{code_point:X}}}'," for variant, code_point in variants),
        "        }",
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def pascal(name):
    return "".join(part.capitalize() for part in name.split("_"))


if __name__ == "__main__":
    main()
