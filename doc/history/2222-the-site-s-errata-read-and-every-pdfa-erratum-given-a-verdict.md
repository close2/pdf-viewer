# Round 2222 — the site's errata read, and every PDF/A erratum given a verdict

An archive round in `lane3`: steps 3 to 5 of the owner's errata plan of 2026-11-24.

- ADR 3589: `tools/spec-errata` reads `doc/errata/site/` as a second input (`site.rs`); each
  `<del>`/`<ins>` change is a `Note` (`Origin::Site`). What a change retires is the printed text at
  it (three words each side); an insertion retires nothing at a block edge, after a sentence or
  before a clause's closing mark, and a passage struck and inserted again is a move. `check` adds
  `doc/pdfa/` and `pdf-archive`'s rule texts (`Quoted::Rule`); `moved` reports a part 4 clause's
  ground as table rows. Where Collection 3 and the site land on one quotation for one issue it is
  reported once, under the PDF's note.
- What `check doc/ISO_32000-2_sponsored_EC3.pdf` names now: 374 quotations on text struck out of
  the clause they cite (192 from the collection's notes, 182 from the site's only, all ISO
  32000-2), 558 site landings folded into the collection's, 1141 elsewhere; `moved` 27. ISO
  19005-4's page: six changes, four retiring; `doc/pdfa/` carries three as printed; nothing in the
  tree lands on them. The 182 are unread, owed in `doc/errata-read.md`'s way.
- ADR 3590: `doc/errata-read-pdfa.md`, a verdict for all 34: five `applied` (#77, #104, #123,
  #253, #314, all already in the tree), 29 `not applicable`, no disagreement. Seven were agreed
  into the DIS revision of part 4 without words (#192, #197, #232, #244, #626, #633, #842); #77's
  change was declined for the revision on 2023-12-11 while the page still publishes it. Q1043
  asks both. `tools/conformance/tests/pdfa_errata_verdicts.rs` fails on an issue without a verdict.
- ADR 3591: the habit in `doc/habits/judging-against-other-implementations.md`; the template with
  its `Errata checked` line in `doc/upstream/README.md`.
- No predicate changed, so no census line can move for this round; the three documents round 2220
  found refused at the content fence (050734, issue17679_2, issue6342) are not answered.

**Census**, with and without the pack, against round 2220's at `fa65fb4ab`: no line moves.
veraPDF over the 5493 outputs written as conforming: 40 fail, 11 documents, 13 rules, 12
unfinished, as round 2220's.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 (`pdf-archive pdf-transform viewer-qt
conformance spec-errata`); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0, 2305 of
2305, none in the manifest, which is empty; rustdoc 0; `cargo test -p conformance` 0 once this
paragraph was written; the new gate fails with #841's row removed. Fuzz not owed: no public type
of a crate `fuzz/` builds changed.

**Habit.** A site's errata are edits, not sentences: a one-word `<del>` is the common case, and
the retired passage has to be cut from the printed text around it before any comparison means
anything.
