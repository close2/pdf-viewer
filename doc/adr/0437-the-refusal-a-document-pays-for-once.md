# 0437 — The refusal a document pays for once

Status: accepted
Date: 2026-08-19
Session: 602

## Context

`doc/todo/41`'s last open line was one sentence: **a refusal is not memoised.** ADR 0317 built the
decoded-stream memo and derived its budget; what it left out was the arm where the decode does not
produce bytes. `FilterRefusal::TooLarge` costs up to `Limits::max_stream_len` — a gibibyte — of
inflation to reach, and the answer was thrown away every time, so a document naming one bomb stream
from every page paid that gibibyte per page. The todo left it out for a stated reason: a refusal
holds no decoded bytes, so charging it to a *byte* budget needs a per-entry overhead, "and this
project does not invent constants."

**Three sessions had moved the ground under the item and the round checked before building.**
Sessions 592, 594 and 595 gave all five of §7.8.2's content streams a window (ADRs 0427, 0429,
0430), and `Document::pumping` decides who gets one: a *single* `FlateDecode` or `LZWDecode` with no
predictor. So the population this item was written about has genuinely shrunk — a bomb in a page's
`/Contents`, in a form, in a pattern cell is pumped now and costs kilobytes. What it has not done is
disappear, and the reason is that `pumping` is a *route* decision a file can decline:

- **any chain of two filters** — `[/ASCIIHexDecode /FlateDecode]` is §7.4.7's own worked
  arrangement, and hex-wrapping a bomb costs its author two bytes per one;
- **any predictor**, which is `/DecodeParms << /Predictor 12 >>` beside the same bomb;
- **everything that is not a content stream at all**: a font program, an `ICCBased` profile, an
  embedded file, a cross-reference stream. Those are read whole by construction, from every page
  that names them.

So the amplification is now reached by a file that *chooses* to defeat the window, which is a worse
shape than the one the item was opened about rather than a better one.

## Decision

**A refusal is an outcome, and the memo keeps it beside the bytes.** `DecodedEntry` holds an
`Outcome` — `Decoded { data, damage }` or `Refused { why, under }` — rather than a decoded buffer,
and `DecodedStreams::refuse` is the entry point `Document::decoded_under` takes when a filter says
no. One map, one eviction rule, one liveness invariant: the `Arc` pin that makes an address a key
(ADR 0317) is what makes a refusal addressable too, and nothing about it is new.

**The bound the refusal was reached under is part of the entry, and it is what keeps the memo
honest.** `decoded_under` is asked under two different limits: the document's, and the smaller
allowance `nested_content_source` computes so that a decode the memo would decline is windowed
instead. `TooLarge { limit }` means "longer than *this* limit" and says nothing about a larger one,
so an entry refused under the allowance may not be served to a caller asking under the document's
own bound — that would refuse a stream this reader can decode, on the strength of a routing decision
somewhere else. `DecodedStreams::get` takes the asking bound and serves a refusal only where
`bound <= under`. The other two refusals (`Unsupported`, `Corrupt`) are properties of the bytes and
the chain alone and are kept under the same rule rather than a second one.

**The per-entry overhead is `size_of::<DecodedEntry>()`, which is derived rather than invented, and
it is charged to every entry rather than only to refusals.** `DecodedEntry::charge` is three terms,
each something the entry holds: itself, the encoded bytes it pins, and the decoded bytes where there
are any. That is what the todo said was owed — a refusal charged nothing would let refusals
accumulate without a ceiling, and this cache's whole shape is that it has one — and making it
uniform removes the asymmetry rather than defending it. What the charge still does not count is heap
the entry *reaches*: the chain's names and parameter dictionaries, and the filter name inside a
`StreamRefusal::Filter`. Those were uncounted before and are uncounted now; ADR 0317's rule is that
the budget be legible rather than exact.

The cost of the uniform charge is about a hundred bytes an entry out of 4 MiB, and it moves
`allowance()` by the same amount — which is the routing threshold `nested_content_source` asks for,
so it is stated here rather than left to be discovered.

## The measurement

The witness is the one the todo said was owed: **Bomb B's shape — a deflate run of zeros inflating
past the gibibyte — inside a form `XObject` that all twenty pages draw, under
`[/ASCIIHexDecode /FlateDecode]` so that no window can take it.** 2.5 MB of file. The instrument is
`viewer-core/examples/find_cost`, a document-wide search for a needle the document does not contain,
which interprets every page once; the arms alternate B A B A B A with the patch applied and reversed
in one sitting, per `doc/habits.md`.

| | cold sweep, 20 pages |
|---|---|
| **without the refusal memo** | 5.92 s / 6.12 s / 5.92 s — 282 to 291 ms a page |
| **with it** | **2.76 ms / 3.23 ms / 2.89 ms** |

The bomb is inflated once for the document instead of once for each page that names it, and what is
left is the one inflation the file is entitled to. On an ordinary document — `ISO-14289-1`, 25 pages,
no refusal in it — the two arms are 47–51 ms against 36–55 ms: inside the noise of a shared machine,
which is what a change that adds one `match` arm and a hundred bytes an entry should be.

**Latency, not throughput** (principle 2): the population this improves is the *first* reading of
each page, which is the launch path and the page-turn path, not a background sweep.

## Consequences

- A file that names a refused stream from many pages costs one refusal. The memory high-water is
  unchanged — one inflation to the bound at a time, as before — and it is the repetition that goes.
- **A bomb whose encoded bytes do not fit the budget still costs its inflation per read**, because
  the entry pins them and `keep` declines what it cannot hold. At `DECODED_BUDGET` that is a stream
  above 4 MiB of *encoded* data, which for `FlateDecode` is a bomb far larger than it needs to be;
  `doc/todo/41` carries what that leaves.
- `image_stream` decodes outside this memo and is unaffected — an image bomb is refused per read
  still. It has no memo at all, which is a separate item and not this one.
- Two tests pin the pair of claims: a stream refused under `max_stream_len` answers from the memo the
  second time, and a refusal reached under a tighter bound is *not* an answer under a looser one.
