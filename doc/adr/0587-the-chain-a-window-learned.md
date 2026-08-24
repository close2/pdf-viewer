# ADR 0587 — The chain a window learned, and the prefix that belongs to one route only

Status: accepted, 2026-08-24. Session 714. Takes the item ADR 0586 reopened in `doc/todo/14`: a
`Pump` ran a *single* filter, so a bomb wearing a second one escaped road D entirely. Amends the
ledger rows for §7.3.8.2, §7.4, §7.4.2, §7.4.3, §7.4.4.2, §7.4.4.3, §7.4.5 and §7.8.2.

## What changed, in one line

**A bomb behind an ASCII armour stops being an allocation.** Twenty pages drawing one
`[/ASCIIHexDecode /FlateDecode]` form `XObject` cost **1 070 828 KB** of peak resident memory and
now cost **22 608 KB**; the same bomb padded past the decoded-stream budget cost **1 103 596 KB**
and now costs **55 016 KB**. The display list of every corpus document's first page is
byte-identical across the change.

## Which of §7.4's filters pump, and on what evidence

The question the round had to answer from the clauses rather than from convenience. A filter can
be windowed when the state it carries between two bytes is bounded and its output length is a
function of its input, and **all five of §7.4's byte-to-byte filters satisfy both** — which was
not obvious before it was checked, because the previous round had ranked three of them out.

| filter | the clause's own arithmetic | the state between turns |
|---|---|---|
| `ASCIIHexDecode` (§7.4.2) | "shall produce one byte of binary data for each pair of ASCII hexadecimal digits" — **1:2** | one nibble |
| `ASCII85Decode` (§7.4.3) | "shall produce 5 ASCII characters for every 4 bytes of binary data", read backwards, or four bytes for one `z` — **4:1 at the very most** | one group of at most five digits |
| `RunLengthDecode` (§7.4.5) | a length byte and "1 to 128 bytes of data"; its NOTE gives "approximately 64:1" at best | one run |
| `LZWDecode` (§7.4.4.2) | §7.4.4.1 NOTE 2: "a compression approaching 1365:1 for long files" | the table, the bit accumulator, the sequence being handed over (ADR 0429) |
| `FlateDecode` (§7.4.4.1) | measured at 1032:1 on `doc/todo/10` §2's Bomb B | `flate2::Decompress` (ADR 0343) |

**The five filters that are not here are not byte filters at all, and that is a stronger answer
than a ratio.** §7.4.6's `CCITTFaxDecode`, §7.4.7's `JBIG2Decode`, §7.4.8's `DCTDecode` and
§7.4.9's `JPXDecode` produce a raster with a width, a depth and a component count — `filter::decode`
answers `None` for all four and the image pipeline runs them — and §7.4.10's `Crypt` is not a
transformation this code performs at all, §7.6 having answered it before any filter is reached.

**ADR 0429 left the first three out on their expansion ratio, and that reasoning was right about
the question it asked.** A filter that cannot name a bomb has nothing for a window to save *on its
own*. What it did not ask is what a bomb does when it is put *behind* one of them, and the answer
is that **a chain is only as windowed as its worst stage**: `Document::pumping` granted a window
to a single `FlateDecode` or `LZWDecode` and handed every chain back whole, so wrapping the bomb
in `ASCIIHexDecode` — which costs an author two bytes per one and nothing else — took the whole
road off. §7.4.1's own EXAMPLE 3 writes that arrangement for a page's marking instructions, so it
is not a hostile-only shape:

> The following example shows a stream, containing the marking instructions for a page, that was
> compressed using the Flate compression method and then encoded in ASCII base-85 representation.

## The construction: one decoder per filter, and a fixed link between stages

Trap 6's hazard is unchanged — a second decoder beside the first is how two implementations of one
clause drift — so each new stage is a resumable state struct whose buffered entry point is a loop
over the same state, exactly as `Lzw` is. `AsciiHex` is one nibble and an ended flag; `Ascii85` is
a group of at most five digits and the four bytes a completed group spills; `RunLength` is a
four-state enum over §7.4.5's own three arms.

`Engine::turn(input, last, out)` is the shape they share, and `last` is the only thing a chain
adds to a stage: over a whole buffer there is no such thing as input still to come, so `flate`'s
"no progress means truncated" and `Lzw::step`'s "no byte means truncated" are correct only when
the source is closed. The driver owns that, because the driver owns the source.

`Pump` holds one `Running` per stage — the engine, a **`LINK`-byte link** holding what it has
produced and the next stage has not taken, and how it finished. One call is one pass over the
chain, front to back, so bytes a stage produces reach the stage after it within the same call. The
last stage writes into the caller's window and needs no link, so **a chain of one allocates nothing
at all**, which is every stream this crate pumped before this round.

