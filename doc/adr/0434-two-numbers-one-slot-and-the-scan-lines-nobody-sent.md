# 0434 — Two numbers in one slot, and the scan lines nobody sent

Status: accepted
Date: 2026-08-19
Session: 599

## Context

`doc/todo/53` carried three residues from the five-hundred-and-fifty-seventh session's reading of
hayro's tracker (ADR 0392), each diagnosed and each deliberately not fixed. This round took the
first of the three, which is the one whose shape is a correctness hazard rather than a missing
diagnostic: **`pdf_sandbox::CcittParameters` carried one `u32` that did two different jobs.**

That field was the bound handed to `hayro_ccitt::DecodeSettings` *and* the height `PackedRows`
pads to and `finish` checks. In every ordinary file the two quantities are equal, which is why one
slot went unnoticed: ISO 32000-2 §7.4.6 Table 11 says of `/EndOfBlock`

> A flag indicating whether the filter shall expect the encoded data to be terminated by an
> end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when it
> has decoded the number of lines indicated by Rows or when its data has been exhausted,
> whichever occurs first.

and its default is true, so `/Rows` usually does not bind at all and the decode is bounded by
§8.9.5.1's `/Height` — which is what `pdf_model::ccitt_rows` derives.

**The exception is the case the clause is actually about.** With `/EndOfBlock` false and a `/Rows`
below `/Height`, the filter is *told* to stop short of the image. The raster is then genuinely
shorter than the grid the image dictionary describes, and the one field could not express both:
padding to `/Height` needs the height, and stopping at `/Rows` needs the bound. So the short
raster came back short and `decode_ccitt`'s height check refused the whole picture — for being
exactly the size Table 11 asked for.

This is the same shape as trap 5's `/Length` instance: a parser holding one slot for two
quantities takes whichever answer arrives and cannot tell that it is the wrong one.

## Decision

**1. The pipe carries both numbers.** `CcittParameters` gains `height` beside `rows`, and the two
are documented as what they are: `rows` is the decode bound Table 11 resolves through
`/EndOfBlock`, `height` is §8.9.5.1's extent. `CCITT_PARAMETERS_LEN` goes from 13 bytes to 17, and
`encode`/`decode` carry the field like every other. The worker's budget check takes the larger of
the two, because either can be: a document may state more scan lines than its `/Height` as easily
as fewer, and the decoder writes what it decodes.

**2. The undelivered scan lines are blank, and that is the same choice already made.**
`PackedRows::pad_to_height` already fills the lines a data-exhausted stream never delivered, and
its doc comment already records that the standard states nothing about them. The `/Rows`-bounded
case ends in the same place for a different reason, and the clause itself puts the two side by
side — "the number of lines indicated by Rows **or** when its data has been exhausted".

**3. It is drawn *and* reported, which makes a twelfth place where this tree does both.** Trap 5's
test for adding one is that suppressing either statement loses information, and it is met in both
directions:

- Suppressing the drawing throws away the scan lines the producer *did* send, which is what the
  refusal did.
- Suppressing the report makes a page whose lower half this program invented indistinguishable
  from one whose lower half the producer left white. Nothing in ISO 32000-2 says what those lines
  contain; blank is ours.

**4. The report's condition is Table 11's three parts and nothing wider** (trap 11).
`image::ccitt_bound_below_its_height` fires only where the codec is `CCITTFaxDecode`/`CCF`, the
`/DecodeParms` state `/EndOfBlock false` explicitly, and `/Rows` is both non-zero and below
`/Height`. Each exclusion is the clause's own: an absent or true `/EndOfBlock` overrides `/Rows`
outright; a `/Rows` of zero is Table 11's "not predetermined" rather than a short one; a `/Rows`
reaching `/Height` substitutes nothing. It is asked of the *dictionary* rather than of the decode,
which is what keeps a raster answered from `RasterCache` saying what a fresh decode says.

**Its cost in gated pages is zero, measured rather than assumed, and the measurement is the
argument for the narrow condition.** Exactly one of the 974 corpus documents writes
`/EndOfBlock false` at all — `ccitt_EndOfBlock_false.pdf`, whose `/DecodeParms` say
`/Columns 81 /Rows 26 /K -1 /EndOfBlock false` against a `/Height` of 26. Its `/Rows` reaches its
height, so the filter substitutes nothing and there is nothing to report. **A report keyed on the
entry rather than on the shortfall would have taken that page off the oracle's judged set for
nothing**, which is trap 11's failure mode in its four recorded instances. The corpus gate's
ratchets held on the run after the change, which is the same fact from the other side: no page
gained a report.

**5. The witness is built, because the corpus has none** (trap 8). `crates/pdf-model/tests/ccitt_bound.rs`
is a *pair* of hand-written one-page documents differing in one entry's value — `/EndOfBlock true`
against `/EndOfBlock false`, with the same `/Rows 2`, the same `/Height 4` and the same seven
bytes — following `pdf-syntax/tests/cross_references.rs`'s construction. Under true the number 2
has no power and all four scan lines are black; under false the filter stops at two and the other
two are white and named. A third fixture holds `/EndOfBlock false` with `/Rows 4` and asserts
silence, which is the trap 11 half.

The encoded data is written out in the test rather than borrowed: four lines of eight black
pixels, Group 3 one-dimensional, `00110101` for a white run of zero and `000101` for a black run
of eight. §7.4.6 defers the coding to ITU-T T.4 and T.6 outright, so this is the one part of the
fixture no clause of ISO 32000-2 could supply.

## Consequences

- `doc/todo/53` loses its first residue. The other two stand with their conditions unchanged: `5f`
  still needs a rule separating it from `12pt`'s deliberate leniency, and the Type 1 encoding is
  still a `read-fonts` API question.
- §7.4.6's ledger row records the case as drawn and reported, and stays `partial` for
  `/DamagedRowsBeforeError`, which is untouched.
- The wire format changed length. Nothing persists it — both ends are built from one workspace and
  the worker is spawned, not installed — but trap 10 is exactly why the round built
  `pdf-sandbox-worker` explicitly rather than trusting `cargo test -p pdf-model`: a stale worker
  would have read 17 bytes as a malformed 13 and refused every CCITT image, which is at least
  loud. A stale worker that *silently* misread would not have been.
