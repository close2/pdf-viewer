# 886 — The exclusion redrawn, a serializer, and a page taken out

2026-09-03. Argued in [ADR 0816](../adr/0816-the-exclusion-redrawn-and-what-writer-side-now-means.md),
[ADR 0817](../adr/0817-a-serializer-that-emits-structure-and-never-content.md) and
[ADR 0818](../adr/0818-a-piece-is-a-page-its-closure-and-what-the-report-says-it-lost.md).
The sixth implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, which a previous agent had already fast-forwarded onto `main`.

The owner answered RFC 0002 §13's first question, verbatim: **"RFC 002 and 003 are approved."**
Everything below was blocked on that sentence and is unblocked by it.

Touched: `CLAUDE.md` (the authoring exclusion); `doc/rfc/0002-…` (the status line);
`doc/conformance/ledger.toml` and `tools/conformance/src/{ledger.rs,bin/ledger.rs}` (the
`writer-side` definition and eleven rows); **`crates/pdf-syntax/src/serialize.rs`** (new),
`src/write.rs` (`real`), `src/lib.rs`; **`crates/pdf-syntax/tests/serialize.rs`** (new);
**`fuzz/fuzz_targets/serialize.rs`** (new) and `fuzz/Cargo.toml`;
**`crates/pdf-transform/src/split.rs`** (new), `src/lib.rs`, `src/range.rs`,
`src/bin/pdf-transform.rs`; **`crates/pdf-transform/tests/split.rs`** and
**`tests/split_corpus.rs`** (new); `crates/pdf-model/src/restriction.rs`
(`Operation::Assemble`), `crates/viewer-confined/src/protocol.rs` (its wire tag);
`doc/todo/02-every-round.md` (one line), `tools/state.sh` (`writer`);
`doc/state-of-play.md`, `doc/crate-map.md`, `doc/todo/57-…`; three ADRs, this file.

## 1. The amendment, sentence by sentence

`CLAUDE.md`'s "Authoring a document from nothing" entry lost its first paragraph — "we do not
*create* PDFs, and no clause whose requirements fall on a generator is in scope: linearisation,
object-stream packing, optimisation, and the rest of what a producer owes" — and gained RFC
§11.1's three paragraphs verbatim, plus §11.1's boundary-line paragraph, which the RFC prints
outside its blockquote and which this round put in on its own argument: an exclusion the file
cannot *apply* is one that decays, and the owner's sub-question with teeth was exactly whether
that fence is where it should be.

One further sentence was edited rather than replaced. "**This exclusion was 'we do not create
files' and was amended by argument rather than by attrition**" now reads "**This exclusion read
'we do not create files', then 'we do not *create* PDFs', and has been amended twice — both
times by argument rather than by attrition**", with the date and the owner's words in a
parenthesis. The rest of that paragraph and the whole `pdf_syntax::Document` immutability
paragraph are untouched, as the RFC intends.

## 2. What the ledger did, and what it did not

The `writer-side` definition — in `ledger.toml`'s header, in `bin/ledger.rs`'s `PREAMBLE` and in
`Status::WriterSide`'s doc comment, all three because the generator stamps the header back — went
from "addresses a PDF generator; this program writes only §7.5.6's updates" to "addresses a
generator; this program's writers emit structure, never content", which is `CLAUDE.md`'s own
enforceable test in the ledger's vocabulary.

RFC §11.2 predicted §7.5.7 and §7.5.8 as "the certain movers". **One of the two moved and the
other was already `implemented`**, and three rows the RFC did not name moved instead. ADR 0816
has the table, and the lesson with it: a status is a claim about this tree's code, and a document
written a hundred sessions earlier cannot know which rows a landing will touch.

## 3. The serializer, and the two defects the corpus found in it

`pdf_syntax::serialize` emits structure and never content — ADR 0817 has the design, the
sub-decisions RFC §10 left open, and what it refuses. What is worth recording here is that the
corpus walk found two defects on its first run and both were real:

- **`write::real` wrote six decimal places**, which had been correct for as long as the only
  writer was §7.5.6's update. A serializer rewrites every dictionary it carries, and
  `0.0009765625` is not `0.000977`; seven corpus documents drew differently after `split`, every
  one off by one antialiasing level on a glyph's edge. Now the shortest decimal that reads back
  as the same double, which never uses an exponent, so §7.3.3's prohibition and Annex C's
  precision are met by one formatting.
- **`split` did not carry `/OutputIntents`**, and §14.11.5's intent decides what a device colour
  means. The corpus's only two documents that state one on page one drew differently; the clause
  makes carrying it right as well as observed.

Both were found by the instrument rather than by the reading, which is the argument for building
the walk in the same round as the verb.

## 4. What was looked at

`qpdf --qdf` dumps of a source and its piece, byte for byte, to establish that the two documents
were semantically identical before the difference was looked for anywhere else; the differing
PPM pixels, printed with their coordinates and their two values, which is what said
"antialiasing" rather than "geometry"; ISO 32000-2 §7.3.3 and Annex C's Table C.1 for what a
real number owes; §14.4's "[w]hen a PDF file is first written, both identifiers shall be set to
the same value", which is the sentence a *created* file is governed by and which no row of this
ledger had ever cited.

## 5. Gates

`pdf-syntax`, `pdf-model`, `pdf-transform`, `viewer-confined`, `fuzz/` and two documents the
conformance gate reads changed, and `doc/todo/02` §2 gained a line; the whole sequence was run in
this worktree, the walking lines under `tools/bounded.sh` — one walk on the machine at a time,
waiting twice for a neighbouring round's survey to finish. The results are in the round's report
and not here.

## 6. What the next transform round does first

`doc/todo/57`'s order, as rewritten this round: `merge` and `pages` on the serializer that now
exists, then `optimize`, which is where §7.5.7's producer half is owed; `split --at-bookmarks`
and the document-level carrying a piece leaves behind; and then the RFC 0003 hand-off, which the
owner sequenced after this stream's writing verbs. What the walks do not see is in that file's
§5, and the serializer's walk has inherited the writer's gap: no foreign reader has been shown a
piece except over the committed fixtures.
