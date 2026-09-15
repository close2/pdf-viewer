# ADR 1090 — What a clause conditions on the pointer, and what it leaves unsaid

## Status

Accepted, 2026-09-15. Session 1076. Two decisions in §12.5's last executable rows, each about an
entry the standard conditions on what a pointer is doing and each about the case the standard does
not then state.

`§N` is ISO 32000-2 and nothing else.

## Context

§12.5.5 gives an annotation three appearances and says which of the three a pointer asks for. Two
of §12.5's `partial` rows had a residue of the same shape and neither had been decided:

- §12.5.6.19's Table 192 conditions six of its eleven entries the same way — `/CA` and `/I` "when
  it is not interacting with the user", `/RC` and `/RI` "when the user rolls the cursor into its
  active area without pressing the mouse button", `/AC` and `/IX` "when the mouse button is
  pressed within its active area". Four of the six had no reader and were reported by name, on the
  argument that "a constructed appearance is one stream rather than §12.5.5's three". That argument
  named a capability rather than a clause, and the capability arrived: `view::Appearance` has
  carried the pointer state since session 132 and `viewer_core::interact` has dispatched every
  Table 197 trigger since session 1065.
- §12.5.6.2's Table 172 makes `R` the default value of `/RT` and states a `shall` for it:
  "[i]nteractive PDF processors shall not display replies to an annotation individually but
  together in the form of threaded comments." Every reply's `/Popup` was opened as a window of its
  own, which is the display the sentence forbids.

## Decision, part 1: the pointer selects a constructed appearance too

Table 192's six entries are selected by the same `view::Appearance` that selects `/AP`'s `/N`, `/R`
and `/D` — `appearance::icon_entries` and `appearance::caption_entries`. One constructed stream per
state is what Table 191 asks `/MK` to build: "an appearance characteristics dictionary … that shall
be used in constructing a dynamic appearance stream specifying the annotation's visual presentation
on the page". The reported refusal for `/RI`, `/IX`, `/RC` and `/AC` is gone with it.

Table 192's own scoping is kept and is not a convenience: `/RC` and `/AC` are "push-button fields
only", while `/CA` is the one entry the table exempts — "the CA entry may be used with any type of
button field, including check boxes … and radio buttons". So a check box has one caption in every
state.

## Decision, part 2: an absent state draws the normal entry

**Where the state's own entry is absent the normal one is drawn.** Table 192 makes all six optional
and states no fallback, so this is a choice rather than a sentence, and it is made for two reasons:

- §12.5.5 states the fallback for the same three conditions in `/AP`. Table 170 requires only `/N`,
  and `annotation::stored_appearance` has read an absent `/R` or `/D` as the normal appearance since
  long before this. Two mechanisms that describe the same three pointer states would be a worse
  reading for disagreeing.
- Taking the entries literally would erase a push-button's picture and its caption the moment a
  pointer crossed it, and put them back when it left. Drawing what the file *does* state is the
  better of the two answers where the standard states neither.

The fallback is per entry and does not reach past it: an `/RI` that names no form `XObject` is not
replaced by `/I`, because the file stated that entry and what it names is wrong.

## Decision, part 3: a thread hangs on the highest window its chain states

`popup::thread_window` climbs Table 172's `/IRT` while the annotation is a reply — `/RT` absent or
`R` — and answers the window of the **highest** ancestor that states a `/Popup`, with the number of
hops to it. `popup::popups` folds every other window into that one as a `popup::Comment`, and
`popup::popup_of` answers the same window, so §12.5.1's activation of a reply exhibits the thread
rather than nothing.

**The head of the chain is the obvious host and it is not enough**, which is a count rather than a
judgement: `examples/annotation_group_census` over ISO 32000-2's own PDF finds 1752 replies, every
one naming a `/Popup` of its own and only 1401 with a chain head that names one. Hanging a thread on
the head alone would drop the text of the other 351.

**A reply whose chain opens no window above it keeps its own.** The clause forbids displaying a
reply separately from what it replies to; it does not require displaying one that has nothing to be
shown together with, and obeying it by losing the text is trap 5's failure. A window is never its
own host either, which is what a file whose `/IRT` chain loops produces.

Table 186's `/Open` on a folded reply opens the window the thread is shown in, by
`popup::opens_with_the_page`'s reading: each entry states a condition under which a window is open
and none states one under which it is closed.

## Consequences

- A host that draws `PopupWindow::text` and ignores `PopupWindow::replies` shows a fifth of a
  reviewed document's comments and no sign that the rest exist; `viewer_host::Window::replies`
  carries them to all three hosts and `viewer_ui::chrome` indents each by its depth.
- No corpus document exercises either decision: no widget of the 833 states `/RI`, `/IX`, `/RC` or
  `/AC`, and one annotation of 34 835 states an `/IRT` at all. Both are defended by hand-built
  fixtures and by ISO 32000-2's own PDF, which is trap 8's converse, and each fixture was watched
  fail with its rule removed (trap 13).
- §12.5.6.19 and §12.5.6.2 stay `partial`, because what is left in each is a *decided* departure
  with its cost written down — `/TP`'s four side codes, and `/RC`'s XFA formatting — which is
  `doc/questions/Q63`'s shape and the owner's to name.
