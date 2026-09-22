# 1203 — Table 22 states two consequences and only one of them stops a print

Session 1183. Status: **accepted**. Carries out the expired premise `doc/todo/65` recorded against
[ADR 0803](0803-every-bit-named-the-four-levels-in-one-place-and-a-file-on-a-page-or-taken-out.md) section 1, and
continues [ADR 1180](1180-a-print-operation-has-two-ends-and-only-one-window-has-a-printer.md)'s
print path. Amends nothing it does not name.

## 1. Bit 12 is a second operation, not a second answer to bit 3

§7.6.4.2's Table 22 states bit 12 as:

> ( Security handlers of revision 3 or greater ) Print the document to a representation from which
> a faithful digital copy of the PDF content could be generated, based on an implementation-
> dependent algorithm. When this bit is clear (and bit 3 is set), printing shall be limited to a
> low- level representation of the appearance, possibly of degraded quality.

Two consequences, and they are not the same shape. Bit 3 clear **withholds printing**. Bit 12 clear
**limits a print that goes ahead anyway** — the cell's second sentence says so in its own
parenthesis, "(and bit 3 is set)". A reader's level attaches to each separately, because a person
may want to be asked before a document is printed at all and never asked about its quality, or the
reverse. One level could not say that, which is `RestrictionPolicy`'s own argument (ADR 1144)
reaching the eighth position of the table.

`Operation::PrintFaithfully` is therefore an arm beside `Operation::Print`, asked of the same
`Command::Print` through the same `Viewer::standing`, with its own entry in the policy, its own
byte on the confined wire, its own `QUORRA_RESTRICTED_PRINT_QUALITY`, and its own word on a command
line — `print-quality`, because what the bit governs is the quality and not a second kind of
printing.

**At revision 2 it falls back to bit 3.** Table 22 marks the position "( Security handlers of
revision 3 or greater )", and below that revision it is inside the range the table reserves and
requires to be 1 — the construction bits 9 and 11 already have. What it falls back to is bit 3,
whose revision-2 cell is "Print the document" with no quality qualification at all: a revision-2
document that permits printing has stated nothing about fidelity, so a reader has nothing to
withhold.

## 2. The implementation-dependent algorithm, chosen and written down

The cell hands the meaning of *faithful* to the processor. A silence there would be
`restriction::Bit::PrintFaithfully`'s old sentence — "this tree has not chosen one" — standing for
ever, which is the shape `CLAUDE.md` calls a claim that decays. So it is chosen, and it has two
halves because one of them cannot answer the other:

- **The resolution.** What leaves this program is a raster of the page, so the resolution is what
  decides whether a faithful digital copy could be generated from it.
  `viewer_host::printing::MIN_DPI` is the low-level representation, and it is the floor this module
  already had rather than a number invented for the occasion — RFC 0004 §3's own "below this a
  printed page is visibly a screen picture". Choosing the floor keeps the degradation a *statement*
  rather than a second budget somebody would have to maintain.
- **The destination.** A job written to a file is a document a faithful copy could be generated
  from *whatever* it was drawn at, so degrading the pixels and then writing them into a file would
  obey the bit in one direction and not the other. `printing::destination_refused` is the sentence;
  `quorra-gtk` refuses a `GtkPrintOperation` whose settings name an `output-uri`, and `quorra-qt` a
  `QPrinter` with an output file name. A window with no such destination has nothing to do here,
  which is how the three stay level (ADR 1190).

## 3. The one held question whose `no` starts something

Every other operation this crate holds under *ask* forgets what it held on a `no`. Bit 12's does
not: a person declining here has asked for the degraded print the cell describes, not for no print.
`Held::PrintFaithfully` is that arm and `Viewer::answer` takes it before the `no` branch. A `no`
that cancelled the job would have made bit 12 a second bit 3, which is exactly the reading section 1
rejects.

`Event::Printing` carries the answer as `Fidelity`, because the two facts are decided together and a
host that had to ask afterwards could open its dialogue on a destination the document withholds.
`Event::Refused` naming `PrintFaithfully` arrives beside the grant — a refusal and a grant in one
turn, which is the pair Table 22 states.

## 4. Qt prints, and QtPrintSupport is asked for rather than assumed

ADR 1180 deferred `QPrinter` because `cxx-qt-lib` binds no `QtPrintSupport` type. The bridge is now
written: `QtUpdate::print_dialogue` is the flag (a `QPrintDialog` is a Qt object and Rust never
calls one), and the C++ side drives the job back through `print_job`, `print_paper`, `print_cells`,
`print_page`, `print_page_pixels`, `print_reports` and `print_finish`. **No new `unsafe` token**:
every one of those is an ordinary `cxx` method on `Host`, and `tests/unsafe_position.rs` still finds
the single hand-written token in `src/bridge.rs`.

`build.rs` asks `qmake6 -query QT_INSTALL_HEADERS` whether `QtPrintSupport/QPrinter` is there before
it asks `cxx-qt-build` to link the module, and `window.cpp` asks the same question of the same
directory as `__has_include`. A preprocessor definition would have been the direct way and needs
`cc_builder`, which is an `unsafe fn` in `cxx-qt-build` 0.9 — this crate's whole position is one
`unsafe` token, so the question is asked twice instead. A machine with Qt 6 Widgets and without
QtPrintSupport is a real configuration, and it keeps the preview and says so.

## 5. What was exercised, and what was not

- `pdf-model/src/restriction.rs`: bit 12's arithmetic, the revision-3 threshold and the fallback to
  bit 3, with `Operation::Print` asserted beside it on every row so that neither alone can pass.
- `viewer-core/tests/printing.rs`: all four levels over bit 12 against
  `doc/pdf.js/test/pdfs/secHandler.pdf` — `/V 5 /R 6 /P −3136`, bit 3 clear and bit 12 clear — and
  the *ask* level answered both ways. **Session 1171 recorded that no such fixture was found after a
  forty-minute walk**; `pdf-model/examples/encryption_census`, extended here to print both printing
  positions, finds every encrypted document in `doc/pdf.js` in about a second, and two of them
  withhold bit 3.
- `viewer-host/src/printing.rs`: the floor at every resolution, and the refusal naming its position.
- `viewer-qt/src/host.rs`: the whole print path driven with no `QApplication` and no printer.
- **`QPrinter` and `QPrintDialog` were compiled, linked and not driven.** A print job is somebody
  else's paper on a shared machine, and the dialogue is modal. `quorra-qt` links
  `libQt6PrintSupport.so.6` and the binary contains `MainWindow::runThePrintDialogue`; what a later
  round owes it is a run in front of a person, which is the owner's loop.
