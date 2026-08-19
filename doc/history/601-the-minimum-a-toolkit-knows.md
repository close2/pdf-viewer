# 601 — The minimum a toolkit knows

Both tracks: `doc/todo/30`'s last item, which closes what that file called *surface rather than
architecture*, and one `partial` ledger row read against the code that consumes it.

## The demand-driven half

`doc/todo/30`'s tail was Qt measuring its controls on the far side of the `cxx` bridge. What
crosses now is four integers per control — the extent the `/Rect` asked for and the extent
`QWidget::minimumSizeHint` will not go below — in one call per placement, and everything done with
them is `viewer_host::ControlFit`'s, the arithmetic `viewer-gtk` has fed since ADR 0346. The
counting that used to live in `cpp/window.cpp` is gone, along with the sentence it formatted.

`w` is bound the way GTK binds it and deliberately not through `keys::command`: the command is
`Zoom::Scale` of a number this page's controls measured, so it cannot be built from the key. A test
says the key table does *not* answer for it, which is the mistake a later session would make.

Driven under `Xvfb` on `160F-2019.pdf`, release binary, `--trace=panel`:

- `13 of 76 control(s) wider than their /Rect (worst +66 on 18 px), 76 taller (worst +20 on 14 px);
  every control fits at 4.667, which `w` sends`
- after `w`: `fitting §12.7's controls at 4.667`, then `0 of 76 control(s) wider than their /Rect
  (worst +0 on 0 px), 0 taller`.

GTK answers 3.278 for the same page and this host answers 4.667. That is the finding intact rather
than a disagreement — a minimum size is a *style's* — and it is the first time the two hosts differ
in the measurement alone, which is what putting the arithmetic in one crate was for.

**No message.** Eleven in eleven rounds of hosts, and the fourth host tail in a row closed with the
vocabulary that was already there.

## The spec-driven half

§12.7.5.4, `partial`, read against the code a host builds its list box from. `doc/habits.md`'s
fifth sweep, in its plainest shape: *the model implements this — who calls it?*

Table 234's `/TI` is "the index in the Opt array of the first option visible in the list".
`pdf_model::form::ChoiceControl::top` has read it since the three-hundred-and-ninety-eighth session
and the page's own appearance obeys it since ADR 0407 — and `viewer_host::form::ControlKind::List`
carried `options`, `selected` and `multi` and dropped it. So every native host's list box started at
row 0 over a picture that started somewhere else, and the ledger row said the top index "crosses",
which was true of `pdf-model` and false of the mapping a host actually reads.

`ControlKind::List` carries `top` now; `viewer-qt` scrolls to it. The test sets the selection and
the top index to *different* rows, because the two agree on most real lists and a control that read
one for the other would pass anything else. `viewer-gtk` does not obey it yet and the reason is the
binding's floor rather than a decision — `ListView::scroll_to` is GTK 4.12, this crate binds
`v4_10` — so `doc/todo/30` grew one item as it lost its last one.

## What this round did not do

`doc/todo/36`'s `Query` for a page's text was the other tail on offer and was left. Its own file
says why and the reason held on reading it: the message is to be taken *when a host wants a page's
text*, and none of the six consumers has asked. A `Query` added because the list looked short is the
thing `doc/ui-boundary.md`'s test exists to refuse.
