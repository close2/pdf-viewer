# 1004 — The tail a salvage drops, and the default a cross-reference stream never had

Session 983. Status: **accepted**. Three pieces of work with one subject — the copy of the
standard this tree checks itself against, and what it lets through — argued in the order of their
weight: the errata remainder read whole and its one finding, the digit run that swallowed an
operator, and the scanner rule session 977 left. Amends §7.2.3's, §7.3.3's, §7.8.2's and §7.5.8.3's
rows; closes `doc/todo/53` item 1 and `doc/todo/48`'s wrong count.

## 1. The errata remainder, re-derived and read to a verdict each

`doc/todo/48` said the unread remainder was nineteen. It was not a count of anything this round
could read: the nineteen were step 3b's struck-passage *lines*, read in the five-hundred-and-
ninety-fourth, and the population still being read is `doc/todo/01`'s recipe's — every erratum
carrying a strike or a caret that no file in this tree names. Re-derived under that recipe's own
two greps against the `emit` of the file on disk: **302 issues carry a strike or a caret and 44 are
named nowhere**; the multi-issue parse gives 310 and 46. The eighteenth use of the rule had left 50,
and the six between were read by rounds that wrote no section in `doc/errata-read.md`, which step
2's second grep is blind to. `doc/errata-read.md`'s new section has the table: 44 verdicts, 43 of
them *confirms*.

**The one that is not is Table 18's type 1 offset default, Issue #500.** The printed table gives
the type 1 entry's byte offset "Default value: 0" and the erratum strikes the sentence. Table 17's
rule for a zero element of `/W` is that "the default value shall be used, if there is one", and
`xref::entry_location` began every record as `[1, 0, 0]`, so *if there is one* did no work: a `/W`
of `[1 0 1]` located every uncompressed object at the file's header, which is the struck text
executed exactly. That is the **sixth** clause this collection has found implemented against struck
text, after §12.5.2, §14.13.5, §7.8.3, §8.9.5.4 and §7.4.3, and like §7.4.3 it sat under an
`implemented` row that quoted the printed default as the clause's.

The reader now refuses a type 1 entry whose layout states no offset and a type 2 entry whose layout
states no stream number, and refuses the section with it, so that `Document::open` rebuilds from the
body's own headers. `cross_references.rs::a_type_1_entry_with_no_offset_field_locates_nothing`
holds it and was run against the reader before the change, which believed the section and failed
the test on that line (trap 13).

### The first form of the fix was too strict, and two siblings' gates found it within the hour

The first form refused a type 2 entry whose *index* field was absent as well — Table 18 states no
default for it either. Session 980's `dates` and `xmp` gates then read **297 XMP packets where 318
had read**, and session 982's `nextest` failed `pdf-syntax::encryption` (`issue3371.pdf` lost its
first page) and `pdf-model::outlines` (174 documents with an outline where 176 had). Both attributed
it to the lexer change beside it in this working tree; a bisection — the `xref.rs` diff reverted
alone, both tests green, re-applied, both red — put it in the cross-reference reader.

`grep -aoE '/W *\[[ 0-9]*\]'` over `doc/pdf.js` and `doc/corpora/` is why: `/W [1 2 0]` above type
2 entries is what dozens of documents write, every compressed object in such a file being the first
of its stream, and every reader on this machine reads the absent index as zero. **Where the standard
states no value and every producer relies on one, a refusal is a document lost to a silence, not a
reading of a sentence.** The index reads as zero and `entry_location`'s doc comment records that as
a choice; the offset is different because the standard *had* a sentence and struck it, and no file
on this disk relies on it (the same grep, for a zero *second* element, finds none). The stream
number has the silence and no witness and stays refused. Three things worth carrying:

- **A before and an after belong to one tree state**, and four sibling rounds were editing the same
  working tree. The first display-list digest of this round moved two `pdf.js` documents that
  turned out to be a neighbour's transparency edit landing between the two builds; rebuilt at one
  neighbour state, only the one document this change touches moved. Every number below was taken
  that way.
- **A bisection is cheaper than an attribution.** The neighbours' evidence named the lexer because
  the lexer was the change they could see; the defect was one file over. `git diff <file> > x.patch`,
  `git apply -R`, run, `git apply` — a minute, and it settles what an argument cannot.
