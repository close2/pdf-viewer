# ADR 0600 — The report that asked the style, and the frame that asked the direction

Status: accepted, 2026-08-25. Session the seven-hundred-and-twentieth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other and against the code — ADR
0538's method in its seventh round (0551, 0560, 0567, 0579, 0593, this). Amends §12.4.4 and
§12.4.4.1 in the ledger; changes `crates/viewer-core/src/transition.rs`, its two call sites in
`crates/viewer-core/src/viewer.rs` and one doc comment in `crates/viewer-host/src/clock.rs`; adds
one test to `crates/viewer-core/tests/headless.rs` and one to `transition.rs`; extends
`crates/pdf-model/examples/presentation_census.rs`. **No status moves and no pixel moves; one
report is widened and it fires on no crawled document.** Extends ADRs 0230 and 0567.

## 1. The pair, and the rule that chose it

ADR 0567's search was run on this base rather than read off a document, with 0579's two rules and
0593's third applied: strip the clause-level parents, let the total rank the family while the pairs
choose the reading, and **take the strongest pair the previous round named and did not read.**

The family order is unchanged — §12.5 heads it, §12.8 second, §12.7 third — and the third rule sends
this round somewhere else entirely. ADR 0593 §1 named two pairs stronger than the one it took and
left both: **§12.4.4 ~ §12.4.4.1** and §10.7.4 ~ §10.7.5. The first is the stronger, and it is the
strongest pair below any clause-level parent in the whole ledger.

**That the rule moves a round out of the family it has been in for three rounds is the point of it.**
Self-reinforcement is what makes a read family score, and a rule that only reorders pairs inside the
head family would never escape it.

## 2. What the two rows share, and where they disagree with themselves

Both rows describe the same division of Table 164's twelve transition styles, and both end on a
tally of what the clause is `partial` for:

> Seven of Table 164's twelve styles are shaped; **four** of the other five are reported by name,
> each because the table states no quantity for it, and the fifth is `R`, which the table defines as
> the cut and which is therefore drawn rather than reported.

and then, in the same note:

> `partial` for the five styles.

**The two sentences contradict each other, in both rows.** `R` is the cut by Table 164's own
definition — "[t]he new page simply replaces the old one with no special transition effect" — so a
reader that shows the page at once has drawn what the file asked for, and it is not something the
clause is owed. The debt is four.

This is 710's shape with a longer history behind it. §12.4's parent row was corrected to four in the
three-hundred-and-eighty-eighth session (ADR 0388 §4, from ADR 0230 saying both things twelve lines
apart), and these two rows' *middle* sentences were corrected in the six-hundred-and-sixty-third.
Both times the closing tally three lines further on was left standing. **A correction that reaches
the sentence stating a mechanism and not the sentence counting it is the same failure twice**, and
the second time it survived a round whose whole subject was that string.

## 3. The finding the pair led to, and it is a report keyed on less than what decides the drawing

`viewer_core::transition::frame` is asked which frame a transition has at a fraction of the way
through, and `viewer_core::transition::note` is asked what to tell a person about one it does not
draw. `note` took a `&Style`. `frame` takes a `&Transition` — and for four of the seven styles it
shapes it asks the direction as well:

```rust
Style::Wipe => revealing(
    viewport,
    vec![swept_from(viewport, quarter(transition.direction)?, done)],
),
```

The `?` is the finding. `quarter` answers `None` for anything but the four quarter turns, so a
`Wipe`, `Cover`, `Uncover` or `Push` whose `/Di` is 315, or the name `None`, or a fractional angle,
**shapes no frame** — and `note`, asked only for the style, has nothing to say about a style it
believes is drawn. The page arrives as a cut with no word said, which is precisely the outcome
trap 5 exists to prevent and the one `note` was written for.

`viewer_host::Clock::shapes` is the third place the same decision appears, and its doc comment
asserted the property that had failed: "the core has already said which by the time an
`Event::Transition` arrives".

**And a test's own doc comment said the report existed.**
`only_the_four_quarter_turns_name_a_sweep`, written in the three-hundred-and-ninety-third session,
reads "a `Wipe` at an angle no rectangle sweeps is **reported** rather than drawn at some nearby
angle the file did not ask for" — over an assertion that checks only that it is not drawn. The
sentence has been false since it was written.

## 4. The fix, and the shape of the test that holds it

`note` takes the whole `&Transition`. The four styles that travel along `/Di` delegate to a new
`askew`, which asks `quarter` — **the same expression `frame` refuses on**, rather than a second
list of legal directions, because a duplicated decision is the defect and reintroducing it one
function over would be the same mistake with a different name.

The test is the property rather than the case:

> `note(t).is_some()` exactly where `frame(t, …).is_none()` and the style is not `R`.

held over thirteen styles crossed with seven directions. A list written out by hand would go stale
the first time a style moved between the shaped and the unshaped; this cannot, and it fails if
either side changes without the other. `headless.rs` carries the end-to-end half: a two-page
document whose second page states `/Trans << /S /Wipe /Di 315 >>` now raises an `Event::Reported`
naming the style and the angle, beside the `Event::Transition` it always raised.

**Calibrated per trap 13.** With `askew`'s condition forced true — the pre-round behaviour exactly —
both the property test and `only_the_four_quarter_turns_name_a_sweep` fail, the latter on the
assertion added to it. The plant was removed and both pass.

## 5. What it costs, measured rather than asserted

`examples/presentation_census` gains a tally of `/Di` on the four styles that travel along one. Over
the 65 703 documents of `CC-MAIN-2021-31` that open — the same population the row's 276, 86 and 1
came from, all three reproduced exactly — there are **464** transitions on those four styles and
**every one of them states 0, 90, 180 or 270.** Not one states 315, the name `None`, or a fractional
angle.

So this report fires on nothing that exists, and saying so is the point rather than a hedge: no
*conforming* file can reach it either, because Table 164 gives those four styles only quarter turns.
What was wrong was not a picture but the shape of the decision — one question answered in two
expressions, where the two had never yet disagreed on any input anybody had fed them. That is 701's
finding and 716's, and it is the third round running in which the duplicate cost the *reach* of a
correction rather than a pixel.

**The census also settled a smaller thing.** The only unrecognised `/S` anywhere in the crawl is the
**empty name**, on 106 pages of seven documents, written beside the private keys `/Curve` and
`/Directional`. `Style::Unrecognised` handled it correctly and the sentence it produced read
`transition: / is not one of Table 164's styles`. A bare slash is not a sentence a person can act
on, so the empty name is now described instead of printed.

## 6. What this does not change

No raster, no verdict, no status. §12.4.4 and §12.4.4.1 stay `partial`, for four styles rather than
five, and the direction refusal is recorded as a report rather than as a clause debt — no conforming
file states one, so it is a robustness answer and not a coverage one.

## 7. Clauses

§12.4.4 and §12.4.4.1 stay `partial` with their counts corrected and the new report recorded with
its census. §12.4.4.1's claim that "every entry of Table 164 is read" is corrected too: the table
has eight entries and the row's own enumeration beneath that sentence held seven. The eighth is
`/Type`, which is not read and is not owed — "if present, shall be `Trans` for a transition
dictionary" binds a writer, and a dictionary under a page's `/Trans` is a transition dictionary by
where it stands. A parent counting what its own list does not hold is the fifth failure shape with
the sign the sweeps cannot see.
