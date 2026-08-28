# ADR 0733 — What the lexer admits that §7.3.3 does not

Status: accepted, 2026-08-28. Session 800. Amends `pdf_syntax::lexer::read_number` and
`pdf_model::function::compile_token`, and adds `pdf-model/examples/numeric_form_census`.
Successor to ADR 0303 (a run holding no decimal digit is the keyword it lexically is), which
took the same shape out of the same function one condition earlier; sibling of ADR 0341, ADR
0370 and ADR 0424, which are about the speed of this path and not about what it accepts.

## The subject

Round 796 recorded, and did not take, a one-line finding: `sci-notation.pdf` writes `/F1 1e2 Tf`
and this lexer reads `1e2` as a hundred through a `parse::<f64>()` fallback, which §7.3.3 admits
in neither of its numeric forms. What made it worth a round rather than a comment is where it
sits — directly under two blockquotes of the clause that are both about decimal digits — so the
code said one thing and did another, in the one function every number in every PDF goes through.

Taking it properly meant three questions, in order: what the clause actually admits, what this
lexer actually admits, and what a reader owes each difference.

## 1. What the clause admits

§7.3.3 states the two forms as sentences and Errata Collection 3 **closes** them as a grammar.
Issue #327 adds a railroad diagram above each of the clause's two EXAMPLEs; both were read this
session by decoding them out of `doc/md/`, because they are images and no round had looked at
them. The integer figure is an optional `+` or `-` followed by one or more `DecimalDigit`. The
real figure is an optional sign followed by either a PERIOD and one or more digits, or one or
more digits, a PERIOD and zero or more digits. **There is no other production in either figure**
— no exponent, no radix, no second sign, no separator.

**Neither instrument put that figure in front of a round, and that is worth recording.**
`spec-errata check` cannot see it — it compares quotations this tree has written against struck
passages, and Issue #327 struck nothing. `spec-errata emit` does name it, and files it on **page
1004**, in the errata appendix at the back of the document, under a heading of its own rather than
under §7.3.3 — as two `FileAttachment` annotations whose whole printable content is their titles,
*Issue #327: EBNF definition of PDF integer object* and *… of PDF real object*. The substance is an
embedded file and an image, so the only way to read it is to decode the figure out of `doc/md/` and
look at it, which is what this session did. A round working on a clause reads `emit` in clause
order and will not scroll 960 pages for it. (Left for `doc/errata-read.md` to absorb rather than
written there, because the round beside this one owns that file.)

The prohibition is stated once more in prose, and the party it binds is what decides this round:

> A PDF writer shall not use the PostScript language syntax for numbers with non-decimal radices
> (such as 16#FFFE) or in exponential format (such as 6.02E23).

That is a `shall not` on the **writer**. The clause states nothing whatever for a reader that
meets one — while naming the syntax and glossing its value in the same breath, so the meaning of
`6.02E23` is the standard's own words rather than a convention we would be importing.

The other reader-facing sentence is about magnitude rather than spelling:

> The range and precision of numbers may be limited by the internal representations used in the
> computer on which the PDF processor is running; Annex C, "Advice on maximising portability",
> gives these limits for typical implementations.

Annex C is **informative** and gives no figure: Table C.1 says integers "can often be expressed
within 32 bits" and that reals are often IEEE 754 single or double. So the clause permits a limit
and leaves its value entirely to the processor — which turns out to be the whole of the second
finding below.

## 2. What this lexer admits, by experiment

Enumerated by running the lexer rather than by reading `str::from_str`'s documentation:

| written | read as | inside the grammar? |
|---|---|---|
| `1e2`, `1E2`, `6.02e23`, `1e-2`, `1e+2`, `-1e2`, `.5e1`, `1.e2` | the exponent's value | no — the form the `shall not` names |
| `1e400`, `-1e400` | `Integer(0)` | no, and beyond a double |
| four hundred `9`s; the same with `.5` after it | `Integer(0)` | **yes** — a conforming §7.3.3 number |
| `inf`, `Inf`, `infinity`, `-inf`, `NaN`, `nan` | `Keyword` | no, and correctly refused |
| `1_000`, `0x10`, `1f64`, `16#FFFE`, `1e` | the leading numeric prefix | no — the standing salvage |
| `1e-400` | `Real(0.0)` | no, and zero is the correctly rounded value |

Two of Rust's three extra spellings never arrive: `inf` and `NaN` hold no decimal digit, so
ADR 0303's condition returns each as the keyword it lexically is, one line above the parse.
Underscores and radix prefixes do not arrive either — `str::parse` refuses them and the salvage
takes the leading prefix. **The exponent is the only Rust-ism that reaches a value**, which is a
narrower answer than "we use the standard library's parser" would suggest and is worth having
written down.

## 3. What a reader owes each difference

### The exponential format: read it, and say so

Three answers were available — refuse, accept silently, accept and report — and the clause
decides between them rather than the corpus.

**Refusing adds a requirement the standard does not state.** The `shall not` binds the producer;
nothing asks a reader to lose a mark over it. And refusing is not neutral: `1e2` becomes a
keyword, the `Tf` that wanted it is refused, and a font size the file states unambiguously is
gone. Against `CLAUDE.md`'s "every PDF that exists renders as its producer specified", that is a
worse answer than reading the value the clause itself glosses.

**Reporting would be a false report.** `Unsupported` is defined as "[s]omething the interpreter
met but could not draw"; here everything is drawn and drawn correctly. Trap 11's rule — a report
is only as good as the condition it fires on — rules this out, and ADR 0152's arithmetic adds the
price: every report costs the oracle a judged page, for a page with nothing wrong with it.

So: **accept, and make the acceptance visible where it happens.** The tolerance is now stated in
`read_number` beside the parse that grants it, with the clause quoted and the party it binds
named. This is the §7.3.10 answer one clause family along, and it is the same one
`function::compile_token` had already reached independently for a type 4 function's operands —
two places, one reading, and neither of them knew about the other until this session.

The instrument that replaces the report is `pdf-model/examples/numeric_form_census`. It splits
every regular run in every stream a lexer is ever pointed at — page `/Contents`, form `XObject`s,
CMaps, calculator functions and §7.5.7's object streams — into the clause's two forms, the
exponent, the radix and the rest, classifying against §7.3.3's grammar written out in the example
rather than by asking the lexer (trap 8). Its answer: **one** exponential run in the whole pdf.js
corpus, `sci-notation.pdf`'s, and four more in an 8300-document sample of the crawl, all in
`govdocs1-error-pdfs`. The form is real and it is vanishingly rare, which is why the decision had
to be made on the clause.

### A magnitude beyond a double: the representation's limit, never zero

This is the finding round 796's note did not contain, and it is the larger of the two.

`text.parse::<f64>()` returning an infinity was mapped to `Token::Integer(0)`, and
`salvage_number` filtered a non-finite value to `None`, which the caller also answers with
`Integer(0)`. **Zero is the worst value available.** It is the smallest magnitude where the
largest was written; it inverts the ordering of every comparison the number then takes part in;
and — unlike a refusal — it *draws*: a coordinate at the origin, a font size of nought, a width
of nothing, in place of a mark the producer had put off the sheet. It is precisely the plausible
fallback trap 5 forbids and precisely the shape ADR 0303 removed from the run holding no digit at
all, surviving one condition below it for as long as this function has existed.

And the run reaching it is not always malformed. Four hundred decimal digits with no sign of
PostScript in them is a **conforming** §7.3.3 integer, so this was not a reader's tolerance of a
bad file — it was a conforming number silently replaced by a different one.

The answer is the clause's own: the range "may be limited by the internal representations used in
the computer", so `within_the_representation` returns the largest finite double carrying the sign
the file wrote — the nearest value the representation holds to the one stated. A refusal was
considered and declined for the reason above: saying "this is not a number" of four hundred
decimal digits would be false. A magnitude too *small* is untouched, because zero is the correctly
rounded value of `1e-400` rather than a substitute for it.

**Nothing in the corpus moves.** 964 pdf.js documents and an 8300-document sample of the crawl
state over seven hundred million runs in the clause's two forms and **not one** of them overflows
a double, and
`display_list_digest` over all 974 first pages is byte-identical across the change. That is the
reason it survived: it costs nothing measurable until the day a file states such a number, and
then it costs a mark in silence.

## 4. The neighbours, swept

Every `parse::<…>` and `from_str` in `pdf-syntax`, `pdf-model` and `pdf-font` was read against the
§7 grammar it stands for. The answer is narrower than the question suggested: **all but two of
them already gate the grammar themselves** before handing anything to Rust —
`date.rs`'s §7.9.4 fields (all `is_ascii_digit`, at fixed widths), `write.rs::startxref`,
`collection.rs`'s §12.3.5.2 folder key, `structure.rs`'s `Hn`, and `fragment.rs::number`, which
spells §7.3.3's grammar out and refuses the exponent *deliberately*, on the reasoning that
"[n]othing has to draw here". §7.3.4.3's hexadecimal strings and §7.3.5's `#`-escapes have their
own `hex_value` and never see a Rust parse at all.

The two that did not are `lexer::read_number`, above, and:

**`function::compile_token` — §7.10.5.2, and a silent one.** The clause sends the operand syntax
to §7.3.3 ("The operand syntax for Type 4 functions shall follow PDF conventions rather than
PostScript language conventions"), both of whose forms are one or more decimal digits, and Table
42 names no operator `inf`, `infinity` or `NaN`. `compile_token` tried `f32::from_str` on every
token, so `{ inf }` compiled to a *pushed infinity* — neither a literal the clause has nor an
operator the table has — which leaves through the function's arithmetic as a `NaN` in a colour
component instead of refusing the function. Requiring a decimal digit is §7.3.3's own test and
sends such a token to the operator match, where it is refused and reported like any other unknown
one. A magnitude past `f32` keeps §7.3.3's answer and is bounded rather than refused, which is the
same reading `within_the_representation` makes of the same sentence.

## Consequences

- The exponential form is a **documented** tolerance rather than an undocumented one, stated
  where it is granted, and its population is countable rather than assumed.
- A number no double can hold is answered with a number, in the right direction, instead of with
  zero.
- A type 4 function stating `inf` is refused and reported instead of evaluating to a `NaN`.
- One instrument gained: `numeric_form_census`, which is the thing to run before anybody argues
  about §7.3.3's forms again.
- What is **not** gained is a report, deliberately, on trap 11's and ADR 0152's grounds: a page
  whose numbers are read exactly as the producer meant them has nothing to report.
