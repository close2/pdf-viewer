//! §8.11.4.5's print operation across the boundary: asked, held, drawn, and given back.
//!
//! The *interpretation* under print intent is `pdf-model`'s and is tested there against Table
//! 167's own cells; this is the other end — the operation, which has a beginning, a duration and
//! an end because §8.11.4.5 says so in as many words:
//!
//! > These changes shall persist only for the duration of the print operation; then all groups
//! > shall revert to their prior states.
//!
//! What is asserted here is that §7.6.4.2's bit 3 is asked where every other operation's bit is
//! asked, that the pages a printer gets are interpreted for paper and at the printer's resolution
//! rather than at the window's, that the window shows the same thing while the operation stands,
//! and that the end puts it all back. ADR 1180.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing, and the object numbers below are the small integers this \
              file wrote"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Printing, Query, Rendered, Sheet, Viewer};

/// The document every test here opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// A one-page PDF whose only annotation carries the `/F` given.
///
/// The page's own content stream is empty, so every mark in a display list came from the
/// annotation and nothing has to be subtracted to see it. `/Rect` is a 60-unit square on a
/// 100 × 100 page, and the appearance fills it.
fn page_with(flags: &str) -> Vec<u8> {
    let appearance = "1 0 0 rg 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] {flags} \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A viewer with that document open and its first page drawn.
fn reading(bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    settle(&mut viewer, events);
    viewer
}

/// Answers whatever renders the events asked for, so that the viewer holds a frame.
fn settle(viewer: &mut Viewer, events: Vec<Event>) {
    for event in events {
        let Event::NeedsRender(request) = event else {
            continue;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("the CPU backend draws this page");
        let more: Vec<Event> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        settle(viewer, more);
    }
}

/// Everything one command produced.
fn sent(viewer: &mut Viewer, command: Command) -> Vec<Event> {
    viewer.handle(command).collect()
}

/// An A4 sheet at 300 dots per inch — the value `viewer_host::printing::sheet` builds for one.
///
/// Written out rather than taken from `viewer-host`, because this crate is below that one and the
/// boundary is the thing under test: a core that needed a host crate to name a sheet would have
/// put a host's arithmetic inside the message.
fn a4() -> Sheet {
    Sheet {
        media: Some([0.0, 0.0, 595.0, 842.0]),
        scale: 300.0 / 72.0,
        page_scale: 1.0,
    }
}

/// How many commands the page's display list holds, or `None` where no page came back.
fn marks(viewer: &Viewer, page: usize) -> Option<usize> {
    match viewer.query(Query::PrintPage(page)) {
        Answer::PrintPage(printed) => Some(printed.list.commands().len()),
        _ => None,
    }
}

/// §7.6.4.2's bit 3 is asked as an operation, and the grant is what opens the pages.
///
/// **The readback is closed until the command has been answered**, which is the whole reason
/// printing is a command and not a query: a query raises no event, so it can carry neither
/// `CLAUDE.md`'s *ask* nor its *warn*, and a page that came out of one would have gone round the
/// question. Three states, each with the one before it as its control.
#[test]
fn no_page_prints_until_the_operation_has_been_granted_and_none_after_it_ends() {
    let mut viewer = reading(page_with("/F 4"));
    assert!(
        marks(&viewer, 0).is_none(),
        "a page asked for before the operation was granted"
    );

    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    let granted = events.iter().any(|event| {
        matches!(event, Event::Printing { document, pages, .. } if *document == DOCUMENT && *pages == 1)
    });
    assert!(granted, "the grant names the document and its page count");
    assert!(
        marks(&viewer, 0).is_some_and(|commands| commands > 0),
        "the page a printer asked for"
    );
    assert!(
        marks(&viewer, 1).is_none(),
        "and a page this document has not got"
    );

    sent(&mut viewer, Command::Print(Printing::Finish));
    assert!(
        marks(&viewer, 0).is_none(),
        "§8.11.4.5's duration is over, so the readback is closed again"
    );
}

/// The page a printer gets is Table 167's printed page, not the screen's.
///
/// `/F 36` is `NoView` and `Print` together, which Table 167 makes the one combination where the
/// two devices disagree outright: bit 6 is "do not render the annotation on the screen" and its
/// own row says "[t]he annotation may be printed (depending on the setting of the Print flag)".
///
/// **The control is the same document with bit 3 cleared**, and it is what makes this a claim
/// about the table rather than about printing: a reader that had simply stopped consulting `/F`
/// under print intent would pass the first assertion and fail the second.
#[test]
fn a_no_view_annotation_that_asks_to_be_printed_reaches_the_paper() {
    let mut viewer = reading(page_with("/F 36"));
    sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert!(
        marks(&viewer, 0).is_some_and(|commands| commands > 0),
        "bit 3 is set, so the annotation prints"
    );

    // And the control: the same page with bit 3 clear prints nothing at all, which is the
    // table's "[i]f clear, never print the annotation, regardless of whether it is rendered on
    // the screen".
    let mut hidden = reading(page_with("/F 32"));
    sent(&mut hidden, Command::Print(Printing::Start(a4())));
    assert_eq!(
        marks(&hidden, 0),
        Some(0),
        "bit 3 clear on an annotation with an appearance stream"
    );
}

/// The printed page is drawn at the printer's resolution and not at the window's.
///
/// The viewport is 400 × 400 at scale 1, so a 100-unit page on the screen is hundreds of pixels;
/// the job states 300 dots per inch, which is 300/72 pixels per unit, so the same page is 416 on
/// each side. A reader that had handed the printer the window's own target would produce the first
/// number, which is why the assertion is on the arithmetic rather than on "bigger".
#[test]
fn a_printed_page_is_targeted_at_the_jobs_resolution() {
    let mut viewer = reading(page_with("/F 4"));
    sent(&mut viewer, Command::Print(Printing::Start(a4())));
    let Answer::PrintPage(printed) = viewer.query(Query::PrintPage(0)) else {
        panic!("the operation was granted, so page one is there");
    };
    let target = printed.target.expect("a 100-unit page at 300 dpi is small");
    // 100 units times 300/72 is 416.66…, and a raster is a whole number of pixels.
    assert_eq!((target.width, target.height), (417, 417));
    assert_eq!(printed.page, 0);
}

/// A sheet chosen after the grant is a sheet and not a second operation.
///
/// §12.5.6.22 places a fixed print watermark against "the dimensions of the target media", and a
/// print dialogue is where a person picks that media — after the permission question, because
/// `CLAUDE.md`'s *ask* level puts a window of its own on the screen first. So the paper arrives as
/// its own message, and it asks nothing.
///
/// What discriminates is the *page's* placement: `/H 0.5` of a 595-point sheet and `/H 0.5` of the
/// 100-unit media box are two different places, and the watermark's marks are in the display list
/// either way, so the assertion is on where the list's ink is.
#[test]
fn a_sheet_chosen_after_the_grant_moves_a_fixed_print_watermark_and_asks_nothing() {
    let mut viewer = reading(watermark_page());
    sent(&mut viewer, Command::Print(Printing::Start(a4())));
    let big = ink_left_edge(&viewer);

    let events = sent(
        &mut viewer,
        Command::Print(Printing::Paper(Sheet {
            media: Some([0.0, 0.0, 200.0, 200.0]),
            scale: 300.0 / 72.0,
            page_scale: 1.0,
        })),
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            Event::Asking { .. } | Event::Refused { .. } | Event::Printing { .. }
        )),
        "a different sheet is not a second operation"
    );
    let small = ink_left_edge(&viewer);
    assert!(
        big > small,
        "half of 595 points is further across the page than half of 200: {big} against {small}"
    );
}

