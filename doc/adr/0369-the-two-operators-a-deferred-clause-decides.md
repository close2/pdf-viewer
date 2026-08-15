# ADR 0369 — The two operators a deferred clause decides, and the tie nobody had chosen

Status: accepted, 2026-08-15. Session 534. Fixes §7.10.5.2's `round` and `eq`/`ne`, audits the
rest of Table 42 and writes down which of its answers are *choices*; amends §7.10.5 and
§7.10.5.2's ledger rows. The two defects were found by the quorra team reading
`crates/pdf-model/src/function.rs` to build a device-side evaluator, and reported in
`doc/QUORRA_FUNCTION_PAINT_ANSWER.md` section 6.

## The clause, and the fact that it does not state one

§7.10.5.2 lists the operators and then hands their meaning away:

> The PostScript Language Reference, Third Edition shall define the semantics of these operators
> and all other syntax rules of the PostScript language. Although the semantics are those of the
> corresponding PostScript language operators, a full PostScript language compatible interpreter
> is not required.

**This is the load-bearing sentence of the whole round, and nothing in this tree had written it
down.** The ledger row for §7.10.5.2 described Table 42 as "the whole language" and said what the
stack bound was; it did not say that the clause states no semantics whatever. So a reading of the
operators had never been taken *against anything*, and two of them were wrong in the direction of
whatever Rust made convenient.

**This project does not hold the PostScript Language Reference.** `CLAUDE.md` principle 5 forbids
quoting a document one does not have as though one did, and so nothing in this round's code or
documents quotes PLRM3 — not one sentence, not one phrase. What is quoted is ISO 32000-2, which is
the document the tree holds.

That leaves a question the project has a rule for, and the rule needed extending by one step.
Principle 5 says that where the standard **defines nothing**, a deliberate choice is made and
documented *as a choice*. Here the standard defines something **elsewhere**, and the elsewhere is
out of reach. The consequence is the same and is now written into `apply_operator`'s own
documentation: **where the deferral is the only answer available, the reading goes in as a choice
with what it rests on, never as a derivation.**

What the standard *does* state about each operator is Annex B, which §7.10.5.3 points at:

> Annex B, "Operators in Type 4 Functions", contains a summary of these operators.

Annex B says **(informative)** on its own title line, and it gives each operator one line. Read
against the arms, that line turns out to settle more than expected and less than needed, and the
audit below is the split.

## The two defects

### `round` took a tie away from zero

§B.2's line is the whole of the ambiguity:

> Round num 1 to nearest integer

A value exactly halfway between two integers is nearest to both. The arm was `f32::round`, which
is half **away from zero** — a rule about how a number is *written* rather than about the number —
so `-6.5` answered `-7` and `-1.5` answered `-2`.

PostScript takes a tie to the **greater** of the two. That is the direction the deferral points
at, and it is not the only reason to take it: half-away-from-zero puts a discontinuity at the
origin, where `-1.5` and `1.5` are rounded by two different rules, and a type 4 program's inputs
cross zero as an ordinary matter — §7.10.5.3's own example function has a `/Domain` of
`[-1.0 1.0 -1.0 1.0]`. Half-toward-greater is a function of the value; half-away-from-zero is a
function of the sign written in front of it.

`round_to_greater` tests the tie against the floor rather than computing `(value + 0.5).floor()`,
and the reason is in the code: adding a half to a value whose exponent is large enough rounds
before the floor sees it, so `8388609` would come back as `8388610`. The test carries both.

**The quorra observation worth keeping**, because it is a fact about the other implementation
rather than about this one: WGSL's `round` is half to **even**, which agrees with half-toward-
greater at `-6.5` and disagrees at `2.5`. A generated shader is therefore not this function, and
a device-side evaluator that wants to be exact against this processor has to say so at `2.5` as
well as at `-6.5`.

### `eq` and `ne` compared within an epsilon

§B.3 gives each of them a relation and no tolerance — "Test equal", "Test not equal" — and
PostScript's `eq` is exact. The arms were `(a - b).abs() < f32::EPSILON` and its complement.

The quorra reading of what that tolerance actually did is the part worth keeping, and it is worth
being precise about: `f32::EPSILON` is the gap between `1.0` and its successor. Near zero it is
enormous — every value below 1.2e-7 was equal to every other and to zero, which is millions of
distinct floats collapsed into one. Above about 8.4 million it is smaller than one unit in the
last place, so the comparison was exact anyway. **It was loosest exactly where a type 4 program
tests a boundary and tightest where nothing needs it**, which is the opposite of what an epsilon
comparison looks like it does. Both arms are now `==` and `!=`.

