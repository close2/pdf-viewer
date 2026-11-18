//! Printing through GTK's print dialogue: the settings, the sheet, and each page painted into the print context.

use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, Printing, Query};

use super::{Host, with};

impl Host {
    /// §7.6.4.2's bit 3, asked before anything appears on the screen.
    ///
    /// **The policy question comes first and the dialogue second**, which is the order
    /// `CLAUDE.md`'s four levels need: *ask* raises [`Event::Asking`](viewer_core::Event::Asking) and is answered in a modal
    /// window of its own, and a print dialogue already on the screen while that question stands
    /// would be two dialogues about one press. So this sends the message and stops; the dialogue
    /// is [`Self::open_the_print_dialogue`], reached from [`Event::Printing`](viewer_core::Event::Printing).
    ///
    /// The sheet sent here is the platform's default page setup, because the person has not been
    /// asked yet. What they choose reaches the core as [`Printing::Paper`] from the operation's
    /// `begin-print`, before any page is drawn.
    pub(super) fn ask_to_print(&mut self) {
        let printer = viewer_host::Printer::Dialogue(sheet_of(&gtk4::PageSetup::new()));
        match viewer_host::print_key(printer) {
            viewer_host::PrintKey::Ask(command) => self.dispatch(command),
            // Answered for the preview alone: a dialogue's job ends the print it started, so its
            // key ends none. Said rather than dropped were it ever answered here (ADR 2157).
            viewer_host::PrintKey::End(said) => self.say(said),
        }
    }

    /// Runs `GtkPrintOperation` once the core has granted the operation.
    ///
    /// **Scheduled rather than run here, and that is not tidiness.** `GtkPrintOperation::run`
    /// turns the main loop until the job is finished, and every callback it raises on the way —
    /// `begin-print`, `draw-page`, `end-print` — comes back into this host through [`with`],
    /// which borrows it. Running it inside this `&mut self` would leave that borrow standing for
    /// the length of the job, and every one of those callbacks would be dropped with the note
    /// [`with`] prints. An idle turn ends the borrow first.
    ///
    /// What GTK draws is what this program drew: the marks are the document's, rasterised by the
    /// processor backend that is this project's correctness oracle, and painted into the print
    /// context. Nothing is composed here — no page number, no header, no watermark — because
    /// `CLAUDE.md`'s authoring exclusion puts marks this program invented on the far side of the
    /// line.
    pub(super) fn open_the_print_dialogue(
        &mut self,
        pages: usize,
        fidelity: viewer_core::Fidelity,
        scaling_held: bool,
    ) {
        let Ok(pages) = i32::try_from(pages) else {
            self.say("this document has more pages than a print job can name");
            self.dispatch(Command::Print(Printing::Finish));
            return;
        };
        // §12.2's half of Table 147, as the dialogue's opening state. Read here rather than
        // inside the operation because the answer borrows this host and the operation must not.
        let defaults = match self.state.viewer.query(Query::Preferences) {
            Answer::Preferences(preferences) => viewer_host::PrintDefaults::of(&preferences),
            _ => viewer_host::PrintDefaults::of(
                &pdf_model::viewer_preferences::ViewerPreferences::default(),
            ),
        };
        // §12.2's `/Enforce` where the reader's level obeys it: said before the dialogue, and
        // then held in it — its scale control locked and the job built on the document's
        // scaling, by [`the_paper_tab`] (ADRs 2840, 2908).
        let held = defaults.held_scaling(scaling_held);
        if let Some(note) = defaults.held_note(scaling_held) {
            self.say(note);
        }
        // §7.6.4.2's Table 22 bit 12, said before the dialogue rather than after a destination
        // was chosen: a person about to pick *Print to File* is told the file is the thing this
        // document withholds, and the refusal below is what enforces it (ADR 1203).
        if let Some(note) = viewer_host::printing::destination_refused(fidelity) {
            self.say(note);
        }
        let window = self.ui.window.clone();
        let me = self.me();
        glib::idle_add_local_once(move || {
            let operation = gtk4::PrintOperation::new();
            operation.set_n_pages(pages);
            operation.set_print_settings(Some(&print_settings(&defaults)));
            // Points, so that the context's coordinates are §8.3.2.3's default user space units
            // and the only scaling left is the one this host applies to its own raster.
            operation.set_unit(gtk4::Unit::Points);
            operation.set_embed_page_setup(true);
            let said = me.clone();
            let by_page_size = the_paper_tab(
                &operation,
                held,
                defaults.tray_by_page_size(),
                Rc::new(move |note: &str| with(&said, |host| host.say(note))),
            );
            let sized = me.clone();
            let last_sheet = Rc::new(std::cell::Cell::new(None));
            operation.connect_request_page_setup(move |_, context, page, setup| {
                if !by_page_size.get() {
                    return;
                }
                with(&sized, |host| {
                    host.choose_the_paper_by_page(context, page, setup, fidelity, &last_sheet);
                });
            });
            let begin = me.clone();
            operation.connect_begin_print(move |operation, context| {
                // Table 22 bit 12's other half, and the one a resolution cannot answer: a job
                // written to a file is a document a faithful copy could be generated from
                // whatever it was drawn at. `output-uri` is what GTK's file backend puts in the
                // settings, so a destination that names one is the destination the bit withholds
                // — refused by name, with the job stopped before a page is drawn (ADR 1203).
                if writes_a_file(operation)
                    && let Some(note) = refused(fidelity)
                {
                    with(&begin, |host| host.say(note));
                    operation.cancel();
                    return;
                }
                // The paper the person actually chose, which is the one §12.5.6.22 places a
                // watermark against. It reaches the core before a page is drawn, and it asks
                // nothing: bit 3 was answered before this dialogue existed.
                let sheet = sheet_of_context(context, fidelity);
                with(&begin, |host| {
                    host.dispatch(Command::Print(Printing::Paper(sheet)));
                });
            });
            let draw = me.clone();
            operation.connect_draw_page(move |_, context, page| {
                with(&draw, |host| {
                    host.paint_a_printed_page(context, page, fidelity);
                });
            });
            let finish = me.clone();
            // §8.11.4.5: the `Print` event's changes "persist only for the duration of the print
            // operation; then all groups shall revert to their prior states". `run` returns when
            // the job is over, however it ended, which is the duration the clause names.
            let outcome = operation.run(gtk4::PrintOperationAction::PrintDialog, Some(&window));
            with(&finish, |host| {
                host.dispatch(Command::Print(Printing::Finish));
                match outcome {
                    Ok(gtk4::PrintOperationResult::Cancel) => host.say("printing cancelled"),
                    Ok(_) => host.say("the print job was sent"),
                    Err(error) => host.say(&format!("the print job failed: {error}")),
                }
            });
        });
    }

