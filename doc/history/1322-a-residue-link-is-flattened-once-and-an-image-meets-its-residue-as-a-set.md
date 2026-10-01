# 1322 — A residue link is flattened once a frame, and an image meets its residue as a set

Raster, from the clauses alone. ADRs 1479, 1480. No question. `crates/render-cpu/` was not opened (A76).

**Measured first (ADR 1479 section 1).** `bug1721218_reduced.pdf` is drawn as two own-space frames,
each one encoder. Each frame has 6 770 residue asks over 3 515 chains, which are distinct by id
and by content alike. One chain of 111 677 points is asked 3 025 times over 2 × 1 to 3 × 2
tiles. Its region was declined as dearer than those tiles, so every ask flattened the chain again
and filled every edge. Content keying could not pay: the repeats already shared one id.

**Built, all byte-identical (ADR 1479).** A link's flattening is kept per frame by
`(outline, transform bits)`. From its second use its edges are listed by device row, and a tile
reads its rows' edges merged in walk order. The topology question asks only the listed subpaths,
and a slab wholly beside a tile is deposited at the border directly. The exact meet now adds left
partial edges to bands by two searches. Callgrind on the page went 488 G → 11.0 G and
`zoom_frame` 7 248 ms → 556 ms, against the CPU backend's 84 ms. The floor: per-tile fill
(~170 ms) and the meet (~120 ms). Admitting the region reaches 387 ms but moves bytes (ADR 0049),
so it was not taken. Levers (b) and (c) do not apply: one encoder per frame, and every chain is
one link.

**Image lane (ADR 1480).** An axis-preserving image's rectangle meets its residue by area in the
pixels both cut. Fixture: a quarter-turned image under the 64-gon is held to the closed form on
three arms; it failed under `min`. Oblique images stay hard-edged (ADR 0011), and `min` is exact
for that set. The fixture also exposed a horizontal edge crossing a pixel that did not bound a
band; this is fixed, with a unit test watched failing.

**Corpus.** These are digests from exported trees, each with its own target directory. Against
HEAD, ADR 1479 is 0 pages different at 1× and 4× on the CPU, compute and GPU lanes, with
one-versus-many 0. ADR 1480 moves 4 pages at 1× and 3 at 4×, all by the band fix; none moves by
the image meet. Two moved closer to the oracle and the rest are equal to four places. Verdicts are
unchanged.

**Gates.** See the report. Proposed trap: a cache keyed by id was blamed for not seeing repeats
that it already saw; count asks per key before re-keying.
