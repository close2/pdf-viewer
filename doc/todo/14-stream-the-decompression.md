# Road D — stream the decompression, so the bomb never becomes an allocation

Status: **done, and done a second time.** Everything the road was opened about is finished and the
one item that came back is closed: all five of §7.8.2's content streams are read through a window
(ADRs 0365, 0427, 0430), all five of §7.4's byte-to-byte filters have a resumable decoder, and a
**chain** of them is pumped end to end (ADRs 0429, 0587). A bomb in a page's `/Contents` costs
8.4 MB (flate) or 10 MB (LZW) where it cost 1032/1035 MB, one in a form XObject 10.7 MB where it
cost 1032, one in a **tiling pattern's cell** 9.4 MB where it cost 1055, one wearing a second
filter — §7.4.1 EXAMPLE 3's own arrangement for a page's marking instructions — 22 608 KB where it
cost 1 070 828, and the witness 194 MB where it took 381.

**The file is kept rather than deleted**, against `README.md`'s rule that a done item's file goes,
for two reasons that are about memory rather than about status: fourteen comments in
`pdf-syntax` and `pdf-model` point a reader here for the road's argument, and `doc/todo/01`'s
sweeps read it. What it holds now is the argument and the measurements; the *decisions* live in
the five ADRs, which is where `README.md` wants them. **One line below is a note for whoever takes
`doc/todo/41` next, and it is that file's item rather than this one's.**

## What a window does not do, and what still belongs to the memo

