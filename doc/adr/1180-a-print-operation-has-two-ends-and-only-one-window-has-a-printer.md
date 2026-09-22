# 1180 — A print operation has two ends, and only one window has a printer

Session 1171. Status: **accepted**. Carries out [RFC 0004](../rfc/0004-print-and-print-preview.md)
sections 4 to 6 as far as one round reaches, on the interpretation [ADR 1179](1179-table-167-states-two-answers-and-the-output-decides-which.md)
built. Amends [ADR 1144](1144-a-level-per-operation-and-a-window-that-can-set-it.md)'s note that
printing is "no verb in this window yet".

## 1. Why printing is a command and not a query

`CLAUDE.md` makes a document's restrictions the reader's, at four levels: `off`, `on`, *ask before
the operation*, *warn before the operation*. Two of those need an event, because two of them need
a person. §7.6.4.2's Table 22 bit 3 is "[p]rint the document", so printing has to be refusable,
askable and warnable — and `viewer_core::Query` raises no event and nothing can wait on one. That
is exactly `Command::Copy`'s argument (ADR 1144) reaching the second of the two operations that
argument named.

`Command::Print` therefore asks `restriction::asserted` for `Operation::Print` through
`Viewer::standing`, the one place every other operation's bit is asked, and `Event::Printing` is
the grant. `Operation::Print` already existed — `pdf_transform`'s `render` verb consumes it — so
what was owed was not the operation but a window that performs one.

## 2. Two ends, because the clause states a duration

§8.11.4.5 does not state an instant:

> When a document is printed by an interactive PDF processor, usage application dictionaries with
> an event type Print shall be applied over the current states of optional content groups. These
> changes shall persist only for the duration of the print operation; then all groups shall revert
> to their prior states.

A single message could say when to apply them and never when to take them back, so `Printing` has
`Start`, `Paper` and `Finish`. `Finish` asks nothing — a reader allowed to start is not asked
again for permission to stop — and is harmless where no operation is running, which is what lets a
window send it from wherever its job actually ended.

**`Paper` exists because the two facts are known at different moments.** The policy has to be asked
before a dialogue appears, or a person answering *ask* would be answering it behind a print
dialogue they have already begun filling in; and the sheet is what the dialogue is *for*. So the
grant carries a provisional sheet and the chosen one arrives afterwards, asking nothing: a
different paper is not a second operation, and Table 22's bit 3 does not distinguish two sheets.

## 3. What the window shows while it stands is what would print

The intent is stated on the document's own `ViewState`, so every page of it — the ones a printer
asks for and the ones already on the screen — is interpreted for paper. That is RFC 0004 §6's
preview, and it costs nothing extra: it is the same interpretation, asked once.

It also means the four windows are not level in what they can *do* and are level in what they
*show*. That asymmetry is this decision's main cost and §6 prices it.

## 4. The pages are a query, and a display list rather than pixels

`Query::PrintPage(n)` answers `Answer::PrintPage`, which carries the page's display list, the
target at the job's resolution, and the sentences saying what could not be drawn on it. It answers
nothing unless a grant is standing, which is what keeps §7.6.4.2's bit 3 a question the command
asked rather than one a readback walked around.

A query rather than a render request, because a printed page is not a frame: the
`Event::NeedsRender` loop draws what the reader is looking at, at the viewport's own magnification,
superseded the moment they scroll — and a print job asks for pages nobody is looking at, at a
resolution the printer chose, and needs all of them. Nothing is cached: a job asks for each page
once, and a thousand pages' lists would otherwise sit in the core for as long as the document is
open.

Marks rather than pixels because the host already has `viewer_host::Drawing` and because an A4
page at 300 dpi is 35 MB of raster: one page at a time is what a host wants, and a display list is
what lets it choose. The confined wire carries the same pair; the worker interprets and the window
process rasterises and spools, which is RFC 0004 §5's own arrangement and the portal's.

**`Answer::PrintPage`'s target is an `Option`, and that is trap 5.** A page whose marks will not go
onto a raster at the job's resolution says which page and why, among its reports, rather than
coming out blank.

## 5. The resolution, which no clause states

RFC 0004 §3 fixes it and `viewer_host::printing` carries it out: the printer's reported resolution,
clamped to 150–600 dots per inch, defaulting to 300 where nothing is reported. Every number there
is a documented choice, because the standard describes marks and not devices; the clamp's upper end
is a stated, revisable budget and the module says where raising it would be argued.

