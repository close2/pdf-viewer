# 2919 — A page chooses its own paper where the dialogue's tray box is checked

Session 2052. Status: **accepted and built** in `viewer-core` (`Query::PrintSize`, `Answer::PrintSize`),
`viewer-confined` (the query and answer on the pipe), `viewer-host` (`printing::Defaults::tray_by_page_size`,
`printing::TRAY_BY_PAGE_SIZE`, `printing::paper_for_page`), `viewer-gtk` (`host/printing.rs`: `the_paper_tab`,
`choose_the_paper_by_page`, `set_the_paper`), `viewer-qt` (`print_page_paper`, the tray box on the dialogue's
*Page* tab) and `viewer-ffi` (`quorra_print_size`). It is `doc/todo/76` row 6, the per-host reads of the print
entries. Clause: ISO 32000-2 §12.2, Table 147's `/PickTrayByPDFSize`; §12.5.6.22 for the sheet.

## Context

Row 6 was `doc/todo/65` section 1's host rows. That section names no row of its own: §12.3.5's collection is
presented in all four windows, §13.2.4.2, §12.7.5.3 and §12.7.8.3.3 stand beside it, and §12.2's ledger row is
`departed` only for the owner-kept `/HideMenubar` (ADRs 1145, 1622). A census of Table 147's print half across the
hosts found one entry read by the library and presented by no dialogue: `/PickTrayByPDFSize`, "[a] flag specifying
whether the PDF page size shall be used to select the input paper tray. This setting influences only the preset
values used to populate the print dialogue presented by an interactive PDF processor. If PickTrayByPDFSize is true,
the check box in the print dialogue associated with input paper tray shall be checked. This setting has no effect
on operating systems that do not provide the ability to pick the input tray by size." `viewer_host::PrintDefaults`
carried it; a C caller read it through `quorra_preference`; `quorra-gtk` declined it in a comment, because
`GtkPrintSettings` has no such setting, and `quorra-qt` said nothing at all.

## Decision

**The box is this program's, on the tab each toolkit gives an application.** Neither GTK's dialogue nor Qt's has a
box "associated with input paper tray", so each window adds one, labelled `TRAY_BY_PAGE_SIZE`: GTK on the
operation's one custom tab, which now also carries ADR 2908's held scaling, and Qt beside its scaling and
pages-per-sheet controls. It opens checked where the document states `true`; unchecked where it states `false`,
and where it states nothing, the entry's "implementation dependent" default chosen as unchecked so that the paper a
person picked stays every page's paper unless the document asked. What the reader leaves it at is what the job
does.

**Checked, each page goes onto paper of its own size.** A print system picks a tray by the paper a sheet asks for,
so the sheet asks for the page's: its shorter side by its longer, turned where the page is wider than tall
(`paper_for_page`), set on GTK's `request-page-setup` and as Qt's page layout before each page. Where the size is a
named paper the toolkit names it (GTK within a millimetre, a documented choice; Qt's `FuzzyMatch`), so that the
tray holding A4 is found from the name; elsewhere it is a custom size, and a printer that cannot feed it is the
clause's last sentence. The page lands at its own size, which is §12.5.6.22's "usual case where the PDF page size
equals the media size", so Qt's scale and pages-per-sheet combos stand aside while the box is checked.

**The size is asked before the page is drawn, which is why the core answers a new query.** §12.5.6.22 places a
`FixedPrint` watermark against the target media, and each page's media is now its own, so the sheet must reach the
core as `Printing::Paper` before `Query::PrintPage` interprets the page — once per change of size, not per page.
Reading the size off the drawn page would draw it against the previous sheet. `Query::PrintSize` answers from the
page dictionary under the print purpose, so it is `/PrintArea`'s boundary after `/Rotate` and `/UserUnit`, the
extent the page's display list then has, without interpreting it; `Answer::None` outside a print operation, for
`Query::PrintPage`'s reason. It crosses the confined pipe, refused there unless finite and positive, and reaches a
C caller as `quorra_print_size`, which `drive_a_window.c` reads; `QUORRA_ABI_VERSION` stays (ADR 0737).

**Why not `GtkPrintSettings::set_default_source("auto")` or `QPrinter::Auto`**: each names a tray, or the
printer's own choice of one, for the whole job; neither says the page's size decides it, which is the entry's
whole sentence, and a document of mixed page sizes would still go out on one paper.

## Consequences

Every window with a print dialogue presets Table 147's five dialogue entries. `quorra` has no printer (ADR 1180
section 6) and the WinUI window's `PRINT` card is a preview; when its dialogue is built (`doc/todo/30`'s N4) it owes
the same box opening on `quorra_preference(QUORRA_PREF_PICK_TRAY_BY_PDF_SIZE)`, and, checked, a
`PrintPageDescription` per page from `quorra_print_size` stated through `quorra_print_placed` before
`quorra_print_page`. §12.2's row stays `departed` for `/HideMenubar` alone.