    /// §12.2, Table 147's `/PickTrayByPDFSize`, for one page: its paper is the page's own size.
    ///
    /// The entry is "whether the PDF page size shall be used to select the input paper tray", and
    /// a print system picks a tray by the paper a sheet asks for — so `request-page-setup`, which
    /// GTK emits before each page is drawn, sets this page's paper to the size
    /// `viewer_core::Query::PrintSize` answers, turned where the page is wider than tall
    /// ([`viewer_host::printing::paper_for_page`]). The page then lands on it at its own size.
    ///
    /// **The sheet reaches the core before the page is drawn, where it changed**: §12.5.6.22 places
    /// a `FixedPrint` watermark against "the dimensions of the target media", and each page's media
    /// is now its own. A run of pages of one size states it once (ADR 2919).
    fn choose_the_paper_by_page(
        &mut self,
        context: &gtk4::PrintContext,
        page: i32,
        setup: &gtk4::PageSetup,
        fidelity: viewer_core::Fidelity,
        last_sheet: &std::cell::Cell<Option<(f32, f32)>>,
    ) {
        let Ok(index) = usize::try_from(page) else {
            return;
        };
        let Answer::PrintSize(size) = self.state.viewer.query(Query::PrintSize(index)) else {
            return;
        };
        let paper = viewer_host::printing::paper_for_page(size.width, size.height);
        set_the_paper(setup, paper);
        let (width, height) = paper.as_placed();
        if last_sheet.replace(Some((width, height))) != Some((width, height)) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a printer's resolution in dots per inch: hundreds, which an f32 \
                          resolves exactly"
            )]
            let dpi = context.dpi_x() as f32;
            self.dispatch(Command::Print(Printing::Paper(viewer_core::Sheet {
                media: Some([0.0, 0.0, width, height]),
                scale: viewer_host::printing::scale_at(fidelity, Some(dpi)),
                // The page on paper of its own size: §12.5.6.22's "usual case".
                page_scale: 1.0,
            })));
        }
    }

    /// Draws one page of the job into the print context.
    ///
    /// The page is asked for under print intent — `Query::PrintPage` answers nothing unless the
    /// core granted the operation — rasterised on the processor at the job's resolution, and
    /// painted at 1/scale so that a raster in printer pixels lands on a context measured in
    /// §8.3.2.3's points.
    ///
    /// Everything the page could not draw is said in the status bar rather than swallowed, which
    /// is trap 5 applied to a page nobody is looking at while it prints.
    fn paint_a_printed_page(
        &mut self,
        context: &gtk4::PrintContext,
        page: i32,
        fidelity: viewer_core::Fidelity,
    ) {
        let Ok(index) = usize::try_from(page) else {
            return;
        };
        let Answer::PrintPage(printed) = self.state.viewer.query(Query::PrintPage(index)) else {
            self.say(&format!(
                "page {} is not this document's to print",
                index.saturating_add(1)
            ));
            return;
        };
        for note in &printed.reports {
            self.say(note);
        }
        let Some(target) = printed.target else {
            return;
        };
        let raster = match CpuRasterizer::new().rasterize(&printed.list, target) {
            Ok(raster) => raster,
            Err(refusal) => {
                self.say(&format!(
                    "page {} did not print: {refusal}",
                    index.saturating_add(1)
                ));
                return;
            }
        };
        let cairo = context.cairo_context();
        let scale = f64::from(sheet_of_context(context, fidelity).scale);
        if let Err(error) = paint_raster(&cairo, &raster, scale) {
            self.say(&format!(
                "page {} did not print: {error}",
                index.saturating_add(1)
            ));
        }
    }
}

