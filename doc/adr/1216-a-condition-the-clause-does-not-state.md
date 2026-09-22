# ADR 1216 — Two clauses gated on a condition neither of them states

Status: accepted, 2026-09-22. Session 1189, the batch's host-UI round.

§12.6.4.15's transition is drawn in a window that is not presenting, and §12.7.5.3's file-select
control submits the contents of a file a person chose. Two clauses, one shape: each was executed
except for one thing, and in both cases the thing was withheld by a condition the clause does not
state.

## 1. §12.6.4.15 — the mode is §12.4.4's and not this clause's

§12.4.4.1 conditions a *page's* `/Trans` on a presentation: the two entries "specify how to
display that page in presentation mode". §12.6.4.15 states no such condition:

> If a transition action is present during a sequence, the interactive PDF processor shall render
> the state of the page viewing area as it exists after completion of the previous action and
> display it using a transition specified in the action dictionary

All three windows nevertheless printed *nothing is presenting, so the page is shown at once* and
showed the end state. What made that a real gap rather than a wording one is that the machinery was
already there: `viewer_core::transition` shapes the frames, `viewer_host::Clock` draws them, and
each host keeps the outgoing page's display list. The only thing gated on the mode was the **clock**
— `Clock::started` was documented as "a clock exists only while a presentation is running".

So `Clock::for_one_transition` is the same clock under the other clause, and the difference between
the two is exactly one sentence of §12.4.4.1: **it never ticks.** The page's `/Dur` is stated as
presentation timing — "how to display that page in presentation mode" — so a transition action
outside a presentation animates and advances nothing, and `Clock::tick` answers `None` for the
lifetime of such a clock. `Clock::spent` is how a host knows the effect is over and the timer may
go, which keeps `CLAUDE.md` principle 2's rule that a window with a still page wakes for nothing.

Two things follow that each host now does, and they are the same three lines in three event loops:
the outgoing face is kept whether or not anything is presenting (an `Arc` and a target, no
rasterisation — the two page rasters are still taken once per transition and never per frame), and
a spent clock is dropped only when nothing is *armed*, because a transition armed and not yet begun
is one render request away from beginning.

**What is not claimed.** A transition action whose previous action changed nothing visible produces
no render request, and then nothing is animated — the state the clause asks to be displayed is
already on the screen. That is the reading, not an omission: the effect is the way from one state
to another, and there is no other state.

## 2. §12.7.5.3 — the filesystem is this program's and not the document's

Table 231 bit 21 makes a file-select control two things at once:

> If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select control. In
> this case, the field's text represents the pathname of a file whose contents shall be submitted
> as the field's value

The pathname was executed and the contents were not, for a reason that is right and was applied one
step too widely. `pdf_model::submission` reads a `/V` in §7.11.1's dictionary form — which reaches
§7.11.4's embedded file stream and therefore the bytes — and names a `/V` in the string form on
`Submission::owed`, because "a specification in the string form names a path on a filesystem
`CLAUDE.md` principle 3 gives this process none of". That row also recorded the alternative as
**rejected rather than deferred**: "a file-select control's pathname is whatever a person typed, so
the parameter would be a request to open an arbitrary path on behalf of a document."

**That sentence is true of a path the *document* wrote and false of a path a *person* typed**, and
the difference is the one `viewer_host::policy` already draws one clause over: §12.7.6.4's file is
named by the document, so `read_import` resolves it against the document's own directory and
nothing else (ADR 1155's neighbourhood); a file-select control's path is chosen by the reader, and
confining *that* to the document's directory would be this program refusing its reader access to
their own files.

So:

- `ViewState::choose_file` does both halves of Table 231 bit 21 at once — `/V` becomes the
  pathname, laid out and drawn like any other text, and the contents are kept beside the log,
  because an `Object` has no place for §7.11.4's stream and `CLAUDE.md` rule 1 keeps the document
  immutable. Setting the value any other way drops the contents, so a pathname and the bytes behind
  it cannot disagree. A field **without** the flag takes nothing and `viewer_core` says so: a
  pathname in an ordinary text field would be the wrong value under the right name.
- `viewer_core::Edit::ChooseFile` carries the file across the boundary whole, for
  `Edit::Attach`'s reason — rule 2 says the host owns the filesystem — and through the confined
  protocol the same way.
- `viewer_host::policy::read_chosen` is the read, with `CHOSEN_FILE_LIMIT` as the explicit memory
  budget principle 3 asks for by name, and it is **a function rather than a refusal at each call
  site** so that `doc/todo/38`'s *ask* and *warn* levels attach in one place. The document's own
  restriction on the same act is `Operation::FillInForm`, which `viewer_core::Viewer` already asks
  before the edit is logged.
- `viewer_host::form::edit_of` is where a person's typing becomes one verb or the other, shared by
  the three windows so they cannot disagree about it.

**When the file is read is a host's question and the answer is: once, when the person finishes with
the field.** Not per keystroke — that would open a file sixty times a second. GTK and Qt read it at
their controls' own commit, and `viewer-ui`, which draws the page's widgets itself, reads it when
Escape takes the keyboard back off the field.

**What is still owed, and it is a widget.** No window opens a file *chooser*: what a person does is
type the pathname the clause already says the field's text is. A `FileChooserNative` and a
`QFileDialog` are a convenience over that, and the round that adds them can drive a dialogue, which
this machine cannot. §12.7.5.3's row says so in those words.
