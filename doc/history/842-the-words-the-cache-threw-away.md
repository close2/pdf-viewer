# 842 — The words the cache threw away

The round was pointed at the one route ADR 0768 left open on `rank_the_contradicted`'s head. A
predicate over rasters cannot tell a genuinely blank page with one broken renderer from a page
nobody decoded with one broken renderer — `pdfref`'s own suite holds both shapes and they have the
same three rasters — and what separates them is that a renderer which failed says so. The harness
was throwing those words away on every cache hit. ADR 0769 is the reading, the rule, the vocabulary
and what it cost.

## What was done

- **`pdfref::cache` stores each renderer's own log beside its raster** and restores it with the
  picture, empty included, so a hit's work directory is what the module comment always claimed it
  was: indistinguishable from a miss's. The log goes in *before* the image, because `read_entry`
  tests for the image and a hit whose testimony was missing is the defect being closed.
  `cache::FORMAT` is bumped to `-2` — the entry's *meaning* changed, which is what a bump is for,
  and the cheap alternative (treat a log-less entry as a miss) would have made "no log stored" and
  "the renderer said nothing" the same thing on disk.
- **That bump was run on its own first, as a control.** All 6707 entries re-rendered, 1296 seconds
  of processor time in `pdftoppm`, `mutool` and `gs`, and every verdict in every class was
  unchanged. Without that run the rule's effect and the bump's would have been one number.
- **`pdfref::Testimony` and `Reference::refusals`** are the new evidence and the condition over it,
  in `reference.rs` beside the renderer they are about. The condition reads what a program says it
  *produced*, never its severity: 28 901 of `poppler`'s `Syntax Error` lines in this population are
  `Type mismatch in PostScript function`, on pages it draws correctly. `poppler` has **no** entry
  and that is a measurement — taking one of its sentences and not the others would be a list fitted
  to the page the round wanted to move.
- **`consensus_abstentions` gains a second route**: a flat sheet whose own log names a refusal takes
  no part in the consensus. Our render never enters it, uniformity is still required, and silence
  concludes nothing — so `triangulate`'s empty testimony slice is exactly the old pixel rule.
- **The audit is printed both ways, every run**, over every flat sheet the corpus produces: the
  refusals matched and the distinct sentences of every flat sheet not matched, with counts and the
  renderer that wrote each. Trap 11's demand is the converse list, and nothing here had it; the
  condition's right-hand side belongs to three other projects, so it decays like a ledger row and
  the gate's own output is the only sweep that can see it.
- Five tests in `pdfref`, two of which carry the three renderers' logs **verbatim** (trap 13 — a
  paraphrased fixture would pass while the rule stopped working), and `end_to_end`'s cache test now
  demands the log a hit leaves be byte-identical to the one the renderer wrote.

## What moved

Four pages, all in the new `NOT_COMPARABLE_THE_RENDERERS_SAID_THEY_DREW_NOTHING`.
`bitmap-symbol-context-reuse.pdf` page 1 leaves `contradicted` — the head of the ranking, where we
draw the JBIG2 image and two programs that said they could not decode it outvoted us. Three were
`agrees` and are the price: `jbig2_file_header.pdf` page 1 and `poppler-90-0-fuzzed.pdf` pages 12
and 16, where we drew nothing either, so the agreement was four programs failing at one file. All
three are pages this tree already reports as incomplete, and the count of agreements on pages the
gate calls **complete** did not move. Nothing moved toward a verdict that flatters us.
`CONTRADICTED_SHARED_JBIG2_DECODER` is three pages and its note says why the fourth left;
`doc/oracle-and-corpus.md` §3f-i is the instrument's own account.

## Second track

§7.4.7 was `partial` for one stated reason — *"the JBIG2Decode filter shall not be used with inline
images"* is not enforced — and the clause says that sentence to whoever **builds** the file, beside
five others of the same shape (the 2-byte marker, the file header, the segment page association,
the globals stream). §7.4.10's row in the same file already draws that distinction for the same
shape, and refusing such an inline image would draw less than the producer wrote. The three
sentences addressed to a reader — bitonal data excluding colour palette coding, Annex D.3's
embedded organisation, Table 12's `/JBIG2Globals` — are all implemented, so the row is
`implemented` and names the test that would fail if it stopped being true. §7.4.9 carries the
identical sentence for `JPXDecode` and now says so; it stays `partial` on its own two debts.

## Gates

The core, whole: `cargo fmt --all --check`, `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"`, `nextest --workspace`, `--doc`, and both `fuzz/` lines — all green. Then
the gates the change→gate map owes a `pdfref` change: the oracle (green, after the page list moved)
and `cargo test -p conformance` (green, after the row named its test rather than its file). Not a
fifth round and not a change that can move a pixel, so the rest of §2 was not run and §5's binaries
were not rebuilt — no measurement here is of a binary.

The sweeps §4 owes for a round that adds a verb: `undenominated`, `pointers`, `quotations`,
`parts`, `overtaken`, `quoted`, `unpriced`, `blockers`, `owed`, `overstated` and `tables`. None
names a sentence of this round's; `unpriced` still has its one accounted hit and no contradicted
page sits in no note.