/// §12.2's Table 147, as the settings a `GtkPrintOperation` opens its dialogue on.
///
/// **Defaults and not instructions**, which is what each of those entries states: "\[t]he page
/// scaling option that shall be selected when a print dialogue is displayed for this document",
/// "\[t]he paper handling option that shall be used when printing the PDF file from the print
/// dialogue", "\[t]he page numbers used to initialise the print dialogue box when the PDF file is
/// printed", "\[t]he number of copies that shall be printed when the print dialog is opened for
/// this PDF file". A person changes any of them in the dialogue this seeds; Table 148's
/// `/Enforce` is a restriction at the reader's level, and where that level holds it
/// [`the_paper_tab`] keeps the scale this function set.
///
/// **`/PrintPageRange` is one-based and `GtkPageRange` is not**, which is the whole reason
/// `viewer_host::PrintDefaults` keeps the clause's own numbering: the conversion is here, at the
/// one place the two vocabularies meet, rather than in a value that would then be
/// indistinguishable from the other kind.
///
/// **`/PickTrayByPDFSize` is not a setting here**: Table 147 states it as a *check box in the print
/// dialogue*, and `GtkPrintSettings` has no such setting — `set_default_source` names a tray rather
/// than asking for one to be chosen by page size. The box is on [`the_paper_tab`], and checked it
/// gives each page paper of its own size (ADR 2919).
fn print_settings(defaults: &viewer_host::PrintDefaults) -> gtk4::PrintSettings {
    use pdf_model::viewer_preferences::{Duplex, PrintScaling};

    let settings = gtk4::PrintSettings::new();
    if let Some(copies) = defaults
        .copies
        .and_then(|copies| i32::try_from(copies).ok())
    {
        settings.set_n_copies(copies);
    }
    if let Some(duplex) = defaults.duplex {
        settings.set_duplex(match duplex {
            Duplex::Simplex => gtk4::PrintDuplex::Simplex,
            Duplex::FlipShortEdge => gtk4::PrintDuplex::Vertical,
            Duplex::FlipLongEdge => gtk4::PrintDuplex::Horizontal,
        });
    }
    // "None, which indicates no page scaling" — GTK states a percentage, and no scaling is 100.
    if defaults.scaling == PrintScaling::NoScaling {
        settings.set_scale(percentage(PrintScaling::NoScaling));
    }
    let ranges: Vec<gtk4::PageRange> = defaults
        .page_range
        .iter()
        .filter_map(|&(first, last)| {
            let first = i32::try_from(first).ok()?;
            let last = i32::try_from(last).ok()?;
            Some(gtk4::PageRange::new(
                first.saturating_sub(1),
                last.saturating_sub(1),
            ))
        })
        .collect();
    if !ranges.is_empty() {
        settings.set_page_ranges(&ranges);
        settings.set_print_pages(gtk4::PrintPages::Ranges);
    }
    settings
}

/// §12.2's `/PrintScaling` as the percentage `GtkPrintSettings` states a scale in.
///
/// Table 147: `None` "indicates no page scaling", which is GTK's 100. `AppDefault` is "the
/// interactive PDF processor's default print scaling", and this host's is GTK's own default scale,
/// which is 100 as well — so the two values print alike here, and what differs is only that a
/// reader may change the second.
fn percentage(scaling: pdf_model::viewer_preferences::PrintScaling) -> f64 {
    match scaling {
        pdf_model::viewer_preferences::PrintScaling::NoScaling
        | pdf_model::viewer_preferences::PrintScaling::AppDefault => 100.0,
    }
}

