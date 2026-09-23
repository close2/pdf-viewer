# 1314 — A section of a project document is written in words, and still attributes its quotation

Status: accepted and **built**. Session 1238.
Context: `raster/` (comment lines only), `tools/conformance/src/citation.rs`,
`tools/conformance/src/cited.rs`, `tools/state.sh`.

`§N` is ISO 32000-2 and nothing else.

## 1. The rule, applied to `raster/`

The rule the round briefs carry is that `§` in any comment means ISO 32000-2 to the gates, and
another document's section is written "section N" with the document named. `raster/` predates
the rule in its own tree: its comments wrote `§5` for the brief's failure contract
(`raster/doc/RENDER_LIBRARY.md` section 5), `§11.2` for the brief's second question, `§8.2` for
a section of the caller's `QUORRA_FEEDBACK.md`, and `§A` to `§D` for `outline_upload.rs`'s own
four measurements. `cargo run -p conformance --bin cited` counted 135 of its (file, clause)
pairs as clauses with no ledger row, and others it did not count at all, because they landed on
a clause that exists: `§6.1`, `§6.2` and `§6.3` are the brief's performance contract and ISO
32000-2's conformance subclauses, and `§11.1`–`§11.5` are the brief's five questions and five
subclauses of clause 11. The sweep read each against its sentence and rewrote only comment lines:
a bare brief section became "brief section N", a document already named kept its name and lost
the sign, and a `§` that is ISO 32000-2's stayed (`§11.2`'s NOTE on committing to a raster is the
standard's, and `§11.5` beside a soft mask is too).

What is left is printed, not written: `tools/state.sh cited` lists the no-row pairs under
`raster/` on lines of their own. Ten are string literals in code, which a comment sweep does not
touch, and one is `§11`, a whole clause of the standard, which the ledger has no row for.

## 2. The scanner reads the words too

The quotation gate attributes a blockquote to the nearest attribution above it in the same
comment, and until this round a project document was an attribution only when a `§` followed its
name. Rewriting `` `raster/doc/PLAN.md` §5 `` as `` `raster/doc/PLAN.md` section 5 `` therefore
cost sixteen blockquotes their attribution: fourteen became quotations nothing could check, and
two fell through to an ISO clause cited higher up and failed as not verbatim. A rule that makes
the correct spelling fail a gate teaches the wrong one, so `citation::section_in_prose` now reads
a document of this project's own named immediately before the word "section" as an attribution.
Another standard written the same way — `ISO 19005-2 section 6.7` — does **not** attribute:
those texts are cited and paraphrased, never quoted, and a blockquote below one stays reported.
A line that cites a clause of ISO 32000-2 still attributes to the standard whatever else it names.
The sixteen are attributed again, and the count of quotations checked against the standard is
unchanged.
