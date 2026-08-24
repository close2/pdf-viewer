# 0586 — The refusal too big to remember, and why remembering it is the wrong fix

Status: accepted
Date: 2026-08-24
Session: 712

## Context

`doc/todo/41`'s other remaining line: **a refusal whose *encoded* bytes do not fit the budget is
re-run on every read.** `DecodedStreams::keep` declines any entry whose `charge()` exceeds
`DECODED_BUDGET`, and a refusal's charge is `size_of::<DecodedEntry>()` plus the encoded bytes it
pins — it holds no decoded ones. So a stream above about four mebibytes of *encoded* data is one
whose answer the cache will not keep, however expensive that answer was to reach. The todo file set
the price of taking it: "Whoever wants it owes a document that reaches it and a reason the budget
should hold what it holds."

## The document that reaches it

It is one hex digit away from ADR 0437's own witness, which is the part worth knowing. That ADR's
bomb is a `[/ASCIIHexDecode /FlateDecode]` form `XObject` drawn from twenty pages — a chain, so
`Document::pumping` declines it a window and it is read whole. `ASCIIHexDecode` costs its author two
bytes per one, so the *encoded* size is twice the deflate stream's, and a bomb large enough to
command the gibibyte `Limits::max_stream_len` allows is already within a factor of two of the
budget. Padding it is free.

Two documents were built, identical but for how many gibibytes of zeros the deflate stream carries,
so that the only difference between them is which side of `DECODED_BUDGET` the encoded bytes fall
on. Twenty pages apiece, `viewer-core/examples/find_cost`, one sitting, on a machine at a load
average of 21–27 — which is stated because it is bad, and because the ratio below is four orders of
magnitude wider than anything load can explain:

| the same bomb, in twenty pages | encoded | one cold sweep |
|---|---|---|
| under the budget | 4 174 537 B | **257–279 µs** |
| over the budget | 12 523 517 B | **6.93–6.98 s** |

**A factor of about 25 000, bought with padding.** The generator is in this session's history file.
So the hole is real, it is reachable, and a file that wants it gets it for nothing.

## Decision

**Not taken, and not because it is small.** The construction `doc/todo/41` asks for — hold the
refusal anyway — cannot be built without giving up something this cache's whole shape rests on, and
the three ways out were each followed to where they break:

- **Charge a refusal `size_of::<DecodedEntry>()` alone and pin its bytes uncounted.** The entries
  are then bounded in *number* (the budget over the entry size, about forty thousand) and unbounded
  in *bytes*, because each pins an arbitrary buffer. For a stream reached through the
  cross-reference table the pin genuinely costs nothing — `Document::cache` is a `BTreeMap` that is
  cleared only once, at §7.6 authentication, so the `Stream` and its `data` outlive any entry naming
  them — but §8.9.7's inline image is built at every `BI` and is not in that map, so a hostile page
  can pin without a ceiling. A cache whose whole argument is that it has one may not lose it here.
- **Let the budget be exceeded by one such entry.** The ceiling survives, stated as "the budget plus
  the largest single stream" — but the oversized entry is inserted as the *most* recently used, so
  every ordinary entry is evicted around it and the cache stops being a cache for as long as the
  bomb is on the page. That is worse than the cost it removes.
- **Drop the pin and key the refusal on a digest of its encoded bytes.** Sound only up to a
  collision, and the consequence of one is a `TooLarge` returned for a stream this reader can
  decode — content dropped in silence, which is trap 5 in the place with the most plausible excuse.
  `image::StreamIdentity::Content` uses a digest and gets away with it *because the content is
  compared exactly beside it*; here the thing to compare against is the buffer, and holding it is
  the pin again. It also costs a hash of the whole encoded stream on every read: 113× better than
  the inflation and not the answer.

**And the cost it would remove is removed better one clause over.** What makes this bomb expensive
is not that its refusal is forgotten but that reaching the refusal inflates a gibibyte. `doc/todo/14`
is the item for that: `filter::Pump` windows a *single* `FlateDecode` or `LZWDecode`, and this bomb
escapes it only by wearing a second filter. §7.4.2's `ASCIIHexDecode` "produces one byte per two",
so it cannot inflate at all and is the easiest stage in §7.4 to window; a pump that accepts a chain
whose every stage is pumpable would take this document to kilobytes on **every** read rather than on
every read after the first, and would need no entry, no charge and no eviction argument. A memo that
remembers an answer is a poorer fix than not spending the gibibyte.

So: the line stays open in `doc/todo/41`, pointed at `doc/todo/14`, with the witness and the number
attached so that no later round has to rediscover either.

## Consequences

- **The hole is documented rather than fixed**, which is `CLAUDE.md` principle 1's "documented as a
  deliberate decision with its cost written down". The cost is the table above.
- **`doc/todo/14` gains a reason it did not have.** Its chain-pump was priced as a convenience for
  §7.4.7's worked arrangement; it is also the fix for a 25 000× amplification, and that ranks it
  differently.
- **The re-derivation this ADR is made of is the one `doc/habits.md` asks for**, and it went the
  other way from the six prices that collapsed in this block: the item was not cheaper than it
  looked, it was *mis-shaped*. A price can be wrong by naming the wrong place (ADR 0469's) or by
  naming the wrong mechanism, and this is the second.