/// The operation's own tab of GTK's print dialogue: §12.2, Table 147's tray box, and `/Enforce`'s
/// hold where the reader's level obeys it. GTK gives an operation one custom tab, so both are on it.
///
/// **The tray box** is `/PickTrayByPDFSize`'s: "\[i]f PickTrayByPDFSize is true, the check box in
/// the print dialogue associated with input paper tray shall be checked". GTK's own dialogue has no
/// such box — `GtkPrintSettings` names a tray rather than asking for one by size — so the tab
/// carries it, opening on `tray_by_page_size` and read back at `custom-widget-apply` into the cell
/// this returns, which `request-page-setup` asks before each page (ADR 2919).
///
/// **The hold**, where `held` is the document's enforced scaling (ADR 2908). Table 147's
/// `/Enforce` names preferences "that shall be enforced by PDF processors and that shall not be
/// overridden by subsequent selections in the application user interface", and the reader's level
/// for `OverridePreference` has said to obey it. Two halves, each through GTK's public print API,
/// and the second does not depend on the first:
///
/// - **The control is locked.** `GtkPrintOperation` builds its `GtkPrintUnixDialog` with
///   `GTK_PRINT_CAPABILITY_SCALE` among the capabilities it handles itself, and the dialogue makes
///   its scale control sensitive only where that capability is offered by the application or by
///   the printer. The tab carries the held sentence, and from it the dialogue it was placed in is
///   reached and that one capability withdrawn, at a priority above the dialogue's first frame. A
///   printer whose own backend states the capability would leave the control live; no backend GTK
///   ships does (ADR 2908).
/// - **The job keeps the scale.** `custom-widget-apply` runs when the reader accepts the dialogue,
///   after its settings are the operation's and before GTK builds the job whose scale it applies to
///   every page, so the scale is set back to the document's there — said when it had moved.
///
/// Every other control of the dialogue stays the reader's: `/Enforce` names `/PrintScaling` alone.
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotation: Table 147 spells PickTrayByPDFSize without backticks"
)]
fn the_paper_tab(
    operation: &gtk4::PrintOperation,
    held: Option<pdf_model::viewer_preferences::PrintScaling>,
    tray_by_page_size: bool,
    say: Rc<dyn Fn(&str)>,
) -> Rc<std::cell::Cell<bool>> {
    let by_page_size = Rc::new(std::cell::Cell::new(false));
    operation.set_custom_tab_label(Some(if held.is_some() {
        "Paper and scaling"
    } else {
        "Paper"
    }));
    let unlocked = Rc::clone(&say);
    // **The handler keeps the tab it answers with.** The signal's return is `transfer none` and
    // gtk-rs hands GTK a borrowed pointer, so a tab nothing on this side still holds is finalised
    // before the dialogue takes it, and GTK finds no tab at all.
    let kept: std::cell::RefCell<Option<gtk4::Box>> = std::cell::RefCell::new(None);
    operation.connect_create_custom_widget(move |_| {
        let tab = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        kept.replace(Some(tab.clone()));
        let tray = gtk4::CheckButton::with_label(viewer_host::printing::TRAY_BY_PAGE_SIZE);
        tray.set_active(tray_by_page_size);
        tab.append(&tray);
        if held.is_some() {
            let note = gtk4::Label::new(Some(HELD_TAB));
            note.set_wrap(true);
            tab.append(&note);
            let placed = tab.clone();
            let unlocked = Rc::clone(&unlocked);
            // The tab is placed in the dialogue as soon as this handler returns, so the next turn
            // of the loop finds it there; `HIGH` runs that turn ahead of the dialogue's first frame.
            glib::idle_add_local_full(glib::Priority::HIGH, move || {
                if !lock_the_scale(&placed) {
                    unlocked(UNLOCKED);
                }
                glib::ControlFlow::Break
            });
        }
        Some(tab.upcast())
    });
    let chosen = Rc::clone(&by_page_size);
    operation.connect_custom_widget_apply(move |operation, tab| {
        chosen.set(the_tray_box(tab).is_some_and(|tray| tray.is_active()));
        if let Some(scaling) = held
            && let Some(settings) = operation.print_settings()
            && keep_the_scale(&settings, scaling)
        {
            say(SET_BACK);
        }
    });
    by_page_size
}

/// The tray box on [`the_paper_tab`]'s tab: its first child.
fn the_tray_box(tab: &gtk4::Widget) -> Option<gtk4::CheckButton> {
    tab.first_child().and_downcast::<gtk4::CheckButton>()
}