**The one thing a chain has that a single stage did not is a rewind.** `Inflate` asks to re-read
its input under raw framing when zlib's produced nothing — `flate`'s fallback, and streams missing
their two-byte header are common — and for the first stage that is free because the whole encoded
buffer is still there. For a later stage the input is a link, so links are compacted in a pass of
their own before any stage runs, and a link is held back from compaction while the stage reading it
may still ask. Where it cannot be — the stage took a whole link's worth without emitting a byte —
the offer is withdrawn rather than the buffer being allowed to grow. A wrong zlib framing fails at
its two-byte header, so reaching that needs a stage that consumes eight kilobytes of a *valid*
zlib stream while emitting nothing and then fails, and such a stream decodes to nothing under raw
framing either.

**Compaction is a pass of its own for a reason worth keeping**: it was first written into the same
visit as the turn, and a link is filled by the stage that writes it and emptied on the authority of
the stage that reads it — so one visit had the writer looking at a full buffer it was about to be
given room in. The chain stalled at eight kilobytes a turn and terminated early with a plausible
prefix, which the agreement test caught only at window sizes above the link.

## The prefix that belongs to one route only — §7.4.3, and the fuzzed document that settled it

This round spent an afternoon with the two routes made to *agree* about §7.4.3's error, and it was
wrong. The reasoning was that

> Any other characters, and any character sequences that represent impossible combinations in the
> ASCII base-85 encoding, shall cause an error.

says nothing about the groups already decoded, that they are the producer's own bytes, and that
ADR 0343 keeps a truncated inflate's prefix for exactly that reason — so `ascii85` was changed to
salvage its prefix and the window's `Ascii85` to report the same `Damage::Corrupt`.

**`display_list_digest` over the corpus moved one document, and the document is the argument.**
`PDFBOX-3148-2-fuzzed.pdf` states its **cross-reference stream** as `/Filter [/ASCII85Decode]`,
with a byte outside `!`..=`u` eight bytes in. Refusing it sends `Parser` to its header scan, the
objects are found and the file's one page is interpreted. Handing back the eight bytes as a decode
makes them a cross-reference *section* with almost every entry missing, and the document loses its
only page **in silence**.

So the answer is not that one route is right: **it is ADR 0343's own distinction arriving by route
rather than by consumer**, and `doc/traps/parsers-and-streams.md` already states the test — *ask
what a prefix of the thing is before deciding whether to draw one*. A prefix of a table is not a
shorter table. A prefix of §7.8.2's "sequence of instructions" is a shorter sequence of the same
kind. And the two routes serve exactly those two populations: the buffered one is what every
consumer but one takes — cross-reference streams, font programs, image samples, ICC profiles — and
the window is only ever run over a content stream, by `Document::stream_source` and
`Document::nested_content_source` and nothing else. Each is right for its own population, and
`a_base85_error_keeps_the_groups_before_it` pins both halves.

## What binds, from principle 3

- **The bound removes the allocation rather than surviving it**, as it did for `LZWDecode` (ADR
  0429). A pumped chain has no allocation to bound and makes no `TooLarge` of its own; the
  aggregate bound is the reader's, §7.7.3.3's, applied by `content::reader` over the whole
  content, and what stops a bomb after that is `MAX_OPERATIONS` over the program it decodes to.
  The memory a chain costs is `LINK` bytes per stage whatever it names.
- **A refusal is refused by name.** Damage met inside a chain is reported where it is met, and the
  first stage's damage is the one kept — a stage fed a truncated prefix has no way to end well
  either — which is `chain_over`'s rule for the buffered route, held to by the pump because a
  stage that has finished closes its successor's input.
- **A pass that moves nothing terminates loudly.** It is unreachable by construction — a stage
  whose source is closed is given `last`, on which every engine here ends or reports damage — and
  saying so costs a branch and keeps a decoder defect from becoming an unkillable loop inside the
  reader's refill.
- `#![forbid(unsafe_code)]` holds; every error is `FilterRefusal`/`ContentIssue`/`StreamRefusal`.

## The measurement, and the thing ADR 0586 predicted that did not happen

`viewer-core/examples/find_cost` over twenty pages each drawing one form `XObject`, both arms built
in one sitting, `RAYON_NUM_THREADS=1`, peak from `VmHWM` in `/proc` sampled every 20 ms. The
witness is ADR 0586's, rebuilt from its generator to the byte — 4 174 537 and 12 523 517 encoded
bytes, the two sides of `DECODED_BUDGET`.

| twenty pages, one form apiece | before, peak | after, peak | before, sweep | after, sweep |
|---|---|---|---|---|
| **the control**: one `/FlateDecode`, 2 GiB of zeros | 13 920 KB | 14 272 KB | 32.82 s | 23.76 s |
| `[/ASCIIHexDecode /FlateDecode]`, **under** the budget | **1 070 828 KB** | **22 608 KB** | 154.98 µs | 14.62 s |
| `[/ASCIIHexDecode /FlateDecode]`, **over** it | **1 103 596 KB** | **55 016 KB** | 6.18 s | 14.12 s |

