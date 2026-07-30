# ADR 0041 — Five widths and two spellings

Status: accepted, 2026-07-30.

## Context

Two items, from the two tracks, and the demand one turned out to be a door into the spec one.

**Demand.** §8.9.5.1's Table 87 permits five component widths and this tree's unpacker read
two: "2, 4 and 16 are legal and do occur. Refusing them is honest; guessing would shift every
sample." Three corpus documents were waiting on it, and the item had sat on the
not-implemented list at "Small" for a dozen sessions.

**Spec.** §7.8, "Content streams and resources", was four `unreviewed` rows and one of the
families the handover named as available.

## What the widths cost, and what they found

The arithmetic was never the problem: `Decode`'s tables are built with `2^bits` entries for
any depth up to sixteen, so §8.9.5.2's map already covered every sample any of the five can
carry. What was missing was the *packing* — one function, `raw_sample`, reading a component at
1, 2, 4, 8 or 16 bits, most significant first, from a row that restarts on a byte boundary.
Two things had to move with it, both about exactness rather than about depth:

- The per-sample conversion memo keys on eight bits per component and up to four components,
  which is exactly the 64 bits available. At sixteen it cannot hold the samples, so the caller
  now supplies no cache rather than a lossy one.
- The one-component palette is `2^bits` conversions, which at sixteen is more work than any
  small image. It is built only up to eight bits and the exact per-sample path takes the rest.

**Four corpus documents left the incomplete row rather than three**, and the fourth is the
find. `issue14256.pdf` is a SafeDocs conformance file — "Inline Image Test for abbreviated and
full key names" — whose page carries **the same picture eight times**, written eight ways. One
of the eight is 4-bit, which is why the whole page had been unreadable. With it readable, five
of the eight were visibly wrong, and `mupdf`, `ghostscript` and `hayro` drew all eight alike
while we and `poppler` did not.

### The abbreviation wins, and that is a choice

§8.9.7 says only that "the abbreviations shown in Table 91 … and Table 92 … may be used in
place of the full names". The two spellings are one entry, so a file writing both with
different values has written the same key twice — and §7.3.7 says "multiple entries in the same
dictionary shall not have the same key" and states no recovery. **The standard is silent, and
this is therefore a decision.**

It is decided by that file's *bytes* rather than by its comments, which is the difference
between evidence and an opinion:

- `#4` writes `/F [/AHx] /Filter [/A85]` over data that is plainly ASCII hex. Under the full
  name the stream does not decode to an image at all.
- `#8` writes `/DP [null << /Predictor 15 … >>] /DecodeParms [null null]` over a Flate stream
  that *was* PNG-predicted. Under the full name every row is off by its predictor.

So the only rule under which that file decodes at all is "the abbreviation wins", and the three
cases it cannot settle — a colour space, a `/Decode` array, an `/Interpolate` flag — follow it
for consistency rather than being decided separately. The alternative, which this crate did
before and `poppler` still does, is "the later spelling wins", and is exactly as defensible
from the clause: not at all.

A second rule sits beside it and is *not* a choice: a key written twice **the same way** keeps
the first, which is what `pdf-syntax`'s parser does for every other dictionary in a file.

## What reading §7.8 found

**`BX` and `EX` were matched and ignored for thirty-one sessions, and they are a rule.** §7.8.2
says an unrecognised operator is ordinarily an error, and that inside a compatibility section
"unrecognised operators (along with their operands) shall be ignored without error". This tree
reported them everywhere. They are now a nesting *depth*, because the clause says the pair "may
be nested", and this is the one place in the interpreter where unsupported input is
deliberately silent — the file has said in advance that ignoring it is the appropriate thing to
do, and nowhere else does.

**No corpus document exercises it.** All nine that report an unrecognised operator report one
no producer meant — `toString`, `undefined`, `inf`, and the byte soup a fuzzed stream lexes as
an operator name. A corpus cannot test a compatibility mechanism: the mechanism exists for
operators newer than the reader, and the corpus is older than this code. `tests/content_streams.rs`
is what defends it.

Two of §7.8's rules turned out to be satisfied by construction, which is worth recording
because neither has a line of code enforcing it. "Indirect objects and object references shall
not be permitted at all" as operands holds because the content lexer has no reference token, so
a `1 0 R` in a content stream lexes as two integers and an unrecognised operator. And the
fourth bullet of §7.8.3 — that a form or Type 3 font omitting `/Resources` inherits the page's
— is why every resource lookup already falls back to the enclosing dictionary.

## Decision

- **`raw_sample` reads all five of Table 87's depths**, and a depth Table 87 does not name is
  still refused rather than rounded to one that is.
- **In an inline image, an abbreviated key beats the full name; an identical key beats
  itself first-come.** Argued on `expand_key`.
- **`BX` … `EX` is a compatibility section**, kept as a depth.
- **§7.8 and §7.3.7 are reviewed** — five rows, four of them `unreviewed`.

## Consequences

| | before | after |
|---|---|---|
| corpus documents drawing with nothing reported | 843 | **847** |
| pages we claim to draw completely | 1640 | **1644** |
| agreeing with the reference consensus | 799 | **801** |
| contradicted by it | 88 | **88** |
| ledger subclauses nobody has read | 403 | **399** |
| tests | 569 | **583** |

Twelve of the fourteen new tests are synthetic, and the reason is the same in both files: the
corpus holds three images at a depth other than 1 or 8, all of them 4-bit `DeviceRGB` with no
partial byte at the end of a row — so a test written against them would pass with the padding
rule missing — and it holds no compatibility section at all. The other two are the strongest
kind this project has: `issue14256.pdf`'s eight images **must agree with each other**, which
needs no reference renderer and is false under any spelling rule but the one adopted.
