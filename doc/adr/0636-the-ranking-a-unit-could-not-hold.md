# 0636 — The ranking a unit could not hold

**Status.** Accepted. Session 737.

Builds the ordering ADR 0349 argued for and left unwritten, and prices what the ordering it sits
beside cannot see. **No pixel moves, no verdict moves and no list changes**: the gate's census and
all 966 non-agreeing per-page lines are identical before and after.

## Context

The contradicted pool is the strongest signal the raster instrument produces, and it has had one
ordering since the four-hundred-and-sixth session: `rank_the_contradicted`, which prints the ten
pages furthest from their *nearest* reference. That is the ambiguous bucket's instrument borrowed
unchanged, and `doc/habits.md` asks for a different one — *rank the suspects by a ratio, not a
distance: our worst measurement over the bound it is held to.*

ADR 0349 took that second ordering **by hand**, found that its head was a page the printed ranking
never prints, and closed with the code left undone:

> Ranking by the ratio is one more sort in the same function and is left for a round that wants it,
> with the argument recorded here rather than the code written.

Three consecutive rounds working this pool — the seven-hundred-and-twenty-second,
-twenty-seventh and -twenty-ninth — each closed on the same line in *Owed*: *nothing ranks the pool
by how far outside its bound each page sits, and `outside_by` already computes it per page.* This
round is that line, chosen under 722's own rule: **do not invent a criterion where an existing one
has an unevaluated precondition.**

Two things had to be settled first, and only one of them existed when ADR 0349 was written.

**Which comparison the ratio is taken over.** ADR 0242 established that a contradicted page's own
line must report a member of the *consensus that convicts it*, because a fold over every reference
can report a renderer taking no part in the verdict — `smask_luminosity_oob_transfer.pdf` printed
27.02 for a `poppler` that is not in its consensus and sits 36 of 255 from everybody. The same rule
binds a ranking made of those numbers.

**Which consensus, where a page has two.** ADR 0616 found that agreement is not transitive and ADR
0617 settled what follows: a verdict is one **every** maximal consensus reaches. That gives the
ratio a definition that did not exist in ADR 0349's session, and it is not cosmetic — see the
finding below.

## Decision

### 1. `rank_the_contradicted_by_the_bound`, printed beside the ranking it does not replace

For each maximal consensus that rejects the page, its ratio is the largest of `Tolerance::accepts`'
four — mean, worst tile, differing fraction, structural similarity — over that set's members, taken
against **that set's own widened bounds**, because a set's bound is derived from its members' spread
and borrowing another's would price the page against a judgement nothing made. The page's number is
the **smallest** of those: since ADR 0617 a contradiction is what every set reaches, so the exemption
a page is granted is only as strong as the set that rejects it least.

Three properties, each deliberate:

- **The name of the measure travels with the number.** 29× on the differing fraction and 29× on the
  mean are two different pages, and a ranked number that does not say what it is a ratio *of* cannot
  be read at all. The spellings are `quoted::Measure::words`', which is the vocabulary two sweeps
  already read notes in.
- **It does not filter on `complete`.** `rank_the_contradicted` does, and `check_the_ratchets` does
  for a reason `oracle.rs` states; ADR 0349's whole finding was that the consequence put the pool's
  two largest disagreements outside every diagnosis in the file. A second ranking repeating the
  filter would re-create the hole. The incomplete pages are labelled instead.
- **The population is `CONTRADICTED` and no other verdict.** On an `ambiguous` page no two
  references agreed, so the bound printed beside them decided nothing and a ratio against it would
  rank a quantity no verdict rests on. That is `--bin unpriced`'s population rule (ADR 0606) applied
  to the ordering built out of the same arithmetic.

`outside_by` — which `measurements` and `consensus_missed_by` already use — now delegates to the
function that also returns the measure's name, so there is one implementation of the arithmetic
rather than two that could disagree about a verdict. Its own doc comment keeps saying why
`Distance::of` is *not* folded into it.

### 2. The ordering is calibrated against two figures this tree recorded independently

Trap 13: a sweep is not believed until it has been run against the thing it is for. Both ends of the
new ranking reproduce a number written down by a round that could not see this code.

- **Its head is `xobject-image.pdf` page 1 at 127.75×**, which is ADR 0349's own hand-taken figure
  for the same page to the hundredth, and it is incomplete — the page that ADR was mostly about, and
  which the ranking beside this one still does not print.
