# 0436 — The minimum a toolkit knows, and the row a list starts at

Status: accepted
Date: 2026-08-19
Session: 601

## Context

`doc/todo/30`'s last item was one sentence long and had been for ninety sessions: *"`viewer-qt`
still measures in `cpp/window.cpp`, so feeding the shared arithmetic means carrying the
`(asked, minimum)` pairs across the `cxx` bridge — a bridge change rather than a decision."*

What was on the far side of that bridge was ADR 0346's finding. A platform control has a minimum
size its style decides and a widget's `/Rect` is whatever the document says, so a control placed
over a form covers the page around it; `viewer_host::ControlFit` turns the pairs into the one
magnification at which every control fits, and `viewer-gtk` sends it with `w`. Qt did the
*counting* — five local integers in `placeControls` and a sentence formatted into `host_->note` —
and could not do the arithmetic, because the number that decides it is the page's current
magnification and that is on the Rust side.

So two hosts computed one finding twice from the same measurements, and only one of them could
offer the fix. That is the situation `viewer-host` exists to prevent (ADR 0246's `panel.rs`
argument): two hosts measuring the same thing must not be able to compute two different answers
from it.

## Decision

**1. The bridge carries the measurement and nothing else.** `QtMeasure` is four integers — the
extent the widget's `/Rect` asked for and the extent `QWidget::minimumSizeHint` says the control
cannot go below, in logical pixels — and `Host::measured` takes a slice of them once per placement.
One call rather than one per control: the answer is the worst ratio over the whole page, and a
bridge crossing per widget would be seventy-six of them on `160F-2019.pdf` to compute one number.

The division by the device pixel ratio stays on the C++ side, where the widget is placed, because
that is the one place that knows both numbers in the same units. Everything after it — the counts,
the worst excess, the ratio, the magnification — is `viewer_host::ControlFit`'s, unchanged and
shared.

**2. `w` is bound by the host rather than by the key table.** `keys::command` maps a `Qt::Key` to a
`Command` and cannot map this one: what `w` sends is `Zoom::Scale(the magnification this page's
controls measured)`, which is not a fact about the key. The constant stays in `keys.rs`, so the
table of transcribed `Qt::Key` numbers is still in one file, and `crate::host` reads it. A test
asserts that `command(FIT_CONTROLS)` is `None`, because the thing a later session would do is add
a `Zoom::Scale` there and invent the number.

**3. No message, and none was needed.** Eleven messages, and the fourth host tail in a row that
needed none. This is a `Query::Fields` answer, a toolkit's own measurement, arithmetic in a shared
crate and a `Command` the vocabulary has had since the hundred-and-thirty-first session.

**4. Table 234's `/TI` reaches the control a host builds.** The spec-driven half of the round, and
it is `doc/habits.md`'s fifth sweep in its plainest form — *the model implements this; who calls
it?* `pdf_model::form::ChoiceControl::top` has carried the entry since the
three-hundred-and-ninety-eighth session, the page's own appearance obeys it (ADR 0407), and
`viewer_host::form::ControlKind::List` dropped it: `options`, `selected`, `multi` and no `top`. So
every native host's list box started at row 0 over a picture that started somewhere else.

`ControlKind::List` carries `top` now and `viewer-qt` obeys it with
`scrollToItem(..., PositionAtTop)`. §12.7.5.4 states the two things separately — *"the index in the
Opt array of the first option visible in the list"* against `/V`'s selection — and the test that
guards it sets them to different rows, because a control that read one for the other would look
right on every list where they agree.

## Consequences

- `viewer-qt` reports the same sentence `viewer-gtk` does, from the same code, and offers the same
  key. Driven under `Xvfb` on `160F-2019.pdf`: `13 of 76 control(s) wider than their /Rect (worst
  +66 on 18 px), 76 taller (worst +20 on 14 px); every control fits at 4.667, which `w` sends`, and
  after the key, `0 of 76 … 0 taller`. The numbers differ from GTK's 3.278 and that is the finding
  standing rather than a disagreement: a minimum size is a *style's*, and the two hosts now differ
  in the measurement alone.
- `cpp/window.cpp` lost its formatted sentence. A host that says nothing about what it measured is
  worse than one that says it twice, which is why the trace line moved rather than went.
- **GTK does not obey `/TI` yet, and it is a debt with a reason rather than a decision**:
  `GtkListView::scroll_to` is GTK 4.12 and `viewer-gtk` binds `v4_10`, so obeying the entry means
  raising the floor or driving `GtkListBase`'s `list.scroll-to-item` action on a view that is not
  yet in a window. `doc/todo/30` carries it.
- The `unsafe` position is untouched: one hand-written token in `src/bridge.rs`, one exemption on
  `mod bridge`, two crates in the tree lifting the denial. `tests/unsafe_position.rs` asserts all
  of it and a shared struct added to the bridge changes none of it.
