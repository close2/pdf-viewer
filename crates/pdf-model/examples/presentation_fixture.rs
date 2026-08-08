//! Writes the slide show no corpus document is: three pages, a `/Dur` and two `/Trans`.
//!
//! **This is a fixture, not a writer.** `CLAUDE.md` excludes authoring a document from nothing
//! and nothing here claims otherwise: it is the same hand-built construction every test in this
//! tree makes in a string literal, moved to a file because §12.4.4 is the one clause whose
//! subject a *person watching the window* has to see. `presentation_census` is the reason it
//! exists — over the 964 openable corpus documents and the 14 in `doc/`, not one page states a
//! `/Trans`, a `/Dur` or a `/PresSteps`, so there is nothing to open and press `p` on.
//!
//! Each page is one flat colour with a white square in its upper left, so that a frame captured
//! mid-transition can be read as pixels rather than described with adjectives — and so that a
//! page drawn upside down is visible at a glance, which is a defect this round had.
//!
//! ```sh
//! cargo run --release -p pdf-model --example presentation_fixture -- /tmp/slides.pdf
//! cargo run --release -p viewer-ui --bin pdf-viewer -- /tmp/slides.pdf   # then press p
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose output is a file and one line saying where it went"
)]

use std::fmt::Write as _;

/// The page box every slide uses, in points: a 4:3 slide rather than a sheet of paper.
const BOX: (u32, u32) = (640, 480);

/// What each page states: its colour, its `/Dur` in seconds, and its `/Trans` dictionary.
///
/// §12.4.4.1 makes `/Trans` the entry of the page moved *to*, so page one states none: nothing
/// moves to it. The two styles are Table 164's `Wipe` "[l]eft to right" and `Split` with two
/// horizontal lines moving "[o]utward from the centre of the page".
const SLIDES: [(&str, &str, &str); 3] = [
    ("1 0 0", "/Dur 2", ""),
    (
        "0 0 1",
        "/Dur 2",
        "/Trans << /Type /Trans /S /Wipe /D 2 /Di 0 >>",
    ),
    (
        "0 0.6 0",
        "/Dur 2",
        "/Trans << /Type /Trans /S /Split /D 2 /Dm /H /M /O >>",
    ),
];

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: presentation_fixture <out.pdf>");
        std::process::exit(1);
    };

    let mut objects: Vec<String> = Vec::new();
    let kids = (0..SLIDES.len())
        .map(|index| format!("{} 0 R", index.saturating_mul(2).saturating_add(3)))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_owned());
    objects.push(format!(
        "<< /Type /Pages /Kids [{kids}] /Count {} >>",
        SLIDES.len()
    ));
    for (index, (colour, duration, transition)) in SLIDES.iter().enumerate() {
        let contents = index.saturating_mul(2).saturating_add(4);
        // The square is in the upper left of a y-up page, which is the corner a flip moves.
        let stream = format!(
            "{colour} rg 0 0 {} {} re f 1 1 1 rg 60 340 80 80 re f",
            BOX.0, BOX.1
        );
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] {duration} {transition} \
             /Contents {contents} 0 R >>",
            BOX.0, BOX.1
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    match std::fs::write(&path, out.as_bytes()) {
        Ok(()) => println!("{} slides, {} bytes, in {path}", SLIDES.len(), out.len()),
        Err(error) => {
            eprintln!("cannot write {path}: {error}");
            std::process::exit(1);
        }
    }
}
