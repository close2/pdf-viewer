//! The viewer, wrapped so that every entry point has one safe function to call.
//!
//! [`crate::abi`]'s job is to turn raw pointers into these arguments and nothing else; every
//! decision about what a command *means* is here, in safe Rust, where the rest of the tree's
//! rules apply unchanged.
//!
//! **Nothing in this module re-decides anything `viewer-core` says.** A method here is a
//! [`viewer_core::Command`] built from C-shaped arguments, or a [`viewer_core::Query`] whose
//! answer is copied out of the borrow before it is handed on.

mod archiving;
mod documents;
mod editing;
mod frames;
mod navigation;
mod policy;
mod printing;
mod reports;
mod scripts;

use crate::events::Events;
use crate::reads::Reads;
use crate::status::Status;
use pdf_render::Rasterizer as _;
use pdf_render::{Raster, RasterFormat};
use render_cpu::CpuRasterizer;
use viewer_core::{Command, Event, RenderRequest, Viewer};

/// One viewer, and the whole of what a `quorra_viewer *` points at.
///
/// A plain wrapper with no interior mutability, because C owns it: every entry point takes it as
/// `*mut` and the caller is the one holding it. That is the ownership `viewer-qt` found to be
/// simpler than `viewer-gtk`'s `Rc<RefCell<…>>` (ADR 0246, section 3), arrived at here for the same
/// reason — the callbacks are the caller's, not ours.
#[derive(Debug)]
pub struct Session {
    /// The state machine.
    viewer: Viewer,
    /// The reader's restriction policy, as this caller has set it.
    ///
    /// Held here because `quorra_restrict_operation` sets *one* operation's level and
    /// `viewer_core::Command::Restrict` carries the whole policy: a C caller that had to resend
    /// all six would be keeping this crate's state for it. The viewer's own copy is the same
    /// value; this is the one a partial change is applied to (ADR 1144).
    restrictions: viewer_core::RestrictionPolicy,
    /// What the document this caller has open departs from that policy in.
    ///
    /// Held beside it for the same reason and for one more: `quorra_restrict_document_operation`
    /// sets one operation's departure, and a departure is *forgotten when the document closes*, so
    /// the mirror is put back at [`viewer_core::RestrictionOverride::NONE`] whenever a document is
    /// opened. A caller that could not see that would be composing its next override on top of the
    /// last document's (ADR 1145).
    departures: viewer_core::RestrictionOverride,
    /// The last page a print operation asked for, rasterised — see [`Session::print_page`].
    ///
    /// Held for the reason a frame is: C's two-call idiom asks for the size and then for the
    /// pixels, and a page rasterised twice at the printer's resolution is the whole page's cost
    /// paid twice. One at a time, because a print job draws a page, spools it and moves on.
    printed: Option<PrintedPage>,
    /// The last conversion to ISO 19005 a grant released — see [`Session::archive`].
    ///
    /// Held for [`Self::printed`]'s reason: C's two-call idiom asks for the size and then for the
    /// bytes, and the report's lines are read one at a time (ADR 1954).
    archived: Option<viewer_host::archiving::Converted>,
    /// The password each open document was opened with, which a conversion opens the file again
    /// with — the library is a C caller's host, and keeps what a window keeps (ADR 1967).
    kept: viewer_host::password::Kept,
    /// The sentence the last key press owes the person, where it owed one — see
    /// [`Session::press`].
    ///
    /// Held for [`Self::printed`]'s reason: C's two-call idiom asks for the size and then for the
    /// text, and a press acted on twice to ask twice would be two presses (ADR 2072).
    said: Option<String>,
    /// The text the last copy released, in the order [`viewer_host::copied`] chose — see
    /// [`Session::copy_text`].
    ///
    /// Held for [`Self::printed`]'s reason, and for one more: C's two-call idiom asks for the size
    /// and then for the text, and a copy sent twice to answer it would ask the reader's level for
    /// Table 22's bit 5 twice. So the text is taken from the [`Event::Copied`] the grant released,
    /// whichever call released it, and a readback hands over only that (ADR 2087).
    copied: Option<viewer_host::Copied>,
}

/// One printed page, rasterised and waiting for the caller's buffer.
#[derive(Debug)]
struct PrintedPage {
    /// Which page, zero-based.
    page: usize,
    /// Its pixels at the job's resolution.
    raster: Raster,
    /// What could not be drawn on it, already worded.
    reports: Vec<String>,
}

