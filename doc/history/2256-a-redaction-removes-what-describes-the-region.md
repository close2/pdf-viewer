# 2256 — A redaction removes what describes the region, and names what repeats it elsewhere

Lane `gwg`. ADR 3725. Subject: every place a redaction's removed content may survive in the
written file (§12.5.6.23, A834), tested end to end first.

- **The test first.** `crates/pdf-transform/tests/redaction_leaves_no_trace.rs` puts one string in
  every place a PDF holds text, redacts the line that shows it, and searches the output's raw
  bytes, every object's strings (bytes and §7.9.2.2 text) and every decoded stream, in Latin-1 and
  UTF-16 of either order, and the extracted text. Before the fix it found the string in the `BDC`
  property list's `/ActualText`; the structure element's `/ActualText`, `/Alt`, `/E`, `/T` and its
  parent's `/ActualText`; the page's `/Metadata`; a text annotation's `/Contents` and `/RC` over
  the region; a widget's `/V` and appearance; the `/Redact` annotation's `/Contents` (carried back
  by its popup's `/Parent`); and the extracted text.
- **Bound, removed** (ADR 3725): `redact/marked.rs` tracks `BDC`/`BMC`/`EMC` in the walk and drops
  `/ActualText`, `/Alt`, `/E` from an inline property list whose sequence lost anything (a named
  list holding them refuses the page); `redact/traces.rs` restates the structure elements the
  reached `/MCID`s or removed `OBJR`s name (and the ancestors' three), removes annotations whose
  `/Rect` meets the region and their popups, writes every removed annotation as null, and drops a
  removed widget's field `/V`, `/RV`, `/DV`; `/Metadata` joins the stale page and form entries.
  Each is a `Departure`; a partly removed element is said to be one.
- **Not bound, named**: outline titles, named destinations, embedded files, OCG names, document
  XMP and `/Info`, annotations outside the region. The removed runs are looked for in every object
  written unchanged; each place is a `Warning` with its path, never the text.
- Planted back: `/Metadata` out of the stale entries, and the bound test names `page-xmp`.
- Ledger: 12.5.6.23 stays `departed` (its JPX refusals); code, tests and note restated.
- Left, in order: a structure element written directly in its parent's `/K`; text under an
  `/ActualText` reads back only the replacement; a per-placement form copy matched by the
  original's number.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p pdf-archive -p viewer-qt`: fmt 0,
clippy `-D warnings --all-targets` 0 (after one `;` the first run named), `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 5340 of 5343, the three failures the manifest's
§10.5 entries (`audit-diff.py`: "failed 3, expected 3"), `pdf-transform` again on the final
source 1391 of 1391; rustdoc of `pdf-transform` 0; no public type changed, so no fuzz check;
`cargo test -p conformance` failed only `records` on this record's unfilled paragraph, and
`--test records` is 0 with it filled.
