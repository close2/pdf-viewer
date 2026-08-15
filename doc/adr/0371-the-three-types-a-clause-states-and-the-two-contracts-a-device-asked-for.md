# ADR 0371 — The three types a clause states, and the two contracts a device asked for

Status: accepted, 2026-08-15. Session 536. Gives §7.10.5's operand stack the three types
§7.10.5.1 states, which fixes `eq`'s reading of a boolean and completes `not`, `and`, `or`, `xor`,
`cvi`, `cvr` and `truncate`; answers the two contract questions the quorra team left with this tree
in `doc/QUORRA_FUNCTION_PAINT_BUILT.md` §3; amends §7.10.5, §7.10.5.2 and §7.10.5.3's ledger rows.
**Supersedes ADR 0369's `not` entry**, which recorded a known incompleteness and named this work as
where it belonged.

## The defect, run rather than argued

`{ true 1 eq }`, through a real `Function` with a `/Range` wide enough not to clamp:

```
{ true 1 eq }   ->  1.0        before this round
{ true 1 ne }   ->  0.0        before this round
{ 63 not }      ->  0.0        before this round, where §B.3's integer operator says -64
```

A boolean was stored as `1.0`, so a numeric comparison could not tell it from the number 1. **The
quorra team found this in their own wave-1 device-side evaluator by running this tree's corpus
against it**, and reported that ours had the same shape (their §5, third item). Nothing in this
tree's 67 690-file population could have found it — see the census below — and the same was true of
both defects ADR 0369 fixed.

## The reading, and how much of it is the standard's

ADR 0369 established the rule this round works under: §7.10.5.2 states **no** semantics, it defers
them to the PostScript Language Reference, and this project does not hold that document, so an
operator's meaning goes in as a documented **choice with its ground** rather than as a derivation.

What this round found is that a great deal more of the answer is in ISO 32000-2 than the deferral
suggests, and it is in Annex B's **operand and result columns** rather than in its one-line
descriptions. Annex B is informative and this tree holds it:

| §B.3's line | what it decides |
|---|---|
| `any 1 any 2 eq bool` | `eq` is defined on **every** object, so it must answer *across* the three types rather than through them — a boolean is not a number and the two are never equal |
| `num 1 num 2 gt bool` | the four ordering operators do **not** admit a boolean at all |
| `bool 1 \| int 1 … and bool 3 \| int 3` | `and`, `or` and `xor` answer in the type they were given |
| `bool 1 \| int 1 not bool 2 \| int 2` | `not` is two operators, and unlike the three above they disagree |
| `int 1 shift bitshift int 2` | `bitshift` is on integers |
| `bool { expr } if` | a condition is a boolean |
| `num cvi int` against `num 1 truncate num 2` | `cvi` **converts**; `truncate` keeps its operand's type |

None of that is implementable on a stack of `f32`. §7.10.5.1 says what the stack holds and it is
three things — "Expressions involving only integers, real numbers, and boolean values" — so the
stack now holds them: `Value::Integer(i32)`, `Value::Real(f32)`, `Value::Boolean(bool)`.

**Which literals are integers is answered by the standard outright**, and it is the one place
§7.10.5.2 does not defer: "The operand syntax for Type 4 functions shall follow PDF conventions
rather than PostScript language conventions." §7.3.2's integer is digits with an optional sign;
§7.3.3's real carries a PERIOD. So `63` is an integer and `63.0` is a real, and `63 not` answers
`-64` because the *file* said so — nothing is inferred, which is the difference between this and
the static slot-typing `doc/QUORRA_FEEDBACK.md` §25.5 had promised.

### The one rule for an operand of a type its operator does not admit

PostScript raises `typecheck`; §7.10.5.1's subset has no value that means *error*. This is the same
shape as `div` by zero, which ADR 0369 already answered with a value, so it gets one rule rather
than a second family of decisions:

> Such an operand is **converted** by the reading that loses least: a boolean is the 1 or 0 it
> stands for where a number is wanted, a number is false exactly when it is zero where a boolean is
> wanted, and a real is truncated where an integer is wanted. `eq` and `ne` are outside the rule
> because there is nothing to convert — `any 1 any 2` admits both types already.

