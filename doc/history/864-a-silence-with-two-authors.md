# 864 — A silence has two authors: an entry the reader never read is not a deletion

Date: 2026-09-01.
ADRs: [0788](../adr/0788-a-row-ranked-against-its-siblings.md),
[0789](../adr/0789-a-declared-entry-that-was-never-read.md).
Touched: `crates/pdf-syntax/src/xref.rs`, `crates/pdf-syntax/src/document.rs`,
`crates/pdf-syntax/tests/cross_references.rs`, `doc/checks/fixed-documents.toml`,
`doc/conformance/ledger.toml`, `doc/traps/parsers-and-streams.md`,
`doc/todo/03-more-corpora.md`, `doc/todo/01-ledger-partial-rows.md`.

## The batches

`batch5.tgz` was on disk from the round before and had never been extracted. It verifies against
Apache's published SHA-512 — checked again here rather than taken off the shepherd's log — and it
is extracted and installed. It is the widest of the six by tracker count. `batch4` was still being
fetched, piece by piece, while this round ran; it was left alone, and its log shows it working
through the same 512 MiB pieces at the Archive's usual rate of one success in five to fifteen
attempts.

## The chunk

`batch2`'s **GHOSTSCRIPT** directory, 5442 `.pdf` documents, never walked — eight shards, one
process each, plus the 154-document TIKA directory beside it. The survey lines are in
`doc/todo/03` section 37 with the ranking, the bounds and the four slow documents timed alone.

Three things worth repeating outside that file:

- **The incomplete rate places the tracker, not the corpus.** GHOSTSCRIPT sits between PDFBOX and
  poppler-gitlab and nowhere near MOZILLA, which is `doc/todo/03` section 30's finding on a fifth directory: a bug
  tracker's rate is a fact about who files bugs against what.
- **`MAX_FORM_DEPTH` has sixteen new witnesses and nobody has run ADR 0271's experiment on them.**
  That experiment — lift the bound sixteenfold and see whether a witness stops reaching it — is
  what says the population is cycles rather than legitimate nesting, and it has been run on eleven
  documents in two corpora. Sixteen more is the largest single addition this constant has had.
- **The `slow` count is the instrument again**, for the fourth chunk running: of four timed alone,
  one is 2.4 s against the survey's 130.4 and one is a large target rather than a structure. And
  the 2.4-second one is byte-identical to the document `doc/todo/03` section 29 found in two other trackers, refused in
  2.4 s by ADR 0780's group bound where it used to take ten minutes — a third copy filed by a
  third person against a third reader.

## The defect

It is not in the directory that was walked, and that is the methodological point:
`standing_count_census` was run over five batches rather than over the chunk, and the one document
in the class its own doc comment calls "a defect of the scan rather than of the file" is in
`batch5`, extracted an hour earlier.

`cairo-85141-3.pdf` is a matplotlib figure whose cross-reference table has had a `/` or a `0x1f`
dropped into seven of its twenty-eight entries. Entry 3's generation field reads `000/0`, which
tokenises as an integer and a *name*, so `read_classic_table` abandoned the subsection there — and
the twenty-five entries after it, the page's own among them, became numbers with no entry. That is
the same state as a deletion as far as `Document::load` could tell, so it returned `Object::Null`,
and the document opened, counted one page and had none.

§7.5.4's subsection header is what separates them, and it had never been read as evidence: its two
integers "denote (respectively) the object number of the first object in this subsection and the
number of entries in the subsection", so a reading that stops inside that range has *mentioned*
every number left in it. The clause gives one way of saying a number names nothing — an `n` or an
`f` entry — and both are entries that were read. ADR 0789 has the reading, the three places that
now record a declared-and-unread number (including a cross-reference stream whose data stops short
of its own `/Index`, which `XrefTable::entries_lost` has counted with this argument in its doc
comment since it was written and nothing acted on), and what it deliberately cannot do.

The page draws now, at the producer's own 378 × 54, reporting five things that are all the file's
own remaining damage. It is pinned in `doc/checks/fixed-documents.toml`, and the rule is pinned as
a pair of hand-built files differing only in whether object 4's entry is unlexable or free.

**And re-running the chunk's own survey against the fixed tree found a second witness inside it**,
which the first walk had recorded as an ordinary report: `GHOSTSCRIPT-692248-0.pdf` said §7.3.10's
null about its own `/Contents` — object 4, one of the numbers its subsection declared and never
described — and drew a blank sheet. It draws with ink and reports nothing now, and it is the whole
of the difference between the two survey lines. That is the argument for re-running a chunk's
survey after a fix rather than recording the walk that found it: the defect's population is not
the population the census that named it was over.

**The general form went into `doc/traps/parsers-and-streams.md`'s trap 28**, because it is that
trap one level along: an absent answer has two authors — the file saying nothing and the reader
having read nothing — and a guard that cannot say which one produced it will attribute its own
failure to the document.

## The spec track

`doc/todo/01`'s blame list has been led by §7.6.4.4 and §11.3.4 since the seven-hundred-and-first.
Reading the §7.6.4.4 family whole found nothing wrong with the code and something wrong with the
row beside the row: §7.6.4.4.2 stood `partial` while §7.6.4.4.3 and §7.6.4.4.4 — the same shape,
a reader running the steps its authentication needs and a writer's remainder — stood
`implemented`. The parent's own note gave as its reason a choice between `partial` and
`writer-side` that leaves out the status its two siblings hold. Both rows move to `implemented`
(ADR 0788), which takes the blame list's ranks 1 and 2 off it and leaves §11.3.4, whose debt is
named and real.

## A note for whoever writes the next one

`§NN` written for a section of a todo file fails the conformance gate — it reads as a clause
citation and no such clause exists. Write `doc/todo/03 section 37`.

## Gates

The full `doc/todo/02` section 2 sequence, because `pdf-syntax` is in the first row of the
change-to-gate map and a change there can move a pixel. Figures are in the run rather than here.
