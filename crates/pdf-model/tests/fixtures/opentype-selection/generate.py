"""Rebuild synthetic audit fonts with fontTools 4.68.0; not needed to run Rust tests.

All outlines are rectangles authored for these tests. No third-party outlines are copied.
"""
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable

root = Path(__file__).resolve().parent
for cff in (True, False):
    names = ['.notdef', 'two', 'two.sups', 'A', 'B'] if cff else ['.notdef', 'A', 'B']
    ps_name = 'AuditCFF' if cff else 'AuditGlyf'
    fb = FontBuilder(1000, isTTF=not cff)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap({50: 'two', 178: 'two.sups', 65: 'A', 66: 'B'} if cff else {65: 'A', 66: 'B'})
    if cff:
        # A Macintosh subtable is a decoy: the Type1 PDF must use its named encoding,
        # even though the TrueType symbolic procedure would map bytes A/B here.
        mac = CmapSubtable.newSubtable(0)
        mac.platformID, mac.platEncID, mac.language = 1, 0, 0
        mac.cmap = {50: "two", 65: "A", 66: "B"}
        fb.font["cmap"].tables.append(mac)
    glyphs = {}
    metrics = {}
    for index, name in enumerate(names):
        pen = T2CharStringPen(500, None) if cff else TTGlyphPen(None)
        if index:
            # Different boxes make glyph substitution observable beyond the numeric index.
            right, top = 150 + index * 50, 300 + index * 40
            pen.moveTo((0, 0)); pen.lineTo((right, 0)); pen.lineTo((right, top))
            pen.lineTo((0, top)); pen.closePath()
        glyphs[name] = pen.getCharString(private=None, globalSubrs=None) if cff else pen.glyph()
        metrics[name] = (500, 0)
    if cff:
        fb.setupCFF(ps_name, {'FullName': ps_name, 'FamilyName': ps_name, 'Weight': 'Regular'}, glyphs, {})
    else:
        fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({'familyName': ps_name, 'styleName': 'Regular', 'uniqueFontIdentifier': ps_name, 'fullName': ps_name, 'psName': ps_name, 'version': 'Version 1.0'})
    fb.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    fb.setupPost(keepGlyphNames=not cff)
    fb.setupMaxp()
    fb.font['head'].created = fb.font['head'].modified = 2082844800
    fb.font.recalcTimestamp = False
    fb.save(root / ('cff.otf' if cff else 'glyf.ttf'))