impl Session {
    /// What the viewer answers, read through the view both handles lend (ADR 2104).
    #[must_use]
    pub const fn reads(&self) -> Reads<'_> {
        Reads::of(&self.viewer)
    }

    /// A viewer for a viewport of this size.
    ///
    /// The scale is device pixels per logical pixel, which is 2.0 on a doubled display.
    /// [`Viewer::new`] takes it because a page must be rasterised at the resolution it will be
    /// shown at.
    #[must_use]
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            viewer: Viewer::new(width, height, scale),
            restrictions: viewer_core::RestrictionPolicy::default(),
            departures: viewer_core::RestrictionOverride::NONE,
            printed: None,
            archived: None,
            kept: viewer_host::password::Kept::new(),
            said: None,
            copied: None,
        }
    }

    /// Sends one command and collects everything it produced.
    ///
    /// The one place a command reaches the viewer, so that "the borrow ends before the caller
    /// sees anything" is a property of one function rather than of every entry point.
    fn handle(&mut self, command: Command) -> Events {
        // A copy asked again forgets the last one's text before the policy answers, so a refusal,
        // an unanswered question or an empty selection leaves nothing a caller could paste in the
        // belief that this copy released it (ADR 2087).
        if matches!(command, Command::Copy) {
            self.copied = None;
        }
        let events = self.viewer.handle(command).collect::<Vec<Event>>();
        // A conversion's grant is answered here, whichever call released it — `quorra_archive`,
        // or `quorra_answer` after the ask level's question — because a C caller cannot run
        // `pdf_transform`'s converter and this library is its host (ADR 1954). On the calling
        // thread: a caller that wants it elsewhere calls from elsewhere.
        // A copy's grant is kept here for the same reason — `quorra_copy`, or `quorra_answer` after
        // the ask level's question — in the order the one decision every host shares chooses.
        for event in &events {
            match event {
                Event::Archiving {
                    document,
                    bytes,
                    asked,
                } => {
                    self.archived = Some(viewer_host::archiving::convert(
                        *document,
                        bytes.clone(),
                        asked.clone(),
                        self.kept.lend(*document),
                    ));
                }
                Event::Copied {
                    logical,
                    page_order,
                    ..
                } => self.copied = viewer_host::copied(logical.clone(), page_order),
                _ => {}
            }
        }
        Events::new(events)
    }
}

/// What the viewer is holding, without the pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameInfo {
    /// Which page these pixels are of, zero-based.
    pub page: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixel layout, which is what a caller blits under.
    pub format: RasterFormat,
    /// How many bytes [`Session::frame_copy`] writes.
    pub bytes: usize,
    /// Where the raster's top-left corner sits in the viewport, in device pixels.
    pub origin: (f32, f32),
}

/// Draws a request with the processor rasteriser.
///
/// **This is why this crate depends on `render-cpu` and it is the only reason.** A
/// [`viewer_core::Event::NeedsRender`] carries an `Arc<DisplayList>`, which is clauses 8 and 9 in
/// a data structure and is not a thing to put in a header; what a C caller wants from it is
/// pixels. The request stays an opaque handle it may move to a thread of its own, and this is
/// what that thread calls.
///
/// # Errors
///
/// [`Status::RenderRefused`] where the rasteriser declined — a coverage or budget refusal, which
/// `CLAUDE.md` requires be reported rather than approximated.
pub fn rasterise(request: &RenderRequest) -> Result<Raster, Status> {
    // `Rasterizer` is the trait `rasterize` is on, and it takes `&mut self` because a backend may
    // keep scratch buffers between frames. One rasteriser per call is what a C caller's own
    // thread wants anyway: the handle it was given is the only state the drawing needs.
    CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .map_err(|_| Status::RenderRefused)
}

/// Draws one window of a request's target with the processor rasteriser: what a caller under
/// [`viewer_core::Oversize::Listed`] allocates in place of the page (ADR 2060).
///
/// The window is `viewer_host`'s, so that its corner moves to the origin by the construction every
/// toolkit window draws through ([`viewer_host::oversize::Window::target_of`]).
///
/// # Errors
///
/// [`Status::OutOfRange`] where the window is empty or reaches past the target;
/// [`Status::RenderRefused`] where the rasteriser declined.
pub fn rasterise_window(
    request: &RenderRequest,
    window: viewer_host::oversize::Window,
) -> Result<Raster, Status> {
    let whole = viewer_host::oversize::Window {
        x: 0,
        y: 0,
        width: request.target.width,
        height: request.target.height,
    };
    if window.width == 0 || window.height == 0 || !whole.holds(window) {
        return Err(Status::OutOfRange);
    }
    CpuRasterizer::new()
        .rasterize_window(&request.list, request.target, window.shown())
        .map_err(|_| Status::RenderRefused)
}

/// The window of a request's target to draw for a page whose top-left corner sits at `origin` in
/// a viewport `viewport` device pixels across: what it shows, grown by `viewer_host`'s margin —
/// the two rules every toolkit window places its window by (ADRs 1892, 1930).
#[must_use]
pub fn window_for(
    request: &RenderRequest,
    origin: (f32, f32),
    viewport: (u32, u32),
) -> Option<viewer_host::oversize::Window> {
    viewer_host::oversize::shown_at(request.target, origin, viewport)
        .map(|shown| viewer_host::oversize::with_margin(shown, request.target, viewport))
}

