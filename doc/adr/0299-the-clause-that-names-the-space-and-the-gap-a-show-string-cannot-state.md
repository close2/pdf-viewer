# ADR 0299 — The clause that names the space, and the gap a show string cannot state

Status: accepted, 2026-08-13. Session 464. Amends §9.3.3's, §9.4.4's, §9.10.2's and §14.8.2.6.2's
ledger rows. Does not amend ADR 0140, ADR 0259 or ADR 0298, whose readings of §9.10.2 this leaves
exactly where they are — what is decided here sits *after* the last of them.

## The question

Session 463 left a finding it deliberately did not take:

> `separate_text` cannot see a gap inside a show string at all. `text_cursor` is set to the
> post-advance matrix and read again at the next code, so the two are equal by construction — only
> a `Td`/`T*`/`Tm` between show operations produces an inferred separator. `Type3WordSpacing.pdf`
> draws a five-em word gap and reads back `abbaabba`.

The finding is exactly right about the mechanism. The question is what to do about it, and the
tempting answer — make the heuristic see *inside* a show string, with a threshold — is the one
`CLAUDE.md` forbids: a constant tuned until a corpus matches is curve-fitting, and there is no
corpus signal to fit against anyway, because both text gates strip whitespace from the comparison
by design.

So the question was taken to clause 9 instead: **what, exactly, can a show string state about the
distance between two of its glyphs?**

## What §9.4.4 says, and what follows from it

The combined displacement is

> tx = ((w0 - Tj/1000) x Tfs + Tc + Tw) x Th

and inside one show string the `Tj` term is absent — that number is an element of `TJ`'s array,
between strings, and this tree already shows each string separately and applies the adjustment
between them. So the distance between two consecutive codes of one string is exactly three things:

| term | what it is | is it a word break? |
|---|---|---|
| `w0 x Tfs x Th` | the first glyph's **own** advance | no — a wide glyph is a wide glyph |
| `Tc x Th` | §9.3.2's character spacing, added to **every** pair alike | no — this is tracking, and `word_gap`'s own comment already says judging it as a break spells *Clarification* as *Clar if ic at ion* |
| `Tw x Th` | §9.3.3's word spacing, applied to **one code** | yes, and the clause says whose |

That is the whole decomposition, and it settles the finding rather than motivating a threshold:
**the only word gap a show string can state is the single-byte code 32**, and it is identified by
its encoding rather than by how far it moves. A rule that measured the distance would find the
same code by a longer route on `Type3WordSpacing.pdf` and would miss it entirely on the document
below, where the gap is the glyph's own advance and no threshold on a *departure* from a glyph's
width can see it.

## What §9.3.3 says, and it names a character

The clause opens by identifying the code with the character:

> Word spacing works the same way as character spacing but shall apply only to the ASCII SPACE
> character (20h).

and closes by saying which codes those are:

> Word spacing shall be applied to every occurrence of the single-byte character code 32 in a
> string when using a simple font (including Type 3) or a composite font that defines code 32 as
> a single-byte code.

The identification is not incidental — it is how the clause says which glyph `Tw` applies to, and
it is a statement about the encoding, not about a particular font's opinion. A font that has no
`/ToUnicode` entry for code 32, no glyph name the Adobe Glyph List knows, no character collection
and no `post` or `cmap` answer has not *contradicted* the clause; it has said nothing, and the
clause has already said it.

**So `Font::text` reads such a code back as U+0020, and it does so last.** §9.10.2's three methods
and its closing permission run first and are believed: a `/Differences` naming code 32 `/bullet`
is the producer's own statement, the Adobe Glyph List answers it, and a space there would be text
the page does not show. The rule fires only where every route has already declined.

**And it is a different thing from `text_from_the_code`**, which takes §9.10.2's closing permission
for 0x21 to 0x7E and excludes 0x20 on purpose. That one reads a code *as* its byte, which is a
choice about a producer's convention; this one reads a clause that names the character. The
exclusion there stands unchanged and for its own stated reason.

## The population, and the document named after the defect

Counted before it was believed (trap 11), over the pdf.js corpus, the `doc/` specifications and
`doc/corpora/pdfbox`, page one of each: **five documents show a single-byte code 32 that every
method declines**, 35 occurrences.

| document | what it reads back now | what it read back before |
|---|---|---|
| `issue4304.pdf` | `Words that should have spaces between them.` | `Wordsthatshouldhavespacesbetweenthem.` |
| `issue5256.pdf` | `printed circuit board feasible` | `printedcircuitboardfeasible` |
| `issue6901.pdf` | `Issue 6901: f ﬀ ﬁ ﬃ` | `Issue6901:fﬀﬁﬃ` |
| `Type3WordSpacing.pdf` | ` ab ba abba` × 6 | `abbaabba` × 6 |
| `font_ascent_descent.pdf` | one separator in a page of symbol codes | — |

