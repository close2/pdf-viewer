# Round 2220 — an agreed erratum governs, and every errata source is held

An archive round in `lane3`, on the owner's decision of 2026-11-24 about errata (steps 1 and 2 and
the principle-5 sentence; steps 3 to 5 are later rounds').

- ADR 3581, and one sentence in `CLAUDE.md` principle 5: an erratum the standard's working group
  agreed — published on pdf-issues.pdfa.org or recorded as agreed in `pdf-association/pdf-issues`
  — is the standard, and the printed text is retired. It refines ADR 0601.
- ADR 3582: `fonts/cid-system-info-agrees-with-the-cmap` compares `CIDFontSupplement <=
  CMapSupplement` (pdf-issues #77; veraPDF closed #1668 on it) and reads a predefined `CMap`'s
  collection from its program (`pdf_font::predefined::collection`). The converter restates the
  descendant's supplement as the `CMap`'s where only the supplement disagrees, for any font and
  for a direct `Type0` font whose descendant is an object (`pdf_archive::cid_fonts_above_their_cmaps`).
  The corpus test's four adjudications of 6.2.10.3.1 / 6.2.11.3.1 are deleted: the corpus was right.
- ADR 3583: `doc/errata/` holds the site's source, every issue and comment of pdf-issues, and a
  manifest; `tools/fetch-errata.sh [--check]` re-fetches.
- ADR 3584: #1669–#1671 re-read against the errata, still veraPDF's; `doc/errata/pdfa.md` lists
  the 34 PDF/A issues with their rows, the input to step 4's verdicts.
- Records corrected: `doc/pdfa-blocking-issues.md`'s row (ours), the departures record's #1668
  note, `doc/pdf-a-mitigations.md` and `doc/pdfa-remedies.md`'s R10.

**The three documents.** `noembed-eucjp`, `noembed-jis7`, `issue11555` under `as-shown` and
`as-if-printed` at 2b and 4: all twelve written, veraPDF 1.30.3 passes all twelve.

**Census**, with the pack, against round 2217's at `3af25f106`: no line moves for this round's row;
`050734` (as-if-printed), `issue17679_2` and `issue6342` (as-shown) are now refused at the
content-repair fence by `5bfbb3d9a`'s classes, which also reword `367594` and `455746`; the
census without the pack moves the same lines and no other. veraPDF over the 5493 outputs written as conforming:
40 fail, 11 documents, 13 rules, 12 unfinished — round 2214's 46, 14, 14 less the six outputs of
the three documents and rule 6.2.11.3.1-1.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 (`pdf-font pdf-archive pdf-transform
viewer-qt`); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0, 2257 of 2257, none in the
manifest, which is empty; rustdoc 0; fuzz `cargo check --bins` 0; `cargo test -p conformance` 0
once this paragraph was written.

**Habit.** Before calling a disagreement another implementation's, or filing it, search
`doc/errata/` (site source and `comments.jsonl`) for the clause: #1668 was filed against a
validator that had followed an erratum published two years earlier.
