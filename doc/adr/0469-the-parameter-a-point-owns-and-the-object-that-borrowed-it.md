# 0469 — The parameter a point owns, and the object that borrowed it

Status: accepted.
Session: 637. Follows ADR 0465, which put §11.7.5.2's row at `silent` and priced what it owed;
this round pays the first half of that price and finds a second gap one clause up while reading
for it. Follows ADR 0455 and ADR 0460 for the rule that chose the second half of the round:
*rank by blame, then read the row whose stated reason is a claim about this codebase rather than
a claim about the standard.*

## The decision

**Three ledger rows move, and the ledger has no `silent` row again.**

- **§11.7.5.2 goes `silent` → `reported`.** The departure it names is detected and reported, on a
  condition derived from the clause; the per-region model it would take to *implement* is priced
  in `doc/todo/13` and deliberately not built.
- **§10.5 goes `implemented` → `partial`.** Reading §11.7.5.2 found a colour the transfer function
  never reaches at all: a shading's. Reported too.
- **§12.7.4 and §12.7.4.1 keep `partial` and gain the evidence they never had.** Both cited a
  §12.7.4.3 test as the whole of their proof for a §12.7.4.1 claim, and that test walks no
  `/Parent` chain.

## §11.7.5.2: what the clause actually requires

The row said, for two hundred sessions, that per-region tracking "needs a second transfer function
competing with a first inside a transparency group". 632 found that the clause says no such thing.
What it says is:

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

and, closing the same paragraph:

> For portions of the page whose topmost object is not fully opaque or that are never painted at
> all, the default halftone and transfer function for the page shall be used

Between them sit the six conditions that define *fully opaque* — the alpha constant, the blend
mode, the soft mask, an image's own `/SMask`, the same four at every enclosing group's `Do`, and
every object of a tiling pattern's cell — and the sentence that says what they are for: "[t]ogether,
these conditions ensure that only the object itself shall contribute to the colour at the given
point, completely obscuring the backdrop."

**So the quantity belongs to a point, and this tree gives it to an object.** `fill_paint`,
`stroke_paint` and `transferred_image` apply the function in force to each object's own colour on
the way to the device, before anything composites. The clause applies the topmost object's function
to the composited colour after everything has.

## The condition, derived rather than guessed

632's warning was that "the report is not one line", and trap 11 has four recorded instances of a
report firing on the wrong condition and costing judged pages. So the condition was derived at the
point rather than at the mark:

- Where the topmost object at a point **is** fully opaque, the two models agree — the composited
  colour there *is* that object's colour, by the clause's own sentence about what the six
  conditions ensure.
- Where it is **not**, the clause says the page's default applies to the composited colour. This
  tree's composite has each contributor's own function already inside it. The two differ if and
  only if at least one contributor carried a function.

**A point is drawn wrong exactly when some object covering it carried a transfer function and the
topmost object covering it is not fully opaque.** That single statement covers both cases a
two-condition report would have split and got wrong in the second: the translucent object that
carries the function itself, *and* the translucent object painted over an opaque one that carried
it. The second is invisible to any rule that only looks at the mark being made.

Nothing in the interpreter knows which objects overlap, so what `Interpreter::note_transfer` fires
on is the geometric over-approximation: **a mark §11.7.5.2 does not call fully opaque, made while
some mark on the page has carried a function.** It cannot under-report, because a point drawn wrong
has both halves on the page and in that order. It over-reports only a page whose translucent marks
all happen to miss its transferred ones — and the population that can reach the condition at all is
measured: `examples/transfer_function_census` finds 13 corpus documents stating a Table 57 `/TR` or
`/TR2` and exactly one stating anything but `/Identity` or `/Default`. That one reports nothing,
which is the same answer `mutool draw -F trace` gave 632 about it.

## The ancestry, which is where a naive flag would have failed

§11.6.6 initialises the blend mode to Normal and both alpha constants to 1.0 and the soft mask to
None *before* a transparency group's content stream runs, and §11.6.7 starts a tiling pattern's
cell from the initial graphics state. So a mark inside either is fully opaque by everything its own
state can tell it, and not fully opaque by the clause. A flag reading the mark's own alpha would
have reported **nothing at all** for the case §11.7.5.2 spends four of its six conditions on.

`Interpreter::opaque_ancestry` carries the answer down instead: one flag rather than a stack, for
`inside_knockout`'s reason — what it guards is a property every enclosing scope shares. It is
narrowed in `group_commands`, from the state at the `Do` rather than from the group's own, and in
`tile`, from the state that painted the pattern.

