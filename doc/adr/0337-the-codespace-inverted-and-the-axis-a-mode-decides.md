# ADR 0337 — The codespace inverted, and the axis a mode decides

Status: accepted, 2026-08-14. Session 502. Closes the last of `doc/todo/22`'s §12.7.4.3 edges,
amends the §12.7.4.3, §9.7.6.2, §9.7.5.1, §9.7.4.1, §7.3.4.2 and §9.8.1 ledger rows, and corrects
a claim in §9.7.4.1's row that had been false since the thirty-sixth session. Changes nothing an
earlier ADR decided: ADR 0112's asymmetry between a font the document named and one this module
inferred is untouched, and so is ADR 0240's band.

## The question

§12.7.4.3 is the one place this program *writes* a content stream. A field's value arrives as a
§7.9.2.2 text string and has to leave as bytes in the font the `/DA` names — which means the
inverse of everything clause 9 does, and clause 9 is written in one direction. For a simple font
that inverse has existed since ADR 0032: 256 codes, ask each what it means, keep the first code
that means each character. For a composite font `pdf_font::LoadedFont::code_for` answered `None`
and `variable_text::set_in` refused the field by name, on this reason:

> `None` for a composite font. A `CMap`'s codespace ranges decide how many bytes a code occupies
> (§9.7.6.2), and inverting the code-to-CID mapping is a different question from inverting a
> 256-entry table; a caller reports the refusal rather than guessing a length.

Every clause in that sentence is true. None of them is a reason. It says the question is harder,
names the clause that makes it harder, and stops — and it stood for four hundred sessions with
`doc/todo/22` carrying it as *the last of the clause's edges*.

## What the standard states

§9.7.6.2, on extraction:

> A sequence of one or more bytes shall be extracted from the string and matched against the
> codespace ranges in the CMap. That is, the first byte shall be matched against 1-byte codespace
> ranges; if no match is found, a second byte shall be extracted, and the 2-byte code shall be
> matched against 2-byte codespace ranges. This process continues for successively longer codes
> until a match is found or all codespace ranges have been tested.

and §9.7.5.1, on the writing mode:

> A CMap shall specify the writing mode — horizontal or vertical — for any CIDFont with which the
> CMap is combined. The writing mode determines which metrics shall be used when glyphs are
> painted from that font.

Three things follow, and the second is the one the refusal had not weighed.

**The inverse is well defined, and it is a *test* rather than a construction.** A code's length
is not something a writer chooses; it is something the codespace ranges decide for the bytes the
writer emits. So the question "which code draws this character" has an exact answer: walk the
codes the `CMap`'s character mappings state, and keep the ones this same `CMap` would extract
from their own bytes. That is `CMap::each_addressable_code`, and its filter is one line —
`self.next_code(&bytes) == candidate` — which makes the round trip a property of the code rather
than a hope about it.

**The filter is not a formality.** A `CMap` whose codespace is `<00> <FF>` and whose `cidrange`
states `<0041> <0042>` maps codes no string can ever contain: a decoder matches the first byte
against the one-byte range and stops. Writing such a code would emit two one-byte codes that
select two other glyphs — a page of plausible wrong text, which is trap 1's archetype. Nothing
in the corpus can show this, so `a_code_the_codespace_excludes_is_never_offered` is a synthetic
`CMap` and is the only thing holding this half to the clause's words.

**And the writing mode is a refusal rather than a shortfall.** §12.7.4.3's layout here places
glyphs along the horizontal axis and measures them with §9.4.4's `w0`. A `CMap` in mode 1 says,
in the sentence quoted above, that those are the wrong metrics — the displacement is §9.7.4.3's
`w1` and each glyph sits at `-v` from the position. Drawing the value horizontally anyway would
be a confident wrong mark, not a partial one, and refusing leaves the document's own appearance
stream standing where it has one. That is ADR 0106's test applied to a whole: the entry a
refusal refuses here is *substitutive*, not additive.

## What was built

- **`CMap::each_addressable_code`** — the inverse walk, shortest length first and ascending
  within a length, which is the order §9.7.6.2 extracts codes in, so a caller keeping the first
  code that means a character keeps the one a decoder finds first.
- **`LoadedFont::addressable_codes`** — one table for both populations, built by *running*
  `text` and `glyph_for` over whichever set of codes the font has. The construction is ADR
  0032's and the argument is the same: the two directions traverse the same tables, so a code
  this returns is a code that draws the character asked for.
- **`variable_text::show` writes a code's bytes**, most significant first, as many as §9.7.6.2
  gives it. It wrote one byte, which was right while nothing could produce a longer code.
- **`Metrics::read` follows `/DescendantFonts`.** Table 119 gives a Type 0 dictionary no
  `/FontDescriptor`; Table 115's is the descriptor. Nobody had noticed, because until this
  session no composite font reached the function.
- **Two refusals with a clause each**: §9.7.5.1's writing mode 1, and a `CMap` stating more codes
  than the bounded walk visits.

