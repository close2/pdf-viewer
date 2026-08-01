# ADR 0099 — A transition is a table, not an action

Status: accepted, 2026-08-01.

## Context

§12.6.4.15's transition action was `reported`, and its row read as a feature owed: "run a page
transition (§12.4.4) in the middle of a presentation. Debt, with §12.4.4."

Table 219 has **two** entries and one of them is `/S`. The other is `/Trans`, "[t]he transition to
use for the update of the display (see 'Table 164 — Entries in a transition dictionary')" — the
*same* dictionary a page's own `/Trans` holds, which §12.4.4 has read since the seventieth
session. There was nothing to build, only something to connect.

## Decision

**`Action::Trans` carries `navigation::Transition` and nothing of its own**, read by the same
function a page's `/Trans` is read by. An action stating no `/Trans` is a dictionary rather than
an action, which is what Table 219 making the entry required means.

What §12.6.4.15 *adds* over §12.4.4 is not a transition but a moment:

> interactive PDF processors shall normally suspend drawing when such a sequence begins and
> resume drawing when it ends. If a transition action is present during a sequence, the
> interactive PDF processor shall render the state of the page viewing area as it exists after
> completion of the previous action and display it using a transition specified in the action
> dictionary … Once this transition completes, drawing shall be suspended again.

Suspending and resuming drawing is a *window's* business — `ViewState` has no screen — so
`perform` answers `Request::Transition` beside `Request::Display` and `Request::Resolve`, and the
caller decides whether it has a presentation to play.

The clause's own effect is satisfied here for a reason worth writing down rather than claiming
credit for: **every request in a `/Next` chain has already been performed by the time the
transition is read**, and this window draws whole pages. So "render the state of the page viewing
area as it exists after completion of the previous action" is what happens anyway; what is missing
is only the animation, and `viewer-ui` names the style and the duration and says so.

No corpus document states a `/Trans` at all — on a page or in an action — measured over every
object of all 964 openable ones, which is why §12.4.4 has said "read as data" since it landed.

## Consequences

`reported` falls 49 → 48, `partial` rises to 235. `Action::Refused` loses one of its twenty
names.

Two clauses now share one reader, which is the shape §12.7.7 and §12.7.8.3.3 took in ADR 0091 and
§12.6.4.3 and §12.6.4.4 did not: `GoToR` and `GoToE` look alike and are opposites, where `/Trans`
on a page and `/Trans` in an action are genuinely the same dictionary meaning the same thing at
two different moments. Telling those apart is what reading the table rather than the clause title
gives you.