One of those three directions is not a choice at all, and separating it out is worth the sentence:
§7.3.3 states that "[w]herever a real number is expected, an integer may be used instead", so an
integer under `sin` is an ordinary operand. The other two are the error that clause's next sentence
names — "A real number shall not be present when an integer is expected" — and a file that does it
anyway is a file this viewer still has to draw.

## The two contract answers

Both were quorra's, both are ours to decide, and both are now pinned by a test so the two
evaluators cannot drift apart on them.

### (a) A pop from an empty operand stack is the integer `0`

`unwrap_or(0.0)` could not express a type, and seven operators can now tell one. §7.10.5.3 makes an
underflowing program malformed twice over, and refusing it would refuse a document that draws —
`doc/corpora-own/pi_seven_segment.pdf` is hand-written, reads an empty stack, and renders. So it is
a value, and the integer is chosen over the other two for a reason each:

- over the **real**, because §7.3.3 makes an integer usable wherever a real is expected while the
  reverse is an error, so of the two numeric types it is the one that is an operand everywhere;
- over the **boolean**, because a `false` would silently satisfy `if` and `not` — the two operators
  that decide what the *rest* of the program does — where an integer only feeds the arithmetic.

It agrees with what quorra built, which matters for a different reason than agreement usually does:
a program that falls back from the device to this evaluator must draw the same page.

**This side does not report it, and the reason is not reluctance.** They can count underflows
statically because they refuse a `copy`, `index` or `roll` whose count is not a constant; this
evaluator admits those, so the depth is not a static quantity here, and a report per evaluation
would be one per device pixel of a shading (ADR 0339).

### (b) `gt`, `ge`, `lt` and `le` read a boolean as the number it stands for

They are typed `num 1 num 2`, so PostScript refuses a boolean and the subset cannot express the
refusal. The answer is the general rule above: `true 0 gt` compares 1 with 0 and is true.

The alternative — answering the zero of the operator's result type, so that it would be `false` —
was considered and declined for two reasons. It puts a second rule beside the one `div` by zero
already follows; and it replaces an answer that is a function of the operands with a constant,
which is the ground ADR 0369 gave for `bitshift`'s width and `round`'s tie. **Quorra offered to
refuse instead and does not need to**: a refusal falls back to this evaluator, which answers this,
so refusing changes nothing but the speed.

## What the types made observable, which is less than expected and worth knowing

Only the **boolean** is widely observable. Integer against real is observable through exactly one
thing — arithmetic exactness, because `add`, `sub` and `mul` of two integers now stay integers:

```
{ 16777216 1 add 1 add }     ->  16777218      integers, exact
{ 16777216.0 1 add 1 add }   ->  16777216      f32, and what this evaluator always answered
```

`1.5 truncate` and `1.5 cvi` differ in type and not in number, and no operator can see the
difference except through that. It is why the census below could be exact.

## The census: 7 360 functions, and not one moves

`crates/pdf-model/examples/type4_type_census.rs`, over the pdf.js corpus, the four corpora, the
owner's two files and the whole SafeDocs and openpreserve caches — 67 690 paths, **7 360 type 4
functions in 2 102 documents**, 44 seconds:

| shape | functions |
|---|---:|
| **compared arm against arm** | **7 360** |
| **whose value moved** | **0** |
| a boolean and an `eq` or `ne` — the defect's shape | **1** |
| an ordering operator with a boolean from elsewhere — answer (b) | **2** |
| `not` | 0 |
| `and`, `or`, `xor` | 2 |
| `idiv`, `mod`, `bitshift` | 2 |
| `cvi`, `cvr`, `truncate` | 2 952 |
| both kinds of literal / integers only / reals only / neither | 3 068 / 793 / 3 328 / 171 |

The one function that can put a boolean where an `eq` will see it is
`doc/corpora-own/pi_seven_segment.pdf` object 6 — the project owner's hand-written file, the same
witness ADR 0369's census found — and both witnesses for (b) are his two files. Its page is
byte-identical before and after and still reads 3.141.

**The "before" arm is a source rewrite and is a derivation rather than an approximation**, which is
what makes 0 a measurement: the old evaluator *is* this one with every value forced to a real. Every
integer literal is rewritten as a real, `true` and `false` become `1.0` and `0.0`, every operator
that now answers an integer or a boolean is followed by `cvr`, `eq` becomes `sub 0 eq cvr`, and
`not` becomes `0 sub 0 eq cvr`. The instrument was validated on ten synthetic files first — five
built to move and five built not to — and it moved exactly the five.

