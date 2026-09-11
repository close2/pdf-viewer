# 964 — The second parser over the same carried files

Branch `round-945/the-fifth-round`, after 963. Stream: the viewer.

## What the round was asked, and what it answered

Generalise session 960's method: **for every compiled-in bound and every compiled-in table, ask
whether the data this binary already carries exceeds it or contradicts it.** The census is in
ADR 0971 and covers `data/icc/`, `data/cmaps/` and `data/standard-fonts/` against every bound
each passes through. Five of the seven rows are clear by wide margins; one was already asserted
by session 960; one was not being asked at all.

That one is the finding. Adobe's 240 `CMap` files go through **two** parsers — `cmap.rs` for
§9.7.6.2's code-to-CID lookup, and `tounicode.rs` for §9.10.2's third method and §9.10.3's
`/UseCMap` — and session 960 built the census for the first. `tounicode.rs`'s three bounds were
`if len < MAX { push }` with no `else`: a `/ToUnicode` cut short answered nothing for the codes
past the cut, and `is_complete` stayed true. Where the font is a composite one with no program,
§9.7.4.2 makes that map the only route to a glyph, so a discarded entry is a mark the page does
not make.

Nothing carried is cut today — the widest is `UCS2-ETen-B5` at 13 291 `bfrange` entries against
16 384 — and no bound was raised. What changed is that a cut now names itself and that the
population is walked by a gate.

## Files touched

- `crates/pdf-font/src/tounicode.rs` — `MAX_SINGLES`/`MAX_RANGES` to module scope with the
  census in their doc comments, `CUT_BY_SINGLES`/`CUT_BY_RANGES`/`CUT_BY_OPERANDS`,
  `truncated`, `cut_by`, `insert_single`, and six tests.
- `crates/pdf-font/src/loading.rs` — `LoadedFont::to_unicode_truncated`, and
  `no_carried_unicode_cmap_is_cut_by_these_bounds` beside session 960's sibling.
- `crates/pdf-model/src/content/font.rs` — `note_to_unicode_truncation`, called at the load and
  where the cross-page cache serves a font.
- `crates/pdf-model/tests/hostile_budgets.rs` — `composite_font_with_to_unicode` and the
  page-level pair.
- `doc/conformance/ledger.toml` — §9.10.3's row only: five tests added, one paragraph appended.
- `doc/adr/0971-the-second-parser-over-the-same-carried-files.md`, this file.

## Gates

Recorded in the report to the orchestrator, off the run rather than off a document.

## Left undone

The census's other four rows are measurements in ADR 0971 rather than walks. Turning the ICC and
standard-font rows into assertions the way the two `CMap` rows are assertions is small and was
not done. Item 4 of the brief (`accessibility_census`'s `refused_open` ceiling) was withdrawn by
the orchestrator mid-round and is not touched here.
