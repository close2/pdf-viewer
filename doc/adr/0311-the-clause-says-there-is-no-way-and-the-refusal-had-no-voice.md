# ADR 0311 — The clause says there is no way, and the refusal had no voice

Status: accepted, 2026-08-13. Session 476. Amends §9.10.2's, §9.6.4's and §9.6.5.3's ledger rows.
Does not amend ADR 0152 or ADR 0270, whose trade and whose split this leaves where they are — what
is added here is a third population beside the two they count, and one correction to the branch
that separates them.

## The question, and the answer is a refusal

`french_diacritics.pdf` has been the sharpest named refusal in the text band since session 463:

> a pdfTeX Type 3 font whose `/Differences` names `/a192`, `/a224` … — the code in decimal — for
> the Latin-1 accented letters. `pdftotext` reads the code as the character and gets all
> twenty-seven; doing the same would mean extending `text_from_the_code` past 0x7E.

The round was asked to work out **from the clause** whether there is a legitimate route to a
character for those codes at all. There is not, and the standard says so in the same sentence it
says everything else about the case. What follows is the reading, because a refusal recorded with
its reading is worth more than the two lines it replaces.

## The file

29 glyph names, 28 of them `/aNNN` where `NNN` is the code in decimal, one `/a49` at code 49:

```
/CharProcs << /a192 12 0 R /a194 14 0 R /a196 16 0 R /a199 18 0 R … /a252 66 0 R /a49 68 0 R >>
/Encoding << /Type /Encoding /Differences [49 /a49 50 /.notdef 192 /a192 193 /.notdef … ] >>
```

The page shows `<e0><c0><e2><c2>…<fc><dc>` and then `(1)`. Every glyph description is an image
mask; the page draws correctly and agrees with every reference. What comes back is `1`.

## The four routes §9.10.2 states, taken in order

The clause opens "[a] PDF processor can use these methods, **in the priority given**".

**1. `/ToUnicode`.** Table 110 makes it *Optional* for a Type 3 font and pdfTeX wrote none. No
entry, no route.

**2. The glyph name, looked up in the Adobe Glyph List.** This one *applies* — and that is worth
saying, because it took until session 326 for this tree to admit that a Type 3 font is a simple
font whose glyph selection uses a name (§9.6.4 step a), §9.6.5.3's "shall be entirely defined by
its Encoding entry"). The clause's instruction is:

> If the font is a simple font and the glyph selection algorithm (see 9.6.5, "Character encoding")
> uses a glyph name, that name can be looked up in the Adobe Glyph List and Adobe Glyph List for
> New Fonts to obtain the corresponding Unicode value.

`a192` is in neither list, is not one of the `uniXXXX` or `uXXXX` forms the Adobe Glyph List
Specification defines, and is not a `_`-joined composition of names that are. `encoding::text_for`
answers `None`, and one of this tree's own unit tests has asserted `text_for("a97") == None` since
session 326.

**And the name is in a *third* list, which is the sharpest fact in this reading.** `a192` is a
`ZapfDingbats` glyph name — Annex D.6 puts it at code 218 of that font's built-in encoding, and
`encoding.rs`'s transcription of the table has it — as are eight more of this file's twenty-nine
names: `a49`, `a194`, `a196`, `a199`, `a200`, `a201`, `a202` and `a203`, with `a206` a glyph of
that font too although its encoding leaves it unencoded. `/a207`, `/a212`, `/a217`, `/a219`,
`/a220`, `/a224` and the rest are not names of anything at all, which is the point about the
convention: it names a *code*, and where the code happens to be one the dingbats font encodes, the
two collide. §9.10.2 does **not** send a reader to Adobe's
`zapfdingbats.txt`, and this file is exactly why it should not: following the one published list
that holds `a192` would read `À` as an ornament. The eighth session already paid for the drawing
half of the same coincidence — a Type 3 font was substituted, the substitute drew dingbats, and
nothing reported it (`type3.rs`'s module comment still carries that).

So the name is the producer's private label. It resolves to a dingbat in one published table, to
`À` under pdfTeX's convention, and to nothing under the two lists the clause names.

**3. The composite-font route.** Steps a) to e) are for "a composite font that uses one of the
predefined `CMap`s … or whose descendant CIDFont uses the Adobe-GB1 …" collections. A Type 3 font
is a simple font. Inapplicable.

**4. The closing sentence, which is an outcome and a permission:**

> If these methods fail to produce a Unicode value, **there is no way to determine what the
> character code represents** in which case a PDF processor may choose a character code of their
> choosing.

The standard answers the round's question in its own words: *there is no way*. What follows is a
licence to invent, not a fourth method — and principle 5 is explicit that a de-facto convention
presented as though it were derived is the failure this project guards against.

## Why the existing choice cannot simply be widened

This tree has already taken that permission twice (session 64's `text_from_program`, session 328's
`text_from_the_code`), and the second is bounded to 0x21–0x7E on a stated argument: that is the
range in which a byte and a Unicode code point mean the same character **under every encoding
§9.6.5 states**. The argument does not survive being carried past 0x7E, and Annex D is what says
so — for the two codes this file starts and ends its accented run with:

| code | `StandardEncoding` | `MacRomanEncoding` | `WinAnsiEncoding` | pdfTeX's `aNNN` |
|---|---|---|---|---|
| 192 | *unencoded* | `questiondown` (¿) | `Agrave` (À) | À |
| 224 | *unencoded* | `perthousand` (‰) | `agrave` (à) | à |

