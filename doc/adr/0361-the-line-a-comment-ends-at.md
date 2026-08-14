# ADR 0361 — The line a comment ends at, and the rule this tree implemented twice

Status: accepted, 2026-08-14. Session 526. Fixes §7.10.5's tokenizer, which read a comment by
guessing at its length; amends §7.2.4, §7.10.5 and §7.10.5.1's ledger rows. The witness is the
project owner's `doc/corpora-own/type4_pi.pdf`, the first PDF bytes this repository tracks.

## The report

```
$ target/pdf-retrieve page doc/corpora-own/type4_pi.pdf 0
  "unsupported": ["Shading { name: \"/Sh1: malformed shading: malformed function: unknown operator Math\" }"]
```

The file's type 4 program opens `% BBP Math for Pi (leaves 3.141 on stack)`. There is no operator
called `Math`, and there is no operator called `BBP` either — what refused was a *comment*.

## The clause, first

**§7.10.5.1 admits comments outright**, in the list of what the language is:

> A Type 4 function ( PDF 1.3 ), also called a PostScript calculator function, shall be
> represented as a stream containing code written in a small subset of the PostScript language.
> This subset is comprised of the following PostScript language features:

and the five entries are: expressions over integers, real numbers and booleans; **comments**; "No
composite data structures (such as strings or arrays)"; no procedures; no variables or names. So
this is not a question the standard leaves open and not a de-facto convention to be adopted with a
documented choice — a program with comments in it is a conforming program, and the code was already
*trying* to accept one. Its bug was arithmetic about lines, not policy.

**§7.2.4 says how long a comment is**, and its extent is the whole defect:

> Any occurrence of the PERCENT SIGN (25h) outside a string or inside a content stream (see 7.8.2,
> "Content streams") introduces a comment. The comment consists of all characters after the
> PERCENT SIGN and up to but not including the end-of-the-line marker. A PDF processor should
> ignore comments. PDF processors shall treat comments as single white-space characters for the
> purposes of lexical conversion (see 7.2, "Lexical conventions"). That is, a comment separates the
> token preceding it from the one following it.

Two requirements, and the old code met neither: a comment ends at a **line**, and what it leaves
behind is **white space** rather than nothing. §7.2.3 supplies the marker — a CARRIAGE RETURN, a
LINE FEED, or the two together — and Table 2 supplies the delimiters, PERCENT SIGN among them, with
the braces §7.10.5 adds noted in the same paragraph.

§7.10.5.2 is the third clause in the reading and it is why the *operator* refusal was loud rather
than silent: Table 42 is a closed list, and `Math` is not in it.

## The defect, and the two shapes it had

`compile_postscript` spaced `{`, `}` and `%` apart from their neighbours, split the **whole stream**
on white space, and then — with the line structure already gone — skipped exactly one token after
each `%`. Its own comment said so: "a comment runs to end of line, which whitespace splitting has
already destroyed; skipping one token is the closest safe approximation."

It is not a safe approximation. It is two different wrong answers depending on what the comment
says:

- **Loud.** `% BBP Math for Pi` → `BBP` skipped, `Math` compiled, function refused, shading
  reported, page blank. This is how the defect was found, and it is the good case: trap 5's
  loudness worked.
- **Silent.** A comment that quotes the arithmetic it documents — `% dup 3 mul`, `% add 1`, or the
  owner's own `% Rect 10,25 85,95` had it been spelled with operators — leaves words that are *all*
  valid tokens, and every one of them was compiled into the program. No error, no report, and a
  function that computes something the file never stated. In a shading or a `Separation` tint
  transform that is a plausible picture in the wrong colours.

## The fix

Comments are cut **before** the split, line by line: each line ends at its first PERCENT SIGN, and
a LINE FEED is written in the comment's place so that the tokens either side stay apart — which is
§7.2.4's own second sentence rather than a nicety. `without_comments` is eleven lines and carries
the reading above.

**Cutting at the first PERCENT SIGN is safe here and would not be in PostScript**, and the licence
comes from the same list that admits the comment: with "No composite data structures (such as
strings or arrays)" there are no string literals, so a PERCENT SIGN in a type 4 program cannot be
anything but the start of a comment. In general PostScript it can, which is why this is written
down beside the code rather than assumed.

The `"%"` arm of `compile_block` is gone: no PERCENT SIGN survives tokenisation now, and a rule
enforced in one place cannot disagree with itself.

## The population, measured

`cargo run --release -p pdf-model --example type4_comment_census` classifies every type 4 program
by what the old rule did to it, running both arms through the *current* compiler and reproducing the
old rule as a text transform. Over `doc/pdf.js`, the four `doc/corpora` submodules, `doc/corpora-own`
and the whole `SafeDocs` cache — **67 461 documents, 7 352 type 4 functions in 2 098 of them** — two
programs contain a PERCENT SIGN:

