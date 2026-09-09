# 0932 — The codes a Type 1 program never encoded

Session 941. Status: **accepted**. `doc/todo/53` item 2, recorded without a witness in the
five-hundred-and-fifty-seventh session and left deliberately unfixed because the fix looked larger
than the finding. It was not; the finding was larger than the record.

## What was wrong

`read_fonts::ps::type1` builds a program's custom `/Encoding` as a vector **pre-filled with
`GlyphId::NOTDEF`** and then fills in the entries the array names — `map.resize(256,
GlyphId::NOTDEF)` in `read_encoding`, at `read-fonts` 0.42.1, the version this workspace pins.
`Encoding::map` returns that slot verbatim, so a code the array never mentions comes back as
`Some(0)`: *encoded, to `.notdef`*. `Encoding::glyph_name` resolves the same slot and answers with
glyph 0's name, so it cannot tell them apart either. There is no third accessor.

The CFF reader beside it produces a genuine `None` for an unassigned code, so **the two producers
of `NameKeyed` disagreed about what "unencoded" means** — while `name_keyed.rs`'s own module
comment says they produce one shape, on ISO 32000-2 §9.6.2.1 NOTE 1's ground that a CFF is "an
alternative, more compact but functionally equivalent representation of a Type 1 font program".

Three things read the difference:

- an unassigned code selected glyph 0 and drew the designer's `.notdef` — commonly a filled box —
  where the program says nothing should be drawn;
- the whitespace departure in `loading.rs` saw the code as encoded, so a code meaning a space
  could deposit that box;
- `name_keyed.rs`'s "no character code maps to a glyph" refusal could never fire for a Type 1
  program carrying a custom encoding array, because such a table is now always fully covered.

## How big it actually is

`crates/pdf-font/examples/type1_encoding_census.rs`, over every corpus on this disk — 5 435 files,
5 411 opened:

| | |
|---|---|
| bare Type 1 programs | 724 |
| with a predefined encoding (unaffected) | 253 |
| with a custom encoding array | 471 |
| documents holding a program whose resolved map claims a code its array never assigns | 51 |
| such codes in all | 109 789 |

That is not an edge case. It is the ordinary shape of a subsetted Type 1 font: a 256-entry array
with a few dozen codes assigned, and the rest of the array standing in the resolved map as though
the program had encoded them.

**Why nothing was drawing wrong loudly.** `name_keyed.rs` consults the built-in table only where
the PDF `/Encoding` names nothing for that code, and a producer that subsets a font usually names
the codes it goes on to show. The population at risk is the intersection — a code the PDF
dictionary leaves unnamed *and* the program never encoded *and* the content stream shows — and no
corpus document is known to be in it. `doc/todo/53` was right that there is no witness and wrong
that this makes the finding small: the wrong answer was being computed 109 789 times and only the
last step kept it off the page.

## The decision

**Read which codes the array assigns from the program's own cleartext header, and let
`read-fonts` keep answering everything else.**

`assigned_codes` scans the bytes before `eexec` — which is where ISO 32000-2 §9.6.2.1 puts the
`/Encoding` array, and why no decryption is needed — for the one shape a PostScript assignment
takes, `dup <code> /<name> put`. It reads *which codes are mentioned* and nothing more: the name,
the glyph and every other property stay `read-fonts`' answer. A `Subrs` entry opens with the same
keyword and is turned away by the `/name put` shape. A program that builds its encoding by some
other construction is read as mentioning no codes, which restores exactly the previous behaviour
rather than inventing one.

**A code the array assigns to a name the program does not contain stays assigned**, and that is
the standard's instruction rather than a convenience:

> If an encoding maps to a character name that does not exist in the Type 1 font program, the
> .notdef glyph shall be substituted.

So `.notdef` is the right answer for that code and the wrong one for a code the array never
mentioned. The distinction the API could not express is exactly the distinction §9.6.5.2 draws.

## Why not upstream first

`doc/todo/53` said this "is a `read-fonts` API question before it is a question here", and that
would have been the right order if the answer needed new API. It does not: the array is in the
part of the file no library has to decrypt, and reading it costs one pass over a header that is a
few kilobytes against a parse of the whole program that has already happened. An upstream change
would be welcome and is not a dependency of this one. **Reading a table by hand for one specific
question is this project's established practice** where asking the renderer's own library would
measure the reader rather than the file — `hollow_glyph_census` reads a `loca` that way and
`composite_fonts.rs` an `hmtx`.

## What moved, and what did not

The text-extraction gate is unchanged at 99.3% over 974 documents and 99.8% against PDFBox, and
the oracle's contradicted set is unchanged — 61 pages, every one held by a group that already
existed. That is the expected result and it is worth stating as one: this fix removes marks that
should never have been drawn, in a population no gate on this disk exercises. A count that does
not move is not evidence that nothing happened.
