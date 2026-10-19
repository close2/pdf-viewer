# 1596 — A host speaks what the tree resolved, a `Private` element is not presented, and a circular role map stops where it returns

Slot 5 of batch ninety-four, 2026-10-19, a clause round. ADR 2028; 2029 is unused; no question.

**Premise and hypothesis.** The premise held three times: a circular role map was walked
`MAX_DEPTH` laps and answered by parity; `Private`'s children were lifted into the platform tree;
a host re-read the role name, so a foreign `Table` would cross as a table. The hypothesis held:
counted first with pikepdf over 3 882 files (veraPDF corpus, pdf.js), no `Private` and no
homonym; 15 files carry a `/RoleMap` cycle, 13 a standard name mapped to itself (as before); two
veraPDF PDF/UA fail files hold the one two-name circle (`Standard` ↔ `Text body`), whose four
elements now answer its other name, said circular, as two non-standard self-maps are. No element
changes whether it is a standard type.

**Built.** `Tree::mapped_role`: the name and the standard type, or `Nonstandard`'s reason
(`Unstated`, `Undefined`, `Foreign`, `Circular`), in one walk that stops at the first (name,
namespace) pair it returns to and answers with the first standard type along the circle, else the
last new name. `AccessibilityNode::standard` carries it; `viewer_accessibility::role::map` takes it
and reads the name only for the description, which says the reason; the confined wire carries it
as one byte (greeting `PDFVCF17`, `doc/ui-boundary.md` moved with it). `viewer-core`'s walk gathers
neither a `Private` element nor its descendants and still reaches them (`withhold`) so §14.7.5.4's
agreement holds; the census counts both.

**Rows.** §14.7.3, §14.8.4, §14.8.6.2 `implemented` → `implemented`, notes and tests extended;
"A PDF processor using the role map needs to follow the chain of associations until it either
finds a structure type it recognises or returns to one it has already encountered."

**Gates.** rustfmt clean on every file touched; `RUSTFLAGS=-D warnings cargo clippy --all-targets`
of `pdf-model`, `viewer-core`, `-accessibility`, `-confined`: exit 0; their nextest: 2 690 passed;
`cargo check` of `viewer-ffi`, `-host`, `-ui`, `-gtk`, `-qt`: exit 0; `cargo test -p conformance
--no-fail-fast`: 412 passed, 2 failed (`records`, on siblings' unfinished 1592 and 1598);
`cargo doc`: no warning on an added line. Planted: the old walk, and the `Private` arm removed,
each fail the test written for it. `t3-accessibility` in HEAD's export and
this round's (HEAD plus its 10 code paths): exit 0 both, every count and ratchet identical, the new
counts 0, the undefined names now with their reason (28 `Foreign`, MathML's; 3 `Undefined`). Drive
step `26-accessibility` on the export's release binaries: works in the three windows with a bus
bridge (`quorra-confined` has none; the full drive skips it there). The six arms on the export:
968, 968, 968, 965, 965, 965 pages, 0 digests differing from HEAD's, 0 pages in one only.

**Unfinished.** `Tree::logical_range`, the copy in logical order, still copies a `Private` element's
text (`structure.rs` was this round's for the cycle stop only). Duration: 7 500 s.