- **Trap 13's calibration has to include the population that must *not* fire.** The type 1 test
  calibrated the refusal against the reader before the change and said nothing about type 2 entries,
  which had no fixture — and the corpus was that fixture.

### The rest of the reading

Forty-three confirmations, most of them one-word substitutions, cross-reference numbers and
examples; the ones worth a sentence are in the section. Three are corrections to *claims* rather
than to code: §14.3.2 gains "All XMP metadata in PDF shall be encoded as UTF-8." (Issue #296), so
`pdf-model/src/xmp.rs`'s argument that the clause "states no encoding" is false since the erratum
and its UTF-16 and UTF-32 tolerance is a departure to be stated as one — the file is not this
round's and is named here for the round that owns it; §12.7.5.2.4 gains the sentence making the off
appearance optional (Issue #170), which confirms `Normal::StateNotDefined` drawing nothing and
reporting nothing; and §12.5.6.24 gains ", and thus an AP dictionary is not required" (Issue #94),
which is the reading `annotation::decide` already gives. The rows say so.

**A rule for the next round, written into `doc/errata-read.md`**: run the recipe's step 2, and if
the unread field is under fifty, read it whole; the three rankings, the tie-break and step 6's
exclusion column were built for a field of a hundred and cannot weight a struck *default* above a
struck typo, because a strike is a strike. What paid this round was the one issue no ranking would
have put first.

## 2. The digit run that swallowed an operator

`doc/todo/53` item 1: `5f` is one token under §7.2.3 and spells no number under §7.3.3, and this
tree salvaged it to `5`, dropped the `f`, painted nothing and said nothing. ADR 0303 had scoped its
correction to runs stating no digit at all, because `12pt` has the same shape and a content stream
that writes a unit after a number still has to draw, and the entry said a rule the standard does
not state should not be invented to improve a report.

### The line, and which clause draws it

Clause 7 draws none. §7.2.3 makes both runs one token, §7.3.3 makes both no number, and there the
syntax stops. **§7.8.2 draws it, by what the dropped tail *is*.** The clause defines an operator as

> An operator is a PDF keyword specifying some action that shall be performed

and says of one a reader does not recognise that "an error shall occur". A tail that is one of §8.2
Table 50's operator names is an action the producer wrote and the salvage threw away — `5f` was a
fill, `0g` a colour, `2w` a line width — and reading the number while dropping the action paints
less than the producer specified and says nothing, the silence trap 5 forbids. A tail that names no
operator — `pt`, `e`, `.3`, `-2` — is a spelling, and dropping it costs no action; ADR 0303's
leniency stands there, unchanged.

So the question is asked where the vocabulary lives and nowhere else:

- **`pdf-syntax` records and does not judge.** `Lexer::salvaged` is `Some` exactly for a number read
  out of a run §7.3.3 does not spell, carrying the run and the tail the value leaves out —
  `Salvage { run, dropped }`. `salvage_number` returns how many bytes it read, which is the whole of
  the change to it. A caller that never asks sees what it always saw; `Token` gains no variant, and
  the seventeen files that match on it are untouched.
- **`pdf-model`'s content reader asks.** `content::reader::next_content_token` wraps the lexer at
  the three places a token is lent to the interpreter, and where the lexer reports a salvage whose
  tail names one of Table 50's operators it hands the interpreter the whole run as
  `Token::Keyword(run)` — the keyword §7.2.3 makes it. The interpreter's own dispatch then reports
  `Unsupported::Operator { "5f" }` as it reports any keyword it does not know, and drops the operands
  before it as it does for any unrecognised operator. Nothing in `run.rs` changed. The number is
  not offered beside the report: an operand rescued from a run that also lost an operator would be
  half of a statement whose other half was refused.
- **The vocabulary is the standard's**, Table 50's sixteen categories in its order, with the one
  spelling Issue #80 corrects (`Sh` is `sh`). It is the standard's rather than this interpreter's
  because a run that swallowed an operator this tree does not implement has still swallowed one.
  It is a second copy of a list `run.rs` holds as match arms, and the doc comment says so; the two
  cannot drift apart in a way that matters, because a name in one and not the other changes only
  whether a *report* names a run or a keyword, and the test holds the copy to the table.

### Calibration, both ways

- **Unit tests.** `reader.rs` holds the pair: `5f`, `0g`, `2w`, `1.5re`, `3T*` reach the interpreter
  as keywords, `5 f` as two tokens; `12pt`, `1.2.3`, `--5`, `1.5-2`, `.-1` and `5fq` stay the numbers
  ADR 0303 left them. `numbers_without_digits.rs` holds the page-level pair: `... re 5f` fills
  nothing and reports `Operator { "5f" }`, and `BT /F0 12pt Tf … Tj ET` draws at 12 and reports
  nothing, with the same display list as `12 Tf`. `lexer.rs` holds the four shapes of
  `Salvage::dropped`.
- **The corpus, by a temporary census not committed** (it lexed every page's content stream of
  every document through `Lexer::salvaged` and classified each salvage by its tail, and interpreted
  every first page for reports of the new shape). Over `doc/pdf.js` and `doc/corpora/`, 1119
  documents and 4 623 556 tokens: **no page-content run carries an operator tail and none carries a
  unit suffix** — the 266 salvages with a tail are all damaged bytes lexed as tokens, inline-image
  data and fuzzed streams — and one document reports: `issue6342.pdf`, `12.9f`, `12m`, `5.65f`, in
  a stream the page invokes rather than its own. Over a sample of ten SafeDocs archives and
  `openpreserve`, 4 303 documents: 25 carry operator-tailed salvages somewhere in their pages, all
  of them damaged streams whose white space is gone (`--.5296l`, `-.3858Q`), and 2 report on their
  first page — `0300856.pdf`, the wholly black page ADR 0303 already named, 1 390 reports, and
  `507676.pdf`, 13. **No `12pt` exists in any of it**: every "unit-shaped" tail is a fragment of a
  stream that lost its spaces (`0ng`, `49incm`). So the salvage the rule preserves has no corpus
  witness either, and is kept on ADR 0303's argument.
- **The display lists.** `examples/display_list_digest` on both arms at one neighbour state over
  `doc/pdf.js` and `doc/corpora/`: one document moves, `issue6342.pdf`, 13 reports to 16 and the
  same 36 commands. Over the 66 sample documents with any salvage: `0300856.pdf` loses one of 397
  commands and `507676.pdf` keeps all 33 854 with parameters moved; `examples/render_at` on both arms
  and `magick compare -metric AE` say **0** pixels differ on either page.

### What this leaves

`fragment.rs`'s claim that the lexer "reads `12pt` as 12 and `x` as zero" is still true and is left
as written. The `Salvage` record is available to `pdf-syntax`'s own parser, which does not ask; a
file body has no operators, and what a salvaged `12pt` means in a dictionary is a different question
with no witness.

## 3. A standard cited with its year is that standard

ADR 0997 section 2 stated the rule and declined to write it because three sites in a sibling's
crate would have gone red. `citation::another_document` now admits `:` into the number after an
acronym, so `ISO 32000-1:2008 §7.3.4.3` is a `ForeignCitation` like `ISO 15076-1 §5.2`, and
`ISO 32000-2:2020` joins `ISO 32000-2` as this standard's own name — naming the document every bare
`§` already means is not a finding. `a_standard_cited_with_its_year_is_that_standard` holds both
halves. Before: the three `pdf-archive` sites resolved as ISO 32000-2 citations and the gate was
silent; after: they are findings, and session 982 owns the three rewrites.

The two residual misquotations session 978 found in files this round owns are gone:
`tools/conformance/src/unread.rs` and `doc/todo/01` (twice) quoted `CLAUDE.md` as saying "write
down the command, not the answer", which it does not say in any wording; what it says is that a fact
that can be counted is not written down and the command that counts it is, and the three sites now
say that without quotation marks.

## 4. Gates

`pdf-syntax` moved, so the whole of `doc/todo/02` §2 ran; `doc/history/983` has the figures as
printed. `--bin quotations` and `--bin pointers` before and after: no rise in diverging quotations,
absent pointers or undefined symbols beyond what this round wrote and can name.