## The audit of the rest of Table 42

An operator set that was wrong twice is a set to audit rather than to spot-fix. Every entry was
read against §B.2, §B.3 and §B.5, and each one now carries either its clause line or its choice.
Nine tests were added; the audit found no third defect, and it found three answers that are
choices and were not marked as such.

**Settled by Annex B, and correct as they stood** — each now with the line beside it and a test:

- `atan` — "Return arc tangent of num / den in degrees". Two operands rather than one ratio, which
  is why `atan2` is the right primitive: a quotient loses the quadrant, and `-1 -1 atan` would
  answer `45` instead of `225`. The circle is closed by adding a turn to a negative answer, and a
  sweep of both operands including the origin stays inside it.
- `idiv` and `mod` — "as an integer" and "remainder after dividing int 1 by int 2". The truncating
  quotient and the remainder that follows its **dividend** are one convention, not two: `-7 2 idiv`
  is `-3` and `-7 2 mod` is `-1`, and the test reconstructs the dividend from the pair.
- `cvi` and `truncate` — "Convert to integer" and "Remove fractional part". Two words for one
  arithmetic here, because the difference in PostScript is of *type* and this stack has none.
- `and`, `or`, `xor` — "Perform logical | bitwise …": two operators sharing a name, and **they need
  no discrimination, which is a property of the representation rather than luck**. A boolean here
  is `1.0` or `0.0`, and over {0, 1} the bitwise operation *is* the logical one, for all three.
- `roll` — §B.5's "Roll n elements up j times". *Up* is toward the top, which is a right rotation
  of a window whose last element is the top, and §B.5's own `mod` in the result column is what
  makes a negative `j` the same operation rather than a different one.
- `floor`, `ceiling`, `abs`, `neg`, `add`, `sub`, `mul`, `sin`, `cos`, `exp`, `sqrt`, `ln`, `log`,
  `dup`, `exch`, `pop`, `copy`, `index`, `true`, `false`, `ge`, `gt`, `le`, `lt`, `if`, `ifelse` —
  each read, none changed.

**Not settled, and now stated as choices where they are made:**

- **`bitshift`'s width.** §B.3 fixes the direction — "Perform bitwise shift of int 1 (positive is
  left)" — and nothing else. A right shift of a *negative* value parts on a number ISO 32000-2
  never states: the width of the integer. Filling from the left with zeros answers `2147483646`
  for `-4 -1 bitshift` at 32 bits and `9223372036854775806` at 64, so that convention requires
  choosing a width, and Annex C's "Integer values (such as object numbers) can often be expressed
  within 32 bits" is informative and about object numbers. **The choice is the sign-preserving
  shift**, on the ground that it is the only one of the two that is a function of the value rather
  than of an unstated register: `-4 -1 bitshift` is `-2` under it whatever the width.
- **An error the subset cannot express.** `div`, `idiv` and `mod` by zero, and `ln`, `log` and
  `sqrt` outside their domains, are errors in PostScript, and §7.10.5.1's subset is "Expressions
  involving only integers, real numbers, and boolean values" — there is no value that means *error*.
  The answer is `0`, and the reason it is not an infinity is that an infinity does not stay put: it
  becomes a colour component, then a coordinate. This was already the behaviour; what it lacked was
  the clause.
- **`not`.** §B.3 makes it two operators wearing one name, and unlike `and`, `or` and `xor` the two
  **disagree** on {0, 1}: the one's complement of `1` is `-2`, and of `63` is `-64`.
  `Instruction::Push` carries no type, so this evaluator can implement only one and implements the
  logical one. **This is a known incompleteness and not a reading**: quorra raised it as contract
  question 3 and `doc/QUORRA_FEEDBACK.md` section 25.5 already answered it — a type on every
  compiled literal, inferred statically. That changes a public type and belongs to that work rather
  than to this round; the test pins today's answer and says so.

One thing the audit found that is neither a defect nor a choice, and is worth the sentence: `exp`
is the only arm that produces a non-number deliberately, because a negative base with a fractional
exponent has no real answer. It cannot escape: §7.10.5.3 makes `/Range` **required** for a type 4
function, and the range clamp maps a `NaN` to a bound. That is now a test through a real
`Function` rather than through the calculator, because the calculator is the half of the path
without the clause in it.

