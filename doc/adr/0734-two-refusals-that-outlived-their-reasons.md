# ADR 0734 — Two refusals that outlived their reasons

Status: accepted, 2026-08-28. Session 801. Takes two of `doc/todo/15`'s remainder, and they are one
subject rather than two items on a list: a **page** refused at one magnification stayed refused at
every other, and a **worker** that died was read as the end of the document. Cites no clause —
this is `CLAUDE.md` principle 3's boundary reaching a person, as ADRs 0713, 0725 and 0729 were —
and the ledger is untouched.

## The subject: what a refusal is about

A refusal is a fact about a *question*, and it keeps only as long as the question does. Both
defects here are the same mistake made at two scales: a refusal recorded against something larger
than what was refused. `render-cpu` refused **this list at this target** and the screen recorded it
against the page; the kernel ended **this worker** and the window recorded it against the document.
In both cases the reader lost something the program had no reason to take away, and in both cases
the fix is to write down what was actually refused.

## Part 1: a page's refusal outliving the zoom that lifts it

`screen.rs` has held `Content::Refused(String)` since ADR 0713, and both arms of `Screen::take`
matched it without a guard:

```rust
// The same drawing, already refused: retrying a refusal is a loop.
Some(Content::Refused(words)) => Content::Refused(words),
```

Every other variant beside it carries the drawing's identity — the list's `Arc` and the target —
precisely so that "the same drawing" can be told from "a new one". This one did not, so the arm
fired for *any* later list payload of that page: a zoom, a resize, a re-interpretation. The
sentence on the status was right the first time and a lie thereafter, and the page never drew
again for as long as the document was open.

The commonest way to earn that refusal is the one that makes it worst: `render-cpu` refuses a
target whose pixels exceed `pdf_render::MAX_PIXELS`, so a magnification that overshoots is refused
and **the next one down is not**. A reader who pressed `+` once too often had permanently lost the
page, including at the magnification they had been reading it at a moment earlier.

**Decision: `Content::Refused { words, of: (Arc<DisplayList>, TargetSpec) }`.** The arm keeps its
guard against the identity every neighbouring arm already checks; anything else falls through to
what a page that has never been drawn does — the device on a device screen, the drawing thread on a
processor screen. The comment's reasoning survives intact and is now true: retrying *the same*
refusal is a loop, and a different drawing is a question the rasteriser has not been asked.

The device arm inherited the defect from the processor arm when ADR 0725 was written, so it is
fixed in both and tested in both.

## Part 2: a worker's death read as the document's end

`doc/todo/15` has carried the breach an allocation budget cannot see as owed "as a refusal", with
the reason it was not done: *making it a refusal needs a fallible allocation on a path this crate
does not own*.

**That sentence is true, and it is about the worker.** A refusal the *worker* makes — "this page
needs more than my ceiling, here is a sentence, ask me something else" — does need `try_reserve`
down through the interpreter, and nothing here changes that. But the reader is not asking the
worker for anything. The reader is asking the **window** for a document, and the refusal they need
is of the **page**: the rest of the document is untouched, and the process that failed held nothing
else.

Inside the confinement that costs nothing. A worker is a process; another one starts in
milliseconds; the document is on this side's filesystem by rule 2 and was already re-read once for
§7.6.4.1's retry (ADR 0718); and the command that killed the last worker is simply not sent again.
**The confinement is what makes this safe rather than hopeful**: a worker's death leaves nothing of
the document behind it, so the second worker begins from the file rather than from wreckage. That
is the same property ADR 0241 leans on for the cancel.

### Decision 2.1: the counting is `viewer-confined`'s, the starting is a host's

`viewer_confined::Resuming` owns exactly the part two confined hosts must not answer differently —
which refusals are worth another worker, how many in a row are enough, and what a resume goes back
to. It reads no file, knows of no window, and its tests need no pipe. What is a host's is what only
a host has: the bytes, the window's extent, and the page.

This is ADR 0713 Decision 3's rule applied to a policy instead of to a queue: the second copy is
where two hosts stop agreeing.

