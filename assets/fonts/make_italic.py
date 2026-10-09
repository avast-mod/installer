"""Builds fs-tahoma-8px-italic.ttf and fs-tahoma-8px-bold-italic.ttf from the regular and bold
fonts the way small pixel fonts are slanted: each glyph is cut into pixel rows, and rows move one
pixel to the right for every four rows above the baseline. Pixels stay whole, so the result is
as sharp as the upright font.

Needs fonttools and skia-pathops: pip install fonttools skia-pathops
"""
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.transformPen import TransformPen
import pathops

PIXEL = 64  # font units per pixel in fs Tahoma 8px
ROWS_PER_STEP = 4


def shift_for(row):
    return (row + 1) // ROWS_PER_STEP


def slant(source, target, family, style, weight):
    font = TTFont(source)
    glyphs = font.getGlyphSet()
    glyf = font["glyf"]
    for name in font.getGlyphOrder():
        whole = pathops.Path()
        glyphs[name].draw(whole.getPen())
        bounds = whole.bounds if whole.area else None
        out_path = pathops.Path()
        if bounds:
            bottom = int(bounds[1] // PIXEL)
            top = int(-(-bounds[3] // PIXEL))
            for row in range(bottom, top):
                band = pathops.Path()
                pen = band.getPen()
                pen.moveTo((bounds[0] - PIXEL, row * PIXEL))
                pen.lineTo((bounds[2] + PIXEL, row * PIXEL))
                pen.lineTo((bounds[2] + PIXEL, (row + 1) * PIXEL))
                pen.lineTo((bounds[0] - PIXEL, (row + 1) * PIXEL))
                pen.closePath()
                piece = pathops.op(whole, band, pathops.PathOp.INTERSECTION)
                if not piece.area:
                    continue
                moved = pathops.Path()
                piece.draw(TransformPen(moved.getPen(), (1, 0, 0, 1, shift_for(row) * PIXEL, 0)))
                out_path = pathops.op(out_path, moved, pathops.PathOp.UNION)
            out_path.simplify(fix_winding=True)
        pen = TTGlyphPen(glyphs)
        out_path.draw(pen)
        glyf[name] = pen.glyph()
    names = font["name"]
    for record in list(names.names):
        if record.nameID in (1, 2, 3, 4, 6, 16, 17):
            names.removeNames(nameID=record.nameID)
    full = "%s %s" % (family, style)
    names.setName(full, 1, 3, 1, 0x409)
    names.setName("Regular", 2, 3, 1, 0x409)
    names.setName(full, 3, 3, 1, 0x409)
    names.setName(full, 4, 3, 1, 0x409)
    names.setName(full.replace(" ", ""), 6, 3, 1, 0x409)
    names.setName("%s of fs Tahoma 8px by ETHproductions, made for AVaSt by shifting pixel rows." % style, 10, 3, 1, 0x409)
    font["OS/2"].usWeightClass = weight
    font["OS/2"].fsSelection = (font["OS/2"].fsSelection & ~(1 << 6)) | 1 | (32 if weight >= 700 else 0)
    font["head"].macStyle |= 2 | (1 if weight >= 700 else 0)
    font.save(target)


slant("fs-tahoma-8px.ttf", "fs-tahoma-8px-italic.ttf", "fs Tahoma 8px AVaSt", "Italic", 400)
slant("fs-tahoma-8px-bold.ttf", "fs-tahoma-8px-bold-italic.ttf", "fs Tahoma 8px AVaSt", "Bold Italic", 700)
