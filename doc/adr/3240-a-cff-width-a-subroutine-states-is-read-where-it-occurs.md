# 3240 — A CFF width a subroutine states is read where it occurs

Session 2134, an archive round in the `lane3` worktree. Status: **accepted**; no code changed,
one test added. Answers `doc/pdfa-blocking-issues.md`'s R10 sentence "a CFF width in a subroutine
(`issue_cff_unsigned_bbox`)". Context: ADR 3185 (the width a charstring omits), ADRs 2540 and 2610
(the width reader and writer); `crates/pdf-font/src/cff_width_tests.rs`. Clauses: ISO 32000-2
§9.6.2.1 through Adobe Technical Note #5177 section 3.1.

## Context

The note places the width as the first operand of the first stack-clearing operator, with an odd
operand count, wherever that operator occurs. `issue_cff_unsigned_bbox`'s charstrings reach their
first stem hints through nested local and global subroutines, and R10 recorded the
document as refused for it. Read again: the document converts at 2b and 4 under both shipped
profiles, veraPDF 1.30.3 passes all four outputs, and page 1 at 150 dpi is pixel-identical to the
source's; it is in no census row.

## Decision

The reading the window uses (`pdf_font::cff::charstring_widths` and `advances`, both through
`read-fonts`' evaluator, whose operand stack is shared across `callsubr` and `callgsubr`) already
follows the note. A test now holds it for the three shapes: an operand a subroutine pushes and the
caller's `endchar` clears, an `hstem` a subroutine states, and an `endchar` inside a subroutine.
The writer's side was held before (`a_width_behind_a_nested_call_and_a_mask_is_restated`). R10's
sentence is retired.