/// §7.9.4's date from a caller's fields, or `None` where a field is outside the clause's range.
///
/// **The clause's own grammar decides**, rather than a second statement of its ranges here: the
/// fields are spelt by the writer that spells Table 166's `/M`
/// ([`pdf_model::attachment::filing::pdf_date`]) and read back by [`pdf_syntax::Date::parse`],
/// and a date is taken only where what it reads is what was given. A month of 13, a 30 February,
/// a five-digit year and an offset of a day or more are each not a date, as they are not in a
/// file (ADR 2060).
#[must_use]
pub fn clock_date(
    (year, month, day): (i32, u8, u8),
    (hour, minute, second): (u8, u8, u8),
    offset: Option<i16>,
) -> Option<pdf_syntax::Date> {
    let given = pdf_syntax::Date {
        year,
        month,
        day,
        hour,
        minute,
        second,
        offset,
    };
    let read = pdf_syntax::Date::parse(&pdf_model::attachment::filing::pdf_date(given))?;
    ((read.year, read.month, read.day) == (year, month, day)
        && (read.hour, read.minute, read.second) == (hour, minute, second)
        && read.offset == offset)
        .then_some(read)
}

/// The format a server's answer is in by its media type: §12.7.8.1's and the registry's names,
/// as [`viewer_host::submit::reply`] reads them, and `Other` for every other type.
#[must_use]
pub fn data_format_of_media_type(media_type: &str) -> pdf_model::action::DataFormat {
    let media = media_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if viewer_host::submit::FDF_TYPES.contains(&media.as_str()) {
        pdf_model::action::DataFormat::Fdf
    } else if viewer_host::submit::XFDF_TYPES.contains(&media.as_str()) {
        pdf_model::action::DataFormat::Xfdf
    } else {
        pdf_model::action::DataFormat::Other
    }
}

#[cfg(test)]
mod tests {
    use viewer_core::PageTarget;

    use super::{Session, rasterise};
    use crate::kinds::EventKind;
    use crate::status::Status;

