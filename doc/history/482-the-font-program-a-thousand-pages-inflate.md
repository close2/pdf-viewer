# 482 — The font program a thousand pages inflate

**Finding.** `doc/todo/41` priced a decoded-stream cache at **0.7% of interpretation** and refused
it; `doc/todo/47` said a cold document-wide search was the half nobody had moved. Both were right
about what they measured and both had measured a population in which the thing cannot appear: a
corpus walked **one page per document** has no repeats, and a decode repeats *between* pages. Over
one sweep of ISO 32000-2's 1023 pages, **8 798 of 12 586 filtered decodes are a second decode of
something already decoded** — 830 MB of re-inflation against 46 MB of first decodes, 23.4% of the
sweep's wall clock, and three font programs inflated 1993, 1486 and 808 times account for 3.2 s of
the 3.9. `pdf_syntax::Document` memoises decoded streams now, under a 4 MiB per-document budget,
and a hundred-page sweep costs **4 933 481 135 → 3 133 405 696 instructions, −36.5%**, with the
readback byte-identical.

**Date.** 2026-08-13.
**ADR.** [0317](../adr/0317-the-font-program-a-thousand-pages-inflate.md).
**Touched.** `crates/pdf-syntax/src/document.rs` (`DECODED_BUDGET`, `DecodedStreams`,
`DecodedEntry`, `allocation`, the memo in `decoded_stream_data_reported`, the clear in
`authenticate`, `Document::decoded_streams`, five tests), `crates/pdf-syntax/src/lib.rs` (one
export), `crates/viewer-core/examples/find_cost.rs` (the page limit that makes the split runnable
under callgrind, and the cache's own line), `doc/conformance/ledger.toml` (§7.4.1),
`doc/todo/41-decoded-stream-cache.md`, `doc/todo/47-search-performance.md`,
`doc/todo/10-bounds-that-cap-size.md` (road D's evidence, not its decision),
`doc/todo/README.md`, `doc/performance.md`, `doc/adr/0317-*`, this file.

## The instrument, and why it is not in the tree

A temporary counter build inside `decoded_stream_data_reported`: wall clock in the call, and the
call sequence keyed by the address and length of the encoded bytes so that a repeat is a repeat of
one allocation. It also recorded enough to *replay* the sequence through a least-recently-used
simulation at eight budgets, which is what makes the 4 MiB derived rather than picked — 1 MiB saves
21.4% of the sweep, 4 MiB 23.1%, and an unbounded cache holding the whole 46.6 MB working set
23.4%. It is not in the tree for ADR 0260's reason: a permanent timer on that path is exactly the
unmeasured cost this project exists to avoid.

## The machine could not be measured on, so it was counted on instead

Five other rounds ran beside this one and the load average sat between 20 and 72. Seven interleaved
wall-clock samples an arm gave medians of **9.75 s against 6.73 s with ranges of 9 s and 8 s** —
the right direction and no evidence whatever, and the same run's user CPU time was no better.
Callgrind's instruction counts do not move with the machine, and the two repeats of the on arm
landed 0.015% apart. **A round that must measure on a busy machine counts instructions**; that is
now written into `doc/todo/47` beside the numbers.

## What the test found that the argument would have left standing

The key is an address, and an address is only an identity while its allocation lives. The entry
therefore *holds* the encoded `Arc`, and
`a_stream_cannot_inherit_the_decoded_bytes_of_one_whose_buffer_it_reuses` is the test of it.
Replacing the pin with a copy of the same bytes — which compiles, and looks like a tidy-up — fails
that test **on the second iteration**: glibc hands the freed buffer straight back, the filter chain
matches, and the second stream is answered with the first one's decoded bytes. The invariant was
argued before it was tested, and the test is what shows the hazard is immediate rather than
theoretical. `doc/todo/41` had already met it in its own census and had to discard every count
below 4 KB for it.

## What was not taken

- **A memoised refusal.** `FilterRefusal::TooLarge` costs up to a gibibyte of inflation to reach
  and a hostile document can name one bomb stream from every page. The memo is the natural place
  to stop that, and it is left out because a refusal holds no decoded bytes and charging one to a
  *byte* budget needs a per-entry constant nobody has derived. Recorded in `doc/todo/41` as the one
  line still open there.
- **`doc/todo/10` §5's roads.** The instruction was to write what the measurement bears on into
  road D as evidence and take no decision, and that is what the entry there says: the repeats are
  *resources*, not content streams, so a streaming lexer and this memo take different bytes and the
  "cuts across" this file assumed is smaller than it looked.
