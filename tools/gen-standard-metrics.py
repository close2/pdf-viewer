"""Regenerate the standard-14 metric tables in `crates/pdf-font/src/standard_metrics.rs`.

Reads `doc/pdf.js/src/core/metrics.js`. Only the width tables are generated; the module
documentation, the `StandardFont` type and the tests around them are written by hand and
are preserved.

pdf.js is Apache-2.0, so the numbers can be redistributed; see ADR 0007. The shape of the
source is a top-level lookup table whose entries are either a single number (the fixed
pitch Courier faces) or a nested table of glyph name to width.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SRC = ROOT / 'doc/pdf.js/src/core/metrics.js'
OUT = ROOT / 'crates/pdf-font/src/standard_metrics.rs'

lines = SRC.read_text().splitlines()

TOP_FLAT = re.compile(r'^  t(?:\.([A-Za-z]+)|\["([A-Za-z-]+)"\]) = (\d+);')
TOP_TABLE = re.compile(r'^  t(?:\.([A-Za-z]+)|\["([A-Za-z-]+)"\]) = getLookupTableFactory')
GLYPH = re.compile(r'^    t(?:\.([A-Za-z0-9]+)|\["([A-Za-z0-9.]+)"\]) = (-?\d+);')

fonts = {}          # name -> dict(glyph -> width)  or  int for fixed pitch
current = None
in_basic = False

for line in lines:
    if 'getFontBasicMetrics' in line:
        in_basic = True            # a second, unrelated table follows; stop collecting
    if in_basic:
        continue

    m = TOP_FLAT.match(line)
    if m:
        fonts[m.group(1) or m.group(2)] = int(m.group(3))
        current = None
        continue
    m = TOP_TABLE.match(line)
    if m:
        current = m.group(1) or m.group(2)
        fonts[current] = {}
        continue
    if current is not None:
        m = GLYPH.match(line)
        if m:
            fonts[current][m.group(1) or m.group(2)] = int(m.group(3))

print(f'fonts: {len(fonts)}', file=sys.stderr)
for name, value in fonts.items():
    kind = 'fixed pitch' if isinstance(value, int) else f'{len(value)} glyphs'
    print(f'  {name}: {kind}', file=sys.stderr)

EXPECTED = {
    'Courier', 'Courier-Bold', 'Courier-Oblique', 'Courier-BoldOblique',
    'Helvetica', 'Helvetica-Bold', 'Helvetica-Oblique', 'Helvetica-BoldOblique',
    'Times-Roman', 'Times-Bold', 'Times-Italic', 'Times-BoldItalic',
    'Symbol', 'ZapfDingbats',
}
missing = EXPECTED - set(fonts)
extra = set(fonts) - EXPECTED
if missing or extra:
    print(f'MISSING {missing}  EXTRA {extra}', file=sys.stderr)
    raise SystemExit(1)

# Rust identifiers for the fourteen faces, in the specification's own order.
VARIANTS = [
    ('Courier', 'Courier'), ('CourierBold', 'Courier-Bold'),
    ('CourierOblique', 'Courier-Oblique'), ('CourierBoldOblique', 'Courier-BoldOblique'),
    ('Helvetica', 'Helvetica'), ('HelveticaBold', 'Helvetica-Bold'),
    ('HelveticaOblique', 'Helvetica-Oblique'),
    ('HelveticaBoldOblique', 'Helvetica-BoldOblique'),
    ('TimesRoman', 'Times-Roman'), ('TimesBold', 'Times-Bold'),
    ('TimesItalic', 'Times-Italic'), ('TimesBoldItalic', 'Times-BoldItalic'),
    ('Symbol', 'Symbol'), ('ZapfDingbats', 'ZapfDingbats'),
]

out = []
for ident, name in VARIANTS:
    value = fonts[name]
    if isinstance(value, int):
        continue
    const = ident.upper() + '_WIDTHS'
    entries = sorted(value.items())          # sorted so lookup can binary search
    out.append(f'/// Advance widths for `{name}`, sorted by glyph name.')
    out.append('#[rustfmt::skip]')
    out.append(f'static {const}: [(&str, u16); {len(entries)}] = [')
    row = []
    for glyph, width in entries:
        row.append(f'("{glyph}",{width}),')
        if len(row) == 4:
            out.append('    ' + ' '.join(row))
            row = []
    if row:
        out.append('    ' + ' '.join(row))
    out.append('];')
    out.append('')

# Only the tables are generated; the module's documentation and lookup code are written
# by hand and preserved across regeneration.
existing = OUT.read_text()
marker = '/// Advance widths for `Helvetica`, sorted by glyph name.'
head = existing.split(marker)[0]
tail_start = existing.find('#[cfg(test)]')
tail = existing[tail_start:] if tail_start != -1 else ''
OUT.write_text(head + '\n'.join(out) + '\n' + tail)
print(f'wrote {OUT}', file=sys.stderr)
