//! ISO 32000-2 §12.2's print half: read, answered to a host, and marking nothing.
//!
//! Table 147's dictionary controls "the way the document shall be presented on the screen or in
//! print", and eight of its entries are about the second of those. Every one of the eight states
//! a `shall` whose condition is an operation this program does not perform — a print dialogue
//! being displayed (`/PrintScaling`, `/Duplex`, `/PickTrayByPDFSize`, `/PrintPageRange`,
//! `/NumCopies`), or a page being rendered for paper (`/PrintArea`, `/PrintClip`, and
//! `/PrintScaling`'s second sentence). What this program owes such an entry is therefore to read
//! it and to put it where whoever holds that condition can take it, which is
//! [`Query::Preferences`].
//!
//! So this file pins **both halves of that sentence at once**, which is the only way either is
//! worth anything:
//!
//! - the value reaches a host, entry by entry, with Table 147's own defaults where the document
//!   is silent — and "implementation dependent" kept distinguishable from a stated default,
//!   because only the first leaves the choice to the host; and
//! - **the page is byte-identical either way.** A preference about paper that moved a pixel
//!   would be this program honouring a print entry on screen, which no sentence of §12.2 asks
//!   for. The pair of documents differs in one dictionary and in nothing else, so a raster that
//!   differed could only have come from that dictionary (trap 13: an instrument that cannot fail
//!   measures nothing, and the second document is this one's control).
//!
//! The fixture is hand-built and trap 8 is why it has to be: `pdf-model/examples/print_preference_census`
//! finds no document in the pdf.js corpus stating any of `/Duplex`, `/PickTrayByPDFSize`,
//! `/PrintPageRange` or `/NumCopies`, and 165 of the 65 944 crawled documents stating anything in
//! the print half that is not its default.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the object numbers below are the small integers this file wrote"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_model::page::Boundary;
use pdf_model::viewer_preferences::{Duplex, PrintScaling};
use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Printing, Query, Rendered, Sheet, Viewer};