## What it costs, in instructions, and where it does not

`valgrind --tool=callgrind`, `RAYON_NUM_THREADS=1`, `callgrind_rasterise <file> <page> 3`, both arms
in one sitting with the patch the only variable:

| document | before | after | |
|---|---|---|---|
| `doc/corpora-own/type4_pi.pdf` p1 | 8 588 416 764 | 7 298 760 428 | **−15.02%** |
| `doc/corpora-own/pi_seven_segment.pdf` p1 | 8 196 872 963 | 7 775 167 523 | **−5.15%** |
| `function_based_shading.pdf` p1 | 1 355 805 012 | 1 440 192 463 | **+6.22%** |
| ISO 32000-2 p101 — **the control, no shading** | 986 012 537 | 986 020 965 | +8 428 instructions |

ADR 0364's two big type 4 documents are **faster** with a typed stack than without one, and the
first arrival of a regression was +22% on all three. Four things closed it, each measured:

1. **The integer is an `i32`**, so a `Value` is eight bytes rather than sixteen. ISO 32000-2 states
   no width; ADR 0369's answers are the ones that do not depend on one and they are unchanged. The
   width decides only where an integer stops being one — a sum past 2³¹ becomes a real, which is the
   direction the `f32` stack always went.
2. **The real arm is written first in every match**, since 3 328 of 7 360 programs write no integer
   literal at all, and `add`, `sub` and `mul` match on the operand *pair* — two reals, a real and
   something else from either side, and only then the integer case. `x 4 mul` is one test rather
   than four.
3. **A one- or two-operand operator writes its answer where its first operand already is**, rather
   than popping and pushing. §B.2's and §B.3's one- and two-operand lines all leave one value
   behind, so the length moves once instead of three times.
4. **The step ceiling is computed once** rather than per instruction, which was 1.6% of that page on
   its own and is nothing to do with typing — it was there before and is fixed here because the
   profile put it at the top.

The residual +6.2% is on a page whose nine type 4 programs are a handful of operators each, where
the per-*evaluation* cost dominates: the operand stack can no longer *be* the output buffer, because
it holds typed values and a caller wants `f32`, so the inputs are written in and the results read
back. Two `extend`s over `TrustedLen` iterators are the cheapest form of that measured — a `reserve`
and a `push` loop costs ten million instructions more. **Where a program is real work rather than
framing, the typed evaluator wins**, which is what the first two rows say.

## What did not move, proven rather than asserted

A function's value decides a colour, so pixels were allowed to move. None did:

- `display_list_digest` over all 974 pdf.js documents, before and after in one working copy with the
  patch the only variable and one `pdf-sandbox-worker` on disk: **`diff` empty**.
- Every gate on its previous number: the corpus's 974 documents / 64 incomplete; the oracle's 1 794
  pages at 906 agrees / 67 contradicted / 786 ambiguous; quorra at scale 1 on 931/23/2/18 and the
  `gpu` lane at 4× on 937/9/5/23; text extraction 99.2%, 99.8% and 98.26%.
- The two witnesses in the whole population — the owner's `pi_seven_segment.pdf` and `type4_pi.pdf`
  — rendered at 2× before and after: byte-identical, and looked at.

`doc/todo/00`'s step 7 ink sweep is therefore not owed: it is for a round that changes what gets
drawn, and this one is measured not to have.

## Consequences

- `Instruction::Push` carries a `Value`, which is a public type change with no in-tree caller. The
  operand stack is the caller's buffer for ADR 0364's reason and is now a second buffer beside the
  outputs; `shading::Components` owns one per thread.
- `not`, `and`, `or`, `xor`, `cvi`, `cvr` and `truncate` mean what Annex B's columns say, and the
  ledger's §7.10.5.2 row no longer carries a known incompleteness.
- Three answers remain choices with their grounds: `bitshift`'s width — where a shift wider than the
  register now leaves the sign repeated, because answering zero would have made the answer depend on
  the width that row says it does not — the error the subset cannot express, and the conversion rule
  above.
- The two contract answers are in `doc/QUORRA_FEEDBACK.md` §26, where the team that asked will read
  them.
