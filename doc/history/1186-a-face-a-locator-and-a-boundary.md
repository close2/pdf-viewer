# 1186 — A face the operator names, a locator their tool fetches, and a boundary a reader ignored

## What was built

- **`--font <base-font>=<path>`** (ADR 1209), repeatable, in `ArchivePlan::supplied_fonts` and read
  by the binary so `apply` opens no path. §9.9.1 makes embeddability a condition of a licence —
  "[o]ne of the conditions may be that the font program cannot be embedded" — and ISO 19005-2
  section 6.2.11.4.1 demands that fact, so it is the operator's to state, and the report and the
  output's `xmpMM:History` record it as theirs. `supplied_face` builds the same §9.6.5 code table
  `shipped_face` does, Table 124 still decides the key, and `pdf_font::restate` makes the advances
  the dictionary's numbers — now **proved** before the program is written; §9.9.2's tag is skipped.
- **`tool = "resolve-external"`** (ADR 1209): a `preserve` row naming the external-data site and a
  tool returns one `ToolRequest` per stream carrying the document's own file specification (on
  stdin — RFC 0007 section 4.1), and what the executor returns lands in `ArchivePlan::external_data`,
  the seam ADR 1199 built. `on-failure` takes only `stop`; `keep-everything.toml` needed no edit.
- **ADR 1199's enumerability wrinkle, as a class.** `decision::CONDITIONAL` names the rows whose
  answer the table does not settle by itself, `census::Kind::Conditional` is the standing,
  `--remedy-sites` says what each waits on, and `loss_sites` reads the per-document loss.
- **`implementation-limits/page-boundary-sizes`** (ADR 1210), from ADR 1200's predicate.
  `archive/boundaries.rs` asks §14.11.2.1 by putting the page through `pdf_model::Page` twice, with
  the entry and without it, rather than writing a second reading of the clause: mechanical where
  the intersection sentence has already collapsed the difference, `--authorise page-boundary` where
  it has not, refused at the media box. Two refusals the predicate did not name are §7.7.3.4's — an
  entry an ancestor states governs every page beneath it, so a removal moving what a reader computes
  for a page that failed nothing is refused; a finding this reader cannot place stops it by name.

## Measured

`archive_corpus` under the lock, before and after: **PDF/A-2b 471 converted / 139 refused →
474 / 136**, the page-boundary site off the top-ten refusals. PDF/A-4 unchanged at 207 / 152 — part
4 states no implementation-limits subclause, and its three external streams are URLs no corpus run
configures a fetcher for. `tools/state.sh remedies`: 2b **58 of 77 sites not built → 57 of 78**;
`keep-everything` 2b **33 of 77 answers not carried out → 32 of 78**.

**`--font` closes no corpus refusal, and that is the finding.** Thirteen documents refuse at
`fonts/font-programs-embedded` and not one is the repertoire case: twelve fail Table 124's
format-to-`/Subtype` pairing — a `Type1`/`MMType1` dictionary needing a bare CFF, or a composite
font, which §9.7.4.2 puts on a route of its own — and one states no `/FontDescriptor`. **Corrected
with it**: §9.9's ledger note and three sentences in `archive/fonts.rs` said Table **126** where
they meant Table 125; Table 126 is §10.6.5's predefined spot functions.