| | verdict | what it is |
|---|---|---|
| `doc/corpora-own/type4_pi.pdf` object 6 | `refused` | the witness |
| `SafeDocs` cc-main-2021-31 `5097152.pdf` object 19 | `harmless` | a CMYK tint transform |

**Nought silently mis-compiled, and the reason is worth more than the number.** A producer does not
comment a generated tint transform, so the population that could be damaged is *hand-written
programs* — and the second file shows what a real one looks like:

```
{
0 %c
0 %m
0 %y
3 index %k
5 -1 roll pop
}
```

Every comment is one word, which is exactly the case where skipping a token and cutting a line
agree; that is how the old rule survived five hundred sessions. **And its line endings are
CARRIAGE RETURNs with no LINE FEED in the file at all** — so a fix that had cut at the first
PERCENT SIGN of each LINE-FEED-delimited line would have read this whole program as one comment and
returned `{ 0`, silently, four outputs short. It is now a case in
`a_comment_runs_to_the_end_of_its_line`.

So the honest statement of the cost is: **the corpora contain no page this defect silently drew
wrong, and no measurement can say that of the files this program has not been shown.** The silent
half is real — it is four assertions in `a_comments_words_are_not_instructions`, each failing when
the old rule is reinstated — and what the census establishes is that it was waiting rather than
firing. A count of nought over a population is a fact about the population; the corpus finds what
documents contain, not what the specification says (trap 8), and the specification says a comment
may be any words at all.

## What was looked at

The witness renders `3.141` as ten rectangles — three bars and a stem for the `3`, a full stop, a
stem, a `4`, a stem — black on white, and the page was looked at rather than counted (trap 1). The
program's colour is `dup sub` inside the shape and `dup div` outside, so the digits are the
function's own value proving it is non-zero, and the value is checkable by arithmetic:

    4/1 − 2/4 − 1/5 − 1/6                       = 3.1333333…
    (4/9 − 2/12 − 1/13 − 1/14) / 16             = 0.0080891…
    sum                                          = 3.1414224…   (π − 1.70 × 10⁻⁴)
    1000 mul truncate 1000 div                   = 3.141

`the_owners_bbp_series_evaluates_to_pi_to_three_places` runs that head through the compiler and
asserts the 3.141; `the_owners_pi_file_paints_the_digits_its_program_computes` samples five digit
strokes and three background points, so it fails on a page that is blank, inverted, mirrored or
shifted.

## The fixtures

Per trap 8, a **pair** in `crates/test-scenes/src/type4.rs`: one program, written out twice, with
comments and without, painting one function-based shading. Three things about it are deliberate.

- **Both arms are literals.** Deriving the bare arm by running this tree's own comment stripper
  over the commented one would compare the compiler with itself.
- **The commented arm carries both shapes**, prose and quoted formula, so it fails loudly *and*
  silently under the old rule.
- **The test asserts the ink as well as the equality.** Two arms that both refused to draw are also
  identical, which is precisely what the old code did to the loud shape.

## Nothing else moved, and that is measured rather than argued

`display_list_digest` over all 974 corpus documents is **byte-identical** before and after — the
same command count, `Debug` length and hash on every line — which is the demonstration
`doc/todo/02` asks of a change to interpretation, and it is stronger than the gates' summaries
because two different lists can rasterise to one verdict. The census says why: no corpus document
carries a commented type 4 program. So no page in the corpus or the oracle changes, `doc/todo/00`
step 7's ink sweep has nothing to sweep, and the quorra lanes have nothing to compare.

## The lesson

**A rule implemented twice is a rule that can be right once.** §7.2.4 has been implemented
correctly in `pdf-syntax`'s lexer since the beginning — `skip_whitespace` skips comments and white
space together *because* the clause makes them one thing — and the ledger row said so and named
that file. The type 4 compiler is a second lexer for a second grammar, it never went through
`lexer.rs`, and nobody had asked the clause the second time. The row now names both.

Two smaller ones:

- **A comment in the code admitting an approximation is a defect report nobody filed.** The line
  "skipping one token is the closest safe approximation" was written by somebody who knew the rule
  and could not reach it from where they stood. The answer was to move the work earlier — strip
  before splitting — rather than to approximate better.
- **The project owner's hand-written file found in one page what 67 461 documents could not.**
  Generated files exercise the constructions generators emit. A specification's constructions are a
  larger set, and trap 8's hand-built fixture is the only instrument that reaches the difference.
