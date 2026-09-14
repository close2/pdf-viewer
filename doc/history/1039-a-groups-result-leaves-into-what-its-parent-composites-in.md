# 1039 — a group's result leaves into what its parent composites in

Date: 2026-09-14. ADR 1056. §11.6.6 and §11.7.2, one debt carried by two rows: a group whose `/CS`
differs from its parent's where the parent composites in a space of its own. Touched
`content/transparency.rs`, `content.rs`, `colour.rs`, `pdf-render/src/display_list.rs`,
`examples/group_space_census.rs`, the tests and the ledger. Both rows stay `partial`, narrowed.

**What the tree did.** A group in a space of its own was drawn only under a device parent; under a
press, a `CalRGB` or a grey parent it was recorded and the whole page fell back to the device with
a report. §11.6.6 asks for the conversion instead: "[i]f colour conversion needs to take place in
order to composite the group into its parent, the rendering intent and black point compensation
from the graphics state at the point of invocation of the Do operator shall be used".

**What it does now.** `Interpreter::group_compositing` builds every pairing of the spaces this tree
draws — press, one component, three CIE-based, and the device's own three inside a parent of
another kind, a press inside a press and a group inside a soft mask's group included — and
`conversion_into_parent` composes the group's conversion out with the parent's conversion in
(`parent_channels`: the marks' own `Compositing::paint`, or `RgbRoute::components_of_srgb` for a
CIE-based parent of three, since a group's result is not §11.7.2's `DeviceRGB` object). The
backend still resolves one grid, curve or cube per group; `blending_paired` lets a pair's halves
pair inside a pair. A change nothing composites through no longer costs the page its space.

**Population.** `group_space_census` gains a parent -> group table and takes `@paths.txt`. Over
88 890 documents (`doc/pdf.js`, `safedocs`, `tika`, `openpreserve`), 31 hold a group whose space
differs from a non-device parent's: 21 of `safedocs`, 10 of `tika`, none of `doc/pdf.js`. Looked
at (trap 1): `PDFBOX-4095-0` p20, `MOZILLA-655276-4` p1, `PDFBOX-3000-41` p1 and
`MOZILLA-1519218-0` p8 agree with mutool and poppler and report nothing; `GHOSTSCRIPT-691218-1`
p1 lacks its photograph and price figures in this tree and in the pre-round tree alike — a
standing miss on that page, not this round's. Remainder: an isolated knockout group naming a
four-component space keeps its report (§11.4.6's rewrite would have to reach both halves of a
pair; no corpus witness); `Lab` and the special spaces as a `/CS` are Table 145's own exclusions.

**Gates.** Conformance 267/267; `transparency_groups` 61/61, and the six tests this round wrote
or rewrote all fail with `group_compositing`'s nested branch planted back to `None`, nothing else
(trap 13). `raster_golden`: held 971, moved 3, all "list only" — `bug1703683_page2_reduced` and
`personwithdog` are last-ulp shading-corner colours from a sibling's in-flight `mesh.rs`, their
lists byte-identical when only this change is planted onto the pre-round tree;
`bug1721218_reduced` is this round's, its luminosity masks' isolated `/DeviceCMYK` groups now
compositing in ink inside the mask (1 -> 11 pairs), no pixel moving. The oracle passes; tier 1's
fmt failure is siblings' files (`pdf-font/metrics.rs`, `mesh.rs`), this round's are formatted.
