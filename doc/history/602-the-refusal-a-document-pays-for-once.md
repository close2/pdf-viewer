# 602 — The refusal a document pays for once

Both tracks: `doc/todo/41`'s last open line, measured and taken, and §7.4.2's `partial` ledger row
read sentence by sentence against the code.

## Before anything: the binaries and the two candidates

`tools/round.sh` had been flagging `target/`'s binaries as older than `HEAD` for two rounds, and §5
says a measurement owes the rebuild first. One invocation, six binaries plus `libviewer_ffi.so`,
installed into `target/`.

Then the two items were compared before either was started, as the round was told to:

- **`doc/todo/45`** — its four remaining rows are (a) quorra's `encode`, which is upstream's and is
  asked for in `QUORRA_FEEDBACK.md` §29.2 and §33, (b) the two other backends' reduced-raster cache,
  which the file itself says wants a measurement *in the confined host* before it is worth the lines,
  (c) the owner's own windowed run, which `doc/environment.md` reserves for the owner's session, and
  (d) §5's regression against covered pixels rather than commands. Nothing there is both this
  round's to take and takeable headless in half an hour.
- **`doc/todo/41`** — one sentence, priced, and the round was told its shape may have moved. It had.

## The demand-driven half

**What had moved.** Sessions 592, 594 and 595 gave all five of §7.8.2's content streams a window,
and `Document::pumping` grants one only to a single `FlateDecode` or `LZWDecode` with no predictor.
So the population `doc/todo/41` was written about — a bomb in a page's `/Contents`, a form, a
pattern cell — is no longer where the amplification lives. What is left is everything a file can do
to decline the window: a chain of two filters, a predictor, or a stream that is not content at all
(a font program, an `ICCBased` profile, a cross-reference stream), each read whole from every page
that names it.

**The witness is that shape.** Bomb B — a deflate run of zeros inflating past the gibibyte — inside
a form `XObject` twenty pages draw, wrapped in `[/ASCIIHexDecode /FlateDecode]` so no window can
take it. 2.5 MB of file. `viewer-core/examples/find_cost` with a needle the document does not
contain interprets every page once, and the arms alternated B A B A B A with the patch applied and
reversed in one sitting (`git apply -R`, never `git stash` — `doc/environment.md`).

| | cold sweep, 20 pages |
|---|---|
| without the refusal memo | 5.92 s / 6.12 s / 5.92 s |
| with it | **2.76 ms / 3.23 ms / 2.89 ms** |

A 25-page ordinary document is inside the noise either way. The code, the bound that keeps the memo
honest and the derivation of the per-entry charge are ADR 0437.

**One thing the instrument nearly got wrong.** `find_cost`'s `cache:` line is the *readback* cache,
not the decoded-stream one, and it prints the same figures in both arms. Read as the decoded-stream
report it would have said the memo did nothing, in the same output where the sweep is two thousand
times faster.

## The spec-driven half

**§7.4.2, `partial`, read against `filter.rs` sentence by sentence.** All five of the clause's
sentences hold as the row says: one byte per pair of digits, white space ignored, `>` the EOD
marker, an odd final digit padded — and "[a]ny other characters shall cause an error" departed from
deliberately, because a hex stream with one stray byte decodes to what its producer meant everywhere
else.

The finding is what the departure rested on: **prose in two places and no test.** A choice nothing
exercises is one a later round undoes by accident with every gate agreeing. There is a test now, and
the row names it.

Two things there read like gaps and are not, which is worth as much as the finding:

- `ascii_hex` is the only filter taking no `Limits`. That is sound rather than an omission — §7.4.2
  is a 2:1 contraction, and `Parser::parse_stream_data` bounds the raw bytes, which is what makes
  `max_stream_len` "raw or decoded" rather than only the second. The reasoning is now beside the
  function instead of only in `Document::pumping`.
- §7.4.3 carries the identical "shall cause an error" sentence and is `implemented`, because
  `ascii85` does enforce it. The two rows differ for a reason: base-85 cannot resynchronise after a
  skipped character — it shifts every group after it — where hexadecimal can.

## Gates

The full §2 sequence, because a change in `pdf-syntax` can move a pixel: fmt and clippy silent,
2207 tests, the doctest line, corpus, oracle, the three text-extraction gates, both censuses, dates,
xmp, jpeg2000, quorra's corpus and the conformance gate — all green, and the core re-run after the
final edit. `--bin quotations` reports no divergence for the new blockquote.
