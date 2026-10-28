# 2570 — A zero-length dash at an open subpath's end is painted, in every backend

Status: implemented. Session 1868, batch one hundred and twenty-seven, on round 1859's finding that a
dotted rule differed 18 levels between `render-cpu` and `render-raster` at its end. Context: ISO
32000-2 §8.4.3.6, §8.5.3.2; ADRs 0762, 2553. Code: `crates/pdf-render/src/dash_end.rs`, the zero-length
dash routes of `crates/render-cpu/src/stroke.rs`, `crates/render-raster/src/stroke.rs` (`dash`) and
`crates/render-gpu/src/scene.rs`. Test: `crates/render-raster/tests/dotted_rule_end.rs`.

## 1. The difference

`[0 2] 0 d`, `1 J`, `0.6 w`, on a straight rule 100 units long: `render-raster` painted 51 dots and
`render-cpu` 50. The missing one was the last, at the rule's end, and the two lanes were 18 levels apart
at its pixel. `tiny-skia`'s dasher dispenses while the distance it has walked is less than the contour's
length (`while distance < length`), so a dash that begins exactly at the end is never dispensed;
`kurbo`'s, which raster and Vello use, dispenses it.

## 2. What the clause says

§8.4.3.6:

> A stroking operation shall take no measures to coordinate the dash pattern with features of the
> path; it simply shall dispense dashes and gaps along the path in the pattern defined by the dash
> array.

§8.5.3.2, of zero-length dashes in the pattern of a subpath that is not degenerate:

> In the latter case, the line caps shall always be painted, since their orientation is determined
> by the direction of the underlying path except in the case of a degenerate subpath.

The path a pattern is dispensed along runs from arc length 0 to `L` with both ends on it. A zero-length
dash at position 0 occupies the subpath's first point and every reader paints it; one at position `L`
occupies the last point in the same way, and neither clause sets the end apart from the start. Drawn
from its other end, the same rule places its dots at the same 51 points, so a reading that drops the end
dot makes the marks of one geometry depend on which way it was written. The dot at the end is painted:
**`render-raster` was right and `render-cpu` wrong.** The coincidence is the clause's exact one, as in
ADR 0762: decided for a subpath of straight segments, whose length is computable, and not claimed for
one holding a cubic.

## 3. The references

Evidence about the reading, not its ground (principle 5). On the planted page (three rules: 100 units
forward, 101 units, 100 units drawn backward) Ghostscript paints the end dot on every rule and so the
same dots in both directions; MuPDF and Poppler drop it, and so draw the backward rule without the dot
at its own end, the point the forward rule starts on. Two references against two, and the clause
settles it as §2 does.

## 4. The construction

The rule is stated in `pdf-render`, as ADR 0762's closing vertex is, because a decision either lane
makes alone is one neither has made. `extended_to_a_zero_dash_at_the_end` finds each open subpath of
straight segments whose pattern position at its end, `(phase + L)` reduced by the period, is exactly
where a zero-length dash begins, and appends a segment of twice `ZERO_DASH` along its last segment's
direction. The zero-length dash is dispensed at `ZERO_DASH`'s length and the gap after it is longer than
that, so every dasher meets the dash strictly inside the extended path, dispenses it facing along the
path and stops in the gap: one mark and nothing else. A dasher whose own arithmetic placed the dash a
hair short of the end dispenses it there, still once. A subpath that returns to its start where the
start already carries a dot is not extended — the two positions are one point. Each of the three lanes
calls it inside its zero-length-dash route, after §8.4.3.4's opening of a closed subpath, so a pattern
with no zero-length dash, and every butt cap, never reaches it.

The alternative — dispensing zero-length dashes in `pdf-render` itself and handing the dashers a
pattern without them — places every dot exactly, but moves every dotted page by the dashers' arithmetic
and needs an arc-length walk of cubics per backend's types; it was not taken.

## 5. What it costs

One walk of the path's commands for a stroke whose pattern holds a zero-length dash under a round or
projecting cap, and a copy of the path only where an end carries a dash. A dotted curve that ends exactly
on a dash keeps whatever its dasher does there.
