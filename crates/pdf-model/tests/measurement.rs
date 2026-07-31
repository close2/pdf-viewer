//! ISO 32000-2 §12.9's viewports, over the corpus.
//!
//! One document of the 974 states a `/VP`, and it is the only witness this clause has. That
//! makes the measurement here worth as much as the assertion: a clause with one witness is a
//! clause written from the standard (trap 8), and the witness's job is to prove the reader
//! survives a real file rather than to rank the work.

use std::path::{Path, PathBuf};

use pdf_model::measurement::{Measure, Viewports};
use pdf_syntax::Document;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// Every page-one `/VP` in the corpus reads, and what it holds is named.
#[test]
fn the_corpus_states_one_viewport_and_it_reads() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut found = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let pages = pdf_model::Pages::new(&document);
        let Some(page) = pages.get(0) else {
            continue;
        };
        let viewports = Viewports::read(&document, &page.dict);
        if viewports.is_empty() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for viewport in &viewports.viewports {
            let kind = match &viewport.measure {
                None => "no /Measure".to_owned(),
                Some(Measure::Rectilinear(measure)) => format!(
                    "RL ratio {:?}, x {} units, distance {} units, area {} units",
                    measure.ratio,
                    measure.x.len(),
                    measure.distance.len(),
                    measure.area.len()
                ),
                Some(Measure::Geospatial(geospatial)) => format!(
                    "GEO gcs {:?} epsg {:?} wkt {} chars, {} registration points, bounds {} points, pdu {:?}, pcsm {}",
                    geospatial
                        .coordinate_system
                        .as_ref()
                        .map(|system| if system.projected { "PROJCS" } else { "GEOGCS" }),
                    geospatial
                        .coordinate_system
                        .as_ref()
                        .and_then(|system| system.epsg),
                    geospatial
                        .coordinate_system
                        .as_ref()
                        .and_then(|system| system.wkt.as_ref().map(String::len))
                        .unwrap_or(0),
                    geospatial.registration().len(),
                    geospatial.bounds.len(),
                    geospatial.display_units,
                    geospatial.matrix_has_priority()
                ),
                Some(Measure::Other(subtype)) => format!("subtype {subtype}"),
            };
            found.push(format!(
                "{name}: {:?} {:?} {kind}",
                viewport.name, viewport.bbox
            ));
        }
    }

    println!("viewports on page one across the corpus:");
    for entry in &found {
        println!("  {entry}");
    }

    assert_eq!(
        found.len(),
        1,
        "viewports the corpus states on a first page"
    );
    let only = &found[0];
    assert!(
        only.starts_with("bug1146106.pdf"),
        "the corpus's only viewport: {found:?}"
    );
    assert!(
        only.contains("GEO gcs Some(\"GEOGCS\")") && only.contains("wkt 145 chars"),
        "the only witness is a geographic system stated as Well Known Text: {only}"
    );
    assert!(
        only.contains("4 registration points"),
        "/GPTS and /LPTS pair the map's four corners: {only}"
    );

    // Two things about that one witness, both of them the *file* rather than the reader, and
    // both worth asserting so that a future change to either is a decision rather than a drift.
    //
    // Its `/Name` begins `\u{FF}\u{FE}` — a UTF-16 **little**-endian byte order mark, which
    // §7.9.2.2 does not define. The clause gives a text string three encodings: UTF-16BE
    // behind `FEFF`, UTF-8 behind `EFBBBF`, and otherwise PDFDocEncoding. So this name *is*
    // PDFDocEncoding by the clause's own elimination, and it decodes to the producer's bytes
    // rather than to "Layers". Accommodating it would mean reading an encoding the standard
    // does not define, on a guess about which producer wrote the file.
    assert!(
        only.contains("\u{ff}\u{fe}"),
        "the name is UTF-16LE, which §7.9.2.2 has no case for: {only}"
    );
    // And its `/BBox` is `[0 522 715.14655 0]`, which Table 265 forbids: "[t]he two coordinate
    // pairs of the rectangle shall be specified in normalised form; that is, lower-left
    // followed by upper-right". Kept as stated, because the clause makes that ordering mean
    // something — it "shall determine the orientation of the measuring coordinate system" —
    // so this viewport's measuring y axis runs down the page. `Viewport::contains` normalises
    // for the containment test alone.
    assert!(
        only.contains("[0.0, 522.0, 715.14655, 0.0]"),
        "the rectangle is stated upper-left first: {only}"
    );
}