- **Its foot is `issue6069.pdf` page 1 at 1.00×**, which is ADR 0606's finding: a verdict made of
  six differing channels of eighty thousand. A page whose exemption is worth nothing sorts last,
  which is the property the ordering exists to have.

## What it found

### The ranking beside it is blind to the bound most of the pool fails on

`Distance::of` reduces a comparison to **three** ratios — mean, worst tile, structural similarity —
and the differing fraction is not among them. That is right for `Distance`: its figures are quoted
in a hundred entries of `oracle.rs` and in `doc/todo/00`, and a page's recorded "0.16 from the
nearest reference" has to stay the number that was recorded. What nobody had priced is the
consequence for the *ordering* built on it, and the gate now prints both halves of it:

- how many of the pool are furthest outside on the differing fraction, and the range they span;
- **how many of the pool have a `Distance::nearest` at or under 1.0** — that unit saying *every
  measure I can see is inside the bound* about a page the gate has just contradicted.

This is ADR 0242's defect surviving one level up. That round found thirty of sixty-eight contradicted
pages printing a **line** on which every visible number was inside the printed bound, and fixed the
line by printing the fourth measure. The *order* those lines are printed in was left in the unit that
could not see it, and it has been ever since.

### One page is convicted twice, at half the price

`issue19633.pdf` page 1 is the only contradicted page in the pool carrying more than one maximal
consensus — the remainder of ADR 0617's census, whose other two populations are the four divided
pages and the thirty-six that concur in agreeing with us. Both of its sets reject us, so the verdict
is untouched. They price the rejection very differently:

```text
                          the pair's own spread          our worst member    against its bound
  {poppler, mupdf}      ssim 0.99896 → bound 0.99000     mupdf   0.97700     2.30x    taken
  {poppler, ghostscript} ssim 0.99088 → bound 0.98176    poppler 0.97959     1.12x    the rival
```

`mupdf` against `ghostscript` is 0.98828, under the class floor, so those two form no set and both
pairs above are maximal. The taken pair agrees so closely that `Tolerance::widened_to` leaves the
bound at the class floor; the rival's own 0.99088 widens the same bound to 0.98176 and admits nearly
all of the same difference.

**So the page's standing exemption is worth 1.12×, not the 2.30× its own line and its group note
quote** — that being what the references' most forgiving reading of it comes to. This is trap 12's
arithmetic with its sign made visible: *the tighter the pair, the harsher the bound derived from it*,
and a page can therefore be convicted twice over at half the price. Nothing about the verdict, the
mechanism or the two clauses the note settles the page on changes; what changes is what the number
beside it means. The rival set rejects us on `poppler`, which the note's own third table names as the
one renderer of the three the page's negative line width is worth anything against.

## Consequences

- `rank_the_contradicted_by_the_bound`, `outside_the_bound` and `worst_ratio` in
  `crates/pdf-model/tests/oracle.rs`; `Examined` gains one field; `outside_by` delegates; the four
  orderings are called through `rank_the_pools`, which is one section of the report and keeps
  `report` under the line limit.
- `CONTRADICTED_NEGATIVE_LINE_WIDTH`'s note gains the two-readings table above. Its existing
  measurements stand: they are the taken set's, correctly labelled as such now.
- `rank_the_contradicted`'s "two rankings" section names the third and says what each asks.
- `doc/oracle-and-corpus.md` §3b, `doc/todo/12` and `doc/habits.md`'s ranking rule.
- No ledger row moves: this round implements no normative requirement and touches no clause. The two
  clauses `issue19633.pdf` rests on — §8.4.1's clipping rule and §8.4.3.2's one-device-pixel minimum
  — are unchanged in the code and in the ledger, and the round's only claim about them is that the
  page's exemption from a *bound* is worth half what was recorded.

## Owed

- **`Distance` and this ratio disagree about the pool and nothing reconciles them.** Both are
  printed and each has an argument; what does not exist is a statement of which a round should reach
  for first, beyond `doc/habits.md`'s preference for the ratio.
- **The pool is now ordered and not yet *read* in that order.** Everything above rank ten is
  diagnosed, and the long tail between 1.0 and about 1.4 is a population nobody has asked a question
  of as a population — which is `doc/todo/12`'s number wearing a different unit.
- Unchanged from 729: a *width* division and a *camp* division are treated alike; `unpriced` cannot
  tell a bound named from a bound accounted for; a voting reference whose raster is constant still
  votes; `freeculture.pdf` page 255; the owner's `git stash drop`.
