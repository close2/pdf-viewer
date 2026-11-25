# Specification audit of ISO 32000-2 §12.1, §12.2 and §12.9 to §12.11 (interactive features' overview, viewer preferences, measurement, geospatial features, document requirements) — round 2239

The project's seventeenth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2237 ([`2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md`](2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md)).
Baseline: main at `600550c3`. Evidence:
[`2239-specification-audit-12-1-12-2-12-9-12-11/`](2239-specification-audit-12-1-12-2-12-9-12-11/)
— `findings.json` (2038's schema) and `expected-failures.json` (the failing tests of each
behaviour finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §12.1 and §12.2 (`doc/md/ISO_32000-2_sponsored_EC3.md` lines 10483
   to 10543, Tables 147 and 148), §12.9 (13730 to 13846, Tables 265 to 268 and §12.9.2's
   algorithm), §12.10 (13847 to 13947, Tables 269 to 272) and §12.11 (13948 to the end of §12.11.6,
   Tables 273 to 276) were read and set against the code that performs each step. The brief's
   "collections" are §12.3.5 in this numbering; Table 153's `/D`, `/View`, `/Split` and `/Sort`
   were read against `collection.rs` and `viewer-host`'s `panel.rs`, which 2038's B05, B06, B76,
   T164 and T167 had already audited.
2. 2038's findings on these clauses were read first and are not repeated (B19 on boundaries outside
   the media box, B22 on open-action order, B38 and D27 on `/V` and the processing override, B73 on
   an image's GEO measure, T24, T52, T89, T153, D53). A834 sets `/Enforce`'s default level to
   `warn`; ADR 2840's `Operation::OverridePreference` carries it and was read as settled. Errata:
   `doc/errata/site/docs/32000-2-2020/clause12.md` H12.2 (`UseAttachments` in
   `/NonFullScreenPageMode`), H12.9.1 (a placeholder NOTE), H12.10.2 (`/PCSM`'s row order, `/LPTS`
   made required), H12.10.3 (EPSG "any of"), H12.11.1 and H12.11.2 (Table 273's `/S` pointing at
   Table 275, Table 274's caption, `U3D`/`PRC` "applies"); `doc/errata/issues.jsonl` holds nothing
   on Table 268 or §12.9.2. None settles a finding below.
3. The code traced: `pdf-model`'s `viewer_preferences.rs`, `page.rs` (the four boundaries),
   `restriction.rs` (`/Enforce`, §12.11.6), `measurement.rs` (viewports, measures, number formats,
   `format`), `geospatial/` by its module comments, `requirements.rs`, `collection.rs`,
   `view/script_sites.rs`; `viewer-core`'s `viewer/opening.rs` (§12.11.6 before any script),
   `viewer/geometry.rs` and `located.rs`; `viewer-host`'s `printing.rs`, `presentation.rs`,
   `native_window.rs` (`/DisplayDocTitle`, `/CenterWindow`), `measuring.rs` and `panel.rs`.
4. The lens was 2237's *a construction that borrows another family's shape is read by every rule
   keyed on it*, turned on Table 268: `/RT`, `/RD`, `/PS` and `/SS` share one shape (a text string
   with a default) and one reading closure, while the table gives `/RD`'s empty string the opposite
   meaning. And *a bound has two sides*: a decimal rounded to `/D`'s precision can round up as well
   as down.
5. Each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/measurement_12_9_spec_audit.rs`, a module of `navigation_suite`.

What holds, probed by reading, is not listed one by one. Among it: every Table 147 entry read with
its default, an entry of the wrong type logged and defaulted, `/NonFullScreenPageMode` ignored
unless `/PageMode` is `FullScreen` and `UseAttachments` admitted by the erratum; `/PrintScaling`'s
unrecognised value as `AppDefault` and `/Enforce` honoured only where Table 148 admits it, then
asked of the reader's level; `/ViewArea`, `/ViewClip`, `/PrintArea` and `/PrintClip` applied with
Table 31's defaults; `/PrintPageRange` one-based and refused where not pairs of page numbers;
`/FitWindow` against the first *displayed* page; `/DisplayDocTitle` from `dc:title`; `/PageLayout`'s
odd pages on the named side. §12.9.1's reverse search of `/VP`, the first point's viewport for a
path, `/D` and `/A` converted through `/X`, `/Y` and `/CYX` before the function, `/CYX`'s refusal,
slope without `/CYX`, §12.9.2's steps c) to e) and the fractional carry. §12.11's requirements
evaluated before any document script on the one route a document is opened by, the penalty total
against 100, `/RH` handlers disabled for every type Table 275 names and an unrecognised handler
type ignored.

Readings left, each a question rather than a defect found: Table 267's `/O` is read and used by
nothing, because no feature shows a rectilinear position; it binds the day one does. §12.9.2 d)
applies `/F` `R` to the last unit alone, so 1.99 ft over feet and inches can show "1 ft 12 in" — the
algorithm as written, not a departure from it. A requirement of an unrecognised type with a `/NoOp`
handler is still counted: Table 276 says `NoOp` "allows older PDF processors to ignore" it, a
permission and not a requirement.

## Findings

Three behaviour gaps, all in what a person reads off the measuring tool:

- **2239-B01** — a decimal display that rounds up to a whole unit loses the carry: 2.999 units with
  `/D 100` is shown as `2 u` (and `2.00 u` with `/FD true`), not 3.00. `fractional_text` formats the
  fraction to `/D`'s precision apart from the whole part; the `F` arm beside it carries.
- **2239-B02** — `/RD ()` is used as an empty decimal position, so 12.5 reads `125 u`. Table 268 says
  an empty `/RD` means the default period, unlike `/RT`, `/PS` and `/SS`, whose empty string adds no
  text; `number_format` reads all four through one closure.
- **2239-B03** — a slope is taken along the page's axes, not the measuring axes Table 265's `/BBox`
  corner order fixes: in a viewport stated `[0 400 400 0]` a line rising on the page reads `1`
  where the measuring system's slope is `-1`. `Viewport::unit_square` honours the order; `slope`
  does not.

One documentation finding: **2239-D01**, three of `Kind::unmet`'s reasons, which the reader is shown
and the penalty total counts, have decayed — `EnableJavaScripts` "excluded by principle 5" while
document scripts run at the reader's level, `Multimedia` "excluded" while audio is opened (A946)
and plays, and `Geospatial2D` "needs the EPSG registry" while WKT systems are projected and a
position is answered.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `12.9.1` (B01, B02,
B03), `12.9.2` (B01), and `12.11.2` (D01's claims about the tree).

## Suggested fix groups

1. **`pdf-model/src/measurement.rs`'s `fractional_text` and `number_format`** — B01 and B02: round
   once to `/D`'s precision and carry; read `/RD`'s empty string as the period and say in the
   closure's comment which of the four entries it describes. One small round.
2. **`measurement.rs`'s `Measure::slope` with `Viewports::traced`** — B03: hand the measure the
   viewport's axis directions (the `/BBox` corner order, or an object's placement) so signed
   quantities are taken in the measuring system.
3. **`pdf-model/src/requirements.rs`'s `Kind::unmet`** — D01: re-answer the three arms against the
   tree and Table 275's wording.