/// A one-page document whose watermark states Table 194's `/H` and `/V`.
fn watermark_page() -> Vec<u8> {
    let appearance = "0 0 0 rg 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Watermark /Rect [0 0 20 20] /F 4 \
         /AP << /N 6 0 R >> /FixedPrint << /Type /FixedPrint /H 0.5 /V 0.0 >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The leftmost column of the printed page that has any ink in it.
fn ink_left_edge(viewer: &Viewer) -> u32 {
    let Answer::PrintPage(printed) = viewer.query(Query::PrintPage(0)) else {
        panic!("the operation was granted, so page one is there");
    };
    let target = printed.target.expect("a small page at 300 dpi");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&printed.list, target)
        .expect("the CPU backend draws this page");
    for x in 0..raster.width {
        for y in 0..raster.height {
            let index = ((y * raster.width + x) as usize) * 4 + 3;
            if raster.data.get(index).copied().unwrap_or_default() > 0 {
                return x;
            }
        }
    }
    raster.width
}

/// A viewer over `secHandler.pdf` at a stated policy, or `None` when the submodule is absent.
///
/// **The fixture Table 22's bit 12 needs, and it is a real file rather than a hand-built one**:
/// `/V 5 /R 6` with `/P −3136`, which is `0xFFFFF3C0` — **bit 3 clear**, so the document withholds
/// "[p]rint the document", and **bit 12 clear** with it, so it withholds printing "to a
/// representation from which a faithful digital copy of the PDF content could be generated".
/// Revision 6 is what makes bit 12 readable at all: Table 22 marks the position "( Security
/// handlers of revision 3 or greater )".
///
/// `cargo run --release -p pdf-model --example encryption_census -- doc/pdf.js/test/pdfs/*.pdf`
/// is what found it, and prints the two printing positions over any path given to it.
fn restricted(policy: viewer_core::RestrictionPolicy) -> Option<Viewer> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/secHandler.pdf");
    let bytes = std::fs::read(path).ok()?;
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(viewer_core::RestrictionScope::Window(
            policy,
        )))
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    settle(&mut viewer, events);
    Some(viewer)
}

/// The grant's fidelity, where a job started at all.
fn fidelity(events: &[Event]) -> Option<viewer_core::Fidelity> {
    events.iter().find_map(|event| match event {
        Event::Printing { fidelity, .. } => Some(*fidelity),
        _ => None,
    })
}

