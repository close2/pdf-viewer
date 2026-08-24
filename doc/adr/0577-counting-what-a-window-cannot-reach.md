# ADR 0577 — Counting what a *window* cannot reach, and the doc comment that was counted as a call

Status: accepted, 2026-08-24. Session 709, beside ADR 0576. Adds `tools/state.sh windows`; corrects
the condition `tools/state.sh hosts` was already counting on.

## 1. The gap, named by the round before this one

The seven-hundred-and-fourth session closed `doc/todo/30`'s item 4 and then wrote down what it had
*not* done:

> §12.3.5's collection and §12.5.6.14's popup windows are still `viewer-ui`'s alone, and neither is
> a *tab*, so `Tab` does not reach them. **Nothing counts what a window cannot reach** the way
> `tools/state.sh hosts` counts what a C caller cannot.

That sentence is the reason this ADR exists rather than the count being folded into ADR 0576. "All
three hosts stay level" is the decision `doc/todo/30` records as costing roughly three times the
host-side work per feature, taken deliberately, and it has had **no instrument** since it was
stated. A rule with no instrument decays exactly the way a ledger row does — which is ADR 0509's
third criterion, and which is the whole reason `state.sh hosts` was built in the first place: *"the
ABI's entry points are the whole vocabulary"* was true when ADR 0346 wrote it and false when
somebody counted.

## 2. What the section counts, and the two choices in its population

`tools/state.sh windows` prints, per host, how much of `Command` and `Query` that host reaches, and
then names any variant **no window reaches at all**.

Two decisions about the population, both of which change the answer:

**The three hosts that put something on a screen, and not the other three consumers.** `viewer-ffi`
has its own section. `viewer-core`'s headless test and `viewer-confined`'s wire protocol name every
variant by construction — the second puts each one on a pipe — so including either would answer 100%
and mean nothing. That is trap 11's shape, and `section_hosts` already carries the same note about
`viewer-ui`'s trace module.

**`viewer-host` is added to each window rather than counted on its own.** It is the crate all three
depend on precisely because a host's non-toolkit half lives there, so a window calling
`viewer_host::page_entry` reaches §12.3.4 and §12.4.2 without naming either query. Counting the host
crates alone would have reported three windows blind to a panel all three draw.

## 3. The first run was wrong, and the way it was wrong is the finding

It reported both native hosts reaching §12.3.5's collection. They do not. The evidence was one line
in `viewer-host/src/panel.rs`:

> `/Collection` is what arranges files into folders, and it is a different answer
> ([`viewer_core::Query::Collection`]) that **this host does not yet ask**.

A rustdoc link is a sentence *about* a question, not a call — so a count whose condition was "the
name appears in the crate" reported the exact opposite of what the sentence said, in the same file,
four words later. **This is trap 11 caught in the act**: a report is only as good as the condition
it fires on, and "the identifier occurs" is not "the host asks".

Both sections strip `//` to end of line before matching now, through one shared `names_in_code`
helper. `section_hosts` had the same latent flaw and had simply not been bitten yet — this round
added doc comments to `viewer-ffi` naming several queries, so the next round to remove an entry point
would have been told the query still reached the ABI.

The general rule is worth more than the instance and is the one `state.sh` should be read by: **a
count over source text is a claim about what the text *is*, and a comment is text.**

## 4. What it reports, and how to read a zero

The numbers are the command's and are not written here. Three things about reading them are not
countable and are:

- **"No window asks for X" is not automatically a debt.** `Query::Dirty` is the standing example:
  all three windows learn about an edit from `Event::Dirty`, which carries the same fact and arrives
  without being asked, so none of them asks the question. The C ABI has both, and correctly — a
  caller that missed the event can still ask. What the line is for is making a round *notice* and
  decide, rather than a capability quietly belonging to one host.
- **`Query::Frame` is a tier distinction rather than a gap.** A tier-2 host draws its own pixels and
  hands the viewer nothing, so it asks the question that answers `Answer::None` for it exactly
  never. The two native hosts do ask.
- **Most of what the native hosts do not ask is a *delegation*, not an absence.** `Query::Caret`,
  `Query::Offset`, `Query::FieldSelection`, `Query::FieldAt` and `Query::FreeTextAt` are what a host
  needs in order to draw its own field and its own caret; a host placing a real `GtkEntry` or a real
  `QLineEdit` has the toolkit doing it. What is left after that reading is the short list this
  section exists to keep visible.

## 5. What it does not do

It counts *reach*, not behaviour: a host that asks a query and draws nothing with the answer is
counted as reaching it. That is the same limit `state.sh hosts` has had since ADR 0509, and it is
honest — an instrument that could tell drawing from asking would be a screenshot, which is what
`Xvfb` and a round's own eyes are for (trap 1). What the count buys is the thing a screenshot cannot:
a claim about parity that cannot be made without running something.
