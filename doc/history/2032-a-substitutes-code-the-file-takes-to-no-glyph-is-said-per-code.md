# 2032 — A substitute's code the file takes to no glyph is said per code

Session 2032, a font and PDF/A round in the audit worktree. Rows 9.6.5.4 and 9.6.2.1; ADR 2880; Q859.

**The slice.** Round 2027's two findings and its owed census: a substituted font's code reaching no glyph is
reported per code (`issue13316_reduced`), a substituted font with no `/Widths`, and R09's census re-taken.

**The premise.** Partly held. `issue13316_reduced`'s `/ToUnicode` decodes (65 is U+5F00); it selects nothing,
§9.10.2 being about extraction. A substitute with no `/Widths` advances by the resembling standard font's
published metrics, the face's only outside the standard set. Built as Table 120's `/MissingWidth`, `issue5470`
drew its line as one pile at 0 (trap 1); a file missing Table 109's required entry is one the standard does not
describe, so the reading is the owner's (Q859) and was withdrawn. ADR 2881 is unused.

**Built.** `fill_substitute_table` takes §9.6.5.4's last step in the face, its `post` table by name.
`LoadedFont::unreached_in_substitute` answers a simple substitute's code that the encoding names nothing, or names
with a name neither the Glyph List nor the face reaches; `draw_program_code` counts it where §9.10.2 names a mark
it owed, and the page says the count, the first code and the clause whatever else the font drew. A character the
machine's face lacks stays ADR 0152's. `corpus.rs` classes the report as the file's.

**Tests.** `substitute_unreached_codes.rs` (four, with the witness), the `post` route in `substituted.rs`, and
`metrics.rs`'s two pins of the present layout (a standard 14 font; a substitute with no `/Widths`). **Plant**:
`unreached_in_substitute` answering `None` and the `post` step removed fail four (three window, one unit).

**Trap 11.** First counted on every code with a non-space readback, the report named `issue18059`,
`standard_fonts`, `issue20489` and `poppler-90-0-fuzzed`: codes the file names nothing for. Gated on a named mark,
it names `issue13316_reduced` alone, which was incomplete already.

**R09.** Re-taken with the CJK pack over every file of `doc/pdf.js` and `doc/corpora`, as its lines count them;
the census walks no corpus cache (safedocs, restored, is outside it), and tika-issue-tracker and openpreserve are
absent from the cache. `issue13316_reduced`
moved from R08 to R09 as `not-built-yet`. Section 2's table and section 9's R09 line are `rank.py`'s output, and
R09's paragraph names the command where it gave counts.

**Gates.** Over `pdf-font`, `pdf-model`, `pdf-transform`, `pdf-archive`: fmt 0; clippy `-D warnings --all-targets`
0; nextest 4587 of 4587 passed; `cd fuzz && cargo check --bins` 0; conformance 0 after `git add -N`. Tier 2:
`corpus` 101 as HEAD's (61 incomplete, the same pair joined); `raster_golden` 101, `issue13316_reduced` moved in
its reports only, re-taken alone, then the 23 of HEAD by name. Census 0, `rank.py` 0; `state.sh docs` 0.

**Left.** Rows stay `implemented`. 9.6.2.1 waits on Q859; a composite substitute's code with no character names no
mark and keeps ADR 0152's report; `issue13316_reduced` stays blank, a Latin face asked for `/g5167`.
