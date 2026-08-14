# Road D — stream the decompression, so the bomb never becomes an allocation

Status: **measured, and the rewrite is what is left** — the project owner ordered the roads
D → B → C in the five-hundred-and-nineteenth session's aftermath. **The producer half was already
written** (ADR 0343's pump), and **the measurement this file used to owe is done** (ADR 0362):
a window-fed lexer holds Bomb B at a window, reads the witness to the same token for +4.10%
instructions, and reads page one of the whole pdf.js corpus identically through a 512-byte
window. §"What the measurement said" has the numbers and §"What the rewrite owes" the four
decisions left.
Priority: 14 — the first road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here
Witness: `tmp/Entwurf.pdf` — **not in the repository and not addable to it**, so no test may name
that path; and Bomb B, which `doc/todo/10` §2 describes precisely enough to rebuild (sessions 519
and 527 rebuilt both bombs to the byte from that description)
Instrument: `cargo run --profile gates -p pdf-model --example window_lexer_spike -- <pdf> <page>
whole|window|both [bytes]` is the experiment, and `--example token_window_census -- <dir>…` the
census behind both design questions; `content_budget_census` still prints a page's operators and
tokens. Peak resident from **`VmHWM` in `/proc`** as well as `ru_maxrss` — the second has a floor
equal to the *spawning* process's resident set, which reads 13 MB for a 4 MB program (ADR 0362) —
and `VmPeak` is what `RLIMIT_AS` compares against; callgrind for instructions, with
`RAYON_NUM_THREADS=1`
Clauses: §7.4 (filters), §7.8.2 (content stream syntax — including the array's token-boundary
rule this road leans on), §8.9.7 (inline images)
Code: `crates/pdf-syntax/src/filter.rs` (`inflate_buffer` — the pump), `crates/pdf-syntax/src/lexer.rs`
(`Lexer::new(&'a [u8])` — the sink), `crates/pdf-model/src/inline_image.rs` (`scan`'s lookahead),
`crates/pdf-syntax/src/document.rs` (`decoded_stream_data`, which stays whole for the paths below)

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
  a content stream once, forwards, one token at a time, and never seeks back. §7.8.2 even blesses
  the shape: where `/Contents` is an array, "the division between streams may occur only at the
  boundaries between lexical tokens", so several parts chain into one reader instead of being
  concatenated into one `Vec` — which is where `doc/todo/10` §3.3's *missing aggregate budget* also
  lives. Every filter that appears on a content stream — Flate, LZW, ASCII85, ASCIIHex,
  RunLength — is inherently streaming.
- **`Lexer::new` takes `&'a [u8]`**, and that is the real work. A reader-fed lexer needs a window
  that can hold the largest single lexical object, and `max_string_len` is 2²⁶, so either the
  window grows for one token or a string gets its own bound. Neither is hard; both are decisions —
  and **the census has now measured what real documents ask for**: the largest token in 39 976
  documents is **390.16 KiB**, two of 225 million pass 64 KiB and none passes 1 MiB, so 2²⁶ is
  168 times anything measured (`token_window_census`, ADR 0362).
- **Inline images are the sharp edge**, and the census says how sharp. `inline_image::scan`
  searches forward from `ID` for `EI` over data whose length the dictionary does not state, which
  is a lookahead of unbounded size inside a bounded window — but **90 304 of 93 930 inline images
  state or imply their length before their data is read** (336 by `/L`, 89 968 by §8.9.3's
  arithmetic), and of the **3 455** that need the search **the largest is 2.99 KiB**.
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

## What the rewrite owes

Four decisions, each now with a number in front of it, and two obligations that are not decisions.

1. **A token longer than the window.** None was seen: 512 bytes over the pdf.js corpus produced
   no such page, and the largest token anywhere in 39 976 documents is 390.16 KiB. Grow the window
   for one token up to a bound, or refuse with a report — but not silently, and not `max_string_len`'s
   64 MiB, which no document comes near.
2. **The inline-image lookahead.** 96.1% of inline images state or imply their length before their
   data; the search route's largest witness is 2.99 KiB. A bounded lookahead with §8.9.7's existing
   `NoTerminator` refusal past it is the shape; the *data* then goes the resource route, whole, as
   an image always has.
3. **Where the pump reads from.** The spike's 98 MB on the witness is two copies of the same
   47 MB — the file, and `Stream::data`'s owned `Arc<[u8]>`. Feeding the pump from a borrowed slice
   of the document's bytes takes one of them off; an `Arc<[u8]>` cannot be sub-sliced, so this is a
   change to how a stream's raw bytes are handed over rather than a smaller buffer.
4. **The other filters.** The spike pumps `FlateDecode` and no filter, which carried 948 of the 958
   pdf.js documents that open with a page one; the other **10** declined for their filter chain.
   LZW, ASCII85, ASCIIHex and RunLength are all streaming by construction (§7.4) and each needs its
   pump written.

The obligations: **a token borrowed from the window may not outlive a refill** — `Token::Keyword`
borrows, and the honest API is one that borrows `&mut self`, so the compiler enforces it rather
than a comment — and **ADR 0343's damage reporting must be produced as the pump goes**, since
`ContentIssue::Damaged` carries how much was kept.

**One consequence lands on another road.** `doc/todo/10` §2's table says `MAX_OPERATIONS` guards
nothing a bomb needs, "because the memory is already spent" before the first operator is counted.
Under a window there is no memory to spend, so the counter is reached four million operators in —
a few megabytes into Bomb B's 1.9 GB — and stops it there instead of after the eleven seconds the
spike, which has no operator bound, actually took.

`doc/todo/10` §6 binds whatever lands: nothing arbitrary replaced by something equally arbitrary,
the gates stay reproducible, a count that reports says what it counted, and **a bound on an
allocation is measured on the allocation** — ADR 0354's lesson, which is why `capacity`,
`ru_maxrss` or `massif`'s peak snapshot has to be read rather than the refusal believed.