### Decision 2.2: only `WorkerDied`, and every other arm refused for its own reason

`Resuming::after` resumes from one variant of `ConfinedError` and stops on the rest, each for a
stated reason rather than by default — `Cancelled` is the reader's own kill and a second worker
would undo the key press; `WorkerMissing` and `Spawn` are about starting one; `Connection` is this
side's channel, which a second worker inherits; and `Malformed`, `UnrecognisedFrame`, `Uncarried`,
`Refused` and `NoRoom` all leave the worker **alive and answering**, so a restart would throw away
a working viewer to fix a message. **The match has no wildcard** — `ConfinedError` is
`#[non_exhaustive]` to its callers but not inside its own crate, so a variant added to it stops the
build here until somebody has decided which of the two answers it is — and a unit test walks every
one of the arms, which is the shape ADR 0729's `Key::ALL` test has. Between them the list is a
claim about the whole enum rather than about the arms somebody remembered.

### Decision 2.3: the budget is consecutive, not cumulative

`RESTARTS` is three, and the load-bearing word is *in a row*: `Resuming::showing` puts the budget
back the moment a frame reaches the screen. A cumulative budget would make a document that recovers
perfectly well fail on its fourth incident an hour into a read, which is a rule about the length of
the reading rather than about the document. Three is the smallest number that tells the cases apart
— one start proves the death was not the open's, a second that it was not the page's, a third
leaves room for a machine momentarily unable to spawn.

The same field is what a resume goes back to: **the last page a frame arrived for**, which is by
construction not the page that killed the worker. So a document that is fatal on page 5 costs one
key press per death rather than a loop the program drives, and one whose *open* is fatal costs
three starts and then a sentence.

### Decision 2.4: the restart happens at the loop's turn, not where the death was seen

A resume issues commands, and a command can see the next death. Recovering inside a recovery would
nest one restart inside another and make the depth of this program's stack a document's to choose.
`Host::died` records what is owed and returns; `about_to_wait` is the one place a worker is started
again, with nothing of the failed exchange on the stack. The deferral has a race worth naming and
testing — the reader can press Escape between the death and the turn — and the abort wins, because
`Host::reopen` declines for a window that has stopped.

### Decision 2.5: the pages already on the screen are left where they are

Nothing is forgotten on a death. The slots hold pixels and marks *this side* already has, and a
window that blanked while it recovered would tell the reader something worse than the truth. The
next frame replaces them page by page as it lands, which is `doc/todo/37`'s show-what-it-had with
the frame arriving from a different process than the one that produced the last.

### The honest limit, stated rather than discovered

**The magnification and the position on the page are not restored.** Nothing on this boundary asks
the viewer what they are — no `Query` asks for the magnification or the offset — so a host can
only replay what it issued, and `Zoom::In`/`Out` and `Scroll` are relative commands the viewer
clamps: replaying them is inexact rather than merely long, because a ladder that clamps makes
`In In In Out` and a net `+2` land in different places. So the view returns to the document's
opening one, and the window says so in the sentence it prints and in its title. `doc/todo/15`
carries the exact restore as the next piece, and names its shape: a question on the boundary, held
by the host per frame.

## Proof, driven under Xvfb on the release programs

See `doc/history/801-*.md` for the run. The instrument for a death is `pkill -9 -x pdf-view-worker`
against a window that is up, which is the same thing a ceiling breach is from the host's side — the
worker's output closes and `read_exact` returns `UnexpectedEof`, exactly as it does for `SIGABRT`.

## Trap-13 calibration

Every new test was run against an injected defect before being believed; the table is in the
history file, and the suite is green as committed.

## What this does not close

`doc/todo/15` keeps the rest: moving the three established windows onto the boundary, the exact
view restore this ADR names, and the real-adapter measurement ADR 0725 owes to the owner's session.
The device path still warns about nothing, for ADR 0729's reason — quorra has no interrupt — and
that stays a question for the dependency rather than a debt of this item.
