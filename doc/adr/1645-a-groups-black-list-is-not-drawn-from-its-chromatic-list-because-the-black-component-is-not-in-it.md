# 1645 — A group's black list is not drawn from its chromatic list, because the black component is not in it

Session 1404. Status: **accepted**. Answers ADR 1632 section 5's first item, the design change it
priced at 26 to 33 ms. Amends nothing; the black run stays as ADR 0272 built it.
Context: ISO 32000-2 §8.6.7, §11.3.4, §11.4.7; ADRs 0272, 1529, 1632, 1644; trap 13.
Code: `crates/render-raster/examples/group_pair.rs` (new: the count below).

## 1. The question

A group whose blending space has four components is its content run twice, once on each plane, and
the second run is most of the costliest turn in `doc/checks/turn-path.toml`. ADR 1632 asked whether
the black list could be made from the chromatic one — its colours re-resolved instead of the content
interpreted again — and named the danger: every colour decision reproduced from a list, each a place
the pair could part without a report. The brief's rule: build it only if a count shows the bytes are
the same by construction; otherwise price it and say what would make it exact. ADR 1529's reuse is
not this question: that is raster drawing two lists that already exist from one geometry.

## 2. Counted

`examples/group_pair` walks every four-component pair on a page and compares the two lists field by
field. On `bug1721218_reduced.pdf` page 1, one pair, 7 078 command pairs (7 022 fills, 28 strokes,
28 groups):

- **Equal on every pair**: variant, transform, fill rule, stroke, clip, blend mode, group flags and
  alpha, and every path by value — although 7 022 of 7 022 are a second allocation, the black run
  having built each path again. Three pairs name two soft masks; all three are equal by value.
- **Different**: the paint, on 6 909 fills and 27 strokes.
- **The question the design turns on — is each black paint a function of the chromatic paint
  beside it? No.** The chromatic list holds 27 distinct paints and two of them are paired with more
  than one black paint: an opaque white solid with 13, and one axial shading with 18. That is the
  arithmetic of the planes (§11.3.4's additive complements, `colour::process_plane`): the chromatic
  plane carries one minus cyan, magenta and yellow, the black plane one minus black, so every colour
  whose first three components are zero is white on the chromatic plane whatever its black is. The
  fourth component is not in the chromatic list, so no function of that list makes the black one.

So the construction ADR 1632 described cannot be exact, and it is not built.

## 3. What would make it exact, and what that would buy

The black list is a function of the colour *inputs*, not of the chromatic colours. The exact form is
a run that carries, beside each mark made while a four-component group runs, what its colour was
resolved from — a solid's space, components, rendering parameters and black generation; a shading's
object, resources and conversion; an image's plan — and then makes the black list by applying the
same resolution to the same inputs under the black plane. That is exact by construction because each
resolution is one pure function asked twice. Two things the count did not exercise are in it too:
§8.6.7's overprint mode is decided per plane (`Interpreter::special_overprint` reads the fourth tint
on the black plane and the first three on the chromatic one), so a blend mode is a resolved input as
well; and the soft masks a pair names are equal by value, so the black run's second registration of
each could be the first one's.

The places that would carry inputs, counted in `crates/pdf-model/src/content*`: 8 calls of
`Interpreter::colour`, `conversion` and `conversion_under` once each, the image plan's conversion,
the two arms of the overprint decision, and 33 reads of `self.compositing` — each a site where the
run asks which plane it is on, and each a place the carried form could disagree with a second run
with nothing to report it.

Priced by callgrind on the tree with ADR 1644 (gates profile, one thread, a run of two less a run of
one): the turn is 1 258.9 M; with the black run replaced by a copy of the chromatic list (wrong bytes,
a probe) it is 772.4 M, so **the black run is 486.6 M, about 24 ms**. Of that, the black plane's
shading builds are 130.4 M, which the exact form still pays because they are the black colours
themselves; parsing is no longer in it (ADR 1644). **The most the exact form could save is about
356 M, 18 ms**, less what carrying the inputs costs. That carrying is a branch on every paint of every
page, which is the shape ADR 1632 section 4 measured for a token recorder at 0.19 to 0.38% of an
ordinary page; a round that builds it measures that tax on ISO 32000-2 page 101 before anything else.
