# A flush is not a truncation

Status: **open** — diagnosed, measured, and the fix is designed but not taken.
Priority: 18 (a defect: a report that fires on a condition the clause does not state)
Corpus witnesses: `comments.pdf`, `highlights.pdf`, `issue3885.pdf` of the pdf.js corpus, and
about one crawled document in three thousand — the measurement is below.
Clauses: ISO 32000-2 §7.4.4.1 (which makes RFC 1950 and RFC 1951 normative), §7.3.8.2 and
Table 5's `/Length`, §7.4.1.
Code: `pdf-syntax/src/filter.rs` — `inflate`, `turn`, `Inflate::turn`, `Damage::Truncated`.

## What the corpus says

Three of the pdf.js corpus's sixty-seven incomplete documents report

> `DamagedContentStream { detail: "a form XObject /Form (§8.10)", damage: Truncated, kept: 851 }`

and **no mark is missing from any of the three**. `comments.pdf` holds three such form
`XObject`s — objects 667, 694 and 697, the last two being the 851-byte one the report above names
— and object 667 is the smallest, a form with `/Length 195` whose encoded bytes end

```
… 6d d4 0f 00 00 00 ff ff
```

Objects 694 and 697 end the same way. `00 00 00 ff ff` is RFC 1951 §3.2.4's *non-compressed*
block with `BFINAL` clear and `LEN` zero — what `zlib` writes for a `Z_SYNC_FLUSH`. The producer
flushed and never called `deflateEnd`,
so the stream carries no final block and no RFC 1950 `ADLER32`. Inflating it yields 648 bytes
ending in a newline after an `f` operator: the whole of the form's content, seven fills of a
highlight's quads, with nothing after it.

So the decode did reach "the original form" §7.4.1 asks for, and `Damage::Truncated`'s own doc
comment — "[t]he encoded data ran out before the filter's end-of-data marker" — is *true* and
*not what the report is for*. Trap 11, exactly: the report exists to keep a page cut short from
looking like a page meant to be sparse, and this page is not cut short.

## How wide it is

- **pdf.js, 974 documents, 25 408 streams**: three documents, seven streams. Every one of the
  three is on the corpus gate's incomplete list *for this and nothing else*, so the fix takes
  three documents off it.
- **The SafeDocs crawl, a 12 000-file sample**: four documents carry a stream ending in the
  marker — about 0.03%, which over the whole crawl is of the order of twenty.

The scan is five lines and worth re-running rather than trusting:

```python
import re; pat = re.compile(rb'\x00\x00\x00\xff\xff\s{0,2}endstream')
```

## Why the obvious test is not the test

"The last five bytes are the flush marker" is a *heuristic*: those five bytes could be the tail
of a Huffman block's bits, in which case the stream really did stop mid-block and data really is
missing. One stream in 2^40 by chance, which is small, but this project prefers a decidable test
to a small probability.

**The decidable test is to supply what is missing and see whether the decoder agrees.** A
deflate stream that ended on a *completed* block is one final block short of whole, so feeding
the live decoder RFC 1951's final empty non-compressed block —

```
01 00 00 ff ff        # BFINAL = 1, BTYPE = 00, LEN = 0, NLEN = 0xffff
```

— must make it report `StreamEnd` **and write no further output byte**. A decoder stopped inside
a block cannot reach `StreamEnd` from those forty bits without emitting something, because
anything it could still emit comes from bits that are not there; and if it reaches `StreamEnd`
emitting nothing, the only symbols it consumed were an end-of-block, which carries no data. Both
directions hold, and the test needs no new dependency and no arithmetic of our own.

## What blocks it, and it is one thing

**RFC 1950's checksum.** Under zlib framing the decoder wants four bytes of `ADLER32` after the
final block, so the probe above returns `Ok` rather than `StreamEnd` and cannot be read. Three
ways out, none free, and choosing between them is what this item is:

1. **Compute the Adler-32 as the decode goes** and append it to the probe. Exact, works in both
   the buffered route (`inflate`) and the windowed one (`Inflate::turn`, which is the route these
   three documents actually take, since ADR 0427 reads a form `XObject` through a window). Costs
   a per-byte checksum on the hot inflate path for every stream in every document — principle 2
   says measure that before believing it is nothing.
2. **Probe with a second, throwaway raw decoder** over the input minus the two-byte zlib header.
   Costs one extra whole inflate, but only on the damaged path and only where the tail marker is
   present, so the cost is confined to 0.03% of documents. Available in the buffered route
   immediately; the windowed route does not retain its input and would have to.
3. **Strip the zlib header and always inflate raw**, giving up the `ADLER32` check outright. The
   cheapest and the one that loses something real: a corrupt-but-grammatical stream would then
   decode in silence, which is trap 5 pointing the other way.

## What the answer should be, once the probe exists

`Decoded::whole`, not a third `Damage`. §7.4.1 asks a reader to "invoke the corresponding
decoding filter or filters to convert the information back to its original form", and this decode
did that; `Damage`'s own doc comment says it is for the case where the second half of that
sentence was not achieved. What is absent is a *declaration* that there is no more, and a
declaration carries no marks.

A round taking this owes: the probe, the calibration (a stream truncated **mid-block** must keep
its `Damage::Truncated` — build both with `flate2`'s `Z_SYNC_FLUSH` and a byte-slice), the three
corpus documents leaving `MAX_INCOMPLETE`, and whatever the oracle then says about pages it has
not been allowed to judge.

## What it is not

Not `doc/todo/14`'s streaming question and not `doc/todo/41`'s cache. Both of those are about how
much of a stream is in memory at once; this is about what the decoder *says* when the stream ends.
