# 706 — The half of a region that is one object

ISO 32000-2 §11.7.5.2's per-region model was priced, in `doc/todo/13` and in its own ledger row, as
a per-pixel channel and a matching pass in all three backends. **Half of it needs no pixel at all.**
The clause chooses the function at a point between exactly two candidates — the topmost enclosing
object's, where that object is fully opaque, and the page's default — so a mark the clause does not
call fully opaque is never the one it chooses anywhere, and its own function is used at no point on
the page. That is one object, one graphics state, one function in `pdf-model`. The same deduction
one step earlier takes the function off every mark inside a **soft mask's** group, whose result
§11.5.3 makes "the luminosity of the resulting colour" rather than ink at a point — and whose
conversion §11.5.3 separately performs "with no compensation for gamma or other colour calibration",
which is what Table 52 calls a transfer function. §11.7.5.2 is `partial` rather than `reported` now.

What is left really is a rasteriser change and is still owed: a fully opaque mark carrying a
function, seen through a *later* mark that is not fully opaque, where the clause maps the whole
composite once and this tree has already mapped the colour underneath. The report narrowed to
exactly that shape.

Date: 2026-08-24.
ADR: [0570](../adr/0570-the-half-of-a-region-that-is-one-object.md).

Habit: `doc/habits.md`'s *Measuring* gains the tell — a price with fewer numbers than the clause has
subjects.

Touched: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/ext_gstate.rs`, `crates/pdf-model/src/content/pattern.rs`,
`crates/pdf-model/src/content/path.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/src/content/image.rs`, `crates/pdf-model/src/content/report.rs`,
`crates/pdf-model/tests/transfer_functions.rs`, `doc/conformance/ledger.toml` (§11.7, §11.7.5,
§11.7.5.2), `doc/todo/13-the-transfer-function.md`, `doc/habits.md`, the ADR and this file.

## What the round actually did, in the order it did it

**Read the clause before the file**, as the briefing said, and the reading is where the whole round
is. §11.7.5.2's second sentence — "[f]or portions of the page whose topmost object is not fully
opaque or that are never painted at all, the default halftone and transfer function for the page
shall be used" — is a *closure*: between it and the first sentence there are exactly two candidate
answers at any point, and one of them is a constant. Which means the question "may this object's
function ever be chosen" is answerable from the object alone, and only the question "what is the
composite at this point" needs a point.

**Re-derived the price**, which is what the briefing asked for and what it turned out to be worth.
The old price was not stale; it was two prices quoted as one. The tell was in the clause's own
words: the first sentence's subject is a *point*, and the qualifier that makes half of it cheap —
"but only if the object is fully opaque" — attaches to the *object*.

**Checked the witnesses.** `examples/transfer_function_census` over `doc/pdf.js` still finds 13
documents stating a Table 57 `/TR` or `/TR2` and exactly one stating anything but `/Identity` or
`/Default`: `issue6931_reduced.pdf`, whose page says in words *The color should be red*. It draws
one image, fully opaque, so it is unmoved by this round, and it was rendered before and after to
check that — the heart is still red. The corpus gate raises no `TransferFunction` report for any
page of the corpus, which is the population claim measured by the instrument rather than quoted from
the file.

**Planted before correcting** (trap 13): all four fixtures were written first and run against the
unfixed tree, where each failed with the exact wrong colour — `Color { r: 0.0, g: 1.0, b: 1.0 }`
where the clause asks for red.

**`doc/todo/00`'s step 7 was not re-run, and the reason is a derivation rather than a shrug.** The
habit binds a round that changes what gets drawn. §10.5's function reaches a colour only where a
document states a real one; the census says exactly one corpus document does; that document's one
mark is fully opaque, so the branch this round changed is not taken on any corpus page — and the
corpus gate's report list and every one of the oracle's verdict counts say the same thing from the
other end. Re-ranking an unchanged population would measure the previous round's tree.

## Two things this round found that were not its subject

- **A ledger sentence named the wrong field, and it had since the session that wrote it.**
  §11.7.5.2's row says `Interpreter::opaque_ancestry` is "scoped … away inside a soft mask's group".
  The field the soft-mask path scoped was `transfer_painted`. Nothing depended on the difference
  while the answer only reached a report — but it decides a colour now, so the row is corrected and
  the mask case is decided explicitly rather than by a flag that happened to be false.
- **Annex N is not a licence, and it is worth knowing that before somebody finds it.** ISO 32000-2
  prints a second, *object-based* model for exactly this parameter (§N.3), under which the function
  is taken from the topmost object that is not fully *transparent* — nearly the opposite rule. It is
  informative, and §N.1 makes both of its approaches conditional on "an output device that requires
  halftoned output", which this is not. What it does contribute is the artefact the remaining work
  has to avoid, which it names: "a fringe using an unexpected halftone" around a soft-edged object.

## The machine, for the record

Loaded throughout — three parallel rounds, load average between 22 and 42 on 24 cores. **No timing
measurement was taken and none is reported here.** The gates were run against the shared warm
`PDFREF_CACHE`, so the reference renderers were answered from disk rather than spawned; the oracle's
own wall clock (42 s) is therefore not a measurement of anything and is not quoted as one. The whole
of §2 ran green, including the oracle and the `render-quorra` corpus gate, with every ratchet held
and every verdict count unchanged — which is what a change with a measured population of zero should
produce.

One thing the shared cache showed that is worth a line: a cached reference failure carries the
*path* of the worktree that first produced it, so this round's oracle log quotes `mutool` failing on
a file under `…/worktrees/r707/…`. Harmless, and confusing for exactly as long as it takes to
notice.
