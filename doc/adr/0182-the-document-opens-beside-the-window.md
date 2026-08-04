# ADR 0182 — The document opens beside the window, not before it

Status: accepted, 2026-08-04 (session 281).

## Context

ADR 0179's launch timeline made the shape of a launch legible for the first time, and the shape is
two independent things done one after the other:

```text
trace:   document open            37.225 ms  (+27.768)   ← the file
trace:   event loop               45.236 ms  (+8.011)    ┐
trace:   window                   45.392 ms  (+0.156)    │ the window
trace:   graphics device          90.519 ms  (+45.127)   ┘
```

**Nothing in the second group depends on the first.** An event loop is a connection to a display
server, a window is a request to it, and a graphics device is an adapter and a surface; none of
them has ever looked at a PDF. And the first group is 21 to 28 ms on ISO 32000-2 — 1023 pages,
101 318 objects — against 0.84 ms on a five-page file, which is [todo 42](../todo/42-the-launch-path.md)
item 1's residue after ADR 0180 took 41% off it.

The other three costs on that path are somebody else's: `EventLoop::new` is winit's, bring-up is
quorra's (`doc/QUORRA_FEEDBACK.md` §8), and the first frame's extra 12 ms is quorra's too (§9).
**This one is ours.**

## Decision

**`main` spawns a thread that reads the file and opens the document, and joins it in `resumed`,
after the presenter exists.**

```rust
let opening = std::thread::spawn({
    let path = path.clone();
    move || open_document(&path, opens_at)
});
```

The thread returns `(Viewer, Vec<Event>)`. `App::receive` then reacts to those events through the
same queue loop `dispatch` uses, so a `PasswordRequired` raised on the thread is answered exactly
as one raised by a command — `dispatch` and `receive` differ only in where the first events came
from, and both end in `pump`.

**`viewer-core`'s rule 4 — "no threads the core was not handed" — is kept, not bent.** The core is
*made* on that thread and moved back to the main one; it is still single-threaded, still owns no
scheduling, and still has no clock. What crosses the boundary is a `Viewer` and a `Vec<Event>`,
both of which are `Send` and neither of which is shared. Nothing in `viewer-core` changed.

**The join is after the presenter and not before it**, which is the whole point: joining before
the device would overlap the document with `EventLoop::new` alone and leave the 45 ms of bring-up
sequential behind it.

## What it is worth

Measured with the same instrument, ISO 32000-2, under `Xvfb` with `lavapipe`:

| | before | after |
|---|---|---|
| document | +27.8 ms, on the launch path | **+3.0 to +5.6 ms**, and that is the *join* |
| process start → first frame | 145 / 152 / 145 ms | **108.6 / 134.7 / 130.3** |

The totals are noisy because `EventLoop::new` on this virtual X server costs anywhere from 14 to
43 ms, which is why the row that matters is the first one: `document joined` now lands three to six
milliseconds after `graphics device`, where `document open` used to be a step of its own. **The
document is ready and waiting by the time the device exists**, and what is left of it is the cost
of the handshake.

A five-page file gains nothing measurable and loses nothing: its open is 0.84 ms, so the thread
finishes long before the join and the whole exercise is one `spawn`.

## What it changes for a person

Two orderings move, both on the terminal and neither on the screen:

- The document's own notes — page count, outline count, §12.11's requirements, §12.8's signatures
  — are printed *after* the window exists rather than before it. They were never synchronous with
  anything a person could see.
- **A locked document is asked for its password after the window opens** rather than before
  (§7.6.4.1). Checked by running one: the prompt appears, an empty line still gives up, and the
  exit is the same. Arguably better — the window is on the screen while the terminal asks — but it
  is a change, so it is written down rather than discovered.

Verified beyond the gates with ADR 0126's recipe: the window opens ISO 32000-2, its sidebar opens
by itself (`/PageMode /UseOutlines`), five arrow keys turn to page 6 presenting in 9.6 to 22.4 ms,
and the photograph shows the outline panel and the page. `--page 3`, `--cpu` and the password
prompt all still do what they did.

## The lesson

**A timeline makes a dependency visible that a profile cannot.** Every one of these steps had been
timed individually for sessions; what nobody had written down is that two of them are independent,
because a per-step duration says how long something took and nothing about what it waited for. The
question a timeline asks — *why is this step after that one?* — is the one that produced this
change, and it took one `spawn` to answer.