Reading `a192` as `À` therefore takes two steps the standard states nowhere: parse the digits
after `a` as decimal, then read that number as Latin-1 — which is `WinAnsiEncoding` in the range
where it and `MacRomanEncoding` disagree about every code. Nothing in the file says which. Getting
the right answer here is getting it by knowing pdfTeX, and this project may not do that.

**And a Type 3 font is where the argument is weakest, not strongest.** §9.6.5.3 makes its
`/Differences` "the complete character encoding for this font" and its NOTE adds that "Type 3
fonts do not support the concept of a default glyph name", so no base encoding stands behind these
names at all: there is not even a §9.6.5 table to appeal to. The refusal stands.

## What was actually wrong, then

Not the answer — the **silence**. The tree drew the page, read back one character of twenty-nine,
reported nothing, and had no channel that could have said so. A page whose fonts §9.10.2 cannot
name and a page with no text on it produce the same `Interpretation`: an empty string and
`unsupported: []`. `complex_ttf_font.pdf` is the extreme of it, 527 glyphs and a readback of
nothing but the placement pass's own inferred line breaks.

So `Interpretation::codes_without_a_character` is added: codes the page showed that §9.10.2 could
not name, whatever was drawn for them. **A count and not a report**, for ADR 0152's reason exactly
— every report costs the oracle a judged page (trap 11), and this is a shortfall in the readback
rather than in the picture, on pages that are otherwise correct. A host that searches or selects
can read it and say so; the oracle never sees it.

Over the pdf.js corpus, counting only pages that report nothing: **1342 codes over 45 documents**,
against the 5 over 2 that reach no glyph and the 57 over 9 that reach a blank one. The readback
band is two orders of magnitude larger than the drawing band, and until this round none of it was
counted anywhere.

## The second half: a code that reads back nothing was a space

Session 464 left this, and it is the same subject one branch over:

> `show_text`'s classification is `self.text[start..].chars().all(char::is_whitespace)`, and an
> empty slice satisfies that vacuously. It is not obviously wrong and it is certainly not
> measured; it wants its own population count before anything is done to it.

The population is counted above and the classification is now `Readback`, a three-state answer
from the *font* rather than an inspection of the *buffer*: characters, whitespace, or nothing.
Three things come out of it, and only the last one is a behaviour change:

- **The tally's rule is unchanged and is now argued.** Only `Readback::Characters` says a mark was
  owed. A space is meant to have no outline; and a code §9.10.2 could not name says nothing in
  either direction — "there is no way to determine what the character code represents" is not
  evidence that a mark was missed, and reporting a font on it would be a guess. Reading the buffer
  reached the same verdict by an accident, and an accident that agrees with the right answer is
  still the thing to remove: it was doing the work of an argument nobody had made.
- **The measurement's rule is unchanged too**, which was worth checking rather than assuming: an
  earlier draft of this change counted every code with no readback as a mark missed and took the
  corpus from 5 codes over 2 documents to 129 over 7, with two new `Unsupported::Font` reports and
  two oracle pages lost. `issue7769.pdf` is one of them — one code, glyph 1, which the program
  contains and draws blank, reading back nothing — and a report saying "this font drew nothing"
  about it is a claim nothing in the file supports.
- **The reversal is no longer blind.** Inside a §14.8.2.5.3 `/ReversedChars` sequence the readback
  is collected per code and appended after the string, so `self.text[start..]` is empty for
  **every** code there, whatever the font said. The buffer rule called all of them spaces. Asking
  the font answers both cases with one question, and `accessibility.rs` holds the pair — the same
  two codes shown plainly and reversed, counted the same.

A third draft moved `coverage.empty` into the "no glyph at all" arm, on the reasoning that ADR
0270 calls a blank glyph "not a mark missed". That is wrong and the corpus said so within a minute:
`issue13316_reduced.pdf`, `recursiveCompositGlyf.pdf` and `issue20232.pdf` all lost their reports,
and all three are blank pages whose codes read back real characters — 开, `h` — through glyphs the
program contains and describes as empty. ADR 0270's split is about *which of two things happened
at the end of §9.6.5.4's route*, not about whether the page is right. `silent_fonts.rs`'s first
test is what caught it, which is what that file is for.

## What was deliberately not done

- **No report for a page §9.10.2 cannot read.** Forty-five documents, most of which draw
  perfectly; trap 11's cost is forty-five judged pages for a shortfall the count already states.
- **No `aNNN` rule, and no widening of the printable-ASCII bound.** Both are above.
- **No third instrument for the `codes_without_a_character` population's shape.** What the 1342
  are made of — how many are `Identity` orderings with no `/ToUnicode` (`doc/todo/21` §2's 40
  fonts), how many are symbolic `TrueType` subsets, how many are `ZapfDingbats` names §9.10.2's
  two lists do not hold — is the next round's, and `doc/todo/21` §5 says so with the number in
  front of it.

## Consequences

- A refusal that had been recorded only in prose now has a number attached to it, on every page,
  in the one struct a host reads.
- `examples/readback` prints it beside the glyph count, so the instrument that shows what a page
  reads back also shows what it did not.
- The corpus gate prints three silence lines instead of two, through one helper, and the two that
  existed are byte-identical to what they printed before this change.
