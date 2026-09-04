# 933 — The arm swept rather than corrected a fifth time, and a requirement written in the indicative

Date: 2026-09-04.
ADR: 0906 (every subtype Table 171 defines, read against one catch-all sentence, and the watermark
placement that was never a printing question); 0907 (the next two blame bands, and the sweep's
second blindness).
Question: `doc/questions/Q33` — does §13.4's poster image come off principle 5's clause 13
exclusion?
Files: `crates/pdf-model/src/appearance.rs`, `crates/pdf-model/src/annotation.rs`,
`crates/pdf-model/tests/annotations.rs`, `doc/conformance/ledger.toml`,
`doc/todo/01-ledger-partial-rows.md`, `doc/todo/25-view-dependent-annotations.md`,
`doc/todo/README.md`, `doc/rfc/0004-print-and-print-preview.md`, `doc/questions/Q33-*.md`,
`doc/adr/0906-*.md`, `doc/adr/0907-*.md`.

A coverage round in two halves, both set by session 930.

## First half — the arm, swept

`appearance::construct`'s catch-all told six annotation subtypes *"its clause states no
geometry"*, and three sessions had each taken one member out of it after reading that member's
clause: the caret (ADR 0457), the redaction (ADR 0461), the screen annotation (ADR 0901). Three
corrections of one construct is one defect in how the arm was written, so this round swept the
population instead of waiting for the fourth. Table 171's own list is the population, twenty-eight
subtypes, and the verdicts are in the test's doc comment rather than in prose.

| verdict | subtypes |
|---|---|
| drawn from what the clause states, unchanged | `Text` `Link` `FreeText` `Line` `Square` `Circle` `Polygon` `PolyLine` `Highlight` `Underline` `Squiggly` `StrikeOut` `Ink` `Widget` `FileAttachment` `Sound` |
| the clause states the outcome — nothing drawn, nothing owed, unchanged | `Popup` `Projection` `Screen` |
| refused in the clause's own terms already | `Stamp` `Caret` `Redact` |
| **refused with a false sentence, fixed** | `Movie` `Watermark` `PrinterMark` `TrapNet` `3D` `RichMedia` |

The two the sentence was outright false of are `Movie` — §13.4's Table 306 states a `/Poster`
image `XObject`, so what refuses it is the clause 13 exclusion and not a silence — and
`Watermark`, whose Table 194 is geometry from end to end, all of it about where an appearance
*goes*. `PrinterMark`, `TrapNet`, `3D` and `RichMedia` each say the appearance stream **is** the
artwork and then require the entry, which is a stronger refusal than the one they were given.

**What the catch-all is left with is the case the old sentence was most obviously wrong about**: an
annotation stating no `/Subtype`, which has no subtype clause at all.

**And reading §12.5.6.22 for the arm found a live departure the ledger explained away.**
`/FixedPrint` was "a printing decision this program does not make" in three places. The clause
introduces its behaviour with "When rendering a watermark annotation with a FixedPrint entry, the
following behaviour shall occur", replaces the rectangle §12.5.5's steps 2 and 3 place the
appearance onto, and states the on-screen media dimensions itself, twice. It is reported now
(`annotation::fixed_print_owed`) and `doc/todo/25` prices the placement, whose one underivable term
is named.

**And the module header above the arm was stale in the same way.** One of its bullets listed
`FileAttachment` and `Sound` beside `Stamp` as refused for an unstated icon; `symbol_icon` has
drawn all six of §12.5.6.15's and §12.5.6.16's names since `a1f9d43a`. `doc/habits.md`'s *a comment
that names a refusal outlives the refusal* is ADR 0105, and ADR 0105 is about this same header.

## Second half — the next two blame bands

Eight rows, rank 656 and rank 667, all eight flagged by `--bin permitted`. Three moved to
`implemented` (§12.8.4.1, §12.8.4.5, §12.8.5.3), five kept `partial` and four of those five had
their stated reason rewritten. §12.8.5.3 is ADR 0897's shape again — a `shall` on this reader, in
the subclause's last paragraph, that the row had never named and that this tree meets by
construction.

**The finding is the sweep's second blindness.** §12.7.8.3.2 is flagged as quoting the standard
with no modal verb, and it is right about the words and wrong about the debt: the clause's
requirement on a reader is written in the indicative — "importing a field causes the values of the
entries … to replace those of the corresponding entries" — so an unapplied optional entry is a
requirement declined rather than a permission declined. The twelve rows in that bucket are to be
read by hand, never moved on the flag.

## What the round found about evidence

Six ledger rows across three families cited a test that asserts a different clause — §12.5.6.17,
§12.5.6.20, §12.5.6.21, §12.5.6.22 and §14.11.6.2 all named
`an_unknown_subtype_still_draws_its_normal_appearance`, which builds a subtype outside Table 171
and asserts Table 167's `Invisible` row, and §14.11.3 named a flags test that states no printer's
mark. `--bin pointers` cannot see any of it: the citation resolves and simply asserts something
else. Third session in a row to find that shape.

## The one gate that failed, and it is not this round's

`doc/todo/02` §2's full sequence, run whole on this branch: **30 lines green, one red** — the
`launch_path` line, on all four documents' `peak_mib` and on `bug1815476.pdf`'s cold open. The
memory figure is the one worth reporting, because it has no clock in it and no probe can decline
it: all four rows failed **below** their floors, by about 13%, and reproduced within a kilobyte on
a second run. That is `doc/checks/launch-path.toml`'s own documented phenomenon — the graphics
driver's allocation moving under a band derived before it moved — happening a third time.
**No band was moved.** A coverage round that did not touch the launch path is the wrong round to
re-derive a driver's number, and `doc/todo/42` now carries the figures and what is owed.

## What the next round takes

`doc/todo/01`'s rank 674 band (`961615a1`): §12.3.2, §12.8.2.3, §7.6.4.3.2 and §9.8.2. And
`doc/todo/25`'s `/FixedPrint` placement, which is now a named departure with one derivation left in
it rather than a capability nobody has.
