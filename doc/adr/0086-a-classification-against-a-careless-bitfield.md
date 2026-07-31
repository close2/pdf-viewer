# ADR 0086 — A classification against a careless bitfield

Status: accepted, 2026-07-31.

## Context

§9.8.3's two substitution hints were the **oldest silences in the ledger** — older than every row
this run of sessions has closed — and the reason they survived is in their own row: "the oldest
silence in the ledger and the one no gate can score". A better substitute changes pixels, but
only on pages where the references substitute differently too, so the oracle cannot rank it.

## Decision

**§9.8.3.2's `/Style` `/Panose` is read and used.** `pdf-font/src/panose.rs` reads the twelve
bytes the clause states — two of OS/2 family class, ten of PANOSE — and answers the four
questions this program can act on: family type, serifs, monospacing, weight.

Where the classification and Table 121's `/Flags` disagree, **the classification wins**, and it
is consulted after the font's *name* and before the flags. The order is an argument rather than a
preference: a name is what the producer called the font, a PANOSE number is a classification of
the face, and `/Flags` is a bitfield `substitute.rs` has always described as one "many producers
set carelessly".

**Serifs decide before proportion**, which is a documented choice the clause does not make. Two
reasons, and the second is this crate's own architecture: a monospaced face standing in for a
serifed design changes the shape of every glyph, and proportion matters least here because
advances come from `/Widths` or `/W` whenever the document states them.

**§9.8.3.3's `/FD` is read and applied to nothing**, with the boundary stated in the clause rather
than chosen here — see below.

## What the corpus says, measured by removing the entry

The question a hint has to answer is not whether it can be read but whether it changes anything.
`tests/panose.rs` derives every font's substitution request twice — once from the descriptor as
written, once from a copy with `/Style` removed — which `Dictionary::remove` made possible two
sessions ago for an unrelated clause.

- 46 fonts reach a descriptor with a `/Style`; 44 of the values are twelve bytes and two are a
  producer's own length.
- 28 of them embed the font program, and a font that carries its own program never asks.
- **Four requests change, in two documents**, and they are the case the entry exists for.
  `noembed-eucjp.pdf` and `noembed-sjis.pdf` embed nothing and name **MS-Gothic** — a Japanese
  *sans-serif* face — while their `/Flags` claim serif **and** fixed pitch **and** both Symbolic
  and Nonsymbolic at once. The PANOSE number says Latin Text, Normal Sans, Medium, Monospaced,
  and the font's own name agrees with it. The flags were careless; the classification was not.

Neither gate moves, and neither could: both documents need Table 116's predefined `CMap`s before
anything on their pages can be judged.

## The clause that forbids what it recommends

§9.8.3.3 says a glyph-class descriptor "shall contain entries for metric information only" and
shall not include the `/FontFile` entries "or any of the entries listed in" Table 120. **Every
metric a font descriptor can state is in Table 120** — the ascent, the descent, the stem widths,
the missing width — so read literally the two halves of that sentence cannot both hold for a
descriptor that states anything at all.

The corpus's single witness settles what a producer does about it. `issue13147.pdf` writes
`/FD << /Proportional … >>` — the class the clause itself recommends, "at least the metrics for
the proportional Latin glyphs" — and that descriptor holds `/Ascent`, `/Descent`, `/CapHeight`,
`/XHeight`, `/StemV`, `/StemH`, `/Flags`, `/FontBBox`, `/ItalicAngle` and `/FontName`, all ten of
them Table 120's, because there is nowhere else to put metrics.

Nothing here enforces the restriction, and the test is the record of why. This is the third
internal contradiction this run of sessions has found — after Table 31 and §12.4.3 disagreeing
about a bead array's order, and Table 265's normalised rectangle that its own witness violates —
and the shape is now familiar enough to state: **when a reader cannot satisfy a sentence, check
whether the standard can.**

## Why `/FD` stops at reading

The class names "depend on the character collection, as identified by the Registry , Ordering ,
and Supplement entries in the CIDSystemInfo dictionary". Knowing which *CIDs* a class holds means
having the collection, which is registered data published outside this standard — the same
boundary Table 116's predefined `CMap`s sit behind, and the same licensing decision. And what the
entry is *for* is building a substitute font, which this tree does not do: it selects an installed
face (ADR 0007).

## Consequences

- `silent` falls 64 → **62**, and **every remaining silence is in clause 12**. Clauses 7, 8, 9,
  10, 11, 13 and 14 have none.
- The substitution path now has three sources ranked with an argument for the ranking: name,
  classification, flags.
- No gate moves, and the ledger row says so — which is the honest form of a change no gate can
  score.
