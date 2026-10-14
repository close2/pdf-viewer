//! Printing through GTK's print dialogue: the settings, the sheet, and each page painted into the print context.

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
        let sheet = sheet_of(&gtk4::PageSetup::new());
        self.dispatch(Command::Print(Printing::Start(sheet)));
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
    ) {
        let Ok(pages) = i32::try_from(pages) else {
            self.say("this document has more pages than a print job can name");
            self.dispatch(Command::Print(Printing::Finish));
            return;
        };
        // §12.2's half of Table 147, as the dialogue's opening state. Read here rather than
        // inside the operation because the answer borrows this host and the operation must not.
        let defaults = match self.viewer.query(Query::Preferences) {
            Answer::Preferences(preferences) => viewer_host::PrintDefaults::of(&preferences),
            _ => viewer_host::PrintDefaults::of(
                &pdf_model::viewer_preferences::ViewerPreferences::default(),
            ),
        };
        if let Some(note) = defaults.enforcement_note() {
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
        let Answer::PrintPage(printed) = self.viewer.query(Query::PrintPage(index)) else {
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
/// **Defaults and not instructions**, which is what each of those entries states: "[t]he page
/// scaling option that shall be selected when a print dialogue is displayed for this document",
/// "[t]he paper handling option that shall be used when printing the PDF file from the print
/// dialogue", "[t]he page numbers used to initialise the print dialogue box when the PDF file is
/// printed", "[t]he number of copies that shall be printed when the print dialog is opened for
/// this PDF file". A person changes any of them in the dialogue this seeds, and Table 148's
/// `/Enforce` is said in the status bar rather than obeyed, because a document telling a reader
/// what they may not change is a restriction and `CLAUDE.md` makes those the reader's.
///
/// **`/PrintPageRange` is one-based and `GtkPageRange` is not**, which is the whole reason
/// `viewer_host::PrintDefaults` keeps the clause's own numbering: the conversion is here, at the
/// one place the two vocabularies meet, rather than in a value that would then be
/// indistinguishable from the other kind.
///
/// **`/PickTrayByPDFSize` is deliberately not carried**: Table 147 states it as a *check box in
/// the print dialogue*, and `GtkPrintSettings` has no such setting — `set_default_source` names a
/// tray rather than asking for one to be chosen by page size, so writing the entry into it would
/// be this host answering a different question (trap 17's shape, answered the other way).
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
        settings.set_scale(100.0);
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
        for (x, pixel) in row.chunks_exact(4).enumerate() {
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