/// Sets a page setup's paper to one page's own, as [`viewer_host::printing::paper_for_page`]
/// chose it.
///
/// **A paper GTK knows by name where the page is one**, within [`NAMED_PAPER_TOLERANCE`]: a print
/// system finds the tray holding A4 from the name `iso_a4`, and a page stated as 595.276 by 841.89
/// points is A4 to every person who drew it. Elsewhere the paper is a custom size of the page's own
/// dimensions, which a print system that cannot feed it says so for — the clause's own "no effect
/// on operating systems that do not provide the ability to pick the input tray by size".
fn set_the_paper(setup: &gtk4::PageSetup, paper: viewer_host::printing::PagePaper) {
    let (short, long) = (f64::from(paper.short), f64::from(paper.long));
    let named = gtk4::PaperSize::paper_sizes(false)
        .into_iter()
        .find(|named| {
            let (width, height) = (
                named.width(gtk4::Unit::Points),
                named.height(gtk4::Unit::Points),
            );
            (width - short).abs() <= NAMED_PAPER_TOLERANCE
                && (height - long).abs() <= NAMED_PAPER_TOLERANCE
        });
    let size = named.unwrap_or_else(|| {
        gtk4::PaperSize::new_custom("page", "This page's size", short, long, gtk4::Unit::Points)
    });
    setup.set_paper_size(&size);
    setup.set_orientation(if paper.landscape {
        gtk4::PageOrientation::Landscape
    } else {
        gtk4::PageOrientation::Portrait
    });
}

/// How far, in points, a page may be from a named paper and still be printed on it — one
/// millimetre, a documented choice: no clause states a tolerance, and the standard sizes GTK names
/// are rounded to the millimetre (ISO 216's A4 is 210 by 297 mm), so a page stated to finer than
/// that is the same paper (ADR 2919).
const NAMED_PAPER_TOLERANCE: f64 = 72.0 / 25.4;

/// What the dialogue's own tab says where the scaling is held.
const HELD_TAB: &str = "This document enforces its print scaling (ISO 32000-2 §12.2, Table 147's \
                        /Enforce) and your level obeys it, so the scale is locked to the scaling \
                        it states. The level is yours to change in the restrictions menu.";

/// Said where the dialogue could not be found to lock: the job is held all the same.
const UNLOCKED: &str = "the print dialogue's scale could not be locked; the print keeps the \
                        scaling the document enforces whatever is chosen there";

/// Said where a scale chosen in the dialogue was set back.
const SET_BACK: &str = "the scale chosen in the print dialogue was set back to the scaling this \
                        document enforces (ISO 32000-2 §12.2, Table 147's /Enforce)";

/// Withdraws `GTK_PRINT_CAPABILITY_SCALE` from the dialogue `tab` was placed in.
///
/// `false` where `tab` is in no `GtkPrintUnixDialog`, which is what the caller says.
fn lock_the_scale(tab: &impl IsA<gtk4::Widget>) -> bool {
    let Some(dialog) = tab.as_ref().root().and_downcast::<gtk4::PrintUnixDialog>() else {
        return false;
    };
    dialog.set_manual_capabilities(
        dialog
            .manual_capabilities()
            .difference(gtk4::PrintCapabilities::SCALE),
    );
    true
}

/// Sets `settings`' scale to the document's held `scaling`; `true` where it had been changed.
#[expect(
    clippy::float_cmp,
    reason = "both sides are GTK's stored percentage and a constant of this file; a scale the \
              dialogue moved by any amount is a moved scale"
)]
fn keep_the_scale(
    settings: &gtk4::PrintSettings,
    scaling: pdf_model::viewer_preferences::PrintScaling,
) -> bool {
    let kept = percentage(scaling);
    let moved = settings.scale() != kept;
    settings.set_scale(kept);
    moved
}

/// The sheet a page setup describes, in §12.5.6.22's terms.
///
/// `PaperSize` answers in points, which is §8.3.2.3's default user space unit, so the paper's
/// rectangle needs no conversion at all. The resolution is the platform's own until the print
/// context reports one, so this is [`viewer_host::printing::DEFAULT_DPI`].
fn sheet_of(setup: &gtk4::PageSetup) -> viewer_core::Sheet {
    let paper = setup.paper_size();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a sheet's dimensions in points: hundreds, and an f32 resolves them exactly"
    )]
    viewer_host::printing::sheet(
        paper.width(gtk4::Unit::Points) as f32,
        paper.height(gtk4::Unit::Points) as f32,
        None,
    )
}

/// The sheet a print context describes, in §12.5.6.22's terms.
///
/// The context answers in points and in dots per inch, which are §8.3.2.3's unit and the number
/// `viewer_host::printing::scale` clamps — so this is the one place the toolkit's numbers become
/// the job's, and [`Host::paint_a_printed_page`] asks the same function of the same context so
/// that the sheet and the raster cannot disagree about the resolution.
fn sheet_of_context(
    context: &gtk4::PrintContext,
    fidelity: viewer_core::Fidelity,
) -> viewer_core::Sheet {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a sheet's dimensions in points and a printer's resolution in dots per inch: \
                  hundreds each, which an f32 resolves exactly"
    )]
    let (width, height, dpi) = (
        context.width() as f32,
        context.height() as f32,
        context.dpi_x() as f32,
    );
    viewer_core::Sheet {
        media: Some([0.0, 0.0, width.max(0.0), height.max(0.0)]),
        // §7.6.4.2's Table 22 bit 12: a withheld one draws the job at this module's own floor.
        scale: viewer_host::printing::scale_at(fidelity, Some(dpi)),
        // This host places the page at the sheet's own corner and at its own size, which is
        // §12.5.6.22's "usual case" and what `paint_raster` draws. GTK's dialogue owns its own
        // page scaling, so a scale mode chosen there is the toolkit's and not one this program
        // composed — and a placement this host did not make is not one it may claim (ADR 1204).
        page_scale: 1.0,
    }
}