**A window removes the allocation. It does not remove the decode**, and ADR 0586 predicted
otherwise — it wrote that a chain pump "would take this document to kilobytes on **every** read
rather than on every read after the first", which is true of the *memory* and not of the reading.
The 25 000× that ADR measured was the distance between a refusal **remembered** (ADR 0437's memo)
and a refusal **re-reached**, and a window re-reaches it too, at 22 MB instead of a gibibyte.

That is not a pathology this road introduced, and the control measurement is what says so: the
*same* bomb with no armour at all — a single `/FlateDecode` form, which this tree has windowed
since ADR 0365 — already cost seconds and fourteen megabytes before any of this, because a bomb of
zeros decodes to white space and `MAX_OPERATIONS` never sees an operator to count. What ADR 0587
did was make the chained arrangement cost what the unwrapped one always cost, in both dimensions
at once. Which dimension to spend is this file's own ranking, below: time is what roads A and C
make interruptible, and memory is what none of them can take back.

**What is left is one fact travelling one hop, and it is `doc/todo/41`'s**: a refusal a *window*
reaches is not remembered, where one the buffered route reaches is. The reader knows it has read
`max_stream_len` decoded bytes out of one stream, which is the same `FilterRefusal::TooLarge` the
buffered route would have recorded under the same key — so the memo could hold it, and a bomb whose
encoded bytes fit `DECODED_BUDGET` would then be reached once per document rather than once per
read. Measured on ADR 0586's own witness: 154.98 µs against 14.62 s over twenty pages.

The producer half was ADR 0343's, the measurement ADR 0362's, the page's rewrite ADR 0365's, the
other three nested streams ADR 0427's, the LZW pump ADR 0429's, §8.7.3.1's tiling cell — which
needed the cell drawn once and its commands repeated before its decode could be windowed at all —
ADR 0430's, and the chain ADR 0587's.
Priority: 14 — the first road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here. **Finished, twice** (above); road B
([`15`](15-ship-the-confinement.md)) is what the owner's order points at next.
Witness: `tmp/Entwurf.pdf` — **not in the repository and not addable to it**, so no test may name
that path; Bomb B, which `doc/todo/10` §2 describes precisely enough to rebuild (sessions 519,
527 and 595 rebuilt it from that description, the last of them inside a pattern cell); and ADR
0586's hex-wrapped pair, whose generator is in `doc/history/712-…` and which the
seven-hundred-and-fourteenth session rebuilt from it to the byte — 4 174 537 and 12 523 517
encoded, the two sides of `DECODED_BUDGET`. A third witness is worth building beside them and is
two lines of the same generator: the **same deflate stream with no armour**, which is the control
that says what the armour costs and what it does not.
Instrument: `cargo run --profile gates -p pdf-model --example window_lexer_spike -- <pdf> <page>
whole|window|both [bytes]` is the experiment, and `--example token_window_census -- <dir>…` the
census behind both design questions; `content_budget_census` still prints a page's operators and
tokens. Peak resident from **`VmHWM` in `/proc`** as well as `ru_maxrss` — the second has a floor
equal to the *spawning* process's resident set, which reads 13 MB for a 4 MB program (ADR 0362) —
and `VmPeak` is what `RLIMIT_AS` compares against; callgrind for instructions, with
`RAYON_NUM_THREADS=1`
Clauses: §7.4 (filters), §7.8.2 (content stream syntax — including the array's token-boundary
rule this road leans on), §8.7.3.1 (the tiling cell, whose loop was the last obstacle), §8.9.7
(inline images)
Code: `crates/pdf-syntax/src/filter.rs` (`inflate_buffer` — the pump), `crates/pdf-syntax/src/lexer.rs`
(`Lexer::new(&'a [u8])` — the sink), `crates/pdf-model/src/inline_image.rs` (`scan`'s lookahead),
`crates/pdf-syntax/src/document.rs` (`pumping`, which decides the route for both
`stream_source` and `nested_content_source`, and is the one function a filter pump changes)

## Why this one is first

It is the only road that changes the *kind* of the quantity. A and C bound time and leave the
allocation untouched; B answers memory by killing the process. This one removes the allocation,
after which the counting bounds have nothing left to justify them — and it needs **no number at
all**, because a window is a buffer size rather than a policy, which is what the owner's brief
objects to.

---

*What follows was `doc/todo/10` §5.1's D section and moved here whole when the owner chose it, so
that the argument lives with the item. `doc/todo/10` §5 keeps the four-road comparison.*

Raised by the project owner, who observed that nobody here had considered it:

> We might be able for instance to prevent gif-bombs by streaming the decompression. There are
> possibly reasons it doesn't fit, but I have the impression that we haven't even considered it.

**They are right that it was never considered, and the code is much closer to it than the other
three roads are to theirs.** When that was written, `filter::flate` held a *streaming* decoder —
`flate2::read::ZlibDecoder`, an `io::Read` — and then called `read_to_end` into a `Vec`: the
decompressor streamed and the consumer did not, and Bomb B's 3.7 GB was that one call.

**The adapter is gone as of ADR 0343, and what replaced it is a pump.** `filter::inflate_buffer`
holds a `flate2::Decompress` across iterations, keeps its own input cursor, writes through
`decompress_vec`, and stops on three named conditions. The producer half of a window-fed decoder is
therefore written, tested and shipped; what is left is the *sink* — a fixed window in place of the
growing `Vec`, and a consumer between the two — plus the lexer and the inline-image lookahead
below.

**What it changes is the *kind* of the quantity, and that is the whole argument.** A window-fed
lexer turns a decompression bomb from an unbounded *allocation* into unbounded *time* — and time
is exactly what roads A and C make interruptible, while memory is what none of them can take
back. A 1.85 MB file inflating to 1.77 GiB would cost a fixed buffer and run until somebody stops
it, instead of taking the machine down before anybody is asked.

**Where it fits, and where it does not** — this is the part that has to be measured rather than
assumed, and the split is not even:

- **Content streams are the good case, and they are the case that matters.** The interpreter reads
  a content stream once, forwards, one token at a time, and never seeks back. **Table 31 blesses
  the shape** — where `/Contents` is an array, "[t]he division between streams may occur only at
  the boundaries between lexical tokens" — so several parts chain into one reader instead of being
  concatenated into one `Vec`, which is where `doc/todo/10` §3.3's *missing aggregate budget* also
  lives. (That sentence is §7.7.3.3's, in Table 31's `/Contents` row; this file said §7.8.2 for
  nine sessions and the quotation gate caught it when ADR 0365 quoted it in code.) Every filter
  that appears on a content stream — Flate, LZW, ASCII85, ASCIIHex, RunLength — is inherently
  streaming. **The criterion in the first sentence is what decided which streams ADR 0365
  windowed**: a form read three times over by §11.6.6's paired runs is not read "once, forwards",
  and stayed whole.
- **`Lexer::new` takes `&'a [u8]`**, and that is the real work. A reader-fed lexer needs a window
  that can hold the largest single lexical object, and `max_string_len` is 2²⁶, so either the
  window grows for one token or a string gets its own bound. Neither is hard; both are decisions —
  and **the census has now measured what real documents ask for**: the largest token in 39 976
  documents is **390.16 KiB**, two of 225 million pass 64 KiB and none passes 1 MiB, so 2²⁶ is
  168 times anything measured (`token_window_census`, ADR 0362).
- **Inline images are the sharp edge**, and the census says how sharp. `inline_image::scan` reads
  forward from `ID` for the end of data whose length the dictionary does not state, which is a
  lookahead of unbounded size inside a bounded window — but **90 304 of 93 930 inline images state
  or imply their length before their data is read** (336 by `/L`, 89 968 by §8.9.3's arithmetic),
  and of the **3 455** that did not, **the largest is 2.99 KiB**. **Since the
  six-hundred-and-thirty-third session most of that remainder is derived rather than searched**:
  §7.3.8.2 makes a filtered extent the filter's own end-of-data marker, so the search is left with
  the chains this crate has no resumable decoder for. `token_window_census` prints the split, and
  it is a pump question rather than a clause one — which is this file's subject from the other end.
  ADR 0466.
- **The image and font paths want the whole thing anyway.** An embedded font program is parsed
  with random access; image sample data is indexed; an ICC profile, an xref stream and JBIG2
  globals are all read as a unit. `decoded_stream_data` returning `Arc<[u8]>` is right for those
  and streaming buys them nothing — so this is an *added* route rather than a replacement, and the
  refusals for those paths (`image::MAX_SAMPLES`, `icc::MAX_PROFILE`, the codec bounds) stay
  exactly as they are.
- **It meets `doc/todo/41`'s decoded-stream cache and the document-wide search** (ADRs 0317, 0330,
  0335) — which want to *keep* a decoded stream rather than stream past it — and the measurement
  says **the two designs disagree about content streams and agree about everything else**. What
  repeats over a document-wide sweep of ISO 32000-2 is not the content streams — those are read
  once each, forwards, which is exactly this road's good case — but the *resources*: 8 798 of
  12 586 filtered decodes are a second decode of something already decoded, 830 MB of
  re-inflation against 46 MB of first decodes, and the three largest are font programs inflated
  1993, 1486 and 808 times. A font program is a random-access parse, and the list above already
  says streaming buys it nothing. So the cache's value and this road's value come from different
  streams: a streaming lexer over content streams would leave 23.4% of a sweep exactly where ADR
  0317 found it, and the memo removes nothing this road was going to remove. **What stays real is
  narrower than a conflict**: a round doing this must not route font, image and profile streams
  through the window, and the memo is one more reason those paths stay whole.
- **One behaviour must survive it.** `flate` deliberately keeps partial output from a truncated
  stream, because "a partially-inflated content stream still renders most of a page" — and since
  ADR 0343 that recovery is *reliable* and *loud*, which is the property a rewrite may not lose. A
  streaming rewrite that stops distinguishing damage (§7.4.1) from the bound is the same bug with
  better memory behaviour.

## What the measurement said

**This section used to say what a round taking the road owes first — the measurement — and the
five-hundred-and-twenty-seventh session took it.** ADR 0362 has the argument and the invocations;
what belongs here is the result and what it decides. `examples/window_lexer_spike` is the
experiment and it is committed, so none of this has to be believed.

**The verdict is yes on both halves.** A `Lexer` fed from a fixed window holds the bomb at the
window and reads the honest document to the same token:

| decode + lex only, `--profile gates` | whole buffer | 64 KiB window |
|---|---|---|
| **Bomb A** 0.39 MB → 400 MB | 765–766 MB, 2.6–2.9 s, 200 M tokens | **4 MB**, 2.5–3.7 s, 200 M tokens |
| **Bomb B** 1.85 MB → 1.9 GB | 1030 MB, 1.8–2.3 s, **0 tokens** | **6 MB**, 10.9–16.6 s, **950 M tokens** |
| **the witness**, 141 MiB of content | 379–380 MB, 0.8–1.4 s | **98 MB**, 0.7–1.8 s |

Bomb B's whole-buffer row is the shipped refusal working: `max_stream_len` stops it at a gibibyte,
so a gigabyte is spent and **not one token** comes out. The window arm reads all 950 million of
them in a buffer that never grows. **The kind of the quantity changes**, which was the claim.

- **The counts agree**, which is the check that matters more than the megabytes: 20 834 587 tokens
  and 3 185 295 operators on the witness either way, and page one of the pdf.js corpus through a
  deliberately small **512-byte** window is **948 agree, 0 disagree** (10 skipped for a filter chain
  the spike does not pump, 10 unopenable, 6 pageless).
- **Instructions: +4.10%** on the witness (callgrind, `RAYON_NUM_THREADS=1`: 8 972 848 710 →
  9 340 854 748). Wall clock did not separate them.
- **The window's size is noise**: 4 KiB / 64 KiB / 1 MiB cost 36 156 / 2 258 / 142 refills and
  120 367 / 7 694 / 453 re-lexed bytes on 141 MiB, at 98 / 98 / 99 MB of peak.

**The prize is larger than this file predicted, and `massif` says why.** The table here used to
read "the witness, 66 MB content stream, 381 MB → about 315 MB", with the 315 attributed to "the
display list and the raster". **Both halves were wrong.** The content stream is **147 972 263
bytes** — one part, 141 MiB, which is what `lexer.rs`'s comment has said since ADR 0341 — and the
peak of an ordinary interpretation is not the display list at all. The four blocks alive at it:

| block | bytes |
|---|---|
| `filter::flate`'s inflate buffer | 147 972 800 |
| the `Arc<[u8]>` the decode is handed over as | 147 972 263 |
| the file, as `Document::open_with_password` keeps it | 49 679 528 |
| the *encoded* stream, copied out of the file by `Parser::parse_stream_data` | 49 678 824 |

The first two are two copies of the same decoded content and are exactly what a window replaces.
The display list is about **99 MB** and arrives after the decode has been freed (the heap grows
247 → 346 MB while the page is interpreted). So the road leaves **about 193 MB where 446 MB of
heap stands today**, and the last two rows are the reason it is not less.

## What the rewrite decided — carried out in ADR 0365

The four decisions this section used to pose are answered in the code, each with the census's own
number in front of it. `crates/pdf-model/src/content/reader.rs` holds all four and says where
each comes from:

1. **A token longer than the window** grows the buffer to `CEILING` = 1 MiB, above every token in
   the population, and past that is `ContentIssue::TokenTooLong` and a step to the next
   white-space byte. Not `max_string_len`'s 64 MiB, as this file asked.
2. **The inline-image lookahead** starts at the window and doubles only while the answer may have
   been cut, to `LOOKAHEAD` = 16 MiB — 1.78 times the largest inline image measured — and past it
   is `InlineImageError::Unbuffered`, which is a *different* sentence from `NoTerminator`.
3. **Where the pump reads from**: `Stream::data`'s `Arc<[u8]>`, which is a clone of a handle
   rather than of bytes. The second copy of the *encoded* stream that ADR 0362 found is still
   there and is `Parser::parse_stream_data`'s, which is `doc/todo/10` §3's residue and not this
   road's.
4. **The other filters** are **not** done — see below, and they are all that is left.

Both obligations were met: `ContentReader::with_token` lends the token to a closure, so the
compiler refuses anything that keeps a borrowed keyword past a refill, and ADR 0343's damage is
produced as the pump goes and reported after the run.

**And one consequence landed on another road, as predicted.** `doc/todo/10` §2's table said
`MAX_OPERATIONS` guards nothing a bomb needs, "because the memory is already spent" before the
first operator is counted. Under a window there is no memory to spend, and Bomb B now reports
`MAX_OPERATIONS` in 0.24 s.

## What the other four content streams did — carried out in ADR 0427

This section asked for "a route that streams the *first* read and remembers the bytes for the
second, or a measurement saying the re-inflation is cheaper than the memo", and the answer was
neither: **the memo already draws that line and draws it for the same reason.**
`DecodedStreams::put` declines any decode it cannot hold beside its encoded bytes, so a stream it
declines is *already* re-decoded on every read — which means windowing exactly that half costs
nothing that was not already being paid, and every bomb is in that half by construction. Nothing
arbitrary was replaced by something equally arbitrary, which is `doc/todo/10` §6's rule: the split
is `put`'s own condition, asked before the decode instead of after it, through one function.

`Document::nested_content_source` is the rule, `NestedContent` is what the interpreter holds
instead of a buffer, and a reader is made per run — which is what lets §11.6.6's paired runs, a
tiling pattern's cell loop and a Type 3 glyph description keep reading the same stream as often as
they like. Measured: a bomb in a form is **10.7 MB against 1032**, ISO 32000-2 page 101 costs
**+0.089%** instructions, and the 974 corpus display lists are byte-identical.

**The rule has one exception and the `page` fuzz target found it.** "A decode the memo declines is
re-run on every read anyway" is a claim about *who reads*, and it is false of §8.7.3.1: `Tiling`
holds the cell's decode for the whole tiling, so windowing that one inflates the cell again for
every cell painted — 0.24 s against 9.0 s on a mutated pattern, with `MAX_TILES` allowing four
thousand cells. The cell keeps its whole decode, in a *type* rather than in a comment, and **the
cost of that is a bomb hidden in a tiling pattern's cell, which still costs its gibibyte** —
exactly what it cost before this round and no more. Whoever takes it needs the cell drawn once and
its commands repeated, which is `pdf_render::Repeats` one step further than `fold_repeated_marks`
takes it today.

## What was owed and is done — the tiling cell (ADR 0430)

**A bomb in a tiling pattern's cell** was this road's last item and the one exception to ADR
0427's rule: `Tiling` held the cell's decode for the whole tiling, so windowing it inflated the
cell once per site — 0.24 s against 9.0 s, with `MAX_TILES` allowing four thousand sites. The fix
was the one this file named: **the cell drawn once and its commands repeated**, `pdf_render::Cell`
one step past `fold_repeated_marks`. §8.7.3.1 asks for exactly that — "identical copies" of one
glass tile — so it is the clause's construction rather than an optimisation of it, and the
exclusion came off with the loop that caused it.

Measured: the bomb **1055 MB → 9.4 MB** and 1.27 s → 0.12 s, its silence replaced by
`MAX_OPERATIONS`; ordinary tiling pages **−90% instructions** (`issue2177.pdf` −94.1%), the
non-tiling control +0.007%. And drawing the cell once made a §8.7.2 misreading visible that
re-interpretation had hidden: a pattern named inside a cell was anchored to the page rather than
to the cell, which `issue8565.pdf` showed as a lost radial glow. ADR 0430.

## What was owed and is done — the filter pump (ADR 0429)

`Document::stream_source` used to pump a single `FlateDecode` and hand everything else back whole.
It now pumps a single `LZWDecode` too, which was the sharper of road D's two remaining bombs
(1365:1 against 585:1 measured on operators). The three §7.4 filters it left out were left out on
their expansion ratio — a filter that cannot name a bomb has nothing for a window to save:
`ASCIIHexDecode` shrinks its input (1:2), `ASCII85Decode` reaches 4:1 from a stream of nothing but
`z`, `RunLengthDecode` 64:1. `Document::pumping` (was `is_pumpable`) is the one place a chain's
route is decided, and it is the one function a further filter pump changes.

**That ranking was right about the question it asked and wrong about the one that mattered**, which
the section below is the correction to: a filter that cannot name a bomb can still *carry* one, and
a chain is only as windowed as its worst stage.

## What was owed a second time and is done — the chain (ADR 0587)

All three of the filters ADR 0429 ranked out now have a resumable decoder — the shape it named,
a state struct whose buffered entry point is a loop over the same state — and `Document::pumping`
walks the whole of `/Filter` instead of accepting one stage. §7.4.1's own EXAMPLE 2 and EXAMPLE 3
are cascades, the second of them over "the marking instructions for a page", so the arrangement
that took road D off was the standard's own worked example rather than a hostile invention.

Each stage's licence is its clause's arithmetic and is written where the route is chosen:
`ASCIIHexDecode` "shall produce one byte of binary data for each pair" (1:2, one nibble of state),
`ASCII85Decode` "5 ASCII characters for every 4 bytes" read backwards or four per `z` (4:1 at
most, one group of state), `RunLengthDecode` "1 to 128 bytes" per length byte (64:1, one run).
`Pump` composes them through a fixed **link** per stage, so nothing between two stages is ever
materialised; a chain of one allocates no link at all.

Measured on ADR 0586's own pair of witnesses, twenty pages apiece: **1 070 828 KB → 22 608 KB** and
**1 103 596 KB → 55 016 KB** of peak `VmHWM`, ISO 32000-2 page 101 **+0.050%** instructions, the
two largest corpus documents stating `[/ASCII85Decode /FlateDecode]` +0.0003% and −0.016%, and
every corpus first page's display list byte-identical.

`doc/todo/10` §6 binds whatever lands: nothing arbitrary replaced by something equally arbitrary,
the gates stay reproducible, a count that reports says what it counted, and **a bound on an
allocation is measured on the allocation** — ADR 0354's lesson, which is why `capacity`,
`VmHWM` or `massif`'s peak snapshot has to be read rather than the refusal believed. The LZW bomb
was measured on `VmHWM` from `/proc`: **1035 MB whole → 10 MB windowed**, 2.12 s → 0.11 s.
