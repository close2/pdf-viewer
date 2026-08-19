# 599 — Two numbers in one slot

Both tracks in one item: `doc/todo/53`'s first residue, and §7.4.6's ledger row that carried it.

## Which of the three, and why that one

`doc/todo/53` held three findings from ADR 0392's reading of hayro's tracker, none witnessed by a
corpus document. Ranked by the file's own evidence rather than by cost:

1. **The CCITT decode bound sharing a field with the image height.** Taken. It is the only one of
   the three whose shape is a *correctness* hazard — one `u32` doing two jobs is trap 5's `/Length`
   instance one clause over, and its consequence is a whole image refused rather than a diagnostic
   missing. The residue's own note said what it needed and the note was right: four lines of wire
   format.
2. **`5f` swallowing an operator in silence.** Not taken, and the reason is unchanged rather than a
   time budget: separating it from `12pt`'s deliberate leniency (ADR 0303) needs a rule ISO 32000-2
   does not state, invented to improve a report. `doc/todo/53` still says what would change that —
   a page that draws *wrong* because of the salvage, which `doc/todo/00`'s step 7 would find.
3. **A Type 1 program's unassigned codes claiming glyph 0.** Not taken: still a `read-fonts` API
   question about reading an encoding *array* rather than its resolved map.

The cheapest was item 2 and the most dangerous item 1; the round took the dangerous one.

## What the clause says

Table 11's `/EndOfBlock` row, verbatim:

> A flag indicating whether the filter shall expect the encoded data to be terminated by an
> end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when it has
> decoded the number of lines indicated by Rows or when its data has been exhausted, whichever
> occurs first.

`tools/spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf` names no annotation on §7.4.6 at
all, so the sentence in `doc/md/` is the sentence.

The default is true, so `/Rows` usually does not bind and the decode is bounded by §8.9.5.1's
`/Height` — which ADR 0392 already implemented. The exception is the case the row is about, and
there the filter is *told* to stop above the image. `pdf_sandbox::CcittParameters` carried one
number that was both the bound handed to `hayro_ccitt` and the height `pad_to_height` fills and
`finish` checks, so the short raster came back short and the image was refused for being exactly
the size Table 11 asked for.

## What was built

- `CcittParameters` gains `height` beside `rows`; the wire block goes from 13 bytes to 17, and the
  worker's pixel budget takes the larger of the two, since a document may state more scan lines
  than its height as easily as fewer.
- The lines between `/Rows` and `/Height` are blank — the same choice `pad_to_height` already
  records for a data-exhausted stream, which the clause itself puts beside this one — and
  `image::ccitt_bound_below_its_height` says so beside the drawing. That is a twelfth place this
  tree reports while drawing; trap 5's list and its test are updated.
- The report's condition is Table 11's three parts: `/EndOfBlock` explicitly false, `/Rows` above
  zero, `/Rows` below `/Height`. It is asked of the dictionary, so a raster answered from
  `RasterCache` says what a fresh decode says.
- **The witness is a hand-built pair** (trap 8), `crates/pdf-model/tests/ccitt_bound.rs`: two
  one-page documents identical down to the seven encoded bytes and differing in one entry's value.
  Under `/EndOfBlock true` the `/Rows 2` has no power and four scan lines are black; under false
  the filter stops at two and the other two are white and named. A third fixture is `/Rows 4` with
  `/EndOfBlock false` and asserts silence, which is trap 11's half.

ADR 0434 has the argument.

## The report's cost, measured

Exactly one corpus document writes `/EndOfBlock false` at all — `ccitt_EndOfBlock_false.pdf`, whose
`/Rows 26` reaches its `/Height 26`, so the honest condition excludes it. A report keyed on the
entry rather than on the shortfall would have taken that page off the oracle's judged set for
nothing, which is trap 11's recorded failure mode. The corpus gate's ratchets held on the run after
the change, which is the same fact from the other side.

## Gates

The whole §2 sequence, because `pdf-model`, `pdf-sandbox` and their crate graph are under every
gate. `pdf-sandbox-worker` and `pdfref-hayro` built explicitly (trap 10), which mattered more than
usual here: the wire block changed length, and a stale worker would have read 17 bytes as a
malformed 13.

- `cargo fmt --all --check` **failed on the sequence's own first line** — the new test file and one
  `matches!` in `image.rs` were mine to wrap and rustfmt disagreed — and is clean on the re-run
  after `cargo fmt --all`. Written down because the sequence script has no `set -e` and a gate that
  fails in the middle of a scrollback is exactly the kind a round records as passing.
  `clippy --workspace --all-targets` silent, run after the final edit.
- `nextest`: 2203 passed, 16 skipped. Workspace doctests pass.
- Corpus: 974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 66 incomplete,
  0 slow.
- Oracle: 907 agrees (863 on pages we call complete), 66 contradicted; reference renders 99.8% from
  the cache.
- `text_extraction`, `selection_census`, `accessibility_census`, `dates`, `xmp`, `jpeg2000`,
  `render-quorra`'s corpus and `conformance` as recorded in the run.

Fuzzed `page`, which is the target whose binary contains `pdf_model::interpret` and therefore the
image path this round changed. **`confined_wire` is not the one that covers this**, which the
instruction was open about: it is the *viewer's* transport, `pdf-view-worker`'s, and no fuzz target
exists for `pdf-sandbox`'s protocol at all. The pipe is reached through `page` or not at all, which
is worth knowing next time this format moves.

**The stated invocation spent its whole budget in the merge and reported nothing**, which is
`doc/todo/02` §2's own warning about `page`'s corpus arriving in practice: fifteen minutes of
`-fork=6` over `fuzz/corpus/page` produced three minutes of compiling and twelve of `MERGE-OUTER`
with not one execution counted before the cap killed it. So the run that says something is the
targeted one: seven hand-built CCITT pages seeded into a corpus of their own — `/EndOfBlock` false
and true, `/Rows` short, exact, over `/Height`, at four billion, and zero, plus a Group 4 — and
**200 000 runs in 40 seconds, no crash, no timeout, no OOM**. Mutating a four-byte field that is
now two four-byte fields is exactly what that corpus makes libFuzzer do, and the dictionary it
recommends at the end has `Rows` in it.