§12.2's half of Table 147 is in the same module, as `printing::Defaults`: `/PrintScaling`,
`/Duplex`, `/PickTrayByPDFSize`, `/PrintPageRange` (one-based, as the table's own NOTE insists),
`/NumCopies`, and the deprecated `/PrintArea` and `/PrintClip` carried rather than dropped. The
clause makes every one of them a statement about a *dialogue*, so nothing there decides anything:
a host opens its dialogue on them and a person changes what they like.

Table 148's `/Enforce` is the one entry that goes further, and it is **reported rather than
obeyed**: a document telling a reader what they may not change is a restriction, and `CLAUDE.md`
makes every one of those the reader's. `Defaults::enforcement_note` is the sentence.

## 6. The key, and the four windows

**Shift and P, not Control and P**, and the argument is the key table's own. `viewer_host::keys`
takes a `shift` and no other modifier, and says why: by the time a press has got past the chrome,
the widget that would have wanted Control has had it. `P` unshifted is §12.4.4's presentation and
has been since that table existed — so the letter a print job is named after was already spoken
for, and what is left is the modifier the table does read. RFC 0004 §5 recommends Ctrl+P; this
departs from it, and the RFC's head now says so.

**The residue is worth naming rather than hiding**: because the hosts discard Control before the
table sees a press, Ctrl+P in these windows is `p`, and `p` is the presentation. That is a
pre-existing property of every conventional Control binding here, not something this round
introduced, and it is handed over rather than fixed.

What each window then does:

- **`quorra-gtk` prints.** `GtkPrintOperation`, `set_unit(Points)`, the dialogue at
  `PrintOperationAction::PrintDialog`; `begin-print` sends `Printing::Paper` from the print
  context's own paper size and resolution, `draw-page` asks `Query::PrintPage`, rasterises with
  the processor backend and paints the raster into the cairo context at 1/scale, and the run's
  return sends `Printing::Finish`. **The operation is scheduled on an idle turn rather than run
  from the arm that received the grant**, and that is not tidiness: `run` turns the main loop, and
  every callback it raises comes back through `with`, which borrows the host — a borrow standing
  for the length of the job would drop every one of them.
- **`quorra-qt` and `quorra` show.** `cxx-qt-lib` binds no `QtPrintSupport` type, so a `QPrinter`
  here means hand-written bridge code and a second linked library; and the winit host has no
  toolkit at all, so RFC 0004 §5's recommendation for it is a panel of its own and a job submitted
  over IPP, which is a new dependency. Both are priced in the RFC and deferred here. What both
  windows do meanwhile is the preview, with the same key ending it — a window that could enter
  print intent and not leave it would have taken §8.11.4.5's revert away from the person who asked
  for the operation.
- **`quorra-confined` answers the grant and asks for none.** It sends no `Command::Print`; the arm
  exists so that an operation added to it later cannot behave like another one in silence.
- **The C ABI starts and ends the operation and draws its pages**: `quorra_print`,
  `quorra_print_finish`, `quorra_print_page`, `quorra_print_page_copy` and the two report
  accessors. The rasterisation is on the library's side of the boundary for the reason
  `viewer_ffi::session::rasterise` already gives — a display list is not a thing to put in a
  header — and the backend is the processor one, which is this project's correctness oracle and
  RFC 0004's whole argument for rendering the printed page ourselves.

## 7. The host composes nothing

`CLAUDE.md`'s authoring exclusion draws the line at the provenance of the marks. Every mark on a
printed page here came out of the document's own content streams and appearance streams, through
the same interpreter the screen uses; the host paints a raster into a context its toolkit gave it
and adds nothing — no page number, no header, no watermark. The spool container RFC 0004 §9 flags
as an open question for the owner is **not** built: on the GTK path the toolkit composes the job,
which is a print system doing what print systems do rather than this program producing a file.

## 8. What was exercised, and what was not

- `viewer-core/tests/printing.rs`: the grant opens the pages and `Finish` closes them; a `/F 36`
  annotation reaches the paper with its bit-3-clear control beside it; the target is the job's
  417 × 417 rather than the viewport's; a sheet chosen after the grant moves a fixed print
  watermark and raises no question.
- `viewer-host/src/printing.rs`: the clamp at both ends and inside, an unreported resolution, a
  resolution that is not a number, and Table 147's eight entries with a silent document as the
  control.
- `pdf-model/tests/print_intent.rs`: ADR 1179's truth table.
- **The GTK print path was compiled and reviewed and not driven.** No print job was sent: this is
  a shared machine and a job is somebody else's paper. What a later round owes it is a run against
  `PrintOperationAction::Export` with an export filename, which produces a file and touches no
  printer — and the reason it is owed rather than done here is that driving it needs a window and
  a display, which is the owner's loop.
