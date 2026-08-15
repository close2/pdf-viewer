# 540 — The quotation mark a conversion changed, and the one no sweep could see

**Finding.** `doc/todo/48`'s three remaining items turned out to be one subject, and the subject is
the instrument rather than the clauses.

**§8.9.5.4 — the conversion is faithful and the refusal was not.** `pdftotext -layout` over
`doc/ISO_32000-2_sponsored_EC3.pdf` pages 279 and 280 gives `doc/md/`'s §8.9.5.4 word for word;
nothing is lost there. What was wrong was this tree's record of the erratum. Errata Collection 3's
Issue #79 was declined in the four-hundred-and-seventeenth because "the amended step a) reads as
terminal and would leave the amended d) unreachable for a hidden base". **It is terminal and d) is
unreachable for a hidden base, and that is the amendment**: a) and b) dispose of every base image
that states an `/OC`, and c) and d) open at "Otherwise", so they belong to a base image that states
none. The five amended steps are total, disjoint and reachable; the four they replace were not,
which is why the 2020 step c) contradicted itself and why this tree carried a documented choice
between two readings of it for a hundred and twenty sessions. Implemented, §8.9.5.4 `implemented`.

**§14.8.6.3 — the enclosure requirement is a producer's, and the clause carried the round's real
finding.** The amended sentence opens "[w]hen including mathematics structured as MathML", so the
`math`-under-`Formula` enclosure and the namespace on every MathML type and attribute are `shall`s
on whoever writes the tagging, which `CLAUDE.md`'s closed exclusion covers. Three lines above it,
the conversion changes the standard's own punctuation: the PDF sets `“http://www.w3.org/1998/Math/
MathML”` and `doc/md/` writes `' http://www.w3.org/1998/Math/MathML '`, a different mark and two
inserted spaces — while §14.8.6.1's namespace name one page earlier comes out with double quotes.
**A rustdoc blockquote quoting §14.8.6.3 verbatim fails the gate today**, and the gate's message
would blame the standard.

**The single-quoted spans.** The ledger writes 106 quotations of the standard in single quotes,
because a note already sits inside a TOML string where a `"` has to be escaped, and no instrument
could see one of them. The rule that tells a quotation mark from an apostrophe is context: an
opening `'` needs a space or a bracket before it, a closing one needs a space or ordinary
punctuation after it, and a double quotation mark ends the search so that §9.4.3's two operator
names cannot swallow the quotations after them.

**What the instrument can see now that it could not.** `conformance::quote` holds one
`quoted_spans` for all three populations, in both marks; `normalise` drops every shape of quotation
mark, so the gate can verify a quotation of a clause whose own text carries one; `prose::folded`
drops hyphens and folds the fraction slash, which are four more conversion defects with witnesses;
and **`--bin quotations` reads `doc/conformance/ledger.toml`'s notes** — `doc/todo/01`'s eleventh
sweep, a hand-written script since the four-hundred-and-thirteenth, is a committed program and its
first run found three defects in the ledger's own quotations.

**Date.** 2026-08-15.
**ADR.** [0375](../adr/0375-the-quotation-mark-the-conversion-changed-and-the-one-no-sweep-could-see.md).

**Sweeps, before and after, verbatim from the runs.** The before state was taken by reverting the
round's diff and running the same commands.

- `conformance --bin quotations`, before: `3402 quotations in 537 documents: 1587 verbatim in a
  specification, 23 matching one for at least 5 words and then diverging, 1792 sharing too little
  with any of them to be a quotation of one.` — one population, and no ledger line at all.
- after, over the same 537 documents: `3385 quotations in 537 documents (0 single-quoted): 1590
  verbatim in a specification, 23 matching one for at least 5 words and then diverging, 1772
  sharing too little with any of them to be a quotation of one.` / `1388 quotations in 794 ledger
  notes (106 single-quoted): 1085 verbatim, 1 diverging, 302 unrelated.` The one remaining ledger
  divergence is §8.4.4's note quoting the wording it retired, which is `doc/todo/01`'s fourth-sweep
  shape. **0 single-quoted spans in 537 Markdown documents against 106 in the ledger** is the answer
  to why nobody had noticed: this project writes single-quoted quotations where a `"` would need
  escaping and essentially nowhere else.