## What the defects cost in the wild: nothing, and the measurement is the point

`crates/pdf-model/examples/type4_operator_census.rs`, over the pdf.js corpus, the four corpora,
the owner's two files and the whole SafeDocs and openpreserve caches — 67 462 paths, 7 353 type 4
functions in 2 099 documents:

| operator | functions reaching it | documents |
|---|---:|---:|
| `round` | **0** | 0 |
| `ne` | **0** | 0 |
| `bitshift` | **0** | 0 |
| `not` | **0** | 0 |
| `eq` | **1** | 1 |
| `ceiling`, `ln`, `log`, `true`, `xor` | 0 | 0 |
| `exch` | 6 773 | 1 872 |
| `sub` | 6 537 | 1 745 |
| `pop` | 3 508 | 1 696 |
| `roll` | 3 435 | 1 780 |
| `index` | 3 238 | 1 650 |
| `cvr` | 2 950 | 1 541 |

The single `eq` is in `doc/corpora-own/pi_seven_segment.pdf` — the project owner's own file,
written by hand — and its value does not move: both arms were evaluated over a nine-point grid of
its `/Domain` and agree at every sample. The page is byte-identical before and after and still
reads 3.141.

**So the two defects were unreachable by every document this project has**, and that is the
finding rather than an anticlimax. Trap 8 states the rule and this is the cleanest instance of it
in the tree: a corpus finds what documents contain, not what the standard says. Sixty-seven
thousand files could not have found either defect; one person reading the source against the
clause found both. The inverse of "a count that improves is not a picture" — a count that does not
move is not evidence that nothing happened.

It also says something about the *shape* of a real type 4 program, which is worth having: the
population is dominated by `exch`, `sub`, `pop`, `roll`, `index` and `cvr`, which is a
`Separation` or `DeviceN` tint transform shuffling components and doing linear arithmetic. Only
324 of 7 353 use `if` at all. The transcendental operators quorra's classification would refuse —
`sin`, `cos`, `atan`, `exp`, `ln`, `log`, `sqrt` — have per-operator counts summing to fifteen, so
at most fifteen functions of 7 353 reach one, which is a useful number for that work.

## Why the census is exact rather than approximate

The "before" arm is a **source rewrite** — the method ADR 0361's census established — and here it
can be exact, because each old operator is expressible in Table 42 using operators this round did
not touch. `eq` was `(a - b).abs() < f32::EPSILON`, which is `sub abs <eps> lt` to the bit, with
the epsilon written out of `f32::EPSILON` itself so no constant is retyped. `round` differs from
the fixed one at exactly one place — a value that is an exact tie *and* negative — so the old
operator is the new one minus that condition, and the condition is `floor`, `sub`, `eq`, `lt` and
`and`. The instrument was validated on seven synthetic files first: a negative tie moves, a
positive tie does not, an `eq` against 1e-8 moves, an `eq` against 0.5 does not.

## What did not move, proven rather than asserted

A function's value decides a colour, so pixels were allowed to move here. None did, and three
independent instruments say so:

- `display_list_digest` over all 974 pdf.js documents, before and after in one working copy with
  the patch the only variable: **`diff` empty**, 974 lines plus the summary.
- Every gate green and every number identical to session 532's post-bump baseline — the corpus's
  974/64 incomplete, the oracle's 906 agrees / 67 contradicted / 786 ambiguous over 1 794 pages,
  quorra at scale 1 on 931/23/2/18 and the `gpu` lane at 4× on 937/9/5/23.
- The one witness in the whole population, rendered at 2× before and after: byte-identical.

`doc/todo/00`'s step 7 ink sweep is therefore not owed: it is for a round that changes what gets
drawn, and this one is measured not to have.

## Consequences

- Two arms of `apply_operator` compute what §7.10.5.2's deferral says, and the file now states
  where its semantics come from and where the standard stops.
- Three answers are marked as choices with their grounds, so a later round with the PostScript
  Language Reference in hand can check three specific sentences rather than re-audit the table.
  Two of the three are unreachable by any document this project has, and the census is how a
  future round learns that cheaply.
- `not` stays incomplete, deliberately, with the work it belongs to named. A test pins what it
  answers today so that the type-tag round moves a number rather than discovering one.
- The ledger's §7.10.5.2 row carries the deferral. The row that described a clause's *content*
  while missing that it has none is the shape to look for elsewhere: a clause family can be
  `implemented` because everything it names is implemented, and still have never been read.
