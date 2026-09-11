# 0953 — The fonts, and the one number that may move

Session 953. Status: **accepted**. The seventh slice of `quorra-transform archive`
(`doc/questions/A22`): the four ranked font rows of the corpus sweep, which is the largest area
the converter had not touched, and the one the owner argued through before any of it was built
(`doc/questions/A47`, `doc/questions/A48`, `doc/pdf-a-conversion-limits.md` section 4.9).

The sweep is the instrument and its ranking chose the order. What each row cost is in the sweep's
output; this file records what was decided and why, which no command prints.

## 0. Two of the four rows are refusals with an argument, and that was the ranking's own answer

`fonts/embedded-programs-define-every-glyph-shown` and `fonts/no-notdef-glyph-shown` were the
first and fourth rows by count, and they convert nothing. Both predicates in `pdf_archive` ask
their question only of a font **whose own program the file carries and this tree reads** — so
every document they stop is `doc/pdf-a-conversion-limits.md` section 2.2's first bullet, where the
mapping is fixed by a program the file supplies and the only ways out are taking a mark off the
page or drawing a glyph that was not there. ADR 0816's fence is where both stop.

They were being refused with `NOT_BUILT_YET`, which says a later slice owes the fix. Nothing does.
Moving them into `REFUSED_BY_NAME` as `Because::TheFence` is the whole of the change and it is the
point of that table existing: a user told "not yet" comes back tomorrow for the same answer.

**The same two rows are also why the substitution below has a gate rather than a fallback.** When
the converter builds the embedded program it chooses what each code maps to, so section 2.2's
second bullet makes the clause satisfiable without anything being edited — and `doc/questions/A48`
allows the empty glyph for a code that draws nothing today. That permission is not exercised here,
because the gate this slice takes is the stricter one: a code the shipped face has no glyph for
refuses the document by name. ADR 0927 calls the empty glyph the thinnest of the four permissions;
reaching for it before the face itself was decided would have spent it on a case that
`doc/questions/A47`'s "refuse rather than guess" already answers.

## 1. Which number may move, and the clause that settles it

ISO 19005-2 section 6.2.11.5 and ISO 19005-4 section 6.2.10.5 require a font dictionary's stated
widths and its embedded program's own to agree. Two statements disagree; one of them must change.
ISO 32000-2 §9.2.4 says which:

> Storing this information in the font dictionary, although redundant, enables a PDF processor to
> determine glyph positioning without having to look inside the font program.

The dictionary's number positions the glyphs. The program's is read by nothing in the rendering
model. So the program's is the one that may move, and `doc/pdf-a-conversion-limits.md` section 4.9
calls the reverse — restating `/Widths` — **never**: it would move every line of every page the
converter touched, and nothing in the report would say so.

**That is asserted rather than intended.** `crates/pdf-transform/tests/archive_corpus.rs` now
carries a third property beside its two: every glyph advance the output's font dictionaries state
is one the source stated, compared as whole sets over all six targets and every corpus document
that converts. Two of the corpus's own width-mismatch witnesses were also rendered before and
after at 100 dpi and came back **byte for byte identical**, which is the same claim measured on
the page instead of in the dictionary.

## 2. The rewrite itself: `pdf_font::restate`, and what it is allowed to touch

A new module, and a widening of `pdf-font`'s stated responsibility from reading font programs to
restating one number inside one. The two backends are as different as the formats:

- **sfnt.** `hmtx` is rebuilt with one record per glyph and `hhea`'s `numberOfHMetrics` raised to
  match — a font that stated one advance for its whole tail cannot state a different one for one
  glyph of it. Every side bearing crosses unchanged, so no glyph moves inside its own advance.
  Every table's checksum and `head`'s `checkSumAdjustment` are recomputed, because a restated
  `hmtx` makes both false and a font that lies about itself is not one to archive.
- **CFF.** Adobe Technical Note #5177 section 3.1 puts the width in one optional operand before
  the charstring's first stack-clearing operator. That operand is replaced — or inserted — and
  every other byte of the charstring is copied. The program is rebuilt the way
  `cff::readable_font_dicts` already rebuilds one (ADR 0808): the Top DICT re-encoded with every
  offset in the five-byte form so its length is known before the offsets are, everything after it
  copied and shifted, and a fresh `CharStrings` INDEX appended past the end.

