# 812 — A flush is not a truncation

Date: 2026-08-28. Branch `round-812`, from `main` at `120951e7`. Parallel round, worktree `r812`.
ADR: [0744](../adr/0744-a-flush-is-not-a-truncation.md).
Touched: `crates/pdf-syntax/src/filter.rs`, `crates/pdf-model/tests/damaged_content_streams.rs`,
`crates/pdf-model/tests/oracle.rs` (one group's list and its note),
`doc/conformance/ledger.toml` (§7.4.1 and §7.4.4.1), `doc/todo/README.md`,
`doc/todo/18-a-flush-is-not-a-truncation.md` (deleted — the item is done), and two new files,
`doc/adr/0744` and this one.

## The subject, and why it was taken

The batch's general-improvement round, told to let the instruments name the subject and to prefer
a clause or a corpus subject unless an instrument ranked something clearly above it. The
instruments were asked first, over the pristine tree:

- **`tools/state.sh quick`** prints one thing the tree says it owes: `Command::View` and
  `Query::View`, added by round 808, are `UNREAD` in the window-vocabulary table's reading — the
  section says in as many words that a round owes a verdict there. It is a *reading*, worth an
  hour, and it does not move a page.
- **The sweeps.** `overstated`, `owed`, `inapplicable`, `counts`, `blockers`, `capabilities` and
  `parts` were run before anything was written. Every hit is a shape those sweeps' own catalogues
  call noise: a table read in part, a correction quoting the wording it retired, a partitive with
  no table to divide it, and — for `parts`, whose closest rung is 48 hits deep in `pdf-render` —
  this project's own trap-2 aphorism about *either backend*, repeated verbatim.
- **`doc/todo/README.md`'s bands.** Band 10–19 is *defects: wrong pixels or wrong output, with a
  diagnosis and usually a fix already argued*, and `18` was in it: **a report that fires on a
  condition the clause does not state**, with three corpus witnesses, a measurement, a decidable
  test written out, and one named blocker with three priced ways round it. That is a corpus
  subject *and* a clause subject, and it outranks a reading.

So `doc/todo/18` was taken, and the choice between its three routes is what the round is.

## The finding, restated in one sentence

A producer that calls `Z_SYNC_FLUSH` and never calls `deflateEnd` has written every byte of its
data and no RFC 1951 final block, so `Damage::Truncated` — whose own words are *true* of it —
reports a shortfall that does not exist, and four consumers that branch on that value treat a
whole stream as a prefix.

## What was decided, and what it cost

Route 2 of the three: a **throwaway raw decoder replayed over the same input**, fed RFC 1951's
final empty stored block, required to answer `StreamEnd` writing nothing. It is the only one of
the three that puts no work at all on the healthy path — the alternative that keeps the live
decoder needs an Adler-32 over every byte of every stream this program decodes, because `flate2`
exposes no accessor for the one the decoder is already computing. The argument, the placement in
both routes, the one stated remainder and the worst case are ADR 0744's.

One thing the todo file did not have: **this is not only a report.** `pdf_font::whole_program`,
`colour.rs`'s ICC route and `Document`'s object-stream reader each decline to *use* a damaged
stream, and each is right to; a stream this decision reclassifies is one all three now use whole.
So the round can move a pixel and owes the whole of §2.

## The measurement

`examples/damaged_stream_census` over the whole pdf.js corpus, before and after, in the same build
directory — the *before* taken by restoring `filter.rs` at the base commit, rebuilding and running,
then putting the file back from a copy rather than from `git checkout`, which is doc/todo/01's
footgun:

| over 17 057 streams of 974 documents | before | after |
|---|---|---|
| streams damaged | 48 in 11 documents | 41 in 8 |
| of them, form `XObject`s | 6 | 1 |
| of them, cross-reference streams | 2 | 0 |
| reports naming damage | 10 over 4 documents | 6 over 1 |

The three that leave are `comments.pdf` (object 667, 648 bytes kept), `highlights.pdf` (object 667,
648) and `issue3885.pdf` (object 12, 424), which are the three `doc/todo/18` named. **The corpus
gate's incomplete population falls by exactly those three**, all out of the `the file` class, and
the mechanism *one of §7.8.2's other content streams, drawn as far as its damage* leaves the
printed composition entirely: no page of the 974 reports one now.

**`MAX_INCOMPLETE` is a `<=` ceiling far above the population and is deliberately left where it
is.** Lowering it by three would be re-baselining an instrument this round was not asked to
re-baseline — its own doc comment's last recorded movement is hundreds of rounds old — and the
gate prints the population.

## Two things that only turned up because the question was asked again

- **The marker is four bytes, not five, and the corpus shows it matters.** A flush pads to a byte
  boundary, so the byte in front of `LEN` holds the last bits of the block it terminated and is
  `00` only where those happened to be zeros. The five-byte scan `doc/todo/18` wrote down — and
  asked a round to re-run rather than trust — finds four corpus documents; the four bytes that are
  actually the marker find five. The fifth, `issue2948.pdf`, carries no damaged stream, so nothing
  moves — and nothing in the fix rests on it either, because the tail bytes are a cost filter and
  never the test. What a scan written from that sentence would do is undercount.
- **`tests/damaged_content_streams.rs` was wrong in both directions, and it took its own witness
  going away to find out.** Its header said no corpus document carries a damaged Type 3 glyph
  description and that the form `XObject` was the one kind a corpus could show; the form it meant
  is `comments.pdf`'s, which is the flush. Asking which corpus page really does report a
  §7.8.2 content stream short found `poppler-90-0-fuzzed.pdf` page 10 naming a `/a14` glyph
  description — a witness that was there the whole time, for a kind the file said had none. Both
  tests now open real documents, one for each answer.

## The oracle, and what a wrong report actually costs

The three pages stop reporting, so they stop being *incomplete* — and the oracle failed,
correctly, on **`comments.pdf` page 1 and `highlights.pdf` page 1 newly ambiguous without a
diagnosis**. Both are back in `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`, the group whose entries were
taken out when ADR 0359 made the report loud, and whose diagnosis has described them all along;
the note there is rewritten around the round trip and cites this round's ADR, which is what keeps
it off the `overtaken` sweep.

**The round trip is more precise than the sentence it replaces, and the precision is the
finding.** The old note said the oracle "stopped judging them". It did not: the undiagnosed check
is over `complete && Ambiguous`, so their verdict was `ambiguous (incomplete)` before and is
`ambiguous` now. What a report firing on a condition the clause does not state cost was never the
verdict — it was the *diagnosis*. A page nobody has to explain is a page nobody opens, and while
that stood no instrument could notice, because a page exempted from explanation cannot be found to
want one. The third page, `issue3885.pdf` page 1, agrees: the run prints it nowhere, which for
this gate is what agreement looks like.

## Trap 13, three plants, above commits `21eb3858` and `83060ecb`

Each planted, run, reverted. No plant fails more than one of the four tests it is aimed at, which
is what says each discriminates its own claim, and the four `RunLengthDecode` fixtures in
`damaged_content_streams.rs` are untouched by all three — no catch-all arm is swallowing the case
(796's lesson).

| plant | what fails |
|---|---|
| `ended_on_a_block` answers `false` always — the defect as it stood | `a_stream_flushed_and_never_finished_is_whole` (both routes, both compression levels) and `the_corpus_witness_turns_out_to_be_a_flush_and_says_nothing` |
| `ended_on_a_block` answers on the tail bytes alone — the heuristic | `a_stream_cut_inside_a_block_that_ends_in_the_marker_is_still_truncated` |
| `ended_on_a_block` answers `true` always — every truncation called whole | `a_corpus_document_that_really_cuts_a_glyph_description_short_names_it` |

The second is the one worth keeping: it is the reading a round that skipped the decidable test
would have shipped, and a stored block carrying the marker inside its own data is what refutes it.

## Gates, the whole of §2

A change in `pdf-syntax` is under everything, so the map assigns the whole sequence, and this
round can move a pixel besides. Run on a machine carrying three sibling rounds — load averages
between 13 and 40 through it — which is worth saying because three of these lines spawn reference
renderers on a wall clock. **The oracle's own numbers say the load did not reach it**: 6707 of
6707 reference renders came out of the cache and 0 were produced, so no reference ran under a
budget at all, and *not comparable* is 42 with 3 remembered timeouts on both of its runs — which
is the figure `doc/todo/02` §2 says a loaded machine inflates.

| line | result |
|---|---|
| `cargo fmt --all --check` | silent |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent, beside the documented cold-build `viewer-qt@0.1.0:` gcc lines and cargo's standing `proc-macro-error2` notice |
| `cargo nextest run --workspace` | 2785 tests, all run, all passed, 18 skipped — **it failed on the first pass**, correctly: §8.10.1's ledger row named a test this round renamed, and the row is fixed |
| `cargo test --workspace --doc` | passed |
| `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins` | silent |
| `cargo build --profile gates -p pdf-sandbox --bins` | built |
| corpus | 974 documents in 4.5 s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **63 incomplete**, 0 slow — three fewer than before, and the mechanism they were on is gone from the printed composition altogether |
| `pdfref-hayro` | built |
| oracle | 1945 pages (1842 complete, 103 incomplete) — 983 agree, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render. **It failed on the first pass**, correctly, on two newly-undiagnosed ambiguous pages; passes with their entries restored |
| text extraction, three gates | 99.3% of words over 974 documents against `pdftotext`, 99.8% over 40 against PDFBox's frozen output, 22 and 4 below 90% |
| `selection_census` | 966 documents compared, 0 selections differing from the interpreter's readback |
| `accessibility_census` | 102 853 elements, 57 116 a caret can move through, ratchets held |
| `dates`, `xmp`, `jpeg2000` | passed |
| quorra corpus | 957 pages: 932 agree, 22 differ, 3 refused, 17 not comparable; median page 2.19× the CPU backend |
| `fixed_documents` | 41 checked, 0 absent, 41 rows |
| `cargo test -p conformance` | passed — 875 rows, 0 new; 11 768 citations; 1107 quotations verbatim |

**The order matters and is stated rather than assumed**: the sequence above ran once whole, two
lines failed and were fixed, and `nextest`, the corpus gate and the oracle were then run again
*after the last Rust edit*, which is §2's rule that a number belongs to the round that ran the gate
last. The lines not re-run — quorra's corpus, both censuses, the text line, `dates`, `xmp`,
`jpeg2000`, `fixed_documents` — read neither of the two files that changed after their run, and
what changed in one of them is a doc comment.

Beyond §2, `doc/verify.md`'s fuzz rule: this round touches a parser, so `document` — §7.5's file
structure, which reaches every filter through `Document::decoded_stream_data` — was run.

## The §4 sweeps, before and after, every delta accounted

Fifteen argument-free conformance sweeps, run over a detached checkout of `main` at `120951e7`
with its own build directory — doc/todo/01's second method, so nothing in the working tree was
restored over — and again here, after the last edit. **Exit statuses identical on all fifteen.**
Ten outputs differ; one is a *gain* and the rest are accounted:

- **`overtaken` loses a hit: 46 → 45**, and it is the rung-2 entry for
  `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`, which had been carrying four ADRs later than the newest it
  cited. Rewriting that note around this round's finding and citing ADR 0744 in it is the sweep's
  own stated rule — *a round that rewrites a note cites its own ADR in it* — working exactly as
  designed. 630 decision records instead of 629.
- **`blockers`, `capabilities` and `owed`** differ only in line numbers inside `oracle.rs`, which
  gained fourteen lines, plus one word now named in one more file. No hit is added or removed.
- **`ledger`** differs only in the absolute path it prints. 875 rows, 0 new, both sides.
- **`counts`** reads 8930 → 8934 sentences governing one of the ledger's own words and 450 → 451
  attributed counts, which is the three notes this round wrote. **Contradictions unchanged at 4**,
  and the two columns that could hide a defect — *the family agrees with* at 150 and *counted no
  such way* at 58 — are identical.
- **`inapplicable`** reorders the printed *cousin* list of two rows, because §7.4.4.1's note now
  carries a word it did not, and counts `RFC` in two more files. No hit changes class.
- **`pointers`** reads 9137 path pointers instead of 9134, with live 5189 → 5185 and absent
  98 → 102. All four new absences are `doc/todo/18`: one in ADR 0730, which is a record and is
  not edited to follow a file that moved under it (ADR 0232 §2), and three in ADR 0744, which
  names the item it closed. That is what `doc/todo/README.md`'s *a number a deleted item used to
  have is not free* is about, and it is why the number is not reused.
- **`quotations`** reads 1099 documents instead of 1098 (two added, one deleted) and 6764
  quotations instead of 6766 — the deleted file's. **Verbatim unchanged at 2825 and diverging
  unchanged at 38.** In the ledger, 1984 quotations instead of 1983 with verbatim 1518 → 1519:
  the §7.4.1 sentence this round quotes is verbatim, and diverging stays at 2.
- **`tables`** reads 2595 attributed key citations instead of 2596 and agreed 2427 instead of
  2428, which is the deleted file's `Table 5` citation. Absent unchanged at 101 and denials at 6.

`cargo +nightly fuzz run document -- -runs=50000`: 50 000 executions, no crash, no timeout, no
leak, coverage and corpus unchanged at the end of the run.

## What was left alone

- **`Command::View`'s and `Query::View`'s reading** in `tools/state.sh`, which the instrument asks
  for by name. It is still owed and is still an hour: the question is whether a window that never
  restores a view is a debt or a tier, and the only host that asks either of them today is
  `pdf-viewer-confined`, which that section deliberately does not count.
- **The chain remainder.** A `FlateDecode` that is not the first stage of a windowed chain still
  reports a flush as a truncation, because the driver no longer holds its input. Stated in
  `Inflate::replayable` with its population — the corpus holds no such stream — rather than left
  to be rediscovered.
- **`decoded_extent`**, which asks a different question and correctly answers `Short`.
