# 714 — The chain a window learned, and the prefix that was not one

`doc/todo/14`'s reopened item, taken and closed (ADR 0587). A `Pump` ran one filter; it now runs a
chain, and all five of §7.4's byte-to-byte filters have a resumable decoder.

## What was built

`filter::Stage` is one of §7.4's filters as a resumable decoder and `filter::stage` is the one
place a filter name and its own parameters become one — asked by `Document::pumping`, by
`Document::filtered_extent` and by `decode_reported`'s `/EarlyChange`, which used to be read in
three places over the same bits. `Pumping` became the *chain*, `Pump` holds one `Running` per
stage with an eight-kilobyte link between them, and one call is one pass front to back.

`AsciiHex`, `Ascii85` and `RunLength` are the three new state structs, each with its buffered
entry point unchanged beside it (trap 6). `Inflate::pump` and `Lzw::pump` became
`Engine::turn(input, last, out)`, where `last` is the only thing a chain adds: over a whole buffer
there is no such thing as input still to come, so "no progress means truncated" is right only when
the source is closed.

## Which filters pump, and on what evidence

Each from its own clause's arithmetic, which is what a window over it needs: `ASCIIHexDecode` 1:2
(§7.4.2, one nibble of state), `ASCII85Decode` 4:1 at most (§7.4.3, one group), `RunLengthDecode`
64:1 (§7.4.5, one run), `LZWDecode` 1365:1 (§7.4.4.1 NOTE 2), `FlateDecode` 1032:1 measured. The
five that are absent are not byte filters at all — four image codecs and `Crypt`.

**ADR 0429 ranked the first three out on their expansion ratio and was right about the question it
asked**: a filter that cannot name a bomb has nothing for a window to save *on its own*. A chain is
only as windowed as its worst stage, and §7.4.1's own EXAMPLE 3 puts a page's marking instructions
behind exactly such a stage.

## The numbers

`find_cost` over twenty pages each drawing one form `XObject`, both arms built in one sitting,
`RAYON_NUM_THREADS=1`, peak from `VmHWM` sampled every 20 ms. ADR 0586's witness pair rebuilt from
its generator to the byte, plus a **control** with no armour at all:

| twenty pages | before, peak | after, peak | before, sweep | after, sweep |
|---|---|---|---|---|
| one `/FlateDecode`, 2 GiB of zeros (control) | 13 920 KB | 14 272 KB | 32.82 s | 23.76 s |
| `[/AHx /Fl]`, 4 174 537 B encoded | 1 070 828 KB | **22 608 KB** | 154.98 µs | 14.62 s |
| `[/AHx /Fl]`, 12 523 517 B encoded | 1 103 596 KB | **55 016 KB** | 6.18 s | 14.12 s |

`callgrind`, `RAYON_NUM_THREADS=1`: ISO 32000-2 p101 ×50 **+0.050%**; `S2.pdf` +0.0003% and
`personwithdog.pdf` −0.016%, the two largest corpus documents stating `[/ASCII85Decode
/FlateDecode]`.

The wall clock is printed rather than argued from, and the control row is why: the same figure
moved 13.91 → 32.82 → 23.76 s across three runs as neighbouring rounds came and went, and a first
attempt at the whole table was thrown away at load average 80–140. The peaks are deterministic.

## The two things that were wrong, and the instrument that said so

**A prefix of a table is not a shorter table.** The round first made §7.4.3's "shall cause an
error" agree between the two routes by giving `ascii85` a damaged prefix, on ADR 0343's reasoning.
`display_list_digest` over the corpus moved exactly one document: `PDFBOX-3148-2-fuzzed.pdf`, whose
**cross-reference stream** is `/Filter [/ASCII85Decode]` with a bad byte eight bytes in — refusing
it sends the parser to its header scan and the file's one page is found; handing back eight bytes
makes them a cross-reference section with almost every entry missing, and the page is lost in
silence. The two routes are ADR 0343's own distinction arriving by *route* rather than by consumer,
because the window is only ever run over §7.8.2's "sequence of instructions" and the buffered route
serves everything else. Reverted; both halves are now pinned by one test.

**A pass that fills a link and drains it in one visit stalls.** A link is filled by the stage that
writes it and emptied on the authority of the stage that reads it, so compaction is a pass of its
own. Written the other way it terminated early with a plausible prefix, and only the agreement test
at window sizes *above* the link could see it.

## What the round found that it was not sent for

**ADR 0586's redirect was half right, and the half that failed is the more useful one.** It wrote
that a chain pump would take its witness "to kilobytes on **every** read rather than on every read
after the first". Kilobytes of memory, yes. The read is still the bomb's whole decode, and the
25 000× that ADR measured was the distance between a refusal *remembered* and a refusal
*re-reached* — which a window does not remove. The control row is what turns that from a regression
into a statement: the same bomb unwrapped already cost the tree seconds and fourteen megabytes,
because `stream_source` has windowed a single `FlateDecode` since ADR 0365 and a bomb of zeros
decodes to white space that `MAX_OPERATIONS` never counts. The chained arrangement now costs what
the unwrapped one always cost.

The line went back to `doc/todo/41` smaller than it left: the refusal a window reaches is the same
`FilterRefusal::TooLarge` under the same key the buffered route would have recorded, so what is
owed is one fact travelling one hop, not a new charge, ceiling or key.

**And the briefing's clause citation was wrong**, which is the fourteen-in-five pattern again:
`doc/todo/14` called `[/ASCIIHexDecode /FlateDecode]` "§7.4.7's own worked arrangement". §7.4.7 is
`JBIG2Decode` and its example is `[/ASCIIHexDecode /JBIG2Decode]`. The arrangement is §7.4.1's —
EXAMPLE 2 is `/Filter [/ASCII85Decode /LZWDecode]` and EXAMPLE 3 is `[/ASCII85Decode /FlateDecode]`
over "a stream, containing the marking instructions for a page", which is a stronger citation
because it is a *content* stream.

## The sequence

Whole, this being a round that can change a pixel and touches `pdf-syntax`.

## Fuzzing

`page` — the target whose binary contains `pdf_model::interpret` and therefore the whole
content-reader and pump path — under `-fork=6 -rss_limit_mb=4096 -timeout=60` for ten minutes over
the reduced corpus. **No crash, OOM or timeout**, and `fuzz/artifacts/page/` is empty.

## Ledger

§7.3.8.2, §7.4, §7.4.1, §7.4.2, §7.4.3, §7.4.4.2, §7.4.4.3, §7.4.5 and §7.8.2, with the new tests
on four of them.

## Tests

`every_pumpable_chain_agrees_with_the_whole_decode` (nine arrangements × five window sizes,
including §7.4.1's own two cascades and three stages so that a link feeds a link) ·
`a_bomb_behind_an_ascii_armour_costs_the_window` · `a_chain_reports_the_damage_the_whole_decode_does` ·
`a_raw_deflate_stream_behind_an_armour_still_falls_back` (the rewind, which is the one thing a
chain has that a stage did not) · `a_base85_error_keeps_the_groups_before_it` (both routes) ·
`a_chained_form_is_read_through_the_window_and_draws_what_the_whole_decode_draws` (the route
asserted before the picture, ADR 0427's rule).
