//! Writes the slide show no corpus document is: four pages, a `/Dur`, three `/Trans` and a
//! `/PresSteps`.
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
//! page drawn upside down is visible at a glance, which is a defect the round that wrote this had.
//!
//! **The last slide is §12.4.4.2's**, added in the four-hundred-and-eighty-first session (ADR
//! 0316): its two squares are optional content, and its `/PresSteps` chain turns them on one at a
//! time, so pressing `p` and then an arrow key is a slide show stepping *within* a page. It states
//! no `/Dur` of its own — Table 165's per-node timing is what advances it — and its second node
//! states one, so a presentation left alone finishes the page by itself.
//!
//! **And since ADR 0470 it can write the window as well as the slides.** `--opens-full-screen`
//! adds Table 29's `/PageMode /FullScreen` — "how the document shall be displayed when opened" —
//! with a §12.2 `/ViewerPreferences` beside it stating `/HideToolbar`, `/HideWindowUI` and a
//! `/NonFullScreenPageMode`, which is the whole of what the standard says about a presentation's
//! *window*. It is a flag rather than the default so that the file this example has always
//! written is still the file it writes.
//!
//! ```sh
//! cargo run --release -p pdf-model --example presentation_fixture -- /tmp/slides.pdf
//! cargo run --release -p viewer-ui --bin pdf-viewer -- /tmp/slides.pdf   # then press p
//!
//! cargo run --release -p pdf-model --example presentation_fixture -- /tmp/full.pdf \
//!     --opens-full-screen                          # opens presenting; Escape comes back
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
const SLIDES: [(&str, &str, &str); 4] = [
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
    // §12.4.4.2's slide, and it states **no** `/Dur`: its own states are what advance, on Table
    // 165's per-node timing, and a page duration would take the page away in the middle of them.
    ("0.2 0.2 0.2", "", "/Trans << /Type /Trans /S /Fade /D 1 >>"),
];

/// The objects after the pages: two optional content groups, two navigation nodes, four actions.
///
/// Numbered from where the page objects stop — object 1 is the catalog, 2 the page tree, and each
/// slide takes a page and a content stream — so that the whole file stays one flat list a person
/// can read with `pdftotext`.
const BULLET_ONE: usize = SLIDES.len() * 2 + 3;
/// The second of §12.4.4.2 NOTE 1's bullet points.
const BULLET_TWO: usize = BULLET_ONE + 1;
/// The `/PresSteps` node: the primary node of the page that has states.
const NODE_ONE: usize = BULLET_TWO + 1;
/// Its `/Next`.
const NODE_TWO: usize = NODE_ONE + 1;
/// The first node's `/NA`.
const ON_ONE: usize = NODE_TWO + 1;
/// Its `/PA`.
const OFF_ONE: usize = ON_ONE + 1;
/// The second node's `/NA`.
const ON_TWO: usize = OFF_ONE + 1;
/// Its `/PA`.
const OFF_TWO: usize = ON_TWO + 1;

/// What the catalog says about the *window*, where `--opens-full-screen` asked for one.
///
/// Every entry here is a sentence about a window rather than about a page: Table 29's `/PageMode`,
/// which is "how the document shall be displayed when opened"; two of Table 147's three hide
/// flags; and the page mode §12.2 says to display on exiting full-screen mode — whose value is
/// the name ISO 32000-2 errata issue #275 restores to that entry's list.
fn window_entries() -> &'static str {
    if std::env::args().any(|argument| argument == "--opens-full-screen") {
        "/PageMode /FullScreen /ViewerPreferences << /HideToolbar true /HideWindowUI true \
         /NonFullScreenPageMode /UseAttachments >> "
    } else {
        ""
    }
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: presentation_fixture <out.pdf> [--opens-full-screen]");
        std::process::exit(1);
    };
    let window = window_entries();

    let mut objects: Vec<String> = Vec::new();
    let kids = (0..SLIDES.len())
        .map(|index| format!("{} 0 R", index.saturating_mul(2).saturating_add(3)))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Catalog /Pages 2 0 R {window}/OCProperties << /OCGs [{BULLET_ONE} 0 R \
         {BULLET_TWO} 0 R] /D << /BaseState /OFF /Order [{BULLET_ONE} 0 R {BULLET_TWO} 0 R] >> \
         >> >>"
    ));
    objects.push(format!(
        "<< /Type /Pages /Kids [{kids}] /Count {} >>",
        SLIDES.len()
    ));
    for (index, (colour, duration, transition)) in SLIDES.iter().enumerate() {
        let contents = index.saturating_mul(2).saturating_add(4);
        let stepped = index.saturating_add(1) == SLIDES.len();
        // The square is in the upper left of a y-up page, which is the corner a flip moves. The
        // last slide's two squares are §12.4.4.2's bullet points instead, each inside §8.11.3.1's
        // marked content so that a navigation node turning its group on is a mark appearing.
        let stream = if stepped {
            format!(
                "{colour} rg 0 0 {} {} re f /OC /one BDC 1 1 1 rg 60 340 80 80 re f EMC \
                 /OC /two BDC 1 1 1 rg 60 220 80 80 re f EMC",
                BOX.0, BOX.1
            )
        } else {
            format!(
                "{colour} rg 0 0 {} {} re f 1 1 1 rg 60 340 80 80 re f",
                BOX.0, BOX.1
            )
        };
        let steps = if stepped {
            format!(
                "/PresSteps {NODE_ONE} 0 R /Resources << /Properties \
                 << /one {BULLET_ONE} 0 R /two {BULLET_TWO} 0 R >> >>"
            )
        } else {
            String::new()
        };
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] {duration} {transition} {steps} \
             /Contents {contents} 0 R >>",
            BOX.0, BOX.1
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    // §12.4.4.2's two states, as NOTE 1 describes one: two groups, and a node per bullet whose
    // `/NA` turns its group on and whose `/PA` turns it off again. The second node states Table
    // 165's `/Dur`, so a presentation left running walks the states by itself.
    objects.push("<< /Type /OCG /Name (bullet one) >>".to_owned());
    objects.push("<< /Type /OCG /Name (bullet two) >>".to_owned());
    objects.push(format!(
        "<< /Type /NavNode /NA {ON_ONE} 0 R /PA {OFF_ONE} 0 R /Next {NODE_TWO} 0 R >>"
    ));
    objects.push(format!(
        "<< /Type /NavNode /NA {ON_TWO} 0 R /PA {OFF_TWO} 0 R /Prev {NODE_ONE} 0 R /Dur 2 >>"
    ));
    objects.push(format!(
        "<< /S /SetOCGState /State [/ON {BULLET_ONE} 0 R] >>"
    ));
    objects.push(format!(
        "<< /S /SetOCGState /State [/OFF {BULLET_ONE} 0 R] >>"
    ));
    objects.push(format!(
        "<< /S /SetOCGState /State [/ON {BULLET_TWO} 0 R] >>"
    ));
    objects.push(format!(
        "<< /S /SetOCGState /State [/OFF {BULLET_TWO} 0 R] >>"
    ));

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
