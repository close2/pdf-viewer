# ADR 0730 — What `incomplete` is made of

Status: accepted, 2026-08-28. Session 796. Amends `pdf-model/tests/corpus.rs`'s
`MAX_INCOMPLETE` doc comment and the refusal `pdf-font` raises for a composite font it cannot
address a substitute for. Successor in method to ADR 0281 (a fact that can be counted is not
written down) and to ADR 0433 (which read one of these mechanisms by hand).

## The subject

The corpus gate's summary line has ended with `N incomplete` since the gate existed, and the
population behind that number had never been read *as a population*. The composition lived in a
hand-kept table in the doc comment above `MAX_INCOMPLETE`, and by this session that comment
carried **three different figures for one quantity in one file**: a headline, a table whose own
rows summed to something else, and the ratchet constant below them — none of them what the gate
printed. The table had been corrected twice before, each time with the sentence "recomputed every
session because a number nothing recomputes is a number that drifts" written beside it. A promise
is not an instrument.

## What was read

Every document on the list, with its reports, against `Unsupported`'s own doc comments and the
clauses they cite. The reading is not "many mechanisms" or "one" — it is **many mechanisms
dominated by one class**, and the class is the interesting axis:

- **The largest single mechanism is a sentence the *file* breaks.** Eighteen documents state
  `/Encoding /Identity-H` over a descendant with no embedded program, which §9.7.5.2 forbids
  outright — "The Identity-H and Identity-V CMaps shall not be used with a non-embedded font" —
  and then state no `/ToUnicode` either, so §9.10.2's three methods are exhausted. ADR 0433 read
  eleven of them off the ink sweep by hand and concluded the same thing; what is new is that the
  *refusal now says so*, so the reading no longer has to be repeated by whoever meets the message
  next.
- **The great majority of the population is the file's own defect.** Malformed content streams,
  names no resource dictionary defines, a `/MediaBox` no ancestor states, an image dictionary its
  own codestream contradicts, an annotation with neither an `/AP` nor the entries its subtype's
  clause needs.
- **A small remainder is closed by a reading or by a decision**: a subset with no glyph for any
  code its own document shows (ADR 0270), a bound this program set and said so, a transparency
  model this tree departs from where §11.4 and §11.6.2 let the two differ.
- **A very small remainder is work owed**, and those are the rows a round takes from.

The exact figures are the gate's to print and are deliberately not written here.

## The decision, in three parts

### 1. The classification is code, and the gate prints it

`whose_defect` places every report under a *mechanism* and one of three classes —
`TheFile`, `NeitherOne`, `ThisReader` — chosen so that the boundary is **who has to do
something**. `print_the_composition` prints, per class, the documents that carry a mechanism and
the documents the mechanism *decides*; the second is a partition, taking each document's
most-owed mechanism, so it sums to the population and never understates the debt.

The `match` on `Unsupported` is exhaustive, so a variant added to the enum is a compile error
here. Three variants carry a string flattened out of a lower layer's typed error — `Font`,
`Image`, `Annotation` — and those are read back with markers.

### 2. A report the table cannot place stops the gate

There is no `other` row, and that is the whole design. An `other` row is what let the old table
drift: a mechanism nobody had classified was counted as though it had been. So an unplaced report
fails the corpus gate, naming itself, and the price — a reworded message costs the round that
rewords it one table row — is paid deliberately.

**Writing the table is itself a reading, and one row was wrong until the clause was opened.** A
JBIG2 refusal looks like a codec gap and was placed as one; §7.4.7 says "[t]he JBIG2 file header,
end-of-page segments, and end-of-file segment shall not be present" in an embedded stream, and the
corpus's one witness is `jbig2_file_header.pdf`, named for carrying the header it may not carry —
so a segment header the decoder calls *unknown or reserved* is one ISO/IEC 14492 does not define,
and the file states it. The row moved to `TheFile`, and the general lesson is that a class is a
claim about a clause: it is worth as much as the reading behind it and no more.

**Trap 13 earned its place twice over here.** Calibrating the assertion by breaking a marker on
purpose left the gate **green**, because the first draft of the `Font` arm ended in a broad
`"cannot be substituted"` row that swallowed the §9.7.5.2 case above it: eighteen documents would
have been silently reclassified by any rewording. *A marker table with a catch-all in it is not a
table*, and the calibration is the only thing that could have said so — the classification's own
output looked entirely correct.

### 3. The refusal that conflated four facts now says which

`pdf-font`'s composite-font path raised one message — "neither a `/ToUnicode` nor a registered
character collection" — for four different facts about a file, and only the last of them is work
owed:

a) the combination §9.7.5.2 forbids (the file's);
b) a descendant with no readable `/CIDSystemInfo`, which Table 115 makes required (the file's);
c) an `Identity` ordering, whose CIDs are §9.7.3's glyph order of a program nobody supplied, so
   no table could exist (neither one's);
d) a character collection this binary carries no table for — the only gap, and §9.7.5.2 requires
   four collections, all of which `predefined`'s own test asserts are carried.

`collection_gap` says which, and the refusal changes from `FontError::UnsupportedEncoding` to
`FontError::NoSubstitute`, because `Identity-H` is read perfectly well — `composite_cmap` built
the `CMap` out of it — and what failed is reaching a substitute through it. That is the case
`NoSubstitute`'s own doc comment describes and the variant the sibling refusal thirty lines above
already uses.

Only (a) occurs in the pdf.js corpus, on all eighteen documents. Whether (d) occurs in the crawl
is a measurement nobody has taken, and it is the one of the four that would be a round's work.

## What this costs and what it does not

No pixel moves. The eighteen documents draw exactly what they drew, report exactly as often, and
stay on the incomplete list — what changed is that the message names the clause the file broke
rather than a method it exhausted. The gate's ratchets are untouched.

## What was found and not taken

`doc/todo/18` — three of the sixty-seven are reported damaged and are not: their form `XObject`
is a `zlib` stream flushed with `Z_SYNC_FLUSH` and never finished, so every byte the encoder
produced is decoded and only RFC 1950's trailer is absent. The file has the measurement (three
of 974 here, about one crawled document in three thousand), the decidable test — feed the decoder
RFC 1951's final empty block and require `StreamEnd` with no further output — and the one thing
that blocks it, which is that zlib framing then wants an `ADLER32` this tree does not compute.

## What would change the answer

A round that adds a report and finds this gate red has met the design working. A round that finds
a class boundary wrong should argue it here rather than adding a row that hides the disagreement.
