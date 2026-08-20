# 616 — The wash a gradient asked for around itself

A spec-driven round, taken at depth, and its finding is about the reading list as much as about the
clause: `doc/todo/01` said nothing below commit 534 of its blame ordering was unread, 38 rows are,
and two of the four read off the bottom of that list were wrong.

Date: 2026-08-20.
ADR: [0452](../adr/0452-the-wash-a-gradient-asked-for-around-itself.md).

Touched: `crates/pdf-model/src/shading.rs` (a new `background_components`),
`crates/pdf-model/src/content/report.rs` (a new `Unsupported::ShadingBackground`),
`crates/pdf-model/src/content/pattern.rs`, `crates/viewer-core/src/report.rs`,
`crates/render-quorra/src/scene.rs` (a comment that had gone false),
`crates/pdf-model/tests/shadings.rs` (a new test),
`crates/pdf-model/tests/oracle.rs` (`AMBIGUOUS_IMAGE_REDUCTION` 17 → 16),
`doc/conformance/ledger.toml` (§8.7.4.3, §8.7.4.1, §8.9.6.4, §8.6.6.5), a new
`doc/todo/17-a-shadings-background.md`, `doc/todo/README.md`, `doc/todo/01`, the ADR and this file.

## The sweeps, and how the rows were chosen

Twelve committed programs plus `spec-errata`'s `emit`, `check` and `applied`, all run; the counts
each printed are in `doc/todo/01`'s new section. None of them chose the rows. What chose the rows
was `doc/todo/01`'s own instrument — `git blame` over each `note = ` line — because reading its
output found that the file's claim about its own progress was false: the bands taken since the
four-hundred-and-forty-second were bands off the *top* of a list, and the sentence written about a
band was afterwards read as a sentence about the file.

`blockers` printed ten expired hits and nine carry `[history]`; the tenth, §10.7.4, is a row whose
note is 24 000 characters of measurement and whose "expiry" is a correction quoting the wording it
retired. `entries` and `unread` between them ranked a dozen annotation and form rows, and every one
I opened — §12.5.6.7, §12.5.6.9, §12.5.6.23, §12.7.5.4, §11.6.5.2 — turned out to have been read in
the five-hundreds and to say so. That is the shape the blame ordering explains: the sweeps rank by
*reason*, and a row read three rounds ago has the best reasons in the file.

## §8.7.4.3 — a `shall` nobody drew and nobody said

Table 77's `/Background` fills "those portions of the area to be painted that lie outside the bounds
of the shading object", and only where the shading is used as a pattern. Nothing read the key. The
gap was accurately described in **three** places — the ledger row, `pattern.rs`'s `domain_clip`, and
ADR 0151's closing section, which exists because ADR 0150 had wrongly claimed the entry was
reported — and the program said nothing about it, which is the finding: a gap three documents agree
about is not a gap anybody acts on.

It is reported now, from the `/PatternType 2` branch and nowhere else, because `sh` is the case the
clause exempts and a report there would fire on every page in the corpus that paints a shading
directly. The paint is priced in `doc/todo/17` rather than taken, and the reason is a reading rather
than a budget: Table 77's own NOTE 1 offers "as if the painting operation were performed twice",
which is stated for the **opaque imaging model** and on an anti-aliasing device puts the background
into every boundary pixel a second time — `(1 − c)²` of backdrop where the clause leaves `1 − c` —
and applies §11.6.4.4's `ca` twice inside the bounds. The exact construction is a paint that answers
the background where it would otherwise answer nothing, which is three backends, four shading kinds
and one field in a library that is not ours.

## The population, taken twice

`witness_census` over all 1251 PDFs on this disk: five state the name `/Background`, and reading all
five leaves **two** that are Table 77's — the other three are an optional content group and a
`/PieceInfo` `/Private` of the same spelling. Both are pdf.js fixtures and both use the shading as a
`/PatternType 2` pattern. `issue13372.pdf`'s axial shading extends at neither end, so the CCITT
stencil it fills is cyan outside the band between its `/Coords` and unpainted here;
`issue18816.pdf`'s is a Coons mesh.

## What the oracle did about it, and it fired in the right direction

`issue13372.pdf page 1` had been in `AMBIGUOUS_IMAGE_REDUCTION` since the hundred-and-eighty-first
session. The gate holds only pages we claim to draw completely, so the new report took it out of the
judged set and the staleness check failed the build — "a diagnosis that outlives what it diagnosed".
It left the group by leaving the comparison, which is the trade §9.3.8, §11.6.2 and the four
`knockout_*.pdf` made before it; the halftone-reduction diagnosis stays in the group's comment with a
sentence saying the page comes back the round the wash is painted.

## Three more rows, and two of them were confirmations

- **§8.9.6.4** said colour key masking was implemented for "both corpus instances". There are
  **three** — `colorkeymask.pdf`, `issue14821.pdf`, `issue15629.pdf` — all three still reach the
  unpacker unfiltered, which is what the sentence was about, and the row had never named its
  population. The 275 documents of the four `doc/corpora/` submodules state none.
- **§8.7.4.1**'s "no corpus document writes an `/ExtGState` on a Type 2 pattern" holds, and it was
  re-*derived* rather than re-read, which is `doc/todo/01`'s sixteenth sweep's own rule: 38 of the
  974 hold such a pattern, none states one, and the submodules hold no Type 2 pattern at all.
- **§8.6.6.5** gained Errata Collection 3 Issue #309, found by `spec-errata emit` over the family
  before writing. It strikes "which may be present only for DeviceN colour spaces that do not have
  the NChannel subtype" out of the sentence that gives `None` its *meaning* and adds the restriction
  separately as a `shall not` on the file. `ColourSpace::parse_at` reads no `/Subtype` at all, so the
  amended text ratifies a reading the base text left arguable. No code moved.

## Gates

`fmt`, `clippy --workspace --all-targets` (silent; the `viewer-qt` `cargo:warning=` lines are gcc's
on generated code, as `doc/todo/02` §2 says), `nextest` 2271 passed / 16 skipped, doctests, and the
whole of §2 because `pdf-model` is in the first row of the change→gate map. Corpus 974 documents, 68
incomplete against a `MAX_INCOMPLETE` of 91 — `issue13372.pdf` is the newcomer and it is the point.
Oracle 1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference geometry,
13 not comparable, 18 no render; the totals are unchanged and one page moved from the complete
column to the incomplete one. Text extraction 10969/11163 words in bounds over 508 documents;
`selection_census`, `accessibility_census`, `dates`, `xmp`, `jpeg2000` and `conformance` all green;
`render-quorra` 957 pages, 932 agree / 23 differ / 2 refused.

**`doc/todo/00`'s step 7 is not owed**: no display list changed, every command is byte-identical,
and the only page whose row leaves the ink sweep leaves it by leaving the ambiguous set.