**And scoped away inside a soft mask's group**, which is ADR 0276's argument arriving one clause
over: §11.5.3 turns a mask group's result into one luminosity, so its marks are never painted at a
point on the page and the clause is not about them. `transfer_painted` is saved and restored there
beside `blending`, `blending_changed`, `nested_space_departed` and `alpha_sources`, all of which
are scoped for the same sentence.

## Why the report and not the model

The per-region model is a rasteriser change and the population is zero, which is the
speculative-work case `CLAUDE.md` forbids. What it would take is now written down in `doc/todo/13`
rather than left to be re-derived: a per-pixel transfer identity rasterised beside the colour —
each fully opaque mark writing its own function's index, each non-opaque one writing the page's
default — and one pass mapping each pixel through the function its index names. That is a second
channel in `pdf-render`'s target and a matching pass in three backends.

The report is what makes deferring it honest. A document that turns up stating a transfer function
under a soft mask now says so instead of being drawn wrong in silence, which is the whole
difference between `silent` and `reported`.

## §10.5: the colour the function never reached

Reading §11.7.5.2 meant enumerating every place a transfer is applied, and the enumeration is
short: two calls to `GraphicsState::transferred` and one to `transferred_image`. What that makes
visible is what is *not* in the list. `fill_paint` returns `Paint::Shading` on the line above the
one that maps a colour, and `sh` never asks at all — so an axial or radial ramp's stops, a mesh's
corners and a sampled shading's program all reach the backend unmapped.

§10.5's subject is the component value, without qualification:

> In the sequence of steps for processing colours, the PDF processor shall apply the transfer
> function after performing any needed conversions between colour spaces.

A shading's colours are component values. This is therefore a second silent departure inside the
feature §10.5's row calls implemented — the shape `doc/HANDOVER.md` says is the one that ships:
"the gap inside a feature that is already there". It is reported rather than implemented for the
same reason as above: reaching every colour a shading carries is `Shading::with_alpha`'s walk done
again with a closure, in `pdf-render`, and the population is zero. §10.5's row is `partial` and
says which half.

`spec-errata emit` over all fourteen documents before writing: **no erratum touches §10.5 or any of
§11.7.5.** The nearest are §10.4.2.4's italic-variable substitutions, §10.6.5.4 and §10.6.5.6 on
halftone types, §10.7.2's flatness range, §11.4.8's `a` → `α`, and §11.6.6's two "Deprecated in PDF
2.0" carets — none of which moves a requirement this round touched.

## §12.7.4: a row that argued about three children and cited one

The blame band `doc/todo/01` keeps is twelve rows at five commits on this base, down from 632's
sixteen at seven, with the same forty-two-commit gap above it. Rank 1 is §12.7.4, and its stated
reason is a claim about this codebase — in fact about the ledger's own bookkeeping:

> A family's parent row is not maintained by the sessions that implement its members.

The row is right about that, and was corrected in the three-hundred-and-seventy-first session for
exactly it. **What nobody corrected was the arrays the corrected note rests on.** Its `test` array
held one entry, `variable_text.rs::quadding_moves_the_line_within_its_box`, and its `code` array
held one file, `appearance.rs`, while every sentence in the note is about `view.rs` and `form.rs`.

§12.7.4.1's row cites the same test, and its opening claim is "Table 226's inheritance is
implemented". That fixture's widget is a **single merged dictionary**: the `/Parent` chain
`Field::read` walks has no links in it, so the clause's own rule —

> Many field attributes are inheritable , meaning that if they are not explicitly specified for a
> given field, their values are taken from those of its parent in the field hierarchy.

— ran zero times in the only test either row offered. `form.rs::a_fields_type_flags_and_value_come_from_the_ancestor_that_states_them`
is the assertion: `/FT`, `/Ff` and `/V` stated **two** links up so that a walk stopping at the
immediate parent fails too, all three read back through `form::fields`, and Table 227's bits 1, 2
and 3 asked together because one inherited integer moves all three. Mutation-checked by cutting
`MAX_FIELD_ANCESTRY` to 1, which fails it — and fails it through the report the clause's forbidden
bound owes, since a half-walked ancestry is refused rather than answered.

Both rows now cite it, and §12.7.4's `code` array names the two files its own note argues about.

## The consequence for the ledger

The rule this round leaves behind is in `doc/todo/01`:

> **When a note is corrected, the `code` and `test` arrays are corrected in the same edit or they
> are not corrected at all.**

That is greppable — for each row, does its `note` name a source file its own `code` array omits? —
and it is named there as a thirteenth sweep — **renumbered the nineteenth in the
six-hundred-and-forty-fifth**, because thirteen was an ordinal two other things already held and
the count of sweeps had been running together with the count of committed programs (ADR 0475 §1).
It sits beside the eighth, which asks whether a path a note names *exists*; this asks whether a
row's arrays agree with its own sentences.
