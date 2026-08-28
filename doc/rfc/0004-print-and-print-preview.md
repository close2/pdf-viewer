# RFC 0004 — Print support and print preview

Status: **draft**
Round: 786, commissioned by the owner
Companions: RFC 0002 (the transform layer — page ranges and any spool-shaping derivative
go through it), RFC 0003 (file-system faces), RFC 0005 (text editing). Numbering was
reconciled at merge (round 788); the numbers here are final.

**Registers, kept apart deliberately in this RFC more than the others**: what GTK, Qt,
CUPS, Evince and Okular do is evidence about *interface convention and plumbing*. What a
printed page must *show* is the specification's, and this RFC has real spec contact —
§9 lists the clauses that become live and what the ledger already says about each.

## 1. Motivation

A viewer that cannot print is a viewer with a hole where most users' second verb goes.
RFC 0001's survey found printing among the most-assumed capabilities (its absence is
filed as a bug, not a feature request). This tree is unusually well placed: it owns a
CPU rasteriser that is *the correctness oracle* — the printed page can be exactly the
page the oracle certifies — and it already has the interpretation switches printing
needs (annotation flags, optional content, transfer functions) sitting one intent-flag
away from being asked.

## 2. Prior art

- **GTK4**: `GtkPrintOperation` — `begin-print`/`draw-page` signals, a cairo context
  per page, `GtkPrintSettings`/`GtkPageSetup`, GTK's own Unix dialog where no native one
  exists; fully exposed in gtk4-rs (`connect_draw_page` etc.).
  <https://docs.gtk.org/gtk4/class.PrintOperation.html>,
  <https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.PrintOperation.html>
- **Evince** keeps vector all the way: poppler renders cairo calls into the print
  context (`GTK_UNIT_POINTS`), with alternative export paths that hand a PS/PDF file to
  GtkPrintJob or to the XDG Print portal when sandboxed.
  <https://gitlab.gnome.org/GNOME/evince/-/blob/main/libview/ev-print-operation.c>
- **Qt 6**: `QPainter` onto `QPrinter`, `newPage()` between pages;
  `QPrintPreviewDialog`/`QPrintPreviewWidget` re-emit `paintRequested(QPrinter*)` per
  preview render; resolution via `PrinterMode` (`ScreenResolution` default,
  `HighResolution` = printer-defined / 1200 dpi for the PDF driver) and
  `setResolution()`. **cxx-qt-lib binds no QtPrintSupport types**, so Qt printing from
  this tree means hand-written bridge code — the same cost `viewer-qt` already paid
  once. <https://doc.qt.io/qt-6/qprinter.html>,
  <https://doc.qt.io/qt-6/qprintpreviewdialog.html>, <https://github.com/KDAB/cxx-qt>
- **Okular**: two modes. Default: write the selected pages to a temporary file and hand
  it to the spooler (`lp`/`lpr` with CUPS options — page selection and n-up happen in
  CUPS). Optional **"force rasterization"**: render each page to an image at a
  hard-coded **300 dpi** on Unix and paint through QPrinter.
  <https://github.com/KDE/okular/blob/master/generators/poppler/generator_pdf.cpp>,
  <https://github.com/KDE/okular/blob/master/core/fileprinter.cpp>
- **CUPS**: since the 2006 OpenPrinting agreement, **PDF is the standard job format**;
  cups-filters' `pdftopdf` does page-ranges/n-up/fit, and rasterisation
  (`pdftoraster`/Ghostscript) happens only for non-PDF printers. Submitting the PDF
  itself (`lp -d p file.pdf`, options like `-o page-ranges=…`, `-o number-up=4`,
  `-o fit-to-page`) preserves vector end to end. CUPS speaks IPP on localhost; the
  pure-Rust `ipp` crate (RFC 8010/8011, active, no C dependency) can submit jobs with
  options directly. <https://github.com/openprinting/cups-filters>,
  <https://www.cups.org/doc/options.html>, <https://crates.io/crates/ipp>