    /// The five-page application note, which every host in this tree is measured on.
    fn note() -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
        )
        .expect("the application note is in the tree")
    }

    /// The whole path a C caller takes, in Rust: open, render, turn a page, ask, copy pixels.
    ///
    /// The same sequence `c/open_a_page.c` performs through the ABI. Having it here as well is
    /// what makes a failure legible: this one says the *vocabulary* broke, and that one says the
    /// pointers did.
    #[test]
    fn a_document_opens_draws_turns_a_page_and_hands_over_its_pixels() {
        let mut session = Session::new(800, 1000, 1.0);
        let events = session.open(1, note(), None, None);
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        let request = events.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's first page draws");
        drop(session.render_ready_raster(&request, raster));

        assert_eq!(session.reads().page_count(), Ok(5));
        assert_eq!(session.current_page(), Ok((0, 5)));
        let before = session.frame_info(0).expect("a frame was handed back");
        assert_eq!(before.page, 0);
        assert!(
            before.bytes > 0 && before.bytes == before.width as usize * before.height as usize * 4
        );

        // A page turn, and the frame that follows it is of the page that was turned to.
        let turned = session.go_to(PageTarget::Next);
        let asked = (0..turned.len())
            .find(|index| turned.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("a page turn asks for the new page");
        let request = turned.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's second page draws");
        drop(session.render_ready_raster(&request, raster));
        assert_eq!(session.current_page(), Ok((1, 5)));
        let after = session.frame_info(0).expect("a frame was handed back");
        assert_eq!(after.page, 1);

        let mut pixels = vec![0u8; after.bytes];
        assert_eq!(session.frame_copy(0, &mut pixels), Ok(after.bytes));
        assert!(
            pixels.iter().any(|byte| *byte != 0),
            "a drawn page is not all zeroes"
        );
        // And the two-call idiom's refusal, which is what a caller that sized its buffer from a
        // stale `frame_info` would get instead of a partial page.
        let mut small = vec![0u8; after.bytes.saturating_sub(1)];
        assert_eq!(
            session.frame_copy(0, &mut small),
            Err(Status::BufferTooSmall)
        );
    }

    /// The open reads no outline, and the preparation read beside it gives page one its section.
    ///
    /// §12.3.3's outline item "Chapter one" names page one, so once the preparation has run on a
    /// thread of its own the page is announced again with that section; the announcement the open
    /// made had none, because neither the outline nor the page tree was read for it (ADR 1543).
    /// A second preparation is `None`, since both are read, and an answer for a document closed
    /// since is dropped rather than taken.
    #[test]
    fn a_preparation_read_on_another_thread_gives_page_one_its_section() {
        let outlined = assembled(&[
            "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 6 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
            "<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>",
            "<< /Title (Chapter one) /Parent 4 0 R /Dest [3 0 R /Fit] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ]);
        let mut session = Session::new(800, 1000, 1.0);
        let opened = session.open(1, outlined.clone(), None, None);
        let first = (0..opened.len())
            .find(|index| opened.kind(*index) == Ok(EventKind::PageChanged))
            .expect("the open announces its page");
        assert_eq!(opened.page_section(first), Ok(None));

        let preparation = session.preparation().expect("the outline is still to read");
        let prepared = std::thread::spawn(move || preparation.run())
            .join()
            .expect("the preparation's thread finishes");
        let said = session.prepared(prepared);
        let again = (0..said.len())
            .find(|index| said.kind(*index) == Ok(EventKind::PageChanged))
            .expect("the page is announced again");
        assert_eq!(said.page_changed(again), Ok((0, 2)));
        assert_eq!(said.page_section(again), Ok(Some("Chapter one")));
        assert!(session.preparation().is_none(), "both are read now");

        // A preparation for a document since closed and opened again names the old file.
        drop(session.close(1));
        drop(session.open(1, outlined, None, None));
        let stale = session.preparation().expect("the reopened file is unread");
        drop(session.close(1));
        drop(session.open(1, note(), None, None));
        assert_eq!(session.prepared(stale.run()).len(), 0);
    }

    /// A randomness source that counts, so the locked fixture below is a function of its plan.
    struct Counter(u8);

    impl pdf_syntax::serialize::Entropy for Counter {
        fn fill(&mut self, out: &mut [u8]) -> bool {
            for byte in out.iter_mut() {
                *byte = self.0;
                self.0 = self.0.wrapping_add(1);
            }
            true
        }
    }

    /// A file of `objects` object bodies, object 1 its catalog, with a classic cross-reference.
    fn assembled(objects: &[&str]) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let size = objects.len().saturating_add(1);
        let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        out.into_bytes()
    }

    /// A one-page file under §7.6.4's standard security handler, user password `abc`.
    fn locked() -> Vec<u8> {
        encrypted(
            &[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
            ],
            "abc",
            pdf_syntax::serialize::Access::ALL,
        )
    }

    /// `objects` as [`assembled`] writes them, under §7.6.4's standard security handler with this
    /// user password and Table 22's flags as `access` states them.
    fn encrypted(
        objects: &[&str],
        user_password: &str,
        access: pdf_syntax::serialize::Access,
    ) -> Vec<u8> {
        use pdf_syntax::serialize::{Assembly, Form, Options, Protection};
        let plain = assembled(objects);
        let source = pdf_syntax::Document::open(plain).expect("the plaintext opens");
        let mut assembly = Assembly::new(vec![&source]);
        for number in 1..=u32::try_from(objects.len()).expect("a fixture of a few objects") {
            assembly
                .copy(0, pdf_syntax::ObjectId::new(number, 0))
                .expect("the assembly takes the object");
        }
        assembly.set_root(pdf_syntax::ObjectId::new(1, 0));
        let mut out = Vec::new();
        pdf_syntax::serialize::serialize_encrypted(
            &assembly,
            pdf_syntax::Version { major: 2, minor: 0 },
            Options::new(Form::Table),
            &Protection {
                user_password,
                owner_password: "owner",
                access,
                encrypt_metadata: true,
            },
            &mut Counter(0),
            &mut out,
        )
        .expect("the locked file is written");
        out
    }

    /// The answer to a `QUORRA_EVENT_PASSWORD_REQUIRED` that follows `quorra_supply` is
    /// `quorra_open` under the name **on the event**, with the bytes of the file that was supplied.
    ///
    /// ISO 32000-2 §7.6.4.1: "If this authentication attempt fails, the interactive PDF processor
    /// should prompt for a password." The file a §12.6.4.3 link named is the one asking, so the
    /// event names the name `quorra_beside` held out for it (Table 203's `/NewWindow true`), and
    /// the source document is untouched. The named wrong answer is a C host that reopens its own
    /// document under the password, which is the retry every caller had before a second file could
    /// ask (ADRs 1332, 1335).
    #[test]
    fn a_password_asked_about_a_supplied_file_is_answered_under_the_name_on_the_event() {
        let source = assembled(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [5 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
            "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToR \
             /D [0 /Fit] /F (locked.pdf) /NewWindow true >> >>",
        ]);
        let mut session = Session::new(400, 400, 1.0);
        drop(session.open(1, source, None, None));
        drop(session.beside(Some(9)));
        drop(session.activate(5, 0));
        let bytes = locked();
        let events = session.supply(viewer_core::Purpose::RemoteDocument, Some(bytes.clone()));
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::PasswordRequired))
            .expect("the supplied file asks for a password");
        assert_eq!(
            events.document(asked),
            Ok(9),
            "the name the file opens under"
        );

        let events = session.open(9, bytes, Some("abc".to_owned().into()), None);
        let opened = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::Opened))
            .expect("the retry opens it");
        assert_eq!(
            events.opened(opened),
            Ok((9, 1)),
            "its one page, beside the source"
        );
        drop(session.focus(1));
        assert_eq!(
            session.reads().page_count(),
            Ok(2),
            "the source is still itself"
        );
    }

    /// `QUORRA_EVENT_NEEDS_FILE` says whether the file opens beside the document that asked:
    /// Table 203's `/NewWindow true` reaches a C caller as `true`, and its `false` — and an absent
    /// entry — as `false`, so a caller asking a person can say "beside" or "in place of". The named
    /// wrong answer is a C host with no way to tell the two apart (ADR 1335).
    #[test]
    fn the_question_about_a_file_says_whether_it_opens_beside() {
        for (entry, beside) in [
            ("/NewWindow true", true),
            ("/NewWindow false", false),
            ("", false),
        ] {
            let source = assembled(&[
                "<< /Type /Catalog /Pages 2 0 R >>",
                "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>",
                &format!(
                    "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action \
                     /S /GoToR /D [0 /Fit] /F (second.pdf) {entry} >> >>"
                ),
            ]);
            let mut session = Session::new(400, 400, 1.0);
            drop(session.open(1, source, None, None));
            let events = session.activate(4, 0);
            let asked = (0..events.len())
                .find(|index| events.kind(*index) == Ok(EventKind::NeedsFile))
                .expect("the link asks for its file");
            assert_eq!(events.needs_file_beside(asked), Ok(beside), "{entry}");
            assert_eq!(
                events.needs_file_beside(asked.wrapping_add(events.len())),
                Err(Status::OutOfRange)
            );
        }
    }

    /// A viewer with no document answers `NoAnswer` rather than zero.
    #[test]
    fn nothing_open_is_an_answer_and_not_a_count() {
        let session = Session::new(100, 100, 1.0);
        assert_eq!(session.reads().page_count(), Err(Status::NoAnswer));
        assert_eq!(session.current_page(), Err(Status::NoAnswer));
        assert!(session.frame_info(0).is_err());
        assert!(session.reads().outline().is_err());
    }

    /// §14.7's per-character offsets and boxes reach a C caller, and the invariant holds.
    ///
    /// **The half of the accessibility answer that crossed nowhere until this test existed.**
    /// `QUORRA_ELEMENT_NAME` is one string for a whole paragraph, so a client building AT-SPI's
    /// `org.a11y.atspi.Text` had the tree and the extents and no way to answer
    /// `GetCharacterExtents`. What is asserted here is the property that interface rests on: the
    /// character byte counts sum to the line's own length, so an offset into the text and an index
    /// into the characters convert into each other without either side guessing.
    #[test]
    fn a_tagged_pages_lines_cross_with_a_place_for_every_character_code() {
        let mut session = Session::new(800, 1000, 1.0);
        let events = session.open(1, note(), None, None);
        // The tree is built from the page's interpretation, so the page has to have been drawn.
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        let request = events.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the note's first page draws");
        drop(session.render_ready_raster(&request, raster));

        let structure = session.structure().expect("the note is tagged");
        let (_, nodes) = structure.page(0).expect("one entry per page on the screen");
        let mut lines = 0_usize;
        let mut characters = 0_usize;
        for node in 0..nodes {
            let count = structure.lines(0, node).expect("in range");
            for line in 0..count {
                let (text, codes) = structure.line(0, node, line).expect("in range");
                let mut bytes = 0_usize;
                for character in 0..codes {
                    let (produced, box_) = structure
                        .character(0, node, line, character)
                        .expect("in range");
                    bytes += produced;
                    assert!(box_[2] >= box_[0] && box_[3] >= box_[1], "a normalised box");
                    characters += 1;
                }
                assert_eq!(
                    bytes,
                    text.len(),
                    "the codes account for every byte of the line"
                );
                lines += 1;
            }
        }
        assert!(lines > 0, "a tagged page's elements drew text");
        assert!(characters > 0, "and every line is made of character codes");
        // Out of range in each of the three directions, rather than a zero that reads like an
        // answer — which is what the rest of this ABI does and what a caller walks by.
        assert_eq!(structure.lines(0, nodes), Err(Status::OutOfRange));
        assert_eq!(structure.line(0, 0, usize::MAX), Err(Status::OutOfRange));
        assert_eq!(
            structure.character(0, 0, 0, usize::MAX),
            Err(Status::OutOfRange)
        );
    }

    /// §12.3.3's outline crosses owned, which is ADR 0247's second amendment from the C side.
    #[test]
    fn the_outline_crosses_as_rows_with_a_depth_and_an_object_on_each() {
        let mut session = Session::new(800, 1000, 1.0);
        drop(session.open(1, note(), None, None));
        let outline = session.reads().outline().expect("the note has an outline");
        assert_eq!(outline.len(), 14, "the note's outline is fourteen rows");
        assert_eq!(outline.depth(0), Ok(0), "the first row is at the top level");
        assert!(
            (0..outline.len()).any(|row| outline.depth(row) == Ok(1)),
            "the note's outline has children"
        );
        for row in 0..outline.len() {
            assert!(!outline.title(row).expect("in range").is_empty());
        }
        assert_eq!(outline.title(outline.len()), Err(Status::OutOfRange));
    }

    /// §7.9.4's ranges are the clause's grammar's: a date the writer spells and the reader reads
    /// back unchanged is taken, and every other is refused (ADR 2060).
    #[test]
    fn a_clock_is_taken_only_as_a_date_the_clause_can_spell() {
        let taken = super::clock_date((2026, 10, 20), (9, 30, 0), Some(120))
            .expect("a date and a zone two hours east of UT");
        assert_eq!((taken.year, taken.month, taken.day), (2026, 10, 20));
        assert_eq!(taken.offset, Some(120));
        assert!(super::clock_date((2026, 10, 20), (9, 30, 0), None).is_some());
        assert!(super::clock_date((2026, 10, 20), (9, 30, 0), Some(0)).is_some());
        assert!(super::clock_date((2026, 10, 20), (9, 30, 0), Some(-330)).is_some());
        for (date, time, offset) in [
            ((2026, 13, 1), (0, 0, 0), None),
            ((2026, 2, 30), (0, 0, 0), None),
            ((10_000, 1, 1), (0, 0, 0), None),
            ((-1, 1, 1), (0, 0, 0), None),
            ((2026, 1, 1), (24, 0, 0), None),
            ((2026, 1, 1), (0, 60, 0), None),
            ((2026, 1, 1), (0, 0, 0), Some(24 * 60)),
        ] {
            assert!(
                super::clock_date(date, time, offset).is_none(),
                "{date:?} {time:?} {offset:?} is not a §7.9.4 date"
            );
        }
    }

    /// The media types every window imports under, and nothing else.
    #[test]
    fn an_answers_media_type_names_its_format_as_every_window_reads_it() {
        use pdf_model::action::DataFormat;
        assert_eq!(
            super::data_format_of_media_type("application/vnd.fdf"),
            DataFormat::Fdf
        );
        assert_eq!(
            super::data_format_of_media_type("Application/XFDF; charset=utf-8"),
            DataFormat::Xfdf
        );
        assert_eq!(
            super::data_format_of_media_type("text/html"),
            DataFormat::Other
        );
    }

    /// A one-page form with one text field, `typed`.
    fn form() -> Vec<u8> {
        assembled(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/Helv 0 Tf 0 g) >> \
             /OpenAction << /S /JavaScript /JS (app.alert\\('opened'\\);) >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Annots [4 0 R] >>",
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (typed) /V (hello) /Rect [10 10 90 30] \
             /F 4 >>",
        ])
    }

    /// The value the form's one field holds now.
    fn typed(session: &Session) -> String {
        let fields = session.reads().fields().expect("the form is open");
        let field = (0..fields.len())
            .find(|field| fields.name(*field, crate::kinds::TextKind::Qualified) == Ok("typed"))
            .expect("the field is there");
        fields.value(field).expect("it holds a value").to_owned()
    }

    /// §12.7.8's answer from a server is imported into the document that asked, by its id, through
    /// the same reader an import-data action uses.
    #[test]
    fn a_servers_form_data_is_imported_into_the_document_that_sent_the_form() {
        let mut session = Session::new(800, 600, 1.0);
        drop(session.open(7, form(), None, None));
        assert_eq!(typed(&session), "hello");
        let fdf = b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [<< /T (typed) /V (answered) >>] >> >>\n\
                    endobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
        drop(session.respond(
            7,
            viewer_core::Answered::Submission,
            "https://example.org/form".to_owned(),
            pdf_model::action::DataFormat::Fdf,
            fdf.to_vec(),
        ));
        assert_eq!(typed(&session), "answered");
    }

    /// Under `ask` the document's open script is put to the person before anything runs, and a
    /// `no` keeps it withheld — no worker is started for either (ADR 2061).
    #[test]
    fn at_the_ask_level_a_documents_scripts_wait_on_the_person() {
        let mut session = Session::new(800, 600, 1.0);
        drop(session.scripts(viewer_host::Scripts::Ask));
        drop(session.open(7, form(), None, None));
        let presented = session.presented();
        let asking = (0..presented.len())
            .find(|index| presented.kind(*index) == Ok(EventKind::AskingToRunScripts))
            .expect("the open script is asked about");
        assert_eq!(presented.document(asking), Ok(7));
        let declined = session.answer_scripts(7, false);
        assert!(
            (0..declined.len()).all(|index| declined.kind(index) != Ok(EventKind::ScriptAsking)),
            "a declined document's scripts ask nothing"
        );
        // And an answer to a script that is not waiting reaches nothing.
        let unasked = session.answer_script(7, viewer_core::ScriptAnswer::Unanswerable);
        assert!(unasked.is_empty());
    }

    /// Under `off`, the default, the same document asks nothing and says its script did not run.
    #[test]
    fn at_the_off_level_nothing_is_asked() {
        let mut session = Session::new(800, 600, 1.0);
        drop(session.open(7, form(), None, None));
        let presented = session.presented();
        assert!(
            (0..presented.len())
                .all(|index| presented.kind(index) != Ok(EventKind::AskingToRunScripts))
        );
    }

    /// A page past `MAX_PIXELS` is asked for whole under `Listed`, a window of it is drawn to the
    /// window's size, and the viewer is told the caller took the list (ADR 2060).
    #[test]
    fn a_page_past_the_bound_is_drawn_a_window_at_a_time() {
        let mut session = Session::new(800, 1000, 1.0);
        let opened = session.open(1, note(), None, None);
        let first = (0..opened.len())
            .find(|index| opened.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("page one is asked for");
        let request = opened.render_request(first).expect("that is the kind");
        drop(session.render_ready_raster(&request, rasterise(&request).expect("it draws")));
        drop(session.oversize(viewer_core::Oversize::Listed));
        let zoomed = session.zoom(viewer_core::Zoom::Scale(40.0));
        let asked = (0..zoomed.len())
            .find(|index| zoomed.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("the zoomed page is asked for, not refused");
        let request = zoomed.render_request(asked).expect("that is the kind");
        assert!(
            u64::from(request.target.width) * u64::from(request.target.height)
                > viewer_core::MAX_PIXELS
        );
        assert!(viewer_host::oversize::oversized(&request, (800, 1000)));
        let window = super::window_for(&request, (0.0, 0.0), (800, 1000))
            .expect("the page's corner is in the viewport");
        let raster = super::rasterise_window(&request, window).expect("the window draws");
        assert_eq!((raster.width, raster.height), (window.width, window.height));
        let past = viewer_host::oversize::Window {
            x: request.target.width,
            ..window
        };
        assert_eq!(
            super::rasterise_window(&request, past).map(|_| ()),
            Err(Status::OutOfRange)
        );
        let listed = session.render_ready_listed(&request);
        assert!(
            (0..listed.len()).any(|index| listed.kind(index) == Ok(EventKind::Damage)),
            "taking a list is followed by the damage the caller draws into"
        );
    }

    /// A field typed into and committed by a caller's own control keeps what was typed.
    #[test]
    fn a_field_committed_by_a_callers_control_keeps_its_text() {
        let mut session = Session::new(800, 600, 1.0);
        drop(session.open(7, form(), None, None));
        drop(session.keys(pdf_model::view::Keys {
            shift: true,
            modifier: false,
            arrows: false,
        }));
        drop(session.set_field(
            "typed".to_owned(),
            pdf_model::view::Entered::Text("committed".to_owned()),
        ));
        drop(session.commit_field("typed".to_owned()));
        assert_eq!(typed(&session), "committed");
    }

    /// The caller's clock is what a save writes as Table 166's `/M` on the widget of the field it
    /// typed into, and a session that sent none writes none (ADRs 1160, 2060).
    #[test]
    fn a_save_writes_the_callers_clock_as_the_modified_date() {
        let saved = |clock: Option<pdf_syntax::Date>| {
            let mut session = Session::new(800, 600, 1.0);
            drop(session.open(7, form(), None, None));
            drop(session.clock(clock));
            drop(session.set_field(
                "typed".to_owned(),
                pdf_model::view::Entered::Text("dated".to_owned()),
            ));
            drop(session.commit_field("typed".to_owned()));
            let events = session.save();
            let index = (0..events.len())
                .find(|index| events.kind(*index) == Ok(EventKind::Saved))
                .expect("the save answers with its bytes");
            String::from_utf8_lossy(events.bytes(index).expect("a save's bytes")).into_owned()
        };
        let now = super::clock_date((2026, 10, 20), (9, 30, 0), Some(120));
        let dated = saved(now);
        // The writer spells every string in hexadecimal: `D:20261020093000+02'00'`.
        let stamp = "/M <443A32303236313032303039333030302B303227303027>";
        assert!(dated.contains(stamp), "no {stamp} in the update");
        assert!(!saved(None).contains("/M <"));
    }

    /// A page whose one paragraph a soft hyphen divides across two lines, in Helvetica with code
    /// 173 named `/softhyphen` — §14.8.2.3's case, and the one where a copy's text and the
    /// selection's readback differ (ADR 1971).
    fn divided() -> Vec<String> {
        let content = "BT /F1 12 Tf 14 TL 20 80 Td (Seiten\\255) Tj (zahl.) ' ET";
        vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
             /Encoding << /Differences [173 /softhyphen] >> >>"
                .to_owned(),
        ]
    }

    /// A session over `bytes` with page one drawn and all of its text selected, copying at
    /// `level`.
    fn selected(bytes: Vec<u8>, level: viewer_core::RestrictionLevel) -> Session {
        let mut session = Session::new(800, 1000, 1.0);
        drop(session.restrict_operation(pdf_model::restriction::Operation::Extract, level));
        let events = session.open(1, bytes, None, None);
        let asked = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        let request = events.render_request(asked).expect("that is the kind");
        let raster = rasterise(&request).expect("the page draws");
        drop(session.render_ready_raster(&request, raster));
        drop(session.select(viewer_core::Selection::All));
        session
    }

    /// Where the batch holds an event of `kind`.
    fn found(events: &crate::events::Events, kind: EventKind) -> Option<usize> {
        (0..events.len()).find(|index| events.kind(*index) == Ok(kind))
    }

    /// `quorra_selection_copy_text` hands over the text a copy released and nothing a copy was not
    /// asked for (ADR 2087).
    ///
    /// Table 22's bit 5 is "[c]opy or otherwise extract text and graphics from the document", so
    /// the text reaches a caller's clipboard through `Command::Copy`, where the reader's level is
    /// asked; and the string is the `Event::Copied`'s own, §14.8.2.3's division after the soft
    /// hyphen undone and the character kept — `Seiten\u{ad}zahl.` where the selection's readback
    /// is `Seiten\u{ad}\nzahl.`. At `on` a document withholding the bit releases nothing; at `ask`
    /// nothing until the person says yes.
    #[test]
    fn the_copy_text_is_what_the_copy_released_at_the_readers_level() {
        use viewer_core::RestrictionLevel;
        let objects = divided();
        let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
        let joined = "Seiten\u{ad}zahl.".to_owned();

        let mut session = selected(assembled(&objects), RestrictionLevel::Off);
        assert_eq!(
            session.selection_text().as_deref(),
            Ok("Seiten\u{ad}\nzahl."),
            "the selection reads back the line break the copy takes out"
        );
        assert_eq!(
            session.copy_text(),
            Err(Status::NoAnswer),
            "a selection alone is not a copy"
        );
        let copied = session.copy();
        let index = found(&copied, EventKind::Copied).expect("an unrestricted copy is granted");
        let event = copied.copied(index);
        assert_eq!(
            event
                .as_ref()
                .map(|(text, logical)| (AsRef::<str>::as_ref(text), *logical)),
            Ok((joined.as_str(), false))
        );
        assert_eq!(
            session.copy_text(),
            Ok((joined.clone(), viewer_host::ContentOrder::PageContent)),
            "the readback is the event's own text"
        );

        let withheld = pdf_syntax::serialize::Access {
            copy: false,
            ..pdf_syntax::serialize::Access::ALL
        };
        let mut session = selected(encrypted(&objects, "", withheld), RestrictionLevel::On);
        let refused = session.copy();
        assert!(
            found(&refused, EventKind::Refused).is_some(),
            "bit 5 is clear"
        );
        assert_eq!(session.copy_text(), Err(Status::NoAnswer));

        drop(session.restrict_operation(
            pdf_model::restriction::Operation::Extract,
            RestrictionLevel::Ask,
        ));
        let asking = session.copy();
        assert!(found(&asking, EventKind::Asking).is_some());
        assert_eq!(
            session.copy_text(),
            Err(Status::NoAnswer),
            "nothing until the person answers"
        );
        let answered = session.answer(1, true);
        assert!(found(&answered, EventKind::Copied).is_some());
        assert_eq!(
            session.copy_text(),
            Ok((joined, viewer_host::ContentOrder::PageContent))
        );
        let declined = session.copy();
        assert!(found(&declined, EventKind::Asking).is_some());
        drop(session.answer(1, false));
        assert_eq!(
            session.copy_text(),
            Err(Status::NoAnswer),
            "a declined copy releases nothing, and the last one's text is not handed out for it"
        );
    }
}