/// The document under test, and its control, differing in one catalog entry.
///
/// `stating` writes Table 147's whole print half with a value that is *not* the entry's default
/// wherever the table states one: `/PrintScaling /None` against `AppDefault`, `/PrintArea` and
/// `/PrintClip` against `CropBox`, and the four PDF 1.7 entries whose default is
/// "implementation dependent" and which therefore say something merely by being present.
fn preferences(stating: bool) -> Vec<u8> {
    let dictionary = if stating {
        "/ViewerPreferences << /PrintArea /MediaBox /PrintClip /MediaBox /PrintScaling /None \
         /Duplex /DuplexFlipLongEdge /PickTrayByPDFSize true /PrintPageRange [1 1] \
         /NumCopies 3 >>"
    } else {
        ""
    };
    // A page that actually marks: a filled rectangle and a stroked line, so that a raster which
    // changed would have somewhere to change.
    let content = "0.2 0.4 0.9 rg 40 40 200 120 re f\n2 w 0 G 20 300 m 380 340 l S";
    let objects: [String; 4] = [
        format!("<< /Type /Catalog /Pages 2 0 R {dictionary} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /CropBox [20 20 380 380] \
         /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// Opens one of the pair into a viewer, and rasterises the page it asks for.
fn opened(stating: bool) -> (Viewer, (u32, u32, Vec<u8>)) {
    let mut viewer = Viewer::new(500, 500, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: preferences(stating).into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("the page is asked for");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws this page");
    let pixels = (raster.width, raster.height, raster.data.clone());
    let _ = viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .count();
    (viewer, pixels)
}

/// Table 147's print half reaches a host entry by entry, and the control states nothing.
#[test]
fn the_print_half_of_table_147_is_answered_to_a_host() {
    let (viewer, _) = opened(true);
    let Answer::Preferences(stated) = viewer.query(Query::Preferences) else {
        panic!("Query::Preferences answers with Table 147");
    };
    assert_eq!(stated.print_area, Boundary::Media);
    assert_eq!(stated.print_clip, Boundary::Media);
    assert_eq!(stated.print_scaling, PrintScaling::NoScaling);
    assert_eq!(stated.duplex, Some(Duplex::FlipLongEdge));
    assert_eq!(stated.pick_tray_by_pdf_size, Some(true));
    assert_eq!(stated.print_page_range, vec![(1, 1)]);
    assert_eq!(stated.num_copies, Some(3));
    assert!(
        !stated.enforce_print_scaling,
        "Table 148's /Enforce is not written here, so nothing is binding"
    );

    let (viewer, _) = opened(false);
    let Answer::Preferences(silent) = viewer.query(Query::Preferences) else {
        panic!("Query::Preferences answers with Table 147");
    };
    // Table 147 states a default for the first three and "implementation dependent" for the
    // rest, and the difference is the point: a host told `None` may choose, and a host told
    // `CropBox` has been told what the table says.
    assert_eq!(silent.print_area, Boundary::Crop);
    assert_eq!(silent.print_clip, Boundary::Crop);
    assert_eq!(silent.print_scaling, PrintScaling::AppDefault);
    assert_eq!(silent.duplex, None);
    assert_eq!(silent.pick_tray_by_pdf_size, None);
    assert!(silent.print_page_range.is_empty());
    assert_eq!(silent.num_copies, None);
}

/// And not one of the eight moves a pixel of the page on the screen.
///
/// `/PrintArea /MediaBox` is the entry that makes this more than a formality: it names a boundary
/// wider than the `/CropBox` this page states, so a reader that confused §12.2's print pair with
/// its view pair would lay a different region of the page into the same viewport. Planted, that
/// is exactly what this assertion names — one line of `page.rs` reading `preferences.print_area`
/// where it reads `preferences.view_area` fails it and leaves its neighbour green, which is what
/// says the pair of documents is an instrument rather than a pair of documents (trap 13).
#[test]
fn a_print_preference_changes_no_pixel_of_the_screen() {
    let (_, stating) = opened(true);
    let (_, silent) = opened(false);
    assert_eq!(
        (stating.0, stating.1),
        (silent.0, silent.1),
        "the page on screen is §12.2's /ViewArea, which neither document states"
    );
    assert!(
        stating.2 == silent.2,
        "no sentence of §12.2 asks a print entry to mark the screen"
    );
}

/// And under a print operation the page *is* §12.2's print pair, which is what the entries are
/// for.
///
/// The same two documents, printed rather than shown. `/PrintArea /MediaBox` names a boundary
/// wider than the `/CropBox` this page states, so the region laid onto paper is 400 units square
/// where the control's is 360 — and the control is the document whose catalog states nothing, so
/// a reader that ignored the entry would produce the control's rectangle for both (trap 13).
///
/// The sheet is the page's own size at one pixel per unit, so nothing here is measuring
/// §12.5.6.22's placement: what the target's width reports is which of §14.11.2's boxes the
/// content stream was interpreted against.
#[test]
fn the_print_pair_decides_the_page_rendered_for_paper() {
    for (stating, expected) in [(true, 400_u32), (false, 360_u32)] {
        let (mut viewer, _) = opened(stating);
        let _ = viewer
            .handle(Command::Print(Printing::Start(Sheet {
                media: Some([0.0, 0.0, 400.0, 400.0]),
                scale: 1.0,
                page_scale: 1.0,
            })))
            .count();
        let Answer::PrintPage(printed) = viewer.query(Query::PrintPage(0)) else {
            panic!("a print operation is running, so page one is answered");
        };
        let target = printed
            .target
            .expect("this page is well inside the pixel budget");
        assert_eq!(
            target.width,
            expected,
            "§12.2's /PrintArea decides the area rendered when printing, and this document \
             states {}",
            if stating { "/MediaBox" } else { "nothing" }
        );
        assert_eq!(target.height, expected, "the fixture's boxes are square");
    }
}
