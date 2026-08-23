# 697 — The witness that was a mask group, and the caret that named a resource dictionary

§11.4's `partial` rows read as a family, on ADR 0538's method one block earlier: pick the family
whose rows quote each other's figures, because that is where a stale figure has somewhere to
disagree with itself. Three of the four findings are such a disagreement, and the fourth is an
erratum that gives a construction the authority it had been running without.

Date: 2026-08-23.
ADR: [0551](../adr/0551-the-count-a-parent-kept-and-the-witness-that-was-a-mask.md).

Touched: `doc/conformance/ledger.toml` (§11.4, §11.4.1, §11.6, §11.6.6),
`crates/pdf-model/src/content/transparency.rs` (`named_press`'s doc comment, and one clause of
`group_press`'s, which already argued the reading the erratum settles),
`doc/errata-read.md`, the ADR and this file. No code, no status, no pixel.

## Why §11.4 and not the top of the list

`git blame --line-porcelain doc/conformance/ledger.toml`, each `partial` row's own `note =` line,
ranked by where its commit falls in `git log --reverse`. This base has 933 commits and 223 `partial`
rows. Rank 1 is §7.6.4.4, which 691 was in; rank 2 is §11.3.4; the cluster at ranks 3–10 holds
§8.6.6, §8.9.6, §8.9.6.2, §9.8.3, §9.8.3.1, §11.3.7, §11.4.1 and §12.5.

Three of the top ten are clause 11's, so clause 11's three candidate families were read for shape
before any of them was read for content. §8.9.5/§8.9.6 and §9.8 are cross-checked families too and
both came out consistent on the first pass. §11.4 was taken because its rows *count each other*:
the aggregate quotes §11.4.4's report count, §11.4.1 defers the colour-space question to §11.6.6,
§11.4.5 answers it, §11.6 restates §11.6.6's population. Every one of those four turned out to be
a place where two rows said different things.

## The four findings

- **§11.4.1's `partial` reason had been false since the four-hundred-and-ninety-second.** It said a
  painted group introducing its own blending colour space "is reported rather than composited in
  it"; ADR 0327 composites such a group in it wherever the space is four components this tree can
  sample, and §11.4.5's row and §11.6.6's have both recorded that all along.
- **§11.4's count of §11.4.4's reports was three; §11.4.4's own row said one; the corpus gate says
  none.** Over `doc/pdf.js` not one incomplete document carries an `Unsupported::TransparencyGroup`
  of any kind, so the whole family's reported population on that corpus is empty.
- **§11.6.6's corpus witness is a mask group, in the paragraph that narrowed the row.**
  `bug1721218_reduced.pdf` was named as keeping a smaller report for its "inner gray-`ICCBased`
  groups"; all eight of them are the `/G` of an `/SMask` dictionary of subtype `/Luminosity`, which
  is §11.5.3's population and is exactly what ADR 0276 had taken off this clause four paragraphs
  earlier in the same note. The document reports nothing at all.
- **§11.6 carried §11.6.6's corpus figure as well**, so correcting the original would have left the
  copy standing — ADR 0101's shape. It defers now.

## Two errata under §11.6.6, and one of them is the finding

`emit` files three annotations across the two pages §11.6.6 spans, all three a `Caret` with no
`StrikeOut` — the shape `check` is blind to by construction, paying for the sixth round running.

**Issue #134** inserts "of the transparency group XObject" into Table 145's `/CS` row after "the
ColorSpace subdictionary of the current resource dictionary", which settles the ambiguous word:
at a `Do` the *current* resource dictionary is the parent's, and the amended sentence asks for the
group XObject's own. `xobject.rs` passes `form_resources` and `named_press` reads `/DefaultCMYK`
out of it, so this tree has been right on nobody's authority since ADR 0327 built the construction.

**Issue #619's two carets are filed under §11.6.6 and belong to §11.6.5.2** — `emit` attributes an
annotation by the *page* the outline puts in a clause, and §11.6.6's heading is at the bottom of
page 436 while the carets are at the top, on Table 143's `/ID` and `/OPI` rows. ADR 0492 recorded
them as §11.6.6's in the six-hundred-and-sixty-sixth session and did not see Issue #134 beside
them. `doc/errata-read.md` has the arithmetic that separates them and the rule it is an instance of.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
The full sequence was run rather than the documents-only column, because the round adds a doc
comment in `pdf-model` and `doc/todo/02` §2's map is by crate rather than by diff. `fmt`, `clippy
-D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.

**The oracle gate fails at this branch point and the failure is not this round's.** It names
`function_based_shading_cmyk.pdf page 2` newly contradicted, on the same document whose page 1 is
already on the contradicted list. Three things were checked before writing that sentence. The
machine was loaded when it first ran, which is `doc/todo/02` §2's known silent third — so it was
re-run at a load average of 5 and reported the identical four figures, so load is not it. The whole
source diff of this round is `///` lines, which cannot change a rasterised page. And the gate was
run once more with `crates/pdf-model/src/content/transparency.rs` checked out from `HEAD` and the
two document edits left in place: same verdict, same page, same counts. It is inherited and is
reported upward rather than diagnosed here, because diagnosing a shading contradiction is not this
round's subject and a round that took it would have taken two.

Two clippy findings were this round's own and both are the same lesson: a verbatim quotation of the
standard collides with `clippy::doc_markdown`, because Table 145's sentence names `DefaultGray`,
`DefaultRGB`, `DefaultCMYK` and `ColorSpace` without slashes and the lint wants backticks the
quotation may not carry. The tree's answer elsewhere is an `#[expect]` with a reason; here the
sentence is not load-bearing word for word, so the comment paraphrases with the tree's own
`/Default*` spelling and keeps the quotation marks off. Paraphrase without quotation marks is
`CLAUDE.md`'s own permission and is the cheaper of the two.

Thirteen sweeps run before the edits, after them, and a third time on the tree carrying the ADR and
this file. Two levels moved on this round's own prose and were put back:

- `--bin counts` gained one attributed count on a phrase using the word *rows* — one of the ledger's
  own words for a row — so the sweep read the cardinal beside it as a claim about §11.6's family. It
  says *places* now and the level is back where it was.
- `spec-errata check` gained a hit on a sentence quoting the four words an Issue #619 caret inserts,
  because Issue #173 struck a sentence three hundred pages away that opens with them. Written as
  data rather than as a quotation, with the reason in the file, and both errata instruments are
  clean at every line this round wrote.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after → before: `pointers` 7594 ← 7566 with absent unchanged at 130 and undefined at 13;
`tables` 6126 ← 6116 sentences and 2294 ← 2286 key citations with absent unchanged at 101 and
contradicted denials at 6; `counts` 7175 ← 7145 sentences with 390 attributed counts both times and
four places counting one family twice; `quotations` 5679 ← 5665 document spans with diverging
unchanged at 34, and 1866 ← 1864 ledger spans with diverging unchanged at 2; `owed` 3669 ← 3647
terms with 175 unnamed over 111 rows unchanged; `overtaken` 515 ← 514 decision records with 39
overtaken unchanged; `entries`, `unread`, `capabilities`, `callers`, `inapplicable` and `overstated`
all unmoved. `blockers` gained one holding sentence in the ledger, which is §11.4's new note naming
§11.4.4 — a live deferral, not an expired one.

§5's binaries were not rebuilt: `tools/round.sh` says this is not a fifth round, and nothing here
measured a launch path, a page turn, a frame or a high-water mark.
