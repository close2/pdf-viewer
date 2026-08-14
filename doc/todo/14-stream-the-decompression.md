# Road D — stream the decompression, so the bomb never becomes an allocation

Status: **chosen, and first of three** — the project owner ordered the roads D → B → C in the
five-hundred-and-nineteenth session's aftermath. **The producer half is already written**: ADR
0343 replaced the `read_to_end` adapter with a pump, so what is left is the sink. Nothing is
built yet, and **what this owes first is a measurement rather than a rewrite** (below).
Priority: 14 — the first road of [`10`](10-bounds-that-cap-size.md), whose §5 table prices all
four and whose §6 binds whatever lands here
Witness: `tmp/Entwurf.pdf` — **not in the repository and not addable to it**, so no test may name
that path; and Bomb B, which `doc/todo/10` §2 describes precisely enough to rebuild (session 519
rebuilt both bombs to the byte from that description)
Instrument: `cargo run --release -p pdf-model --example content_budget_census -- <dir>…`; peak
resident from `ru_maxrss` and `VmPeak` from `/proc` (session 519's note: `/usr/bin/time -v` is not
on this machine, and `VmPeak` is what `RLIMIT_AS` compares against); callgrind for instructions,
with `RAYON_NUM_THREADS=1` for a serial arm
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
  window grows for one token or a string gets its own bound. Neither is hard; both are decisions.
- **Inline images are the sharp edge.** `inline_image::scan` searches forward from `ID` for `EI`
  over data whose length the dictionary does not state, which is a lookahead of unbounded size
  inside a bounded window.
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

## What a round taking this owes first

**The measurement, not the rewrite.** Feed `Lexer` from an `io::Read` behind a fixed window, run
Bomb B and the witness through it, and report peak resident and `VmPeak` for both against
`doc/todo/10` §1's and §2's figures. If a 64 KiB window draws the witness at about a second and
holds Bomb B at a few megabytes, the rest of that file's arithmetic changes shape.

**The prize, in today's numbers** (ADR 0354 re-took these; a third of the old prize went with the
buffer fix, which is why the table is here rather than remembered):

| | today | what this road would leave |
|---|---|---|
| Bomb B, 1.85 MB of file | 1031 MB (the bound, exactly) | a window — 64 KiB |
| the witness, 66 MB content stream | 381 MB | about 315 MB |

The 315 MB it would leave on the witness is the display list and the raster, which no road in
`doc/todo/10` touches — so a round that wants the witness frugal needs this road *and* something
that is not in that file.

`doc/todo/10` §6 binds whatever lands: nothing arbitrary replaced by something equally arbitrary,
the gates stay reproducible, a count that reports says what it counted, and **a bound on an
allocation is measured on the allocation** — ADR 0354's lesson, which is why `capacity`,
`ru_maxrss` or `massif`'s peak snapshot has to be read rather than the refusal believed.
