# 0955 — The converter had no coverage instrument, and one requirement in three had no answer

Session 954. Status: **accepted**. It **amends ADR 0951**'s account of `REFUSED_BY_NAME` — that
table is now the converter's *considered answers* rather than its arguments-only refusals — and it
adds the instrument ADR 0947's three stages were missing.

## The correction that started it

> remember that the corpus is just a nice feedback but not the target. The target is the complete
> spec.

Every slice of the `archive` verb until now ranked its work by **corpus weight**: how many veraPDF
documents a decision-table row unblocks. `CLAUDE.md`'s two-denominator table says what is wrong
with that as the *only* ranking — coverage asks how many of the standard's requirements are
answered and its denominator is the specification, robustness asks what share of the files that
exist convert and its denominator is the world. `tests/archive_corpus.rs` is an instrument for the
second question. **There was none for the first.**

The shape of the gap is worth stating precisely, because it is not that the converter was doing
badly. It is that nothing could say how it was doing at all: `pdf_archive::table::binding` knows
every requirement a target is held to, `decision.rs` knows what the converter does about some of
them, and **nothing had ever compared the two lists**.

## The instrument

`crates/pdf-transform/src/archive/census.rs` joins them. Every requirement a target binds gets one
of seven `Standing`s, and the seven are seven different facts rather than degrees of one:

| standing | what it means | whose it is |
|---|---|---|
| `Processor` | the rule binds a program, not a file | this project's, in the conformance ledger |
| `OutsideValidation` | the committee has put it outside a validator's remit | nobody's |
| `Unchecked` | the validator does not judge it | `pdf-archive`'s |
| `Remedy` | a row of `REMEDIES` | answered |
| `WriterEmits` | the serializer satisfies it by construction | answered |
| `Refused` | a refusal with a sentence of its own | answered |
| `Unconsidered` | in none of the three tables | **nobody has looked** |

Only the last is a gap, and it is the product. `cargo run -p pdf-transform --example
archive_census` prints the table and then that list in full, by identifier, clause and target.

**It found 107.** One requirement in three across the six targets reached `decide`'s catch-all and
was refused with a sentence that says only "this converter does not yet meet this requirement" —
true of every gap and informative about none. Not one of them had been looked at, because not one
of them was ever the top of a corpus ranking.

## What a considered answer is, and why most of these are refusals

The slice then worked the list. The answers available were the ones the verb already had, and
**choosing among them was the whole work**; nothing here builds a rewrite. Where the outcome is a
refusal it is *finished*, not deferred:

- **The fence (18 rows).** Bytes inside a content stream, an embedded font program held against its
  own encoding, a CMap that is the encoding, a blend mode name the standard states no fallback for.
  ADR 0816's fence closes these permanently and the sentence says why.
- **Another target conforms (11 rows).** ISO 19005-2's ten implementation limits, which part 4 does
  not state at all, and a stream whose data lives outside the file.
- **A rewrite named as owed (77 rows).** `NOT_BUILT_YET`, but never the generic sentence: each one
  names what it waits on — an authorisation word and an action-removal rewrite, a JP2 wrapper box
  nothing in this tree writes, `/Info` reconciled into the XMP packet, the vertical half of the
  metric restatement section 4.9 already does horizontally.

The one requirement that turned out to be answered all along is `file-structure/file-identifier`:
§14.4's pair of identifiers, which `pdf_syntax::serialize` writes into every file it creates. It
joined `WRITER_EMITS`, and it is the only row of the hundred and seven that changed what the
converter *does* rather than what it says. Three PDF/A-2b documents and two PDF/A-4 documents
convert because of it.

## Three readings the list forced, which a corpus ranking would never have reached

- **`/CIDToGIDMap` has no default in ISO 32000-2.** The obvious answer to a Type 2 CIDFont stating
  none was to write `Identity` — ISO 32000-1's own default — as a `Stated` rewrite. §9.7.4.2's
  Table 121 makes the entry *required* and states no default at all, so that rewrite would assert a
  mapping the base edition does not supply. The row says so and names what it waits on instead.
- **A blend mode is two cases, and the standard answers one.** For an **array** — deprecated in
  PDF 2.0 — §11.6.3's own entry says a reader takes the first mode it recognises or Normal if it
  recognises none, so reducing it is `doc/adr/0948`'s `Stated` class. For a bare **name** the
  standard says nothing, so choosing one is `doc/questions/A48`'s forbidden half and will stay
  refused. One requirement, two permanent halves.
- **An unrecognised rendering intent has a stated answer and a fenced one.** §8.6.5.8 says a
  processor that does not recognise the name uses `RelativeColorimetric`. In an `/RI` entry or an
  image's `/Intent` that is a rewrite waiting to be built; as the `ri` operator's operand it is
  inside a content stream, and the fence stands.

## The ratchet, and why it is equality rather than a ceiling

`crates/pdf-transform/tests/archive_unconsidered.txt` holds the list, and `tests/archive.rs`
compares it in **both** directions — `doc/todo/00`'s discipline. A requirement arriving with no
answer fails the build on arrival, which is the regression this exists to catch; one leaving has to
be struck from the file in the commit that answered it. The file is empty today and that is not
"finished": seventy-seven of the rows are refusals naming work nobody has done, and the census
example is the list to work from.

**Why not a ceiling.** A ceiling would let an answered requirement silently become unanswered as
long as the total held, which is exactly the substitution `doc/todo/00` was built to prevent.

## The cost, stated

The corpus numbers moved by five documents. That is the expected shape of coverage work and it is
worth saying plainly rather than apologising for: the requirements answered here were answered
because the *standard* states them, and most have no witness in the veraPDF corpus at all. A slice
that had chased the corpus instead would have produced a larger number and known less.
