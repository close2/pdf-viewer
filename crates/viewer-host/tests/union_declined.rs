//! A mark the union's exact measure declines is carried from the draw to the reader by the pieces
//! both toolkit windows use: the drawing thread hands it back with the window it drew, and the
//! viewer, told of it, says it once (ADRs 2280, 2290).

use std::path::Path;
use std::time::{Duration, Instant};

use viewer_core::{Command, DocumentId, Event, Oversize, Viewer, Zoom};
use viewer_host::Drawing;
use viewer_host::oversize::Window;

/// The identity the one document is opened under.
const DOCUMENT: DocumentId = DocumentId(1);

/// `inks.pdf` at sixteen times holds a mark of 19 743 pieces, which costs the union more than
/// `UNION_WORK` allows: the window that shows it is drawn by the library's converter, and the
/// decline — `PastTheBudget`, the mark's pieces and its pixels — comes back with the window and is
/// said to the reader as the condition's sentence, and only once for two draws of it.
#[test]
fn the_union_s_budget_is_said_where_the_window_is_drawn() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/inks.pdf");
    let bytes = std::fs::read(&path).expect("doc/pdf.js is checked out (a submodule)");
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Oversize(Oversize::Listed))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let request = viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(16.0),
            at: None,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("a zoom asks for the page again");
    assert_eq!(request.target.width, 9792, "sixteen times 612 points");
    // Where the mark is at this magnification, as its own decline names it.
    let window = Window {
        x: 528,
        y: 5255,
        width: 602,
        height: 74,
    };
    let mut said = 0;
    for _ in 0..2 {
        let mut drawing: Drawing = Drawing::new();
        drawing.ask_window(request.clone(), window);
        let began = Instant::now();
        let landed = loop {
            drawing.collect();
            let landed = drawing.windows();
            if !landed.is_empty() {
                break landed;
            }
            assert!(
                began.elapsed() < Duration::from_mins(2),
                "the window never landed"
            );
            std::thread::sleep(Duration::from_millis(2));
        };
        let [(finished, _)] = landed.as_slice() else {
            panic!("one window: {landed:?}");
        };
        let [declined] = finished.declined.as_slice() else {
            panic!("the mark is declined once: {:?}", finished.declined);
        };
        assert_eq!(declined.condition, "it costs more than the union's budget");
        // The pixels are the window's part of the mark, on the page's grid.
        assert!(
            declined
                .sentence
                .starts_with("a mark of 19743 pieces, 602×72 pixels from (528, 5257)"),
            "{}",
            declined.sentence
        );
        said += viewer
            .handle(Command::Declined {
                document: DOCUMENT,
                page: request.page,
                declined: finished.declined.clone(),
            })
            .filter(|event| {
                matches!(event, Event::Reported { notes, .. }
                    if notes.iter().any(|note| note.ends_with("the union's budget")))
            })
            .count();
    }
    assert_eq!(said, 1, "two draws of the mark, one sentence");
}