**The hard case, and it is most of a subroutinized face.** Where a charstring begins with
`callsubr` the width is inside the subroutine, and prepending a second one would flip the parity
the rule turns on and corrupt the outline. The call is therefore *inlined* — which is exactly what
the interpreter does — until the width is where the rule puts it. Finding where a subroutine ends
means reading it as Type 2 rather than looking at its last byte, because `hintmask`'s mask bytes
follow the operator as data and one of them may be the `return` opcode; a subroutine that both
calls another and carries a mask is declined rather than guessed at. Measured over the ten
compiled-in CFF faces, between two and six glyphs of roughly two hundred and thirty decline, and
they are named per glyph rather than swallowed.

**The claim that no outline changes is a test, not a sentence.**
`restate::tests::an_outline_survives_its_advance_being_restated` restates every glyph of all
twenty compiled-in faces one at a time and compares the drawn path before and after.

## 3. The face, and why it is never the machine's

`pdf_font::standard::shipped_face` rather than `substitute::find`, and the difference is two
requirements rather than a preference. ISO 19005-2 section 6.2.11.4.1 admits only a program that
may lawfully be embedded for unlimited universal rendering, which a face installed on somebody's
machine generally may not be; and an archived file that depended on which machine converted it
would be the indeterminacy PDF/A exists against. The compiled-in fourteen are licence-checked
(`data/standard-fonts/PROVENANCE.md`, `/NOTICE`) and metric-compatible with §9.6.2.2's fourteen,
which is what makes section 4.9's *first* metric route — a face whose own advances already are the
file's numbers — the free and common case.

Substituting is a `Decision::Stated` rather than a `Decision::Mechanical`, and the sentence it
carries says why: nothing is lost, because a file that names a font and does not carry it **had no
appearance of its own** — every reader picks a face at display time and they pick different ones —
and all the same something is different afterwards, because the shapes are now this program's
choice. That is section 4.9's whole argument for making it the default, and `doc/questions/A47`
settled it.

**Three things go with the face, and each is the standard's rather than a convenience.** §9.9's
Table 124 decides which `/FontFile` key a program of a given format may go under, and a pairing it
does not state is refused by name rather than written. Table 126's `/Length1` is restated, because
a rewritten program is not the length the producer's was. And the descriptor's `/CharSet` and
`/CIDSet` are **removed**: §9.8.1 makes each a description of the *embedded* program, this
descriptor described one the file never carried, and ISO 19005-2 section 6.2.11.4.2 requires such
a description to be complete — so leaving a producer's beside a face this converter chose leaves
the file stating something false about its own bytes. The corpus found that: the output failed a
requirement its source had met, and stage 3 refused to write it. The net under the verb worked
exactly as ADR 0947 said it would.

## 4. A fourth kind of *no*

`--no-substitute` is section 4.9's own flag, and it needed a reason `Because` did not have. The
three it had are facts about the document (`NotThisTarget`), about this program (`NotBuiltYet`) and
about what converting *is* (`TheFence`). A refusal the caller asked for is none of those: it goes
away the moment the flag does. `Because::Declined` is the fourth, and a report that had said "this
converter does not yet meet this requirement" about a flag the user had just typed would have been
wrong in the most confusing direction available.

## 5. What is refused, by name, and what each refusal is waiting on

- **A composite font nothing embedded.** §9.7.4.2 makes a CID an index into the glyphs of the font
  that defined it, so a code of such a font names a glyph of a program that is not here and names
  nothing in any other face. Section 2.1 is the reading.
- **A font with no descriptor at all**, which is §9.6.2.2's permission for the standard 14.
  Embedding needs a descriptor and §9.8.1's Table 122 makes `/StemV` required of one — a
  measurement of a face's stems that nothing in the file states.
- **A `/Type1` dictionary whose face is a `glyf`-based sfnt.** Table 124 admits a Compact Font
  Format program there and this program's only sans-serif faces are Liberation's sfnts. **This is
  the one refusal that a shipped font would close**, and it is a licence job rather than a code
  one: `doc/questions/A47` authorises shipping an OFL family, under the same discipline the sRGB
  profile took.
- **A code no shipped face has a glyph for**, which is A47's "refuse rather than guess" stated
  outright, and which is also what keeps both parts' `.notdef` clause satisfied without anything
  being invented.

## 6. What this cost, and where

One extra content walk per document, and only for a document that failed a font requirement:
`pdf_archive::check` makes the walk for its own rules and does not keep it, and both font
preparations need the codes the content streams showed. A document whose fonts all conform pays
none of it.

A restated CFF grows, because inlining a subroutine duplicates its body. Only the glyphs a page
shows are restated, so the growth is a handful of charstrings rather than a face; restating *every*
glyph of a compiled-in face roughly doubles it, which is the number to remember if a later slice
ever wants that.
