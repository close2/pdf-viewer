# ADR 0195 — A character the interface cannot set is a box, and the silence is measured

Status: accepted, 2026-08-05 (session 316).

## Context

Everything this program draws for *itself* is set in §9.6.2.2's fourteen font programs, compiled
into the binary (ADR 0133). That is what makes the interface reproduce on a machine with no fonts
installed, and it is the right default — but the fourteen are Latin, and a document's own text is
not.

`Chrome::text` walked a string, asked the face for each character's code, and **skipped the
character where there was none**; `Chrome::width` gave it no advance either. For a title being
elided to a panel's width that is defensible. For text the document states and this program is
showing on purpose it is trap 5 in this host's own interface:

> a person shown an empty row has been told the document states nothing there.

§12.5.6.14's popup window has said so since the three-hundred-and-twelfth session, in a sentence
under the note (ADR 0191). Every other string this host draws from a document was **unmeasured**,
and `doc/todo/27` said so.

## The measurement

`viewer-ui/examples/chrome_coverage` opens every corpus document through `viewer-core` with a
zero-pixel viewport — so nothing is interpreted or rasterised — and asks the four queries the
sidebar asks. What it counts is therefore the strings this host actually draws, not every string
in the file. Over 964 documents that open:

| population | documents | of those, short | strings | short | drawn as nothing | characters |
|---|---|---|---|---|---|---|
| §12.3.3 outline titles | 150 | **7** | 343 | 24 | 3 | 46 |
| §8.11.4.3 layer names | 21 | **1** | 91 | 2 | 0 | 61 |
| §7.11.4 file names and descriptions | 10 | **0** | 64 | 0 | 0 | 0 |
| §14.3.3 `/Info` values | 492 | **45** | 1293 | 80 | 4 | 196 |
| §14.3.2 XMP properties | 317 | **21** | 1437 | 38 | 2 | 72 |

**74 documents** state something in the sidebar that this program's own font cannot set, and
**nine strings would have been drawn as nothing at all** — Japanese, Thai and Chinese, mostly:
`issue2884_reduced.pdf`'s outline is あいち電子調達共同システム, `issue16176.pdf`'s is
ローカルディスク, and `issue13211.pdf`'s `/Info` is a Thai sentence of 46 characters of which
Helvetica sets one.

**The largest single loss was not a language at all.** `bug1146106.pdf`'s layer names lose 51
characters of 98, and they are U+FFFD: the file writes its text strings as UTF-16
**little**-endian, which is none of §7.9.2.2's three encodings — those are `FEFF`-prefixed
UTF-16BE, `EFBBBF`-prefixed UTF-8, and Table D.3 — so `text_string` reads the bytes as
PDFDocEncoded and every second one is the clause's `U`, "[u]ndefined code point in
`PDFDocEncoding`". That is a correct reading of a malformed file (principle 5), and
`pdf_syntax::text_string`'s own comment already said what a caller owes it:

> Nothing draws it, so a caller laying the string out reports it rather than dropping it silently.

It was dropping it silently. The box is that report.

## Decision

**A character the interface's own face states no code for is drawn as a box, 0.6 em wide, and it
advances.** Three cases, in `Chrome::set`, which is the one place `Chrome::text` and
`Chrome::width` agree about a character:

| | drawn | advances |
|---|---|---|
| the face states a code | its glyph | the face's advance |
| whitespace with no code (U+00A0, U+3000) | nothing | the face's space |
| a control character with no code | nothing | 0 |
| anything else with no code | a box | 0.6 em |

`doc/todo/27` listed three answers and this is a fourth. Its three were: say so on every row
(cheap, and it turns a panel of Japanese headings into a panel of apologies); fall back to a face
on the machine (works, and costs ADR 0133's argument — the interface stops looking the same on two
machines); compile in a face with the coverage (a licence question, a megabyte question, and a
decision the project owner has not been asked for). **The box is none of those and it is what a
text engine has always done**: it says *that* a character is there and that this program cannot set
it, without claiming to know what it is, and it leaves the other three open rather than pre-empting
them.

**0.6 em is a choice and the argument for the number is §9.6.2.2's own Courier**, which advances
every code by exactly that. A placeholder claims nothing about the character it stands for, so the
one width the fourteen state for everything is the honest width to give it. The standard says
nothing whatever about an interface's own text, which is why this paragraph exists: it is a
documented choice, not a reading.

**A control character is blank rather than a box**: it has no visible form to be missing, so a box
would be saying something untrue rather than saying nothing. Exactly one character in the corpus is
one, in an `/Info` value — the difference between the two runs of the sweep, 196 characters and
195. **U+FFFD is deliberately not in that arm**, for the reason above: it *is* the report.

**`without_a_code` counts the boxes and not the gaps.** It is what §12.5.6.14's popup says out loud
under its note, and a count that disagreed with the picture beside it would be worse than no count
— so it is computed through `Chrome::set` like everything else.

## Consequences

- A Japanese outline is a panel of boxes instead of a panel of empty rows. That is not *good*, and
  it is honest: `doc/todo/27` stays open on what would make it good, with the measurement above in
  place of the "unmeasured" it used to carry.
- Every width the panel computes moved, because a placeholder that is drawn has to be measured:
  elision, the popup's title bar and `wrap` all go through `Chrome::width`, and a box that measured
  zero would put the rest of every line in the wrong place. That is what
  `a_title_this_interfaces_font_cannot_set_draws_a_box_for_each_character` asserts on both sides —
  ink on the row, and five boxes' width for five characters.
- **No gate could have seen any of this**, which is trap 12b's shape one more time: the corpus
  interprets page one, the oracle rasterises pages, and neither opens a sidebar.
  `viewer-ui/tests/panel.rs` is the only instrument that can, and it is where the new test lives.
