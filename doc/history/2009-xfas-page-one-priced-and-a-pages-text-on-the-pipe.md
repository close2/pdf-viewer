# 2009 — XFA's page one priced, and a page's text on the pipe

Session 2009, a single ledger round, the ledger lane. A828 group 4; `doc/todo/76` rows 10, 5 and 8; rows K and
9.10.1; ADRs 2836 and 2837; Q840.

**The slice, and why.** The brief's order: row 10's measurement first, then row 5, then row 8 as far as the wall
budget went. The first two are done; row 8 is left. Rows 6, 7 and 9 and ADR 2833's reporting are later rounds'.

**The premise.** Held for both: row 10 is a measurement only (nothing of the dynamic half built), row 5 was unbuilt
and effort only — `Interpreted::text` was already kept, `Find` and `Selection` read it. Two briefed facts did not
hold: the C ABI has no generic query, so "one number in `QUORRA_*_COUNT`" is an accessor of its own, left; and
row 8's ADR 0705 deferral is a scene-reuse optimisation in `render-raster`, unbuilt, which a later round prices.

**Measured (row 10).** `pdf-model`'s `xfa_dynamic_census` example, this tree's own reader, over 70 453 held PDFs
(safedocs 65 968, pdf.js, veraPDF, `doc/corpora`, GWG; tika and openpreserve are not restored): 49 state an XFA
resource, 10 `/NeedsRendering`, 3 of them the crawl's. pdf.js's 55 `xfa_*.link` forms were fetched (53 arrived) for
the shapes. `tools/xfa-census.py` counts the layout grammar per resource. Page one costs the merge, the scripts and
the drawing whole (XFA 3.3 pages 288, 308, 388): about 21 to 26 rounds of 23 to 28. `doc/todo/78`; Q840 asks the
amendment with a recommendation (accept, build after the ledger, lay out at `off` without scripts and say so).

**Built (row 5, §9.10.1).** `Query::Text` → `Answer::Text(Vec<PageText>)`: the pages on the screen, the characters
`Find` searches; on the confined pipe as kind 38 both ways (ADR 2836).

**Tests.** `viewer-core/tests/page_text.rs` (WinAnsi octal 351 is Annex D.2's `eacute`, §9.10.2's glyph-name method
gives U+00E9; `Find` finds the same text); `viewer-confined` round trips (38 questions; two pages' text, non-ASCII);
`conformance/tests/xfa_census.rs` runs the census's `--self-test`. **Plants**: an empty text in
`viewer-core`'s answer and an empty string on the pipe fail the two tests this round wrote; reversed, they pass. The
census: a mutated `occur` count fails the self-test, and the corpus run re-found RFC 0011's four by name.

**Gates.** `cargo fmt --check` and clippy `-D warnings --all-targets` on `viewer-core`, `viewer-confined`,
`pdf-model`, `conformance`: exit 0; nextest of the first three: 3286 of 3286 passed; `cd fuzz && cargo check
--bins`: exit 0; rustdoc `-D warnings --document-private-items` on the two viewer crates: exit 0; conformance after
`git add -N`: exit 101 on `records` (this record's gates unwritten), then exit 0; `state.sh docs`: exit 0
after 8 350 s under a load above 300. Tier 2: `corpus` exit 101, 61 incomplete; `raster_golden` exit 101, 23 moved — the same 23 by name as round 2007's walk of HEAD, so nothing moved; `selection_census`
and `accessibility_census` exit 0; `awkward_classes` exit 0. `launch_path` not run (load above 300);
`script_corpus` not owed, no script reached.

**Left of the rows.** 9.10.1 stays `implemented`; K stays `implemented`, its dynamic half excluded until Q840. Row 5
leaves the C ABI's accessor; row 8 is unbuilt (ADR 0705's dots moved into the encode). Trap proposed: a census over
a rotating, shared disk reads where the offsets point and in parallel, or it reads at seek speed.
