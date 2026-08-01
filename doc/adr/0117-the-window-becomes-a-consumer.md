# ADR 0117 — The window becomes a consumer, and a locked file gets asked

Status: accepted, 2026-08-01.

## What this decides

`pdf-viewer.rs` runs on `viewer-core`. Everything about documents, pages, zoom, links and
§12.6's actions moved into the crate; what is left in the binary is a window, a keyboard, a GPU,
and the two decisions a *host* owns — which files a document may name, and what to do when one
asks for a password. The file is 692 lines against 991 — and it is a *different* 692, with a
password prompt, a zoom, a scroll and a cursor in it that the 991 did not have.

Three things a person can now do that they could not:

- **A locked document asks.** §7.6.4.1 says a processor tries the empty user password and then
  prompts. The prompt has been owed since the twenty-second session and this file has called it
  "the missing piece, not the clause" for twenty of them. `viewer-core` answers a locked file
  with `Event::PasswordRequired` — not a failure, because a document that wants a password is not
  one this program cannot read — and the binary asks at the terminal, up to three times, with an
  empty line to give up. Eight corpus documents reach it; `issue6010_1.pdf` opens with `abc`.
- **The cursor knows what it is over**, and §12.5.5's rollover and down appearances are chosen by
  a pointer for the first time. They were implemented in the seventy-sixth session and nothing
  had ever driven them: `ViewState::set_pointer` had no callers outside its own test. A feature
  switched off in one place is switched off everywhere it is not switched on.
- **Zoom and scroll.** `+`, `-`, `0`, and the arrow keys down a page taller than the window.

## The decisions inside it

**Hit testing lives in the core, not the host.** The interesting half of "the mouse followed a
link" is a clause: Table 176's activation region, §12.5.2's coordinate space, §7.7.3.3's
rotation, and undoing the centring and the magnification the frame was drawn with. The mouse is
four lines. A host that did this itself would be re-deriving the page's transform from the
outside.

**A press activates on release, over the same annotation.** §12.5.5 describes appearances, not
activation, so this is a choice and it is recorded as one: a press dragged away before release is
a press the person changed their mind about. `a_press_dragged_off_a_link_does_not_activate_it`
pins it.

**The pointer state is only changed for an annotation that states the appearance.** Setting it
invalidates the page's display list, so a cursor crossing a link whose only appearance stream is
`/N` would re-interpret the page — 2 000 M instructions — for a picture that cannot differ. Table
170's `/R` and `/D` are both optional and most annotations state neither, so the check is cheap
and the saving is every mouse move over every link in every document.

**§12.7.6.4's policy went to the host, and its performance stayed in the core.** The clause says
a processor "shall import data … from a specified file" and says nothing about *which* files,
because that is a property of the processor. So the core emits `Event::NeedsFile` with the
document's own words, and the binary decides: one path component, resolved against the directory
the open document is in, and nothing else. That division is rule 2 in practice rather than in a
doc comment — and the identical `Command::Supply` answers a host that refuses, which is printed
rather than swallowed.

**Nothing in `viewer-core` is `#[non_exhaustive]` any more.** It was, for one session. A
`#[non_exhaustive]` message type forces every host to write a catch-all arm, and a catch-all arm
is exactly where a message added later goes to be ignored in silence — trap 5 wearing an
attribute. Closed enums mean a new `Event` fails to compile in every consumer until somebody
decides what it should do there. The cost is a breaking change for out-of-tree consumers, and it
is paid deliberately until there are any.

## The defect this found, which round one's tests could not

**A tier-2 host would have been asked to draw the same frame for ever.** `Rendered::Presented`
means "I drew it onto my own surface and hand you nothing", and the scheduler asked *what is the
viewer holding* to decide whether the screen was up to date. For a tier-2 host that is always
nothing, so every `RenderReady` produced another `NeedsRender`, immediately, without end.

Session 131's twelve tests all played tier-1 hosts. The protocol's two tiers were designed,
documented and given a variant apiece, and only one of them was ever run.

The fix separates two facts that had been one field: `Open::shown` is *what is on the screen* and
`Open::frame` is *what the viewer is holding*. Tier 1 sets both; tier 2 sets the first.
`a_tier_two_host_is_not_asked_to_draw_the_same_frame_twice` fails against the old condition.

## Consequences

Tests 907 → 912. The five are the tier-2 loop, a click that follows a link to its page, a press
dragged off one, §12.6.4.8's URI handed over, and a document saying what it carries before a page
is drawn. Ledger rows updated where the code moved: §7.6.4.1 (the prompt exists), §12.5.5 (hit
testing has a home), §12.7.6.4 (which half is whose). The four gates are unmoved.

`pdf_model::link::Link` gained an `id`, because §12.5.5's appearances are keyed by the
annotation's object and a link that could not say which annotation it came from could not have
one.

Verified without a display, which is as far as this machine goes: the document opens and prints
its page count, the password prompt accepts `abc` and gives up after three refusals, an
attachment is named, and the program then fails at `XOpenDisplayFailed` exactly where the
environment says it must. The window itself needs a person.

## The lesson

**A protocol with two modes has two implementations, and a test suite that plays one of them
tests half a protocol.** The variant was there, the doc comment explained it, and nothing had ever
sent it. That is the same shape as the assumption a test cannot exercise (the GPU backend
demultiplying onto an opaque background for fifteen sessions) — and the cheap defence is the same:
**ask which arm of your own enum no test has ever taken**.