/// Whether this operation's destination writes a document rather than marking paper.
///
/// GTK's file backend states `output-uri` in the print settings, which is the one fact that
/// distinguishes *Print to File* from a printer without this host naming a backend or a printer
/// by its label. A job with no settings at all is not a file.
fn writes_a_file(operation: &gtk4::PrintOperation) -> bool {
    operation.print_settings().is_some_and(|settings| {
        settings.get(gtk4::PRINT_SETTINGS_OUTPUT_URI).is_some()
            || settings
                .get(gtk4::PRINT_SETTINGS_OUTPUT_FILE_FORMAT)
                .is_some()
    })
}

/// The sentence a refused destination is said with, shared with the other windows.
fn refused(fidelity: viewer_core::Fidelity) -> Option<&'static str> {
    viewer_host::printing::destination_refused(fidelity)
}

/// Paints one rasterised page into a print context's cairo surface.
///
/// **Two conversions and no third.** cairo's `ARgb32` is native-endian 32-bit words with the
/// alpha premultiplied, and [`pdf_render::Raster`] is straight RGBA bytes, so each pixel is
/// reordered and multiplied here; and the raster is in printer pixels while the context is in
/// points, so it is scaled by the reciprocal of the job's scale. Nothing else is applied — no
/// centring, no fitting — because the page is placed at the sheet's own origin, which is the
/// placement §12.5.6.22's cancelled matrix assumes and `viewer_host::printing::sheet` states.
///
/// # Errors
///
/// cairo's own, where the surface could not be made or the paint failed.
fn paint_raster(
    cairo: &gtk4::cairo::Context,
    raster: &pdf_render::Raster,
    scale: f64,
) -> Result<(), gtk4::cairo::Error> {
    let format = gtk4::cairo::Format::ARgb32;
    let Ok(width) = i32::try_from(raster.width) else {
        return Err(gtk4::cairo::Error::InvalidSize);
    };
    let Ok(height) = i32::try_from(raster.height) else {
        return Err(gtk4::cairo::Error::InvalidSize);
    };
    let stride = format.stride_for_width(raster.width)?;
    let Ok(row_bytes) = usize::try_from(stride) else {
        return Err(gtk4::cairo::Error::InvalidStride);
    };
    let (Ok(pixels), Ok(rows)) = (
        usize::try_from(raster.width),
        usize::try_from(raster.height),
    ) else {
        return Err(gtk4::cairo::Error::InvalidSize);
    };
    let mut data = vec![0_u8; row_bytes.saturating_mul(rows)];
    for (y, row) in raster
        .data
        .chunks_exact(pixels.saturating_mul(4))
        .enumerate()
    {
        let start = y.saturating_mul(row_bytes);
        for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
            let alpha = u32::from(pixel[3]);
            // Premultiplied, rounded the way cairo's own converters round: the value times the
            // alpha plus a half, divided by 255. Every operand is a byte, so the largest
            // intermediate is 255 × 255 + 127 and the saturating forms are exact arithmetic here
            // rather than a clamp that could fire.
            let premultiplied = |channel: u8| -> u8 {
                let scaled = u32::from(channel).saturating_mul(alpha).saturating_add(127);
                let rounded = scaled
                    .saturating_add(scaled.checked_div(255).unwrap_or_default())
                    .checked_div(256)
                    .unwrap_or_default();
                u8::try_from(rounded).unwrap_or(u8::MAX)
            };
            let word = [
                premultiplied(pixel[2]),
                premultiplied(pixel[1]),
                premultiplied(pixel[0]),
                pixel[3],
            ];
            // Native-endian words, which is what cairo means by `ARgb32`; on a little-endian
            // machine that is blue, green, red, alpha in memory order, which is what is written.
            let at = start.saturating_add(x.saturating_mul(4));
            if let Some(room) = data.get_mut(at..at.saturating_add(4)) {
                room.copy_from_slice(&if cfg!(target_endian = "little") {
                    word
                } else {
                    [word[3], word[2], word[1], word[0]]
                });
            }
        }
    }
    let surface = gtk4::cairo::ImageSurface::create_for_data(data, format, width, height, stride)?;
    cairo.save()?;
    // The raster is in printer pixels and the context is in points, so one pixel is one point
    // divided by the job's scale — the same scale `viewer_host::printing::scale` gave the sheet,
    // asked of the same reported resolution, so the two cannot drift apart.
    let factor = if scale > 0.0 { 1.0 / scale } else { 1.0 };
    cairo.scale(factor, factor);
    cairo.set_source_surface(&surface, 0.0, 0.0)?;
    cairo.paint()?;
    cairo.restore()
}