- **Sandboxed printing**: `org.freedesktop.portal.Print` — `PreparePrint` shows the
  dialog and returns settings + a token; the app then passes a **file descriptor of a
  finished document** to `Print`. The portal model *is* the confined model: whoever is
  sandboxed produces bytes, an unconfined broker spools them.
  <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Print.html>
- **Raster DPI precedent**: Okular 300 (explicit, in source); Acrobat "Print as Image"
  defaults to 300 (corroborated, not first-party-documented); `pdftoppm` defaults
  to 150 (screen-grade). 300 is the field's rasterise-for-print floor.

## 3. The central decision: who renders the printed page

Two honest routes exist, and the recommendation is firm.

**Route A — hand the PDF to the print system** (Okular's default). Preserves vector
data; CUPS does ranges and n-up. But the printed page is then **Ghostscript's (or the
printer's) rendering, not ours** — the project whose stated goal is Acrobat-class
fidelity would print another implementation's reading of the file, bugs included, and
none of our print-time semantics (current form-field values not yet saved, the user's
layer switches, our constructed appearances) travels unless we first bake a derivative
file. Route A is also blind on encrypted-but-printable files (the spooler would need
the password).

**Route B — we render** (recommended): `render-cpu` — the oracle — rasterises each
selected page at printer resolution with print intent (§4), and the raster is handed to
the toolkit's print path (cairo `draw-page` in GTK; `QPainter::drawImage` in Qt; a
raster-wrapping PDF via fd for the portal/IPP). What prints is what this project
certifies, on every host, identically.

