# 1987 — The confined window's click is the pointer's two ends, and the pins stay

Session 1575, slot 5 of batch ninety-one. Status: **accepted** and **built** in `quorra-confined`.
Context: ADRs 0713, 1155, 1190, 1227, 1592, 1827, 1960, 1961; `doc/todo/30`'s list of what each
future feature brings to this window. Code: `Host::click` and the `MouseInput` arm in
`quorra-confined.rs`. Drive: step `89-rendition-click`. Cites no clause and moves no ledger row.

## 1. What the brief supposed, and what held

The brief said a click on a rendition's screen annotation does nothing in `quorra-confined`, and
asked for `Request::Rendition` and `Event::Media` to cross the wire "as the other events cross
(kind 28 exists; the click's route does not)". **The first half held, but the cause was one level
nearer the window.** `Request::Rendition` is `viewer-core`'s, inside `interact::perform`, and it
never crosses. The worker's viewer resolves it from a pointer release, as every window's viewer
does. `Command::Pointer` was already on the wire as kind `POINTER`, and the protocol's own test
round-trips it. `Event::Media` was already carried (28) and already answered by the window
with `viewer_host::handed_media`. What was missing was the press itself: the window sent
`Command::Pointer` for nothing. A left press reached the page only while §12.9's measuring was on,
as a point. So the click is one host change, and no new command kind.

## 2. Decision

1. **Both ends of a left press go to the worker as `Command::Pointer`**, `Pressed` and then
   `Released`, at the cursor in the window's device pixels. They go through `dispatch`, so a busy
   worker holds them in order (ADR 1827). Excluded: measuring, which keeps the press as a point,
   and §7.6.4.1's card.
2. **After the release the keyboard follows §12.6.3's focus**, as after a Tab (ADR 1592). A press
   inside a single-line text field is how a person says "type here", and the worker's viewer has
   moved the focus there.
3. **The pins stay.** `doc/todo/30` said click handling would bring `--links=` and
   `--remote-documents=` with real *ask* arms. ADRs 1155 and 1227 pinned both to `refuse` for want
   of a dialogue to put the *ask* level's question in, and that reason has not changed. So a click
   on a link to another program, or to another file, is now refused out loud, through the arms that
   were already there for an action no click could reach. A link inside the document is followed,
   a check box toggles, and a rendition's clip is judged and handed over, each through the events
   the window already answers.
