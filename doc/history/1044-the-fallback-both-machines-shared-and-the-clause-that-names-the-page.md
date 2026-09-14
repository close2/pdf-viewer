# 1044 — The fallback both machines shared, and the clause that names the page (`batch-1044-1049`)

Date: 2026-09-14. ADR 1059. Takes ADR 1055 section 5's two questions — a defect no cross-check can
find because both machines share it. Touched `pdf-model`'s `content.rs`,
`content/{xobject,text,pattern,run}.rs`, `tests/missing_resources.rs`; `pdf-archive`'s `survey.rs`,
`tests/cross_check.rs`, a new `tests/resource_fallbacks.rs`, three fixtures; four ledger rows.

## What the clause says
**A form XObject or a Type 3 font stating no `/Resources` is read against the page's dictionary.**
§7.8.3 stated the fallback in a fourth bullet until Errata Collection 3 Issue #128 retired it into
an informative NOTE 3; both name "the resource dictionary of the page on which they are used", and
Table 93's cell says the same as a `shall` for the files that omit the entry — "all named resources
used in the form XObject shall be included in the resource dictionary of each page object on which
the form XObject appears". For a glyph description that erratum replaces §9.6.4's step d) and Table
110's fallback with §7.8.3's four-step search, whose last two steps are that page and what §7.7.3.4
gave it. **No sentence anywhere names the invoking stream.**

**A tiling pattern stating no `/Resources` inherits nothing.** Table 74 makes the entry
"( Required )" and §7.8.3 requires it of "form XObjects, patterns, and annotation appearances"
alike; the leniency names a form, a Type 3 glyph description and an appearance stream, never a
pattern. A required entry absent is a malformed pattern, and the answer is a refusal by name rather
than a fallback: every name the cell uses is reported, what needs no name is still drawn (trap 5).
That is ADR 1059's decision; the rest the clause decided.

## What both machines do now
The interpreter holds the page's dictionary once (`Interpreter::page_resources`); `draw_form`,
`draw_type3_glyph` and `build_soft_mask` (round 1045's, coordinated onto the same field) read it,
`tiling` reads `unwrap_or_default`. The survey holds `Walk::page_resources`, set at each page, and
its `form`, `type3` and tiling arm do the same three. The rule is stated once.

## The calibration (traps 13 and 8)
No corpus document witnesses either reading — 1038 measured that — so three fixtures carry them,
each built so the page's dictionary and the invoker's disagree, and `cross_check.rs` runs over them
every round rather than with the corpus. With each old reading planted back into **one** machine
(form → interpreter, Type 3 and pattern → survey): `8 places both machines ran, 10 selections
compared, 10 disagreements`, exit 101, each naming document, page, the run of streams, the operator
ordinal, the category and both outcomes. Unplanted: `0 disagreements`, exit 0. The six per-machine
tests were calibrated the same way, each naming the inverse report. Rendered at 4× and looked at
(trap 1): the page's square draws in both form cases, the cell draws nothing and says `/Sq is not in
/XObject`. Statuses unchanged — §7.8.3, §9.6.4 `implemented`; §8.10.2, §8.7.3.1 `partial`.