`issue4304.pdf` is the case worth stating in full, because of what it says about instruments. It
is 895 bytes: a non-embedded `/Times-Roman` whose `/Differences` maps 32 to `/.notdef`, one `Tj`
showing the whole sentence. The **four-hundred-and-fifth session fixed its picture** — ADR-less,
because the ordering it corrected was already decided: `.notdef` has an advance of 250/1000 em in
the substitute's own program, `pdf_font::cff::advances` was written to read it, and the page has
drawn the words apart ever since, agreeing with four references to the device column. Nobody asked
what it read back. Fifty-nine sessions later it still read back one word.

`Type3WordSpacing.pdf` is the other end of the same rule: its Type 3 font gives code 32 no glyph
name at all and its six lines carry `Tw` of 50, 40, 30, 20, 10 and 0. The last of those is the one
a distance-based rule would get wrong in the other direction — at `0 Tw` the two words are
genuinely adjacent on the page, and the readback still owes a space, because the file wrote one.

## What no gate could see, and that is the point

**Neither text gate moved.** `the_text_we_draw_agrees_with_an_independent_extractor_across_the_pdfjs_corpus`
is 24 012 of 24 191 words and 22 named documents before and after; the frozen PDFBox comparison is
14 257 of 14 281 with 4 below its floor, before and after. That is not a disappointment and not a
failure of the change — it is both gates working as designed. Their module comment says so in as
many words: word boundaries "are deliberately not compared, because a content stream does not
record them", so the comparison strips whitespace from both sides. A change to word separation is
outside what either can measure, **by construction, in both of them at once**.

The corpus gate, the oracle (905 agrees, 68 contradicted, 786 ambiguous, 1794 verdicts), quorra,
dates, XMP, JPEG 2000 and the conformance gate are all line for line unchanged, which is the other
half of the evidence: a readback rule cannot move a pixel, and none did.

So the instrument here is the readback itself — `cargo run --release -p pdf-model --example
readback` — and the reason that instrument exists is written on it: "a rule that invents text would
not move [the gate], so a round that adds one needs this". This round is the first to need it for
the opposite direction as well, and the habit is in `doc/habits.md`: **a page that draws right can
read back wrong, and this project's text gates are built not to see it.**

Who does see it: text selection, `/`-search across a document, `pdf-retrieve`'s JSON, and the
screen reader that speaks a page over AT-SPI. All four take `Interpretation::text`, and all four
were being handed `Wordsthatshouldhavespacesbetweenthem.`

## Decision

1. **A single-byte code 32 reads back as U+0020 where §9.10.2 has wholly declined**, on §9.3.3's
   naming of the character, and behind every one of §9.10.2's methods and its permission.
2. **`separate_text` is called once per show operation rather than once per code**, because
   §9.4.4 leaves nothing inside one string to read. This is exactly equivalent to what the code did
   — the cursor was compared with itself after the first code — and stating it is the point:
   the finding above was invisible because the call *looked* like it ran per glyph.
3. **The position heuristic keeps its two separators and is documented as a choice**, with
   §14.8.2.6.2's own NOTE 1 quoted beside it: a clause that describes what stating word breaks
   *spares* a reader is a clause acknowledging what an untagged page leaves it relying on. It is
   named as a heuristic by the standard; it is not a clause obeyed, and it no longer reads as one.

## What was deliberately not done

- **No threshold inside a show string.** There is nothing to threshold: §9.4.4's other two terms
  are a glyph's own width and a tracking value that applies to every pair alike, and a constant
  distinguishing them would be fitted rather than derived.
- **A two-byte code 32 gets nothing**, and the exclusion is the clause's own: "It shall not apply
  to occurrences of the byte value 32 in multiple-byte codes." A composite font whose space is
  `<0020>` and whose `/ToUnicode` is silent still reads back nothing there — the same answer §9.3.3
  gives to `Tw`, and `a_two_byte_code_32_is_not_read_back_as_a_space` pins it. If such a page ever
  needs an answer it is §9.10.2's to give, not this rule's.
- **`text_from_the_code`'s exclusion of 0x20 stands**, in both `pdf-font` and `pdf-model`'s Type 3
  reader. Widening it would have been the same outcome by a worse argument.
- **A code that draws no glyph *and* reads back nothing is still not counted as a missing mark**,
  and this round found out why rather than fixing it: `show_text` classifies a code as a space when
  `self.text[start..].chars().all(char::is_whitespace)`, and an **empty** slice satisfies that
  vacuously. So the "codes reaching no glyph in silence" tally is 5 over 2 documents partly because
  a code with no readback at all falls in the space arm. After this change the population it was
  hiding is smaller — those code 32s genuinely are spaces now — but the vacuous `all` remains, and
  it wants its own population count before anything is done to it. Named here so that the next
  round has it rather than rediscovering it.
