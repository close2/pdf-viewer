# Specification audit of ISO 32000-2 §12.10 and Annexes K, L, O and Q (geospatial features, XFA forms, structure element nesting, fragment identifiers, the transparency method) — round 2271

The project's twenty-second specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2260 ([`2260-specification-audit-annexes-d-e-f-i.md`](2260-specification-audit-annexes-d-e-f-i.md)
is the one before). Baseline: main at `cdbea7234`. Evidence:
[`2271-specification-audit-12-10-annexes-k-l-o-q/`](2271-specification-audit-12-10-annexes-k-l-o-q/) —
`findings.json` (2038's schema) and `expected-failures.json` (the failing test of each behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. The `shall`s and `should`s of §12.10 and Annexes K, L, O and Q (`doc/md/ISO_32000-2_sponsored_EC3.md`
   lines 13847 to 13947, 21045 to 22067, 22168 to 22216 and 22236 to 22262) were read with the
   ledger's twenty-one rows for them. Errata: `doc/errata/site/docs/32000-2-2020/clause12.md`
   (§12.10: #534 on `/PCSM`, `/LPTS` made required, #321 on §12.10.3's *either or both*, #656),
   `clauseAnnexL.md` (#83, #440 on Table L.2), `clauseAnnexO.md` (#700, numbering only) and
   `clauseAnnexQ.md` (#51 on an annotation's `/BM`, #295 *as if it was*).
2. **Annex Q, every route to the method.** `pdf_archive::survey::Survey::page_is_transparent` is
   the one implementation; its callers are the validator's two blending-space rows
   (`table/graphics.rs`), which the converter acts on. No renderer asks the page-wide question
   (`grep` for any other transparency predicate over `crates/`). The walk was read operator by
   operator for each way the annex sends content into Q.2: a form's `Do`, a pattern's `scn`, an
   annotation's `/AP`, a font's glyph procedures — and for each way the *same* content can be
   reached twice (the `(page, object)` visited set, the soft mask's walk and its take-back).
3. **Annex K** as ISO 32000-2 states it: K.1's indication (`viewer-core`'s `notes::about`) and the
   writer rules of K.2 against every writer that modifies a file with an `/XFA` entry — the
   viewer's save, `merge` (drops the packet with a warning), `split` (carries `/AcroForm` whole,
   fields and packet together), the archive converter (removes or keeps by decision), and
   `redact`.
4. **Annex L**: the population helper `Tree::in_pdf_2_0_namespace`, `merge`'s one `Document`
   wrapper, `split`'s pruning (it drops subtrees and never re-parents), the converter's appended
   `Part` (no `/NS`, so PDF 1.7's namespace), its role-map supply (refuses an element stating a
   namespace). The reader takes a tree as it finds it, which the annex does not bind.
5. **Annex O**: `pdf_model::fragment` and `viewer_core::Open::apply_fragment` for the eleven
   parameters `tools/state.sh annex-o` counts as carried out: the split-then-decode order, `page`'s
   positive integer, `zoom`'s both-or-neither offsets, `highlight`'s `left,right,top,bottom` order,
   `structelem`'s first-page fallback, `nameddest` through both of §12.3.2.4's tables, `comment` on
   the page chosen before it, `search`'s quoted word list.
6. **§12.10**: `measurement.rs` and `geospatial/` against Tables 269 to 272 with the errata, and the
   two readers that display a position — `viewer_core::located` and `viewer-host`'s
   `measuring::geospatial_sentence` (*every reader of a dictionary is a route to it*).

What holds, probed by reading: Q.2's four conditions as a `gs` sets them, restored by `Q`, judged
at each rendering operator; Q.3's group test on forms and annotation appearances; Q.4's two image
conditions; Q.5's walk of every `/CharProcs` entry where `Tf` selected the font; Issue #51. K.1's
note on every window. Annex L in every tree this program writes. Annex O's eleven parameters as the
row states them. §12.10's `/Bounds` default, `/PCSM` only over a projected `/GCS` (#534), the
`/GPTS`–`/LPTS` pairing, `/DCS` on both display routes, Table 270's mutual exclusion.

## Findings

Four behaviour gaps. In order of how much a person would lose:

- **2271-B04** — a redaction leaves the redacted field's value in the XFA packet. `redact` removes
  the widget and the field's `/V`, `/RV`, `/DV` (ADR 3725) and carries `/AcroForm`'s `/XFA`
  untouched, so the `datasets` packet still holds the value. Annex K's first writer rule ("The XFA
  field values shall be consistent with the corresponding V entries of the PDF field objects") is
  broken by a writer modifying the file, and §12.5.6.23's "shall remove all traces" is not met: the
  redacted text is in the output, in a decoded stream any reader opens.
- **2271-B02** — a form a soft mask's group draws is never walked for the page. The survey walks a
  `gs`'s mask group, then takes back what it found (ADR 2994), but the `(page, object)` pairs it
  visited stay visited; when the page later draws the same form itself, its content is skipped. A
  page whose only transparency is inside that form is reported opaque, so the PDF/A rows that
  need a blending space pass and the converter supplies none.
- **2271-B01** — a Type 3 font that a graphics state's `/Font` selects (Table 57, ADR 3411) is never
  given to Q.5: `show` returns before `type3` on that route. A glyph procedure painting at `ca`
  0.5 leaves the page opaque.
- **2271-B03** — a tiling pattern's cell is walked with the state in force at `scn`, where
  §8.7.3.1's step b) installs "the graphics state that was in effect at the beginning of the
  pattern's parent content stream". A `ca` 0.5 set and restored to 1 before the fill marks a page
  on which no element is transparent; the validator then demands a blending space the page does
  not need, and the converter writes one.

Two documentation findings: **D01**, the 12.10.3 row is `implemented` while a coordinate system
named by `/EPSG` alone — one of the two forms the clause permits — is refused, on a corpus census
(ADR 1586) that the owner's A865 says is not what counts; the row should say `partial` or the
refusal should be argued as a departure. **D02**, the 12.10.2 row lists `/PDU` among "Table 269
entire", and the preferred display units are read and carried to the host and never applied to
any display; the table states no `shall` on them, so it is a choice to write down, not a gap.

Not filed: `highlight`'s and `viewrect`'s "top left corner of the page" is measured on the
unrotated display box; on a page with `/Rotate` the corner a person sees is another one. The annex
does not say which, so it is a reading to record rather than a requirement broken. And the survey
treats elements inside an optional content group that is off as rendered (Q.2's population is
elements "to be rendered on the page"); PDF/A forbids nothing there, so the effect is a
blending space asked for where none is needed, and the fix round may take it with B03.

## Ledger rows contradicted

Each stays as it is until the fix round, which re-reads it: `Q`, `Q.3` (B02), `Q.5` (B01), `Q.2`
(B03 — the row says the cell is walked "like a form's", and a form's state is its invocation's),
`K` and `K.2` (B04 — "three of the writer's rules are kept on save" is the viewer's save only; a
second writer breaks the first rule), `12.10.3` (D01), `12.10.2` (D02). All are `implemented`
except 12.10.2, `departed`.

## Suggested fix groups

1. **A redaction and the XFA packet** — B04: where a redaction removes a field's value, either the
   `datasets` node of that field goes with it (K.2 kept) or the whole `/XFA` entry goes with a
   warning naming it (the converter's `XfaRemoved` is the precedent); a question to the owner if
   the choice between the two is not the fix round's to make. The `redaction_leaves_no_trace`
   fixture gains an `/XFA` place.
2. **Annex Q's routes** — B01, B02, B03 in `pdf-archive/src/survey/`: the `gs` font route calls
   `type3`; the mask walk restores the visited set it added to (or keys it by route); a pattern
   cell starts from the parent stream's initial state. The not-filed optional content reading
   with them.
3. **§12.10 rows** — D01, D02: the 12.10.3 status argued again against A865 (a question if a
   registry is the remedy), and `/PDU` written as read-and-not-applied.