Cost of B, priced: raster spools are large (A4 at 300 dpi RGB8 ≈ 26 MB/page
uncompressed) — mitigated by banding the render (constant memory per band) and by
Flate-compressed raster in the spool PDF; and text prints as contone raster rather than
as printer-resident vector, which at 300–600 dpi is visually fine (it is Okular's
force-raster mode and Acrobat's print-as-image made default) but is a real difference
prepress users can measure. **Route A stays on the map as a later, explicitly named
"pass the file to the printer" option** — through the transform layer, which would bake
the viewer state into a spool derivative first — once RFC 0002 exists to build it on.
Not v1.

**DPI policy**: render at the printer's reported resolution, clamped to [150, 600],
default **300** where the printer reports none (the field's floor; the clamp's upper
end bounds spool size and render time, and is a stated, revisable budget). Preview
renders at screen scale through the same intent, so preview and print differ only by
resolution, never by content.

## 4. Print intent — the interpretation switch, and the clauses it flips

Printing is not a new renderer; it is the *same* interpretation asked with a different
intent. Proposal: the render request gains an intent (screen | print) — one field, the
mechanism the boundary prefers (ADR 0457's precedent of adding a field over a message).
Under print intent:

- **Annotation flags, Table 167** (§12.5.3): visibility is re-decided by the print
  bits. Bit 3: "If set, print the annotation when the page is printed unless the Hidden
  flag is also set. If clear, never print the annotation, regardless of whether it is
  rendered on the screen. If the annotation does not contain any appearance streams
  this flag shall be ignored." Bit 6 NoView ("The annotation may be printed (depending
  on the setting of the Print flag)") and bit 2 Hidden complete the truth table. The
  ledger's §12.5.3 row has carried, for hundreds of sessions, the sentence that Print
  "is a printing decision this viewer does not yet make" — and its census already
  counted bit 3 as *by far* the most-stated flag in both corpora, so this switch is the
  single highest-population piece of unimplemented annotation semantics in the tree.
- **Optional content, §8.11.4.5**: "When a document is printed by an interactive PDF
  processor, usage application dictionaries with an event type Print shall be applied
  over the current states of optional content groups. These changes shall persist only
  for the duration of the print operation; then all groups shall revert to their prior
  states." — applied over the *current* states, i.e. on top of the user's manual layer
  switches, transiently. Table 100's `Print` usage category (`PrintState` ON/OFF)
  supplies the per-group answer. The §8.11.4.4/8.11.4.5 rows both name this as the
  still-owed half ("an operation this program does not perform") — print intent is
  exactly that operation arriving.
- **Watermarks, §12.5.6.22**: a watermark annotation with a FixedPrint dictionary
  represents "graphics that are to be printed at a fixed size relative to the target
  media", so under print intent its Table 194 matrix/offsets are applied against the
  *paper*, not the media box. The §12.5.3 row already flags `/FixedPrint` as "a
  printing decision".
- **Transfer functions, §10.5**: already implemented for the screen (the ledger row is
  `implemented`; `CLAUDE.md`'s scope entry was amended on its evidence). They apply
  under print intent identically — the clause's subject is the component value
  transmitted to the device, whichever device.
- **Halftones, §10.6**: stay inapplicable *on the standard's own condition*, and the
  reasoning must be stated because the device changed: our print path emits contone
  RGB raster to a print system that screens downstream; §10.6.1's own sentence for
  such devices — "[h]alftoning is not required for such devices; after gamma
  correction by the transfer functions, the colour components shall be transmitted
  directly to the device" — describes exactly this hand-off. If a later route ever
  drives a halftoning device directly, the row decays; say so in the row when print
  lands (the `CLAUDE.md` lesson: an inapplicability entry is a claim that decays).
- **Form fields**: print the current appearance — the value the user has typed (from
  the view state, saved or not), rendered through the same appearance construction the
  screen uses; the widget-delegation path (`Command::Delegate`, §6.3.2.2's "unless
  otherwise instructed") is a screen affair and does not apply to print.
- **Chrome, selection, caret, focus rings, hover states**: never printed — they are
  the host's, not the page's, and print intent renders pages, not windows.

Interpretation stays a pure function: (bytes, view state, edits, **intent**). The
oracle gains a second intent column it can regression-test the same way it tests the
first.

## 5. UI entry points — all three hosts, plus the confined window

`viewer_host::keys` gains print (Ctrl+P) and the hosts' menus/toolbars gain the entry;
what happens next is per-host, because the dialog is chrome:

- **GTK (`pdf-viewer-gtk`)**: `GtkPrintOperation` — settings/page-setup from the GTK
  dialog, `draw-page` paints our banded raster into the print cairo context at the
  context's reported DPI. gtk4-rs covers the whole surface. Portal routing (sandboxed
  GTK) comes free from GTK itself.
- **Qt (`pdf-viewer-qt`)**: `QPrintDialog` + `QPainter::drawImage` onto `QPrinter`
  (`HighResolution` mode, `setResolution` to our clamped DPI). Requires hand-written
  cxx bridge additions (QtPrintSupport is unbound in cxx-qt-lib) — the crate has paid
  exactly this kind of cost before and its `unsafe` discipline (one exempted bridge
  module, tested position) extends unchanged.
- **winit (`pdf-viewer`)**: no toolkit, so no toolkit dialog. Recommendation: a print
  panel drawn in `viewer-ui`'s own chrome (it already draws panels, forms and prompts
  with its compiled-in font) and job submission over **IPP to CUPS via the `ipp`
  crate** — pure Rust, no C linkage, options mapped from the panel (printer list via
  IPP `CUPS-Get-Printers`/`Get-Printer-Attributes`). Alternative considered: the XDG
  print portal (needs a D-Bus client and a portal frontend present — fine on desktops,
  absent on bare X11 boxes); IPP is the lower common denominator on a machine that can
  print at all.
- **The confined window (`pdf-viewer-confined`)**: the stance the sandbox dictates,
  and it is the portal's own shape. The **worker renders, the host spools**: print
  intent crosses the wire as part of the render request; the confined worker (which
  has no filesystem, deliberately) renders banded pages at print DPI and ships pixels
  — the arm the wire already has; the *window process* composes the spool (raster
  pages wrapped in a minimal PDF container, Flate-compressed) and submits via IPP or
  hands an fd to the portal. The spool file is written by the host into its own
  runtime dir (`$XDG_RUNTIME_DIR`), never by the worker. No new confinement holes: the
  worker's seccomp/Landlock profile is untouched, and the host-side spool writer is
  small, parser-free code.

The **spool container** (raster pages in a minimal PDF wrapper) is worth naming as a
shared piece: GTK/Qt paths do not need it (they paint into the toolkit's context), but
winit-IPP, the portal, and the confined window all submit a *file* — one small,
well-tested writer in the core serves all three, and it is not authoring-from-nothing
in any problematic sense (its content is our raster; still, it is a new *file
producer*, and §9 notes the scope sentence to confirm with the owner).

## 6. Preview — a view mode, not a second window

**Recommendation: preview is a mode of the existing window**, in all four hosts, not a
separate window and not Qt's `QPrintPreviewDialog` (using the latter only in Qt would
fork the preview's look and semantics per host; and preview *is* our page rendering —
the one thing this project does best in its own window).

Entering print (Ctrl+P) opens the print panel docked beside the page area, which
switches to paper-composed view: pages on simulated paper (paper size and printable
margins from the chosen printer/setup), scale mode applied, n-up composed, page ranges
filtered live. The same keys navigate sheets. Print intent is applied to the preview
render, so what the preview shows — which annotations, which layers, watermarks against
the paper — is what prints, at screen resolution.

    +--------------------------------------------------------------------+
    | doc.pdf — print                                          [x close] |
    +----------------------------------------+---------------------------+
    |   .----------------.  .-------------.  | Printer:  [ Office-Laser ]|
    |   | +------------+ |  | +---------+ |  | Copies:   [ 1 ]  Collate ▢|
    |   | |  page 1    | |  | | page 2  | |  | Range:    [ 1-4,9      ]  |
    |   | |  contents  | |  | |         | |  |   (RFC 0002 range syntax) |
    |   | +------------+ |  | +---------+ |  | Paper:    [ A4        ]   |
    |   |   sheet 1      |  |   sheet 2   |  | Scale:    (o) Shrink to   |
    |   '----------------'  '-------------'  |             printable     |
    |     dashed = printable area            |           ( ) Actual size |
    |                                        |           ( ) Fit        |
    |  sheet 1 of 3            [< prev][next>]| Pages/sheet: [1|2|4]     |
    |                                        |                           |
    |                                        |   [Print]    [Cancel]     |
    +----------------------------------------+---------------------------+

- **Page ranges reuse the transform suite's range syntax** (RFC 0002) — one grammar
  project-wide, parsed by the transform layer, so `1-4,9` means the same thing in the
  CLI, the print panel and (via `page-ranges`) anything delegated.
- **Scale modes**: actual size / shrink-to-printable-area / fit-to-printable-area
  (Evince's three, which are the field's convention). Shrink is the default.
- **n-up** (1, 2, 4 in v1): composed by *us* (we are compositing rasters anyway;
  delegating n-up to CUPS would fork preview from output on the toolkit paths).
- GTK/Qt native dialogs still appear at the moment of submission for printer-specific
  settings (duplex, trays) — the panel owns what changes the *page image*; the
  toolkit/CUPS dialog owns what does not. On winit, the panel is the whole dialog and
  duplex/media options map to IPP attributes.

## 7. What crosses the UI boundary

Small, in the boundary's own idiom (final shapes are the implementing round's):

- `RenderRequest.intent: Intent { Screen, Print }` — the §4 switch (a field, not a
  message).
- Print-DPI rendering and banding ride the existing render/raster arms; the confined
  wire needs no new message kind, only the field.
- The panel's state (printer, range, scale, n-up) is chrome — host-side, like every
  dialog decision the hosts already own (§12.7.6.4's file prompt precedent).
- Possibly one `Query::PrintReport` — what print intent changed (annotations shown or
  suppressed, layer flips, watermarks applied) so hosts can surface "3 annotations
  will not print" in the panel; report-shaped, and it must fire only on conditions the
  clauses state (trap 11).

## 8. Difficulty

| piece | grade | why |
|---|---|---|
| print intent in interpretation (Table 167 bits 3/6, §8.11.4.5 transient states) | **easy–moderate** | the readers all exist; this is a re-ask of decided questions under a flag, plus the truth table's tests |
| §12.5.6.22 FixedPrint under print intent | **moderate** | new geometry against paper; small population, synthetic fixtures (the census found `/FixedPrint` essentially absent from the corpora) |
| banded print-DPI rendering via render-cpu | **moderate** | the rasteriser exists; banding at 300–600 dpi and its memory budget are engineering, and the perf principle demands the budget be measured |
| GTK print path | **easy** | GtkPrintOperation is made for exactly this; gtk4-rs complete |
| Qt print path | **moderate** | hand-written cxx bridge for QPrinter/QPrintDialog — known cost, paid before |
| winit print panel + IPP submission | **moderate** | pure-Rust `ipp` is solid; the panel is chrome work in an established idiom |
| confined window: render-inside, spool-outside | **moderate** | the split exists (ADR 0713); print adds a host-side spool writer and the intent field on the wire |
| spool container writer (raster-in-PDF) | **moderate** | small and testable, but it is this tree's first file *producer* — deserves its own fuzzing and validation care |
| preview as view mode with paper composition | **moderate** | new composition layer over existing rendering; shared across hosts once |
| Route A (file-to-CUPS passthrough with state baking) | **hard, deferred** | needs the transform layer to bake view state into a derivative; explicitly not v1 |
| prepress (separations, trapping, printer marks, PDF/X output intents §14.11) | **out** | a different product; say so now to keep the exclusion deliberate |

## 9. Spec contact — the clauses this makes live, against the ledger

| clause | today's ledger state | what print does to it |
|---|---|---|
| §12.5.3 Table 167 bits 1/2/3/6 | `partial`; the note itself says Print "is a printing decision this viewer does not yet make" | the print truth table becomes implemented; the row's oldest debt closes |
| §8.11.4.4, §8.11.4.5 | `partial`; Print/Export usage events named as owed ("an operation this program does not perform") | the Print event's transient application lands; Export stays owed (no export operation exists) |
| §12.5.6.22 | flagged in §12.5.3's row as a printing decision | FixedPrint geometry implemented under print intent |
| §10.5 | `implemented` (screen) | unchanged in code; the row gains the print sentence |
| §10.6 | `inapplicable` on the standard's condition | condition re-derived for the contone hand-off (§4); row updated, still inapplicable |
| §6.3.2.2's three obligations | ranking rule in `CLAUDE.md` | apply verbatim to the printed page as to the screen |
| §14.11 (prepress) | — | named out of scope, deliberately, with this RFC as the record |

One scope sentence for the owner: the spool container writer produces a (trivial,
raster-wrapping) PDF file. `CLAUDE.md`'s authoring exclusion is about creating
*documents* and generator obligations; a spool is a transport envelope our own print
path consumes. The RFC reads the exclusion as not applying — but it is the owner's
sentence to confirm (open question 5).

## 10. Open questions for the owner

1. **Route B confirmed as v1?** (We render; passthrough deferred.) The fidelity
   argument says yes; a prepress-leaning owner might want Route A sooner.
2. **DPI clamp** [150, 600] with default 300 — acceptable budget, or raise the ceiling
   (1200 doubles axes, quadruples spool)?
3. **Preview as view mode** in all hosts (recommended) versus Qt's stock
   `QPrintPreviewDialog` on Qt only?
4. **n-up in v1** (1/2/4 composed by us), or defer n-up entirely to a later round?
5. **The spool-writer scope sentence** (§9) — confirm the exclusion reading.
6. **winit host**: IPP-direct (recommended) or XDG portal as the submission path where
   available, IPP as fallback?
7. **Encrypted documents**: Table 22's `/P` bit 3 is the document's *print*
   permission. Per `CLAUDE.md`, it routes through the reader-owned restriction policy
   (`restriction::asserted`) like every other assertion — confirm printing consults it
   at the same four-level shape (off/on/ask/warn) rather than growing its own switch.

## 11. Recommendation

Route B: our oracle renders the printed page, at the printer's resolution clamped to a
stated budget, under a print intent that finally asks the questions Table 167 bit 3,
§8.11.4.5's Print event and §12.5.6.22 have been waiting on — the highest-population
dormant semantics in the ledger. Preview is a mode of the window every host already
has; entry is Ctrl+P everywhere; the confined window renders inside and spools outside,
which is the portal model and the sandbox's own logic. GTK first (the path is free),
Qt second (bridge cost), winit third (panel + IPP), confined fourth (field on the wire
plus the shared spool writer).