/// Whether the list refuses, asks about or warns of one operation.
fn about(events: &[Event], operation: pdf_model::restriction::Operation) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Refused { operation: it, .. } if *it == operation => Some("refused"),
            Event::Asking { operation: it, .. } if *it == operation => Some("asking"),
            Event::Warned { operation: it, .. } if *it == operation => Some("warned"),
            _ => None,
        })
        .collect()
}

/// §7.6.4.2's Table 22 bit 12 is a second operation, asked at all four levels of its own.
///
/// **The cell states two consequences and this is the pair that discriminates between them**: bit
/// 3 clear withholds printing outright, and bit 12 clear limits a print that goes ahead —
///
/// > When this bit is clear (and bit 3 is set), printing shall be limited to a low- level
/// > representation of the appearance, possibly of degraded quality.
///
/// — so a reader who has turned bit 3's level off and left bit 12's on prints, degraded, with the
/// fidelity refused by name. A reader who set one level for both could not express that, which is
/// why the policy has two entries (ADR 1203).
#[test]
fn table_22s_bit_12_limits_a_print_that_bit_3_let_through() {
    use pdf_model::restriction::Operation;
    use viewer_core::{Fidelity, RestrictionLevel, RestrictionPolicy};

    let off = RestrictionPolicy::default();
    let Some(mut viewer) = restricted(off) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    // Every level off: the document withholds both and this reader obeys neither.
    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert_eq!(fidelity(&events), Some(Fidelity::Faithful));
    assert!(about(&events, Operation::PrintFaithfully).is_empty());

    // Bit 3 on: the operation is refused and no job starts at all, so bit 12 is never reached —
    // a fidelity is not withheld from a print that is not happening.
    let mut viewer = restricted(RestrictionPolicy::uniform(RestrictionLevel::On))
        .expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert_eq!(fidelity(&events), None);
    assert_eq!(about(&events, Operation::Print), ["refused"]);
    assert!(about(&events, Operation::PrintFaithfully).is_empty());

    // Bit 3's level off, bit 12's on: the print goes ahead and the *fidelity* is refused by name.
    let policy =
        RestrictionPolicy::default().with(Operation::PrintFaithfully, RestrictionLevel::On);
    let mut viewer = restricted(policy).expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert_eq!(fidelity(&events), Some(Fidelity::Degraded));
    assert_eq!(about(&events, Operation::PrintFaithfully), ["refused"]);
    assert!(
        marks(&viewer, 0).is_some(),
        "the job is limited rather than refused"
    );

    // Warn: the job is faithful and the document's reasons are said afterwards.
    let policy =
        RestrictionPolicy::default().with(Operation::PrintFaithfully, RestrictionLevel::Warn);
    let mut viewer = restricted(policy).expect("the submodule is checked out");
    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert_eq!(fidelity(&events), Some(Fidelity::Faithful));
    assert_eq!(about(&events, Operation::PrintFaithfully), ["warned"]);
}

/// The *ask* level over bit 12 is the one question whose `no` starts something.
///
/// Every other held operation forgets what it held on a `no`; this one prints degraded, because
/// Table 22 makes bit 12's clear state a limit on a job rather than the end of one. A `no` that
/// cancelled the print would have made bit 12 a second bit 3 (ADR 1203).
#[test]
fn asking_about_the_fidelity_starts_a_job_either_way() {
    use pdf_model::restriction::Operation;
    use viewer_core::{Fidelity, RestrictionLevel, RestrictionPolicy};

    let policy =
        RestrictionPolicy::default().with(Operation::PrintFaithfully, RestrictionLevel::Ask);
    let Some(mut viewer) = restricted(policy) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let events = sent(&mut viewer, Command::Print(Printing::Start(a4())));
    assert_eq!(about(&events, Operation::PrintFaithfully), ["asking"]);
    assert_eq!(
        fidelity(&events),
        None,
        "nothing starts until it is answered"
    );
    assert!(marks(&viewer, 0).is_none(), "and no page is handed out");

    let events = sent(
        &mut viewer,
        Command::Answer {
            document: DOCUMENT,
            proceed: false,
        },
    );
    assert_eq!(
        fidelity(&events),
        Some(Fidelity::Degraded),
        "a `no` asked for the degraded job the cell describes"
    );
    assert!(marks(&viewer, 0).is_some());

    // And a `yes` prints faithfully, which is what makes the two answers a pair rather than one
    // answer and a silence.
    let policy =
        RestrictionPolicy::default().with(Operation::PrintFaithfully, RestrictionLevel::Ask);
    let mut viewer = restricted(policy).expect("the submodule is checked out");
    sent(&mut viewer, Command::Print(Printing::Start(a4())));
    let events = sent(
        &mut viewer,
        Command::Answer {
            document: DOCUMENT,
            proceed: true,
        },
    );
    assert_eq!(fidelity(&events), Some(Fidelity::Faithful));
}
