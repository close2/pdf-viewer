# ADR 0303 — A number is made of digits, and a lone period was a size of zero

Status: accepted, 2026-08-13. Session 468. Amends §7.3.3's ledger row and closes one of the two
defects ADR 0302 diagnosed and left. Changes nothing any earlier ADR decided.

## The question

`doc/todo/03` §7 carried two findings from the four-hundred-and-sixty-seventh session, each blank in
silence in `openpreserve/format-corpus`'s hand-built corpus and each blocked on a clause reading.
This is the second of them, `T02-05-01_006_font-size-operator-missing.pdf`:

```text
BT
/F0 . Tf
(Hello PDF-world!) Tj
ET
```

The size operand is a lone PERIOD. This reader set a text font size of **nought**, drew invisible
text, and said nothing at all — one of the files in that corpus that come back blank with no report
of any kind, which is the failure `CLAUDE.md` principle 1 forbids outright.

## What the standard states

§7.3.3 writes both numeric forms the same way round, and the word that decides this is *digits*:

> An integer shall be written as one or more decimal digits optionally preceded by a sign.

> A real value shall be written as one or more decimal digits with an optional sign and a leading,
> trailing, or embedded PERIOD (2Eh) (decimal point).

So a run holding no decimal digit is **neither**, however many signs and points it carries: `.` is
not a real, `-` is not an integer, and neither is any object at all. EXAMPLE 2 is the other half of
the same sentence and is what keeps the condition honest — `4.` and `-.002` are as legal as `0`, so
the rule may not ask for a digit *before* the point or for a point at all. It asks for a digit
*somewhere*.

What such a run **is** follows from §7.2.3, which makes PERIOD and the two signs regular characters:
a run of regular characters that does not spell an object. §7.8.2 then says what one means inside a
content stream —

> An operator is a PDF keyword specifying some action that shall be performed … An operator keyword
> shall be distinguished from a name object by the absence of an initial SOLIDUS character (2Fh) (/).

> Ordinarily, when a PDF reader encounters an operator in a content stream that it does not
> recognise, an error shall occur.

— and §7.3's object grammar says what one means in a file body, which is that nothing may be parsed
from it. Both of those are behaviours this tree already had, for `foo` and for `>` and for every
other keyword. What the lexer was doing was **smuggling `.` past them** by pretending it was a
number.

§9.3.1 is why the silence on the witness is the defect rather than the blank page:

> There is no initial value for either font or size; they shall be specified explicitly by using Tf
> before any text is shown.

A `Tf` that states no size leaves the show undrawable. Drawing nothing is the only thing this reader
can do; saying nothing is not.

## The decision

`Lexer::read_number` returns `Token::Keyword(raw)` for a run holding no ASCII digit, instead of
`Token::Integer(0)`. Five lines, one condition, and it is the clause's own condition rather than a
list of the spellings anybody has seen.

Everything downstream was already written and already argued:

- **In a file body** the parser refuses it — `SyntaxError::Unexpected` — exactly as it refuses any
  other keyword where an object was expected. `<< /Rotate . >>` used to be `/Rotate 0`.
- **In a content stream** the interpreter reports `Unsupported::Operator`, exactly as it reports any
  other unrecognised operator, and inside an array it reports §7.3.6's own refusal.
- **The operator that follows is then short of operands and is refused rather than half-applied**,
  which is ADR 0302's rule read the other way: `/F0 . Tf` gives `Tf` nothing, so §9.3.1's absent
  size is absent, and `show_text` reports the mark it cannot make.

**What the old code said about itself is the part worth keeping.** Its comment read "[a]nything
unparseable becomes `Integer(0)`, matching what other viewers do" — principle 5's forbidden
direction of inference, written down and unchallenged for four hundred and sixty-seven sessions.
Agreement with another reader is evidence that we read the clause right; it is never the reason.

**One thing is deliberately left alone.** A run that *does* state a digit and still salvages nothing
— `.-1`, where the sign arrives after the point and before any digit — keeps the older reading of
zero. That is a different question from this one, `salvage_number`'s recovery of `--5` and `1.2.3`
is argued where it lives, and no corpus on this disk offers a witness for the hybrid.

## The population, counted before the condition was believed

Trap 11. `tools/safedocs survey --dir`, one process per archive, over every corpus this disk can
reach, on both sides of the change:

| population | documents | incomplete before → after | documents whose report changed |
|---|---|---|---|
| pdf.js corpus | 974 | 65 → **67** | 3 |
| `doc/corpora/` (three submodules) | 108 | 9 → 9 | **0** |
| `openpreserve/format-corpus` (five directories) | 267 | 22 → **23** | 2 |
| SafeDocs `CC-MAIN-2021-31`, **all 145 archives** | 65 944 | 823 → **823** | 7 |

**Twelve documents of 67 293, and nine of the twelve were already reporting something else.** The
three that were not are the finding:

- `T02-05-01_006_font-size-operator-missing.pdf`, the witness, which is now
  `[Text { operations: 1 }, Operator { operator: "." }]` where it was silent.
- `issue9252.pdf`, whose sharpPDF producer wrote `. .59 .84 rg` — meaning `0 .59 .84`, and the file
  does not say so. This is the one page in all four populations where the change is visible: the
  word *Test* was teal and is now black, because `rg` is left with two operands and is refused. **A
  guess that happens to be right is still a guess**, and the alternative is the fallback trap 5
  forbids. What the page gained is a report naming the token.
- `bug1953099.pdf`, whose `TJ` array contains `(v)-(e)` — a kerning adjustment that lost its digits
  in whatever damaged the file. The ink is unchanged and the report is new.

The other nine already report between one and eighteen other malformed operators apiece; on those
the change adds a name to a list. **The rate is what the clause predicts**: a producer does not
write `.` where a number belongs, so this is a defect of *damaged* files, and 0.011% of the crawled
web carries one.

## What it moves, measured

`examples/display_list_digest` on both sides, in one sitting, over pdf.js's 974, `doc/corpora`'s
108, `openpreserve`'s 267 and 10 000 SafeDocs documents — the four whole archives ADR 0302 used plus
the six that changed a report:

| population | documents | display lists that changed |
|---|---|---|
| pdf.js corpus | 974 | **1** — `issue9252.pdf` |
| `doc/corpora/` | 108 | 0 |
| `openpreserve` | 267 | **0** — including the witness, whose blank page is blank either way |
| SafeDocs, 10 archives | 10 000 | **2** — `0300856.pdf`, `4605705.pdf` |

**Three of 11 349, and all three were looked at** (trap 1). Only the first changes what a person
sees. `0300856.pdf` loses one command of 474 and its page is **pixel-identical** at
`magick compare -metric AE 0` — it is a wholly black page from a content stream that reports forty
other malformed operators. `4605705.pdf` keeps all 272 of its commands and 849 bytes of their
parameters move; its page cannot be rasterised **on either side**, because a `cm` in it is singular
by twenty-two orders of magnitude and `render-cpu` refuses it out loud. Every other document that
gained a report draws exactly what it drew, `magick compare` agreeing at zero: the `-` in a `TJ`
array was a kerning adjustment of zero either way, and `issue5039.pdf`'s `-inf` sat beside an `inf`
that was already an unrecognised operator, in front of a `d1` already refused for want of operands.

## The one page where three references disagree with us

`issue9252.pdf` is the whole of it, and it is a principle-5 moment worth writing down rather than
around. `pdftoppm`, `mutool` and `gs` all draw its word in colour — mean channels 252.9 / 254.1 /
254.7 over the word's box, r < g < b in all three — so all three read `.` as zero, and this reader
now does not.

**The direction of inference runs one way.** Three implementations agreeing is evidence about a
*reading*, and the reading they share here is a habit rather than a clause: none of them can point
at a sentence admitting an object spelled `.`, because §7.3.3 states none. It is trap 9's first
shape one step over — not a shared gap producing a shared picture, but a shared *leniency* producing
one. This project's answer to that is written in `CLAUDE.md` principle 5 and in trap 5: find the
clause, and where the clause says the file states nothing, say so instead of inventing what it
would have said.

**The oracle's verdict does not move**, which is worth knowing and is not the argument. The word is
forty pixels wide on a 612 × 792 page, so the page stays inside the text tolerance and inside the
905 that agree; the totals are unchanged at 905 / 68 / 786. Had it moved, the answer would have been
the same one.

## What was not taken

**A page tree node with no `/Kids`**, `doc/todo/03` §7's first item, which is a different clause
(§7.7.3.2, Table 30) in a different crate and wants a population nobody has counted. It stays where
it is, and the hand-built corpus is down to **four** files blank in silence, two of them rightly so.
