# ADR 0200 — A sixth tab, for a clause no corpus document exercises

Status: accepted, 2026-08-06 (session 347).

## Context

§12.4.3's articles are a reading order laid over pages that are not consecutive — a story that
starts on page one and continues on page seven. `pdf_model::article` has read the whole structure
since the two-hundred-and-fifty-fifth session, and §12.6.4.7's thread *action* has followed a
thread since. What the clause also says is:

> Interactive PDF processors may provide navigation facilities to allow the user to follow a thread
> from one bead to the next.

and there was no way in. An article was reachable only if a *document* stated an action to reach
it. §12.4.3's ledger row said so, and explained itself the way `doc/todo/01`'s third sweep watches
for: "[t]hat is a panel rather than a clause — the same shape §12.3.3's outline was before the
hundred-and-sixty-sixth session." **The sidebar arrived in that session and the row did not move
for forty-seven rounds.**

`doc/todo/36` had already made the ordering argument: articles before §12.3.5's collection, because
it is a third of the work and exercises the `Command::Activate` path the outline already uses.

## Decision

### A sixth tab, and it sends the same message the first one does

`Query::Articles` → `Answer::Articles(Vec<Thread>)`, in the `/Threads` array's own order.
`viewer_ui::chrome::Tab::Articles` draws one row per thread: Table 162's `/I` title, the bead count
beside it, and `Act::Activate(thread.id)` — the *object*, not a destination.

That is the rule ADR 0144 settled for the outline and it is worth restating because it is what made
this cheap: **a panel row hands over an object and the document decides what activating it means.**
Nothing in `chrome.rs` knows what a thread is beyond having a title and some beads.

A thread with no `/I` is still a thread — Table 158 makes the information dictionary optional — so
the row falls back to the clause's noun and the array's position.

### Activating one composes §12.6.4.7 rather than adding a route

A thread dictionary states neither `/A` nor `/Dest`, so `activate_object` would have done nothing
with it. The temptation is to teach it to find the first bead and jump. The decision is the other
one: build the `ThreadJump` §12.6.4.7 already defines, with `ThreadTarget::Object(id)` and no bead,
and hand it to `perform`.

**One place, one behaviour.** That is how a click on a row lands on Table 163's `/R` — the
rectangle "specifying the location of this bead on the page" — framed by Table 149's `/FitR`,
rather than on the page the first bead happens to sit on. A second route would have had to
rediscover that, and `doc/todo/01`'s fourth sweep exists because two places describing one
mechanism is how this project's rows go stale.

Table 209 makes the action's `/B` optional and states what its absence means: the thread's first
bead. A thread activated from a *list* has named no bead either, so it means the same thing.

`is_thread` decides what a thread dictionary is by the entry Table 158 makes **required** — `/F`,
an indirect reference to the first bead — accepting the optional `/Type /Thread` where it is there.
A bead is not mistaken for one: Table 163 names a bead's own reference to its thread `/T`, pointing
the other way, and `/F` on a file specification names a file rather than a dictionary with `/T` or
`/N` in it.

### Shipped with a witness count of zero, said out loud

**Not one of the 974 corpus documents states an article thread.** Two catalogs carry a `/Threads`
entry — one an empty array, one a null — and no page carries a `/B`. So every assertion here is
against a fixture this project wrote: a two-page thread of three beads in `viewer-core/tests`, and
a hand-built list in `viewer-ui/tests/panel.rs`.

That is trap 8's converse and it is the same position §12.7.4.3's comb, right-quadded and password
fixtures have been in since the twenty-third session. `CLAUDE.md`'s two tracks make it explicit: a
project running only the demand side "finishes when the corpus goes quiet, which can happen with
much of the standard unimplemented".

## Consequences

- **The tab strip is six wide and `panel.rs`'s own comment predicted the cost.** Its `tab()` helper
  divides the panel by the number of tabs, with a note reading "**Update the divisor when a tab is
  added**: two tests failed the day §12.3.4's arrived". Three failed the day this one did, and the
  note now says so. A warning written where the work is *does* fire when it is a compile-time
  neighbour of the thing it warns about — which is the opposite of `requirements::unmet`'s failure
  in ADR 0178's story, and worth the contrast.
- **The label is "Read", not "Articles".** Six tabs share 300 logical pixels; the clause's own noun
  for what a person does with a thread is to follow it, and a label that does not fit is a label
  that says less than a short one.
- **What stays `partial`**: `beads_on_page` answers which article is under a point and no host asks
  it, so "which thread am I reading" has no question yet. That is the half a reader actually uses
  once the list has got them started.
- §12.3.5's collection is still open, and `doc/todo/36` keeps the argument for it — including the
  one decision this round did not have to make, whether a collection's container pages stay on the
  screen when the panel opens.

*(This ADR said the tab draws "Table 158's `/I` title" until the five-hundred-and-forty-fifth
session. A thread's `/I` information dictionary is Table **162**'s; 158 is the collection split
dictionary, one family over, and `article.rs` has read Table 162 all along.)*
