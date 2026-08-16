# 0385 — The base a lost encode did not need, and the word every refusal now says

**Status.** Accepted. Session 550. A defect round on the project owner's own trace, the **third**
report of the same sentence in three sessions. Corrects ADR 0383 section 2 (the base as a field of
`Settled`) and ADR 0384 section 6 (which read the atlas repack as something that "cannot be worked
around from here"), and finishes ADR 0384 section 2b, which made *one* refusal speak. Rests on ADR
0378 (the reprojection and its five rules, all of which still bind) and ADR 0351 (the retained
frame).

**The lesson in one sentence, before the detail:** *this feature has now failed three times, and
every time the failure was a refusal asking for something it did not need — a measurement it could
only get by not refusing, a ratio nobody had measured, and here a capture whose answer was already
in memory.*

## Context

`tmp/trace3.entwurf.txt`, the owner's second run on their own machine — AMD Radeon 890M under RADV,
a surface stating 120 Hz — carries this line **twice in twenty-four presents**, and each time the
frame before it is a rendering that repacked the glyph atlas (5.595 → 6.085, and 7.572 → 7.575):

```text
5.595 frame p1 58009cmd presented 236.2 | … repacked | 40 up, 38376 culled
6.085 no reprojection: the device has no retained encode to replay — the last frame repacked its
      glyph atlas, or none has reached it yet
```

ADR 0384 section 6 diagnosed the cause correctly — quorra throws the retained encode away with the
tile placements when it repacks (its ADR 0050) — and then concluded:

> **It cannot be worked around from here** … the repack happens *after* the frame … so by the time
> the host could react the encode is already invalid, and capturing then would re-encode, which
> rule 4 refuses by name and for good reason.

Every clause of that is true about **capturing**. It is a complete non-sequitur about **drawing**.

## The reading, verified against the code before anything was changed

Three questions, in the order the brief asked them.

**What does `Settled` hold after a repack?** Nothing. `Stale::settled` — called for every real frame
— constructed a fresh `Settled { page, target, base: None }`, so the pixels of the *previous*
rendering were dropped by the frame that replaced it. ADR 0383 section 3 chose that deliberately
("the pixels and the placement they were drawn at are the same fact, so they are replaced
together") and it is the right invariant; what was wrong was reading it as a licence to destroy the
older pair rather than to keep the two halves of it consistent.

**Does the base survive?** No, and that is the whole defect. In the owner's trace the base *had*
been captured — `5.354 approximated: … (read back in 5.4 ms …)` — and was thrown away by the
5.595 rendering a quarter of a second later, so that the 6.085 view change found nothing. The same
shape repeats exactly at 7.296 → 7.572 → 7.575.

**Was `plan` refusing for want of a capture?** Not `plan` — the brief's reading is right in
substance and one function out. `Stale::plan` returned `Reproject`; the refusal was in
`App::approximate`, which called `capture_base`, got `None`, and returned `false` in silence. But
`plan` carried the *same mistake in the run-level form*: its first line was `if self.refused`,
where `refused` means "the window will not be read back again on this machine" — so a device that
declined one readback switched off reprojection for the whole session, including for pixels this
host was already holding.

So the reading is confirmed, and it generalises: **a base is unusable when there has never been
one, when the page changed, or when the window changed shape. A lost capture is not on that list.**

## Decision

### 1. The base outlives the frame it was captured from, and carries what it is a picture of

`Base` moved from `Settled` to `Stale` and gained two fields: the page `Arc` it is of, and the
placement it was drawn at. That makes it a complete statement — *this window, showing this page,
placed here* — rather than a field whose meaning came from the record it hung off, and it is what
lets it outlive that record.

`Settled` keeps a `captured: bool` in place of the pixels, which is all `wants_base` ever needed.

**The invariant ADR 0383 built is not weakened; it is moved onto the thing it is about.** The
composition is now `Stale::composed`, which reads the placement **off the base it is going to
draw**, so a base and the transform that carries it are the same fact by construction. Before, the
transform was composed in `plan` — against `settled`, which after a re-base is not necessarily the
frame the pixels came from. `Plan::Reproject` therefore carries the *view being asked for* rather
than a composition, and `Stale::reproject` takes the page and the target and does the rest. Nothing
outside this module can supply either half.

That is a strictly stronger form of "compose, do not chain": a caller could not resample the base
under a foreign transform even if it tried, because there is no method that accepts one.

### 2. A refusal to capture is not a refusal to draw

`Stale::refuse` became `Stale::refuse_captures`, and `plan` no longer reads it as a veto. What
`plan` now asks is whether there is anything to draw:

```rust
if self.base.is_none() && self.captures_refused { … }
```

— no base held, and none may be asked for. That is the honest condition, and it is the same
correction as section 1 seen at the run level rather than at the frame level.

Four sites in `capture_base` follow from it. None of them refuses a reprojection any more; each
says only that *this frame's* pixels could not be had, and names whether a base is standing:

- **no retained encode** (the owner's line): the base already held stands in;
- **the device declined the readback**: the same, and no frame is read back again in this run;
- **a layout this host cannot resample**: the same;
- **a capture that re-encoded**: captures are still switched off for the run — that judgement is
  unchanged and right — but **the pixels it produced are now kept**. They have been paid for and
  they are this frame's own; discarding them made every later reprojection of the run compose
  against an older base for nothing.

### 3. Every refusal says which of two kinds it is, and the count reaches the summary

ADR 0384 section 2b made `Plan::TooDear` speak because it was "a judgement about two measurements
rather than an impossibility". This round audited **every** path that declines a reprojection
against the project owner's three words — *impossible*, *unwise*, *unnecessary* — and the table is
the deliverable:

| where | the refusal | kind | what it does now |
|---|---|---|---|
| `plan` | nothing has been rendered | impossible | says so |
| `plan` | another page | impossible | says so |
| `plan` | the window changed shape | impossible | says so |
| `plan` | rule 5: the frame lands inside the refresh | **unwise** | says so, with both numbers |
| `plan` | rule 4: standing in would not buy a refresh | **unwise** | says so, with all three |
| `plan` | no base held and no capture permitted | impossible | says so |
| `approximate` | no graphics device | impossible | says so |
| `approximate` | the device refused the approximated frame | impossible | says so |
| `reproject` | the placement does not invert onto this view | impossible | says so |
| `plan` | **captures refused for this run** | **unnecessary** | **removed** (section 2) |
| `capture_base` | no retained encode to replay | **unnecessary** | **removed** (section 2) |
| `capture_base` | the device declined the readback | **unnecessary** | **removed** (section 2) |
| `capture_base` | a layout this host cannot resample | **unnecessary** | **removed** (section 2) |
| `capture_base` | the capture re-encoded | **unnecessary** in part | **its pixels are kept** (section 2) |
| `plan` | the picture up already depicts this view | **not a refusal** | silent, deliberately |

Five refusals were unnecessary and all five were the same mistake: *refusing to draw because a
capture failed*. Not one of them is a judgement and not one is an impossibility — they asked for
something the design does not need, which is exactly the category the owner named.

**The last row is the one worth arguing about.** A frame whose placement is already on the window —
as a rendering quorra replays for the price of a replay, or as the approximation rule 1 forbids
drawing twice — is not a view change at all, and it is *every* frame of a document nobody is
touching. Counting those would make the summary's number meaningless and printing them would make
the trace unreadable. So `Plan::Render` is silent and `Plan::Refused` is not, and the split is
enforced by where it sits in `plan`: everything after the "did this view move" question is a
genuine refusal on a genuine view change, and every one of them speaks and is counted.

**There is deliberately no word for *unnecessary* in the trace.** An unnecessary refusal is a
defect rather than a state; the vocabulary a reader can see is two words, and the third is the one
this round removed. A vocabulary that could describe it would invite the next one to be labelled
rather than deleted.

The summary gained the line rule 3 was missing:

```text
N view change(s) showed the real frame instead: A had nothing true to move and B were a
judgement between two measurements — each says which, above (ADR 0385)
```

Printed at zero, for the same reason the reprojection count is: "nothing was refused" is the answer
a person reading a trace of a slow session most needs, and a line that appears only sometimes
cannot give it.

### 4. The order in `plan` changed by one line, and it was a defect

The page check now comes **before** the "did this view move" question. It did not, and a page turn
at an unchanged magnification therefore satisfied `settled.target.transform == target.transform`
and fell out as `Render`. Harmless before, because everything downstream refused anyway; not
harmless once that answer means "not a view change, say nothing", because the one thing this
feature may never approximate would have been the one thing it never mentioned. Caught by
`a_page_turn_is_never_reprojected` on the first run of the new tests.

## What this cost

- **`Stale` gained two fields and `Settled` lost one.** Net state is one `Option<Base>` more, which
  is the window's own pixels — already allocated, now not freed a frame early.
- **A base is held for the life of the run rather than for the life of a frame.** That is 800 ×
  1000 × 4 bytes on the owner's window, 3.2 MB, and it was already being held between a capture and
  the next rendering; what changed is that the interval no longer ends. It is bounded by the window
  and does not grow.
- **A lost capture is now asked for again on the next view change.** `capture_presented` answers
  `Ok(None)` off two field reads, so the cost of asking is nothing measurable, and asking is what
  gets the base back to the newest rendering as soon as one has an encode to replay.
- **Nothing on any judged path changes.** Every line of this round is in
  `crates/viewer-ui/src/bin/pdf-viewer/`, a binary, and `doc/todo/37` rule 2's structural test still
  walks every `.rs` outside `viewer-ui/src/bin` and finds nothing.

## What the harness establishes, and what it cannot

`Xvfb :77` at 900×1100, llvmpipe, release binaries of this tree at `b5505453` and after, driven by
`xdotool`: twelve `+` spaced 1500 ms so each waits for the frame before it, then a burst of six at
30 ms, then `Escape` — which matters, because the summary is only printed by a clean exit and a
`SIGTERM` skips `exiting` entirely. The witness is `tmp/Entwurf.pdf`, the owner's own, **not in the
repository and named in no test**; `doc/PDF20_AN001-BPC.pdf` renders inside a refresh on this
adapter and never reaches the case at all.

**Two runs of each binary, and the honest reading is that only one number moved.** The
reprojection count is dominated by run-to-run variance on this adapter — 14, 17 before and 17, 17
after — so the table ADR 0384 could write is not available here and inventing one would be worse
than saying so. What *is* deterministic is the frame the change is about:

| | at ≈38.67 s of the same scripted sequence | outcome |
|---|---|---|
| before, run 1 | `no reprojection: the device has no retained encode to replay` | **nothing moved** |
| before, run 2 | the same line, at 38.674 | **nothing moved** |
| after, run 1 | `…the base already held stands in, composed against the frame that produced it` | **drawn in 4.1 ms**, against a frame expected to take 96.5 |
| after, run 2 | the same, at 38.676 | **drawn in 4.7 ms**, against a frame expected to take 120.3 |

The same view change, at the same point of the same sequence, four times: refused twice and drawn
twice, and the picture cost a twentieth of the frame it stood in for.

**What the harness cannot say** is the part that matters most and is stated plainly, because a
harness is what hid this feature's defect twice already:

- **llvmpipe's costs are unlike a real device's**, which is the standing caveat of ADR 0384's own
  measurement section, and it is why the reprojection *counts* above are noise: on this adapter the
  frame and the readback are both processor work and move together, while on a real device they are
  produced by different hardware and do not.
- **`Xvfb` states no refresh rate**, so every run here takes `doc/todo/36`'s 60 Hz floor. The owner's
  surface states 120, which halves the period rule 5 compares against and doubles how often the case
  arises.
- **The atlas repack is quorra's and not the adapter's**, which is the one thing that makes the
  harness able to reach this case at all — it reproduced at the same second of two runs. That is
  luck rather than design, and it is why the run below was worth taking.

**One command answers it on the owner's own machine**, and its whole content is one line of the
trace:

```sh
./target/pdf-viewer --trace=frames tmp/Entwurf.pdf 2>&1 | grep -E 'no retained encode|approximated:|view change'
```

Zoom in eight or ten times. Before this round every atlas repack printed `no reprojection: …` and
the window did not move; after it, the same repack prints `…the base already held stands in` and is
followed by an `approximated:` line whose reprojection cost is the number that had never been
measured on real hardware. The last line of the run says how many view changes were refused and of
which kind.

## Clauses

**None**, for ADR 0378's, 0383's and 0384's reason unchanged: this is presentation and not a
reading. Nothing reprojected is a *rendering* of the page, so §10.7.4's scan-conversion rule does
not reach it, and the conformance ledger is unmoved.

## What did not move

`fmt`, `clippy --workspace --all-targets`, the workspace test run, the doctests, the conformance
checker, the corpus gate, the oracle, both text gates and both of `render-quorra`'s coverage lanes.
The session's own file carries the figures.
