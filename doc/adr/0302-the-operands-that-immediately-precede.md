# ADR 0302 — The operands that immediately precede

Status: accepted, 2026-08-13. Session 467. Amends §7.8.2's ledger row. Changes nothing any earlier
ADR decided; the interpreter's operand handling had never been argued anywhere.

## The question

`doc/todo/03`'s standing item is *take a chunk a round*, and this round took one nobody had:
`openpreserve/format-corpus`, 267 PDFs across five directories, examined without being committed.
Its `pdf-handbuilt-test-corpus` is 89 files each carrying **one** deliberate structural defect, which
makes it an instrument rather than a sample: every file draws the same *Hello PDF-world!* and a file
that comes back blank is a file whose defect cost a mark.

**Fourteen of the 89 render blank, and nine of the fourteen say why.** Five are blank *in silence*,
and two of those five are right: Table 31 makes a page stating no `/Contents` an empty page, and a
file whose text-showing operator has been deleted has nothing to show. Of the three that are left,
one is a page tree node with no `/Kids` and one is a `Tf` whose size operand is a lone `.`; the
third is this:

```text
BT
/F0 36.
(Hello PDF-world!) Tj
ET
```

`T02-05-01_008_Font-set-operator-missing.pdf`, whose defect is the deleted `Tf` **keyword** with its
operands left standing. This tree drew a blank page and **reported nothing at all**.

## What the standard states

§7.8.2, and the sentence is the whole decision:

> In PDF, all of the operands needed by an operator shall immediately precede that operator.
> Operators do not return results, and operands shall not be left over when an operator finishes
> execution.

with NOTE 2 beside it ruling out the reading a PostScript habit would supply:

> This postfix notation, in which an operator is preceded by its operands, is superficially the same
> as in the PostScript language. However, PDF has no concept of an operand stack as the PostScript
> language has.

So an operator's operands are the **last** *n* objects before it. The interpreter read the **first**
*n*: `number_at(&operands, 0)`, `string_at(&operands, 0)` and their siblings index from the front of
everything the stream has stated since the previous operator. On a conforming stream the two
readings are the same slice — nothing is ever left over — which is why this survived four hundred
and sixty-six rounds and every gate in the tree.

On the witness the two readings differ completely. `Tj`'s operand list is `/F0`, `36.`,
`(Hello PDF-world!)`; read from the front it is a **name**, `string_at` returns `None`, and the show
operator vanishes before `show_text` can be reached. Read from the back it is the string,
`show_text` runs, §9.3.1's "[t]here is no initial value for either font or size; they shall be
specified explicitly by using Tf before any text is shown" makes it undrawable, and the page says
so.

**That is the part that makes this a principle-1 defect rather than a rendering nicety.** Drawing
nothing here is the only thing this reader can do. Saying nothing is not, and the mechanism that
produced the silence was not a missing report but an operator that never arrived at the code holding
one.

## The decision

`operands_before(pending, operator)` hands each operator the last `count_of(operator)` objects of
what preceded it, and `count_of` is the operand list stated by the table Annex A points at for that
operator. Four consequences, each deliberate:

- **The accumulator is renamed.** `pending` is what the stream has stated since the last operator;
  `operands` is what *this* operator is given. The dispatch table below reads unchanged, which is
  the property that made a forty-four-site change a five-line one.
- **`None` means "read them all", and four operators have it.** `TJ` and `d` take an array the
  content lexer deliberately leaves flattened — one operand per element — and `sc`, `scn`, `SC` and
  `SCN` take as many components as the current colour space has. A leftover in front of one of those
  is indistinguishable from one of its own, so nothing here can improve on reading them whole.
- **Fewer operands than the count is unchanged.** The slice is then everything there is, indices run
  from the front as before, and an operator missing an operand is still refused rather than
  half-applied.
- **Nothing is reported for the leftovers themselves.** §7.8.2 makes such a stream malformed, so a
  report is *available*; trap 11 is why there is not one. After the fix the mark the file states is
  drawn correctly, so the report would name no lost mark while taking every page carrying one off
  the oracle's judged set. `issue6342.pdf` would be the first of them.

## What it moves, measured

`examples/display_list_digest` on both sides of the change, over every population on this disk:

| population | documents | display lists that changed |
|---|---|---|
| pdf.js corpus | 974 | **2** |
| SafeDocs `CC-MAIN-2021-31`, archives `0100`, `2100`, `4100`, `6100` | 4000 | **0** |
| `openpreserve` (five directories) | 267 | 0 |
| `doc/corpora/` (three submodules) | 108 | 0 |

Which is exactly what §7.8.2 predicts: a producer that leaves an operand over has written an invalid
stream, and the web is full of *damaged* files rather than of files a producer wrote wrong this way.
The handbuilt corpus's own count is 0 for the same reason its verdict moved — its blank page stayed
blank, and what changed was that it now says so. The corpus gate's incomplete list is unchanged at
65; the handbuilt corpus's went 11 → **12**, and that rise is a new report rather than a regression
(trap 5).

**The two pdf.js documents were looked at rather than counted** (trap 1). `issue6342.pdf` is named
*Form XObject with errors* and its form's content stream is corrupted from byte 1300 on, so the `c`
operators after the damage run with junk in front of them. Ours before the change painted a fat green
blob; after it, a thin crescent — and **`mupdf` paints the crescent**, while `poppler` gives up at
the first bad keyword and paints nothing there. That is evidence about our reading in principle 5's
direction and no more: the clause decided it, and mupdf agreeing raises confidence that the clause
was read right. `poppler-90-0-fuzzed.pdf` is a fuzzer's output and gains six commands.

## What was not taken

Two things this chunk diagnosed and this round left alone, both written into `doc/todo/03`:

- **A page tree node with no `/Kids` becomes a leaf**, so `T02-02_005_page-tree-no-kids.pdf` — whose
  `/Pages` states `/Count 1` and no children — yields the `/Pages` dictionary itself as page one and
  draws a silent blank, while object 2 sits there stating a `/Contents`. The rule is deliberate and
  argued in `page.rs` ("[t]rusting `/Type` instead would drop pages from files that omit it"); what
  is not argued is the silence.
- **`openpreserve/format-corpus`'s licence**, which is a reading this round did and a decision it did
  not take. The finding is in `doc/todo/03` §2 with the question stated for the owner.