/// §12.2's `/Enforce` held in GTK's own dialogue, driven through the signals `GtkPrintOperation`
/// raises and the dialogue it builds. Run under a display: `xvfb-run -a cargo test -p viewer-gtk
/// --lib printing`.
#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gtk4::glib;
    use gtk4::prelude::*;
    use pdf_model::viewer_preferences::PrintScaling;

    use super::{SET_BACK, print_settings, set_the_paper, the_paper_tab, the_tray_box};

    /// The capabilities `GtkPrintOperation` gives the dialogue it builds, in `get_print_dialog`
    /// of GTK's `gtkprintoperation-unix.c`: the ones it applies to the job itself.
    fn the_operations_capabilities() -> gtk4::PrintCapabilities {
        gtk4::PrintCapabilities::PAGE_SET
            | gtk4::PrintCapabilities::COPIES
            | gtk4::PrintCapabilities::COLLATE
            | gtk4::PrintCapabilities::REVERSE
            | gtk4::PrintCapabilities::SCALE
            | gtk4::PrintCapabilities::PREVIEW
            | gtk4::PrintCapabilities::NUMBER_UP
            | gtk4::PrintCapabilities::NUMBER_UP_LAYOUT
    }

    /// What an operation's tab said, one sentence apiece.
    type Said = Rc<RefCell<Vec<String>>>;

    /// An operation with its tab — the hold attached where `held`, the tray box opening on `tray` —
    /// what it said, and the cell the tray box is read back into.
    fn operation(
        held: bool,
        tray: bool,
    ) -> (gtk4::PrintOperation, Said, Rc<std::cell::Cell<bool>>) {
        let operation = gtk4::PrintOperation::new();
        let said = Rc::new(RefCell::new(Vec::new()));
        let heard = Rc::clone(&said);
        let by_page_size = the_paper_tab(
            &operation,
            held.then_some(PrintScaling::NoScaling),
            tray,
            Rc::new(move |note: &str| heard.borrow_mut().push(note.to_owned())),
        );
        (operation, said, by_page_size)
    }

    /// The dialogue as `get_print_dialog` builds it — its capabilities, then the operation's
    /// `create-custom-widget` and the tab it answered — and the loop turned until idle.
    fn the_dialogue(
        operation: &gtk4::PrintOperation,
    ) -> (gtk4::PrintUnixDialog, Option<gtk4::Widget>) {
        let dialog = gtk4::PrintUnixDialog::new(None, None::<&gtk4::Window>);
        dialog.set_manual_capabilities(the_operations_capabilities());
        let tab = operation
            .emit_by_name::<Option<glib::Object>>("create-custom-widget", &[])
            .and_downcast::<gtk4::Widget>();
        if let Some(tab) = &tab {
            dialog.add_custom_tab(tab, &gtk4::Label::new(Some("tab")));
        }
        while glib::MainContext::default().iteration(false) {}
        (dialog, tab)
    }

    /// The reader's choice of 150 % in the dialogue, accepted: the settings become the
    /// operation's and `custom-widget-apply` runs, which is `handle_print_response`'s order.
    fn accepted(operation: &gtk4::PrintOperation, tab: Option<&gtk4::Widget>) -> f64 {
        let settings = print_settings(&viewer_host::PrintDefaults::of(
            &pdf_model::viewer_preferences::ViewerPreferences {
                print_scaling: PrintScaling::NoScaling,
                enforce_print_scaling: true,
                ..pdf_model::viewer_preferences::ViewerPreferences::default()
            },
        ));
        settings.set_scale(150.0);
        operation.set_print_settings(Some(&settings));
        let placeholder = gtk4::Label::new(None).upcast::<gtk4::Widget>();
        operation.emit_by_name::<()>("custom-widget-apply", &[tab.unwrap_or(&placeholder)]);
        operation
            .print_settings()
            .map_or(f64::NAN, |settings| settings.scale())
    }

    /// §12.2, Table 147's `/Enforce`: a held preference "shall not be overridden by subsequent
    /// selections in the application user interface", so the dialogue's scale control is locked —
    /// `GTK_PRINT_CAPABILITY_SCALE` withdrawn, every other capability left the reader's — and its
    /// own tab says why.
    #[gtk4::test]
    fn a_held_scaling_locks_the_dialogues_scale_and_nothing_else() {
        let (operation, said, _) = operation(true, false);
        let (dialog, tab) = the_dialogue(&operation);
        assert_eq!(
            dialog.manual_capabilities(),
            the_operations_capabilities().difference(gtk4::PrintCapabilities::SCALE)
        );
        let tab = tab
            .and_then(|tab| tab.last_child())
            .and_downcast::<gtk4::Label>()
            .expect("the held sentence is the tab's last line");
        assert!(
            tab.text().contains("Table 147's /Enforce"),
            "{}",
            tab.text()
        );
        assert!(
            said.borrow().is_empty(),
            "locked, so nothing more to say: {said:?}"
        );
    }

    /// The same sentence's second half: whatever the dialogue let through, the job GTK builds after
    /// `custom-widget-apply` keeps the document's scaling — Table 147's `None`, "no page scaling",
    /// GTK's 100 — and the reader is told the choice was set back.
    #[gtk4::test]
    fn a_held_scaling_is_what_the_job_is_built_on() {
        let (operation, said, _) = operation(true, false);
        let (_dialog, tab) = the_dialogue(&operation);
        assert_eq!(accepted(&operation, tab.as_ref()), 100.0);
        assert_eq!(said.borrow().as_slice(), [SET_BACK]);
    }

    /// The control (trap 13): where the reader's level does not hold it — `off`, `warn`, or a
    /// `yes` at `ask`, each of which grants `scaling_held` false (ADR 2840) — the dialogue is
    /// GTK's own, the scale stays live, and the reader's 150 % is the job's.
    #[gtk4::test]
    fn an_unheld_scaling_is_the_readers() {
        let (operation, said, _) = operation(false, false);
        let (dialog, tab) = the_dialogue(&operation);
        let tab = tab.expect("the tray box's tab is always there");
        assert!(
            tab.last_child().and_downcast::<gtk4::Label>().is_none(),
            "nothing held, so no held sentence"
        );
        assert_eq!(dialog.manual_capabilities(), the_operations_capabilities());
        assert_eq!(accepted(&operation, Some(&tab)), 150.0);
        assert!(said.borrow().is_empty());
    }

    /// §12.2, Table 147's `/PickTrayByPDFSize`: "\[i]f PickTrayByPDFSize is true, the check box in
    /// the print dialogue associated with input paper tray shall be checked" — and "\[t]his setting
    /// influences only the preset values", so the box is the reader's: what is accepted is what
    /// the job does, each way (ADR 2919).
    #[gtk4::test]
    #[expect(
        clippy::doc_markdown,
        reason = "verbatim quotation: Table 147 spells PickTrayByPDFSize without backticks"
    )]
    fn the_tray_box_opens_as_the_document_asks_and_prints_as_the_reader_leaves_it() {
        for (stated, flipped) in [(true, false), (true, true), (false, false), (false, true)] {
            let (operation, _, by_page_size) = operation(false, stated);
            let (_dialog, tab) = the_dialogue(&operation);
            let tab = tab.expect("the tray box's tab");
            let tray = the_tray_box(&tab).expect("the tray box is the tab's first line");
            assert_eq!(
                tray.is_active(),
                stated,
                "the box opens on the document's value"
            );
            assert_eq!(
                tray.label().as_deref(),
                Some(viewer_host::printing::TRAY_BY_PAGE_SIZE)
            );
            if flipped {
                tray.set_active(!stated);
            }
            accepted(&operation, Some(&tab));
            assert_eq!(by_page_size.get(), stated != flipped, "the box as accepted");
        }
    }

    /// The box checked, a page's paper is its own size: a landscape A4 page is A4 turned — the
    /// paper a print system finds the tray for — and a page no paper is named for is a custom size
    /// of its own dimensions.
    #[gtk4::test]
    fn a_page_is_printed_on_paper_of_its_own_size() {
        let setup = gtk4::PageSetup::new();
        set_the_paper(
            &setup,
            viewer_host::printing::paper_for_page(841.89, 595.276),
        );
        assert_eq!(setup.paper_size().name().as_str(), "iso_a4");
        assert_eq!(setup.orientation(), gtk4::PageOrientation::Landscape);
        set_the_paper(&setup, viewer_host::printing::paper_for_page(300.0, 200.0));
        let paper = setup.paper_size();
        assert!(paper.is_custom(), "{}", paper.name());
        // GTK keeps a custom paper in millimetres, so the points come back to within a rounding.
        assert!((paper.width(gtk4::Unit::Points) - 200.0).abs() < 1e-6);
        assert!((paper.height(gtk4::Unit::Points) - 300.0).abs() < 1e-6);
        assert_eq!(setup.orientation(), gtk4::PageOrientation::Landscape);
    }
}

#[cfg(test)]
mod print_defaults_settings_spec_audit;
