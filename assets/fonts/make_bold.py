"""Builds fs-tahoma-8px-bold.ttf from fs-tahoma-8px.ttf the way Tahoma Bold is built from
Tahoma at small sizes: every pixel is doubled to the right, and each glyph gets one pixel wider.

Needs fonttools and skia-pathops: pip install fonttools skia-pathops
"""
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.transformPen import TransformPen
import pathops

PIXEL = 64  # font units per pixel in fs Tahoma 8px

font = TTFont("fs-tahoma-8px.ttf")
glyphs = font.getGlyphSet()
glyf = font["glyf"]
hmtx = font["hmtx"]

for name in font.getGlyphOrder():
    path = pathops.Path()
    pen = path.getPen()
    glyphs[name].draw(pen)
    glyphs[name].draw(TransformPen(pen, (1, 0, 0, 1, PIXEL, 0)))
    path.simplify(fix_winding=True)
    out = TTGlyphPen(glyphs)
    path.draw(out)
    glyf[name] = out.glyph()
    advance, lsb = hmtx[name]
    hmtx[name] = (advance + PIXEL if advance else 0, lsb)

names = font["name"]
for record in list(names.names):
    if record.nameID in (1, 3, 4, 6, 16, 17):
        names.removeNames(nameID=record.nameID)
names.setName("fs Tahoma 8px Bold AVaSt", 1, 3, 1, 0x409)
names.setName("Regular", 2, 3, 1, 0x409)
names.setName("fs Tahoma 8px Bold AVaSt", 3, 3, 1, 0x409)
names.setName("fs Tahoma 8px Bold AVaSt", 4, 3, 1, 0x409)
names.setName("fsTahoma8pxBoldAVaSt", 6, 3, 1, 0x409)
names.setName("Bold version of fs Tahoma 8px by ETHproductions, made for AVaSt by doubling every pixel to the right.", 10, 3, 1, 0x409)
font["OS/2"].usWeightClass = 700
font.save("fs-tahoma-8px-bold.ttf")
