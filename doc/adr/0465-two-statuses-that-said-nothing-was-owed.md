# 0465 — Two statuses that said nothing was owed

Status: accepted.
Session: 632. Follows ADR 0455 and ADR 0460, whose rule for choosing a row off `doc/todo/01`'s
blame ordering this round applied for the third time: *rank by blame, then read the row whose
stated reason is a claim about this codebase rather than a claim about the standard.*

## The decision

**Two ledger rows change status in opposite directions, and both changes are about the same
mistake: a status that asserts nothing is owed, resting on an argument nobody re-derived.**

- **§14.8.4.2 and §14.8.4 go `partial` → `implemented`.** Their `partial` stood on a debt the
  ledger records, in three other rows, as not this program's.
- **§11.7.5.2 goes `inapplicable` → `silent`.** Its `inapplicable` stood on a claim about what
  §11.7.5.2 requires, and the clause requires something else.

The two are worth one ADR because the failure is one failure. `implemented`, `inapplicable`,
`writer-side` and `out-of-scope` are the ledger's four settled statuses; each is a claim that
nothing is owed, and a claim that nothing is owed is the one kind a sweep cannot check, because
there is no missing thing to look for. `partial` and `reported` at least name a debt that a reader
can go and find. What this round found is that the settled end of the vocabulary decays in *both*
directions — a row can sit at `partial` when its own argument settles it, and a row can sit at
`inapplicable` when its argument does not.

## §14.8.4.2: a row that argued itself `implemented` and stayed `partial`

The clause states exactly one `shall`, and this tree executes it:

> Structure elements other than the standard structure elements identified in clause 14.8.4
> "Standard structure types" or in the standard structure namespaces (see 14.8.6.2, "Role maps and
> namespaces") shall nest in relation to standard structure elements according to the requirements
> of the structure elements to which they are role mapped.

`Tree::standard_role` is that sentence — a non-standard type answers as whatever the role map takes
it to, which is what makes the mapped type the question a consumer asks — and
`structure.rs::a_mapped_role_reads_as_its_standard_type` reaches it, mapping `/Chapter` to `/H1`
and leaving an unmapped `/Sidebar` as no standard type at all. Everything else in the subclause is
a pointer to Annex L and an informative NOTE.

The row's own note said why Annex L is not owed:

> a nesting rule is a statement about whether a *document* is well formed, and nothing here
> validates documents — the same position §7.11.2.1's path rules and §7.12.4's version ordering are
> recorded in.

**Both of the rows it names as precedent are `implemented`**, each for that reason in as many
words; §7.12.4's says "[t]he rule that it be no greater than the file's own version is a *writer's*,
and this reader does not validate files". And Annex L's own row is `writer-side`, a status whose
definition is that nothing is owed. So four rows held one argument and one of them held a status
that contradicts it — for two hundred and fifty sessions, including a "read and kept" in the
five-hundred-and-first, which is ADR 0455's point about a dismissal that cites rather than checks.

§14.8.4 follows, and mechanically: the clause states no prose of its own — §14.8.4.1 begins
immediately under the heading — so its status is its family's arithmetic. Its one non-settled child
was §14.8.4.2, and its own note attributed its `partial` to Annex L in the same words. Both move.
The sixth sweep was re-run in the same session, which is ADR 0455's rule about a chain: one hit
remains, §O, which is `doc/todo/39`'s and ADR 0209's and was the one hit before.

## §11.7.5.2: an `inapplicable` that described the wrong requirement

The row said:

> what the clause asks for is *per-region* tracking, decided by the topmost fully opaque object at
> each point, and that needs a second transfer function competing with a first inside a
> transparency group.

The first half is the clause. The second half is not in it, and it is what the status rested on:
with one function stated, the argument goes, the topmost object's is in force by construction.
§11.7.5.2's rule needs no second function, because it is about *opacity* rather than about
competition:

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

and, after the six conditions that define fully opaque:

> For portions of the page whose topmost object is not fully opaque or that are never painted at
> all, the default halftone and transfer function for the page shall be used

So a page with exactly one `/TR`, drawn under a constant alpha below one, is a page whose marks
shall carry the **default** transfer function and which this tree draws with the stated one. That
is not per-region tracking being unnecessary; it is per-region tracking being necessary for a
reason the row had not read. The consequence sits in the clause's final paragraph, which is where
626 observed that a standard puts the consequence after it has finished defining its terms.

The halftone half of the clause is untouched by any of this: §10.6.1 makes halftoning conditional
on a device that performs it and says that for a screen "after gamma correction by the transfer
functions, the colour components shall be transmitted directly to the device". §10.5's row already
carries that reading and `CLAUDE.md`'s scope line was amended on it.

### Why `silent` and not `partial`, and why no code

`partial` promises a note saying "what is reported"; nothing is reported here, and writing a
`partial` row that reports nothing would be the understatement the status exists to prevent.
`silent` is the ledger's own word for a gap inside a feature that is already there — "not
implemented, and nothing says so. A page is drawn wrong without a word" — and §10.5 is exactly
such a feature. §10.5's row was `silent` for one round in the three-hundred-and-fifty-seventh
session for the same reason.

No code, and the reason is a measurement rather than a judgement.
`examples/transfer_function_census` over the corpus finds thirteen documents stating a Table 57
`/TR` or `/TR2` and exactly one stating anything but `/Identity` or `/Default`; `mutool draw -F
trace` shows that one document's image drawn at `alpha="1"`, Normal, with no soft mask and no
`/SMask`. **No page on this disk is drawn wrong by this today.** Building the per-region model for
a population of zero is the speculative optimisation `CLAUDE.md` forbids in the other direction,
and building a *report* for it is worth doing but is not one line: the clause extends each opacity
condition to the groups an object is inside and to a tiling pattern's cell, so a flag reading only
the mark's own alpha would under-report the nested case the clause spends four of its six
conditions on. `doc/todo/13` prices both, in that order, and now says so instead of repeating the
sentence this ADR retires — the file carried it too, one bullet under "what it does not need".

## What the round did not decide

**§8.7.3 stays `partial` and the reason was re-derived rather than repeated.** A stroke whose
colour is a tiling pattern is still reported rather than tiled, and the row's stated reason is a
claim about this codebase: "that outline is the backends' to compute … so there is no path here to
tile". It still holds — no crate that builds a display list depends on `kurbo` or on a rasteriser,
and all three backends expand a stroke themselves (`tiny_skia::PixmapMut::stroke_path`, `vello`,
`kurbo::stroke`). The clause names the stroke in the same breath as the fill — "[w]hen performing
painting operations such as S (stroke) or f (fill), the PDF processor shall paint the cell on the
current page as many times as necessary to fill an area" — so this is a debt, and the debt is that
tiling it here means computing the outline a fourth time in the one crate whose whole point is that
it does not.

**§14.9.4 stays `implemented` and gains the test that was missing.** Errata Collection 3's Issue
#483 adds a third location for a replacement text — an `Artifact` tag's property list rather than a
`Span`'s — and the interpreter already meets it, because it asks *every* `BDC`'s property list for
§14.9's four entries. Nothing in the tree ran it. That is ADR 0455's newest shape, the row that is
right with evidence that never reached the requirement, and the answer is the same one 626 gave
three times: write the test.
`accessibility.rs::an_artifacts_replacement_text_replaces_what_it_encloses` draws a folio as `vii`
under an `/ActualText` of `7` and asserts the readback, and the artifact's range over the
replacement rather than over what it replaced. Mutation-checked: restricting the property list to
a `/Span` tag fails it.

## Consequences

- Two fewer rows claim that nothing is owed when something is, and one more row says plainly that
  something is.
- `doc/todo/01` gains the shape: **a settled status is an argument, and the argument is checkable
  against the rows it names.** Where a row's note cites another clause as precedent, the precedent
  has a status, and the two either agree or one of them is wrong. That is a sweep somebody could
  write and it is not written; `doc/todo/01` records it as such rather than as a rule to remember.
- `doc/todo/13` carries the per-region debt with its population measured at zero, so the next
  document that states a transfer function under a soft mask promotes it rather than discovering
  it.