- **and after this round wrote its own record**, which is the level moving because a round narrated
  what it did: `3401 quotations in 539 documents (0 single-quoted): 1596 verbatim … 24 matching one
  … and then diverging, 1781 …`. The twenty-fourth is this round's ADR quoting the erratum's amended
  step a), which *cannot* be verbatim in a conversion that carries the unamended text — the
  sweep's oldest known class, in the file that explains the class.
- The ledger's first run printed nine divergences: **three defects** (Table 147's `dc:title`
  *element* quoted as an *entry*, §12.3.2.2's crop-box parenthesis closed where the standard has a
  semicolon and a cross-reference, §14.8.4.1 quoting two bullets as one sentence with an invented
  full stop), one known false positive, and **five conversion defects** — `text-tospeech`,
  `implementationdependent`, `markedcontent`, `1 ⁄ 72` and a full stop the conversion set as a list
  bullet. The first four are repaired in `prose::folded`; the fifth is left visible.
- `spec-errata -- check doc/*.pdf`, before: `151 struck passage(s) of 4 words or more that doc/md/
  still carries as current text` / `75 quotation(s) quote text struck out of the clause they cite
  (blockquote=8 document=46 ledger=11 prose=10)` / `229 more match a passage struck out of another
  clause (blockquote=14 comment=15 document=122 ledger=12 prose=66)`.
- after: `151` unchanged / `70 quotation(s) … (blockquote=5 document=46 ledger=10 prose=9)` / `238
  more … (blockquote=14 comment=15 document=122 ledger=20 prose=67)`. The in-clause fall is
  §8.9.5.4's doc comment no longer quoting the retired steps; **the ledger's rise from 12 to 20 is
  the single-quoted spans becoming visible**, and every one of the ten in-clause ledger landings is
  a row quoting the wording it retired.

**Rows corrected in this commit.** §8.9.5.4 (**`implemented`**, the amended five steps with the
carets' own words, the false reason for the refusal named, the tests renamed), §14.8.6.2 (the
conformance requirement it called a validator's is half a reader's, and that half has a caller),
§14.8.6.3 (the enclosure requirement settled as a producer's `shall`, the conversion's quotation
marks recorded), §12.2 (`dc:title` **element**), §12.3.2.2 (the crop-box sentence quoted to its
end), §14.8.4.1 (two bullets quoted as two).

**Code.** `crates/pdf-model/src/content/image.rs` (`alternate_image` is the amended step d) and
nothing else), `crates/pdf-model/src/content/xobject.rs` (a), b) and e)),
`crates/pdf-model/tests/optional_content.rs` (six fixtures, one per step and one for a defect the
clause has no step for), `crates/pdf-model/src/structure.rs` (`Tree::resolved`, and
`standard_role` refusing a foreign namespace's homonym — `Namespace::is_standard`'s first caller),
`tools/conformance/src/quote.rs` (`MIN_WORDS`, `Mark`, `quoted_spans`, the quotation-mark drop),
`tools/conformance/src/prose.rs` (the shared rule, `Shape::Apostrophed`, hyphens and the fraction
slash), `tools/conformance/src/bin/quotations.rs` (the ledger population, the per-shape count),
`tools/spec-errata/src/lib.rs` (one rule instead of two).

**Touched.** `doc/todo/48-the-specification-we-check-against.md` (items 1 to 3 closed, the scope
that remains), `doc/errata-read.md` (findings 4 and §14.8.6.3, the owed list),
`doc/todo/01-ledger-partial-rows.md` (the eleventh sweep is a program; the twelfth's known gap is
closed), `doc/todo/02-every-round.md` §4 (what `--bin quotations` reads now),
`doc/ledger-and-claims.md` (nine programs), `doc/adr/0375-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints; `cargo nextest run --workspace` **1987 tests run: 1987 passed, 15 skipped**;
`cargo test --workspace --doc` green; `cargo test -p conformance -- --nocapture` 5 passed.
**The corpus and the oracle were run, because §8.9.5.4 is raster-reaching**, and the corpus gate
prints the same line before and after the round's diff: `974 documents … 0 unopenable, 8 locked,
2 encrypted beyond us, 6 pageless, 64 incomplete, 0 slow`. Oracle: `1794 pages … agrees 906,
contradicted 67, ambiguous 786, our geometry 1, reference geometry 2, not comparable 13, no render
19`. `text_extraction` 10969/11163 words in bounds over 486 of 508 documents; `dates` 1514 of 1545
conform; `xmp`, `jpeg2000` and `render-quorra --test corpus` green.
