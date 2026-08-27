# 0704 — A quick frame is waited for, not stood in for

**Status.** Accepted — the project owner's report of 2026-08-27, verbatim: *"when
zooming out the newly visible border is always for an instant blurry. Shouldn't we, at
least for the iso spec, which only shows some text, be fast enough to render the frame
fast enough, so that we don't need to show the blurry text?"* The answer is yes, and
the machinery already said so — `Refusal::InsideTheRefresh` is rule 5's own sentence
that a frame landing inside one refresh *is* the frame every refresh — but two
measurement defects meant it almost never fired. Found by driving window resizes on
the owner's own 890M through the measurement loop and reading the trace.

## The two defects

**The prediction measured to the collect, not to the finish.** `Landed::waited` was
`in_flight.elapsed()` taken when the event thread *collected* the frame — and a
finished frame sits in the channel until the next tick reads it, so every wait was
quantised up to roughly one refresh. A dense text page whose render took 4–7 ms
reported ~8.7–9.2 against the owner's 8.333 ms period: the prediction sat permanently
just over the bar it was compared to, and every view change of a quick page was stood
in for. Self-sustaining, because the stand-in cadence is what collects frames. The
render thread now stamps `Done::finished` the moment the frame is done, and the wait
is `finished − asked`.

**The observation had no unit.** Rule 5's second way of knowing was a boolean — "a
render asked for at an earlier tick is *still* being drawn" — and on a 120 Hz display
the tick after a view change arrives 2–8 ms into a render that will land well inside
its refresh. Being in flight is not being late. The observation is now `out_for >
period`: a render out past one whole refresh has been *watched* missing, and one out
for two milliseconds has missed nothing yet. The bound is the display's own period —
no constant was introduced.

One sentence was also made honest: the stand-in's trace line always printed "expected
to cost X ms … so it misses", even on ticks where the prediction was under the period
and the *observation* was what fired — a sentence about the wrong number. `Stand` now
carries which way the miss was known (`Missed::Predicted` / `Missed::Observed`), and
the line prints that.

## What it changes on the screen

A zoom step on a page whose frames fit the refresh — the ISO specification's text
pages, at ~4–7 ms against 8.333 — now refuses the stand-in and waits: the true frame
lands inside the same refresh and the blurry proxy border never appears. The 890M
trace shows the refusal firing (`no reprojection (unwise): this frame is expected to
take 5.7 ms … and has not missed one yet`) where before the fix every one of those
ticks drew the blur. A frame that genuinely misses — the worst page's ~60–95 ms, a
resize's rebuilt-chrome frame, a prediction that turns out wrong — stands in exactly
as before, one refresh later at worst, which is the frozen tick rule 5 always priced
against a wrong picture.

What this deliberately does not do is make the stand-in *better* for the frames that
genuinely miss; that is the render getting faster (quorra's ADR 0084 stages), not the
policy getting cleverer.

## Held by

The rule-5 test gains the distinguishing case — a render out for two milliseconds
with a cheap prediction refuses the stand-in; the same render out past a period is
stood in for — beside the existing prediction and observation cases, unmodified. The
viewer-ui suite (99) and the full workspace pass; the 890M trace above is the
behavioural evidence.
