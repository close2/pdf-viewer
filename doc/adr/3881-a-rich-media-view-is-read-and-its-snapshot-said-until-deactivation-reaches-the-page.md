# 3881 — A rich media view is read, and its snapshot said until deactivation reaches the page

Session 2295. Status: **accepted and built** (the reading); the snapshot's display **planned**.
Code: `pdf-model`'s `media/rich_media/views.rs` (`view`, `View`), `media/rich_media.rs`
(`invoked_view`, `activated`), `viewer-core`'s `interact/rich_media.rs` (`pressed`). Clauses: ISO
32000-2 §13.7.2.2.2, §13.7.2.3.5, §13.7.2.3.6, Tables 335, 341, 344, 345. Plan: `doc/todo/80`
section 1 step 4.

## Context

Table 335's `/View` names "a 3D view dictionary ... that shall also be referenced by the Views
array", "Default value: The first element in the Views array". A rich media view is Table 315's 3D
view plus Table 344's two entries: `/Snapshot`, "an image XObject ... to be displayed when the view
is invoked", with "If the Snapshot entry exists, the image it describes shall be displayed in place
of the current artwork"; and `/Params`, Table 345's state data "passed to the instance when the
view is triggered", which §13.7.2.3.6.3 has "the run time" load — for 3D content by an ECMAScript
`StateEvent`, and for any other asset by nothing the standard states.

## Decision

1. **The view is read for every configuration**: `/View` where stated (a direct object or one not
   in `/Views` is said and still used), else the first of `/Views`; none, no view.
2. **Table 315's components are said for a `Sound` or `Video` configuration**: camera, background,
   render and lighting modes, cross sections and nodes set up a 3D scene, and a clip has none. For
   the `3D` configuration they are the excluded artwork's and are not said twice.
3. **`/Params` is read past by name.** Loading state data is the rich media run time's protocol;
   the standard gives it only as ISO 21757-1's `StateEvent`, which the JavaScript exclusion holds
   out, and this program's players take no state data. §13.7.2.3.6.2's saving — "When the View is
   created" — is never reached: this program creates no view.
4. **`/Snapshot` is checked (a stream whose `/Subtype` is `Image`) and said as not displayed.**
   Displaying it is the page's work: the annotation's normal appearance is set aside for the image,
   fitted in `/Rect` as Table 306's poster is (ADR 1561), from activation to deactivation. The page
   learns of activation (a press, Table 197's `PO` and `PV`) and of Table 336's `PC` and `PI`; it
   does not learn of `XD`, the reader's stop of the player — Escape or the controller bar — which
   every host answers in its own player. Showing the image without that would leave it on the page
   after the reader stopped the clip, which is a defect rather than a reading.
5. **What would build it**: a command from the players to the core when a clip associated with an
   annotation leaves their hands, as `Command::PlayingIn` already carries for a sound in a screen
   (ADR 3407), raised by both the sound and the picture player; then a `ViewState` set of shown
   snapshots read by `annotation::decide` as a set-aside of the stored appearance, the way
   §12.5.6.6's retyped text is. The 13.7.2.3.5 row is `reported` until then.

## Consequences

A view's every part is either honoured, read past by name, or said as not displayed; nothing the
file states about a view is passed over silently. The image is still unshown, and that is said on
every activation that invokes a view holding one.
