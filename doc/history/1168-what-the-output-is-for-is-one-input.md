# 1168 — What the output is for is one input, and the adjustment it feeds was an AND that should set

2026-09-22. ADR 1173 (the input, the duration, the assignment, §8.9.5.4 step c)) and ADR 1174
(which operations are exports; what a writer owes an image with alternates).

Two owner revisit notes, both about "this device is a screen". Both premises tested: ADR 1106
§5's Export half had expired (export operations exist) and its Print half rested on nothing
listed; ADR 0375's device premise was never on the closed exclusion list either. One correction
to the notes: the Export half's *examples* were mostly wrong. Of the five operations the note
names, four are not exports in §8.11.4.5's sense — FDF/XFDF carries field values no group
governs, an extracted attachment is its own bytes, a clipboard copy is a range of the page
already on screen, and every derived PDF writes a format that *does* support optional content.
`quorra-transform render` is the one that qualifies, on both of the clause's conditions and on
Table 100's own example of a raster image format.

The unplanned finding is the larger one. Building the `Print` event exposed that `apply_view`
implemented §8.11.4.4's per-group rule as an AND with the state the group already had, so a
usage application dictionary could turn a layer off and never on. The clause says "set to ON",
and its own example settles it: under `/BaseState /OFF` with `/ON [1 0 R]`, objects 2 and 3
begin off and the example says the `View` dictionary manages all four groups' states by zoom
level. The correction needs a second one beside it — a category whose entry the group does not
state must recommend *nothing* rather than ON — and the pair is invisible under an AND and
decisive under an assignment.

Measured before building: of the six pdf.js corpus documents that state an `/AS`,
`bug1650302_reduced.pdf` is the only one whose events disagree, and its page states no `/OC` at
all. So nothing moves, which `raster_golden` confirmed (the one page that moved is the overprint
round's — no `/OCProperties`, no `/Alternates`).

Rows: §8.11.4.5 `partial` → `implemented`; §8.11.4.4 and §8.9.5.4 re-read and kept
`implemented`, each with its deferral retired rather than restated.

Files: `crates/pdf-model/src/optional_content.rs`, `crates/pdf-model/src/view.rs`,
`crates/pdf-model/src/content/image.rs`, `crates/pdf-model/src/content/xobject.rs`,
`crates/pdf-model/tests/optional_content.rs`, `crates/pdf-transform/src/render.rs`,
`crates/pdf-transform/src/redact.rs`, `crates/pdf-transform/tests/redact.rs`,
`doc/conformance/ledger.toml`, `doc/adr/1173-*.md`, `doc/adr/1174-*.md`.

What remains is a caller, not a capability: RFC 0004's print path is still unbuilt, and its
section 4 "print intent" is now `Purpose::Print` waiting to be passed on a render request.