**The control row is why the wall clock is printed and not argued from.** A bomb of the same shape
that this tree *already* windowed costs seconds and fourteen megabytes in both arms, and its own
figure moved 13.91 → 32.82 → 23.76 s across three runs as neighbouring rounds came and went. The
peaks are deterministic to the kilobyte; the seconds are the machine's (load average 10–12, and
80–140 on a run that was thrown away).

**Ordinary documents pay nothing measurable**, by `callgrind` under `RAYON_NUM_THREADS=1`: ISO
32000-2 page 101 interpreted fifty times **+0.050%** (1 314 888 437 → 1 315 548 715), and the two
largest corpus documents stating `[/ASCII85Decode /FlateDecode]` **+0.0003%** (`S2.pdf`) and
**−0.016%** (`personwithdog.pdf`).

**And what ADR 0586 predicted did not happen, which is the finding worth more than the item.** It
wrote that a chain pump "would take this document to kilobytes on **every** read rather than on
every read after the first". Kilobytes of *memory*, yes — but the read itself is still the bomb's
whole decode, and the 25 000× ratio that ADR measured was between a refusal **remembered** and a
refusal **re-reached**. A window re-reaches it too. So the third row of the table above improves in
memory and worsens in seconds, and the second row — where ADR 0437's memo could remember the
refusal — goes from 154.98 µs to 14.62 s.

That is not a new pathology and the control row is the proof: **the same bomb without its armour
already cost the tree seconds and megabytes**, because `stream_source` has windowed a single
`FlateDecode` since ADR 0365 and a bomb of zeros decodes to white space, which `MAX_OPERATIONS`
never sees. What this round did was make the chained arrangement cost what the unwrapped one always
cost, in both dimensions at once, which is what "one function decides the route" is for.
`doc/todo/14`'s own ranking is the standing answer to which dimension to spend: time is what roads
A and C make interruptible, and memory is what none of them can take back.

**What is left is the memo's half rather than the road's**, and `doc/todo/14` carries it: a refusal
a *window* reaches is not remembered, where one the buffered route reaches is (ADR 0437). Closing
it is one fact travelling one hop — the reader knows it read `max_stream_len` decoded bytes out of
one stream, and that is the same `FilterRefusal::TooLarge` the buffered route would have recorded
under the same key.

## Correctness

- **`display_list_digest` over every corpus document's first page is byte-identical**: 975 lines,
  964 documents opened, 958 first pages interpreted, `sha256 769b86af…`, with the same
  `pdf-sandbox-worker` on disk for both arms. Nine corpus documents state
  `[/ASCII85Decode /FlateDecode]` and two `[/ASCII85Decode /LZWDecode]`, so unlike ADR 0429's
  `LZWDecode` this change is one the corpus **can** see.
- `every_pumpable_chain_agrees_with_the_whole_decode` drives nine arrangements — each filter
  alone, §7.4.1's EXAMPLE 2 and EXAMPLE 3, ADR 0586's witness, a compressing stage in front of an
  armouring one, and three stages so that a link feeds a link — through windows of 1, 3, 64, 4096
  and 100 000 bytes, against the buffered chain, on a payload that crosses a link many times.
- `a_bomb_behind_an_ascii_armour_costs_the_window`, `a_chain_reports_the_damage_the_whole_decode_does`,
  `a_raw_deflate_stream_behind_an_armour_still_falls_back`, `a_base85_error_keeps_the_groups_before_it`,
  and `nested_content_window.rs`'s `a_chained_form_is_read_through_the_window_and_draws_what_the_whole_decode_draws`,
  which asserts the *route* first because a route decision is invisible in its output (ADR 0427).

## Files

- `crates/pdf-syntax/src/filter.rs` — `Stage`, `stage`, `early_change`, `Pumping` as a chain;
  `Running`, `Ending`, `LINK`, `Turned`, `Standing`; `AsciiHex`, `Ascii85`, `Run`, `RunLength`;
  `Engine::turn`/`may_rewind`/`settle`; `Pump::pump` the driver; `Inflate::turn` and `Lzw::turn`
  in place of their `pump`s; `decoded_extent` driving one stage directly; the six new tests and
  their encoders.
- `crates/pdf-syntax/src/document.rs` — `pumping` walks the whole chain; `delimiting` asks
  `filter::stage` so Table 8's `/EarlyChange` is read in one place.
- `crates/pdf-syntax/src/lib.rs` — exports `Stage`.
- `crates/pdf-model/src/content/reader.rs` — `Nested::Windowed` carries a chain.
- `crates/pdf-model/tests/nested_content_window.rs` — `Coding::HexFlate` and the chained arm.
- `crates/pdf-model/examples/token_window_census.rs` — `Delimiting::Decoded` takes a `Stage`.
- `doc/conformance/ledger.toml`, `doc/todo/14`, `doc/todo/41`, `doc/todo/README.md`, and this
  round's history file.
