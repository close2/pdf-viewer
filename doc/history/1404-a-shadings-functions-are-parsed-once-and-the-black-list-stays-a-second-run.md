# 1404: a shading's functions are parsed once an interpretation, and the black list stays a second run

Pixels slot, batch sixty-three. ADRs 1644 and 1645; no ledger row moved, no question.

**Premises.** `pdf_colour::shading::Cache` keys a build by its conversion, so a function was parsed
again per build: held, and larger than priced — 137 parses of the file's 41 function objects a run,
145.3 M of the turn's 1 357.6 M instructions (callgrind, `examples/turn_interpret`, a run of two
less a run of one), the chromatic run repeating parses too. The batch's first export of HEAD's arms
held no per-page digest; the second, from a clean tree, did, and is what the arms below compare to.

**Built** (ADR 1644): a memo of parsed `/Function` groups in the cache, keyed by the objects the
entry names (`/Function 8 0 R` and `/Function [8 0 R]` two keys), exact because a parse reads the
document and the object alone, bounded at 2^22 numbers. Turn 1 357.6 → 1 258.9 M, parses 137 → 41.
Clock, `frame_budget`, three interleaved pairs, pinned, load 1.3: turn 144.00–145.37 → 141.00–142.13
ms, seventh 142.16–143.74 → 140.97–141.48, step unchanged. `doc/performance.md` 3e's turn and
seventh rows re-taken; `turn-path.toml`'s two bands re-derived by the file's rule with the reason.

**Decided, not built** (ADR 1645): `examples/group_pair` (new) counts the group's pair on
`bug1721218_reduced.pdf`: 7 078 command pairs equal in every field but paint (6 909 fills, 27
strokes); 2 of 27 chromatic paints pair with several black ones (white with 13, a shading with 18),
because the chromatic plane does not carry black. The black list is not a function of the chromatic
list. Priced: the black run is 486.6 M (about 24 ms) of which its own shading builds are 130.4 M; an
exact form carrying each mark's colour inputs could save at most about 356 M, less a tax on every
page's paints that a builder measures first.

**Unfinished.** The exact form of ADR 1645 section 3, by its decision. Trap 1: no page moved on any
arm, so there was no new picture to look at.

**Gates.** `rustfmt --check` on the three source files: 0. Clippy `-D warnings`, `pdf-colour` and
`render-raster`, all targets: 0. `cargo nextest run -p pdf-colour`: 0, 124 passed;
`-p render-raster`: 0, 100 passed. `cargo test -p conformance`: 0, 397 passed. The three memo tests
watched failing with the key planted wrong twice. Behind the lock: `render-raster --test corpus`
six arms (one build, copied, run by path), all exit 0, digests by page against the batch's export
0 moved of 968 / 968 / 968 / 964 / 963 / 964, verdict lines equal, in 1 474 s; `pdf-model
raster_golden`: 0, held 974, moved 0, in 36 s; `turn_path` twice: 0 and 0, 33 of 33 judged, 0
outside, the page's turn 147.51 and 145.81 ms; `tools/batch.sh raster-examples`: 0, 14 passed.