### The bound, and why it is all or nothing

`MAX_ADDRESSABLE_CODES` is 2¹⁷. Principle 3 forbids a document's own numbers driving an
unbounded loop, and a `cidrange` may span the whole four-byte space. What makes 2¹⁷ the right
number is measured rather than argued: `every_registered_cmap_is_inside_the_addressable_bound`
walks all of §9.7.5.2's registered `CMap`s this binary carries, plus Table 116's two Identity
maps whose 65 536 codes are the largest of them all.

A `CMap` past the bound is declined **whole**, and the walk visits nothing rather than stopping
part way. A partial inverse would answer that the font has no code for a character the font has
a code for, and the caller's report would then name the wrong reason — trap 5's failure with a
table in front of it.

## What it cost the corpus, measured before the code

**0 objects.** `examples/variable_text_census` grew a count of the `/DA` fonts `/DR` defines that
are Table 119's Type 0, and of those in writing mode 1, over the 964 corpus documents it opens:
zero and zero. So this is a clause implemented for the documents that will arrive, the corpus and
oracle gates are expected to be identical line for line, and every rule here is defended by
hand-built pairs differing in one entry — `CLAUDE.md`'s two denominators, and trap 8's own
instrument.

The fixtures, in `crates/pdf-model/tests/variable_text.rs`:

| pair | differs in | asserts |
|---|---|---|
| one-byte against two-byte codespace | the `begincodespacerange` bounds, carried into the `cidrange` and the `bfrange` | the same value, the same CIDs, the same `/W` — and the same picture |
| a `/ToUnicode` present against absent | that one entry | the second draws nothing and names the font it could not use |
| `/Identity-H` against `/Identity-V` | one byte of one name | the first draws; the second is refused by name and draws nothing |

The descendant embeds no font program on purpose, so §9.7.4.2's substituted route is what is
exercised — and the *advances* are still the document's `/W`, which is what lets the first
fixture assert a position rather than a shape. Trap 1: all four were rendered and looked at.

## What was found on the way

**A line break was a code, and a composite font can state that code.** The layout carried
§12.7.5.3's break through as `Code::single_byte(b'\n')`, which is sound exactly as long as no
font gives code 10 a glyph. §9.6.5.1's `/Differences` lets a simple font's encoding put one
there and §9.7.6.2 lets a `CMap` state a one-byte code 10 outright; either would have had a
character of the *value* laid out as a break, dropped by `show`, and never drawn, with nothing
reported. `Placed` is an enum now and the compiler keeps the two apart, which is principle 4's
clear construction over a comment warning about a collision.

**Two claims in the tree were false and one of them was in a ledger row.** §9.7.4.1's row said
`/DW2` and `/W2` are unread and that a vertical font is refused before they would be needed —
false since the thirty-sixth session, while §9.2.4's and §9.7.5.1's own rows said the opposite.
`variable_text`'s module comment said a `/DA` naming a font `/DR` does not define is refused by
name — false since the hundred-and-twenty-third, when ADR 0112 gave that case a stand-in. Both
were found by reading the neighbourhood of a clause being worked on, which is `doc/todo/01`'s
fourth sweep happening by hand.

**And one test's contract changed, deliberately.**
`loading::tests::a_code_for_a_character_means_that_character` asserted
`font.code_for('A').is_none()` for every composite font — the refusal itself, written down as an
expectation. It now walks each font's whole inverse table, checks that each code means the one
character it was filed under, and checks that the code's own bytes decode back to that single
code; it requires more than a hundred *composite* codes to have been checked, so the
specification corpus has to exercise the new direction or the test fails. Neither of trap 1's two
load-bearing checks — `the_pdf_widths_agree_with_the_font_programs_own_advances` and
`an_uncovered_code_has_no_glyph_rather_than_a_guessed_one` — was touched.

## What this does not do

~~**`doc/todo/21`'s per-character fallback is still owed.**~~ `freetext_no_appearance.pdf` remains
the one corpus document whose `/DA` value is refused: `/Helv`, a paragraph of Arabic, and no
Helvetica has the glyphs. Nothing here reaches it — that one is about a *simple* font that
cannot draw part of a value, and ADR 0112 decided what happens then.

**And it is not the per-character fallback's witness either**, which the five-hundred-and-thirteenth
session read out (ADR 0348) and the five-hundred-and-seventeenth's retired-claim sweep found still
filed that way here. No compiled-in face carries one Arabic glyph — Liberation Sans's `cmap` maps
the whole range to glyph 0 and its `GSUB` has no `arab` script — so a chain asked per character
finds nothing to chain to, and even with glyphs it would draw isolated forms left to right. What
that document needs is a glyph source, Unicode's joining-form selection and right-to-left ordering
**together or not at all**. `doc/todo/21` and `doc/todo/22` carry the corrected filing; this
paragraph did not until now.
