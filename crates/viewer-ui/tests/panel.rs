//! The outline panel, drawn and clicked with no window at all.
//!
//! `viewer-ui`'s panel is chrome and chrome has no gate — the corpus interprets page one, the
//! oracle rasterises pages it is handed, and neither of them opens a viewer. So the two things
//! that can silently go wrong here are checked directly: that the panel **draws ink where it
//! says it does**, by rasterising it with the same `render-cpu` the oracle is built on, and that
//! a click lands on the row a person aimed at, by asking the same function the window asks.
//!
//! Trap 1's rule applies to an interface as much as to a page: a display list holding the right
//! commands can still draw nothing.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing, and a pixel index over a raster this file sized itself \
              cannot overflow"
)]

use pdf_model::destination::{Destination, Target, View};
use pdf_model::outline::{Item, Outline};
use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::ObjectId;
use render_cpu::CpuRasterizer;
use viewer_core::Layer;
use viewer_ui::chrome::{Chrome, Content, Hit, Sidebar};

/// A window this test pretends to have: wide enough for the panel, tall enough for the rows.
const WIDTH: u32 = 400;
/// How tall the pretend window is.
const HEIGHT: u32 = 300;

/// An item with a destination naming an object, which is §12.3.2.2's first form.
fn item(title: &str, page: u32, children: Vec<Item>) -> Item {
    Item {
        // The item's own object. `page + 100` keeps it distinct from the page it points at, so
        // a test that confused the two would fail rather than pass by coincidence.
        id: ObjectId::new(page + 100, 0),
        title: title.to_owned(),
        destination: Some(Destination {
            target: Target::Object(ObjectId::new(page, 0)),
            view: View::Fit,
        }),
        open: true,
        italic: false,
        bold: false,
        colour: [0.0, 0.0, 0.0],
        children,
    }
}

/// A sidebar showing nothing but this outline, which is what the outline tests need.
fn only(outline: &Outline) -> Content<'_> {
    Content {
        outline,
        layers: &[],
        attachments: &[],
    }
}

/// Two levels, so that indentation, disclosure and identity all have something to act on.
fn outline() -> Outline {
    Outline {
        items: vec![
            item("Chapter one", 10, vec![item("Section 1.1", 11, Vec::new())]),
            item("Chapter two", 20, Vec::new()),
        ],
        stated_count: None,
    }
}

/// How many pixels of the panel are darker than its background, within a band of rows.
///
/// The background is a light grey (240 of 255) and the hover highlight is 219, so "ink" is
/// anything under 180 — which counts black text and the 107 a dimmed row is set in, and counts
/// neither background. Counting rather than comparing to a golden image: a golden would pin the
/// shape of Liberation Sans, and what is being checked is that glyphs reached the raster at all.
fn ink(panel: &pdf_render::DisplayList, rows: std::ops::Range<u32>) -> usize {
    ink_within(panel, rows, 0..WIDTH)
}

/// [`ink`], counted only inside a range of columns.
fn ink_within(
    panel: &pdf_render::DisplayList,
    rows: std::ops::Range<u32>,
    columns: std::ops::Range<u32>,
) -> usize {
    let raster = CpuRasterizer::new()
        .rasterize(
            panel,
            TargetSpec {
                width: WIDTH,
                height: HEIGHT,
                transform: Transform::IDENTITY,
            },
        )
        .expect("the panel is paths and nothing else");
    let mut dark = 0;
    for y in rows {
        for x in columns.clone() {
            let at = ((y * WIDTH + x) * 4) as usize;
            if raster.data.get(at).is_some_and(|red| *red < 180) {
                dark += 1;
            }
        }
    }
    dark
}

/// The panel draws its heading and its rows, and a hidden panel draws nothing.
#[test]
fn the_panel_puts_ink_on_the_rows_it_lists() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = outline();
    let mut panel = Sidebar::default();

    let hidden = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    assert_eq!(ink(&hidden, 0..HEIGHT), 0, "a hidden panel draws nothing");

    panel.toggle();
    let shown = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    // The heading sits above the separator and the first row below it; both must have glyphs,
    // because a panel that drew its background and no text would pass any command count.
    assert!(ink(&shown, 0..26) > 40, "the heading has no glyphs");
    assert!(ink(&shown, 26..46) > 40, "the first row has no glyphs");
    assert!(
        ink(&shown, 90..HEIGHT) == 0,
        "three rows, and nothing drawn below them"
    );
}

/// A click follows a destination, a click on the triangle discloses, and neither does the other.
#[test]
fn a_click_lands_on_the_row_it_was_aimed_at() {
    let outline = outline();
    let mut panel = Sidebar::default();
    assert_eq!(
        panel.click((10.0, 40.0), only(&outline), 1.0),
        None,
        "a hidden panel owns no clicks"
    );
    panel.toggle();

    // Outside it: the page's, and the panel says so by answering nothing.
    assert_eq!(panel.click((300.0, 40.0), only(&outline), 1.0), None);
    // In the tab strip: the first of three tabs, which is the one already showing. Answered
    // rather than passed on — a click falling through would start a text selection on a page
    // nobody can see.
    assert_eq!(
        panel.click((40.0, 8.0), only(&outline), 1.0),
        Some(Hit::Redraw)
    );

    // The first row's title. §12.3.3 asks for the *item* to be activated rather than for its
    // destination to be followed, because the same sentence covers `/A`.
    assert_eq!(
        panel.click((80.0, 36.0), only(&outline), 1.0),
        Some(Hit::Activate(outline.items[0].id))
    );

    // The first row's disclosure triangle, which is the left fourteen pixels.
    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "the triangle discloses rather than navigating"
    );
}

/// Closing a subtree hides it, and does **not** renumber what is below it.
///
/// The identities are what a toggle is keyed by, and the trap is numbering them by *visible*
/// row: close the first chapter and the second would inherit the first's identity, so the next
/// click would toggle the wrong thing. They are numbered in pre-order over every item instead,
/// and this is what says so.
#[test]
fn closing_a_subtree_does_not_renumber_the_items_below_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = outline();
    let mut panel = Sidebar::default();
    panel.toggle();

    let three_rows = ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90);
    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "chapter one closes"
    );
    let two_rows = ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90);
    assert!(
        two_rows < three_rows,
        "the closed subtree is still being drawn: {two_rows} against {three_rows}"
    );

    // Chapter two is now the second visible row, and clicking *it* must reach chapter two
    // rather than the section that was hidden.
    assert_eq!(
        panel.click((80.0, 56.0), only(&outline), 1.0),
        Some(Hit::Activate(outline.items[1].id)),
        "the row below a closed subtree is chapter two"
    );

    assert_eq!(
        panel.click((6.0, 36.0), only(&outline), 1.0),
        Some(Hit::Redraw),
        "chapter one opens again"
    );
    assert_eq!(
        ink(&panel.draw(&chrome, only(&outline), HEIGHT, 1.0), 26..90),
        three_rows,
        "reopening restores exactly what closing hid"
    );
}

/// A title longer than the panel is cut with an ellipsis rather than drawn over the page.
#[test]
fn a_long_title_is_elided_to_the_width_available() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let long = "Annex Q (normative) Method for determining something at very great length";
    let mut panel = Sidebar::default();
    panel.toggle();
    let outline = Outline {
        items: vec![item(long, 10, Vec::new())],
        stated_count: None,
    };
    let list = panel.draw(&chrome, only(&outline), HEIGHT, 1.0);
    // The whole title is wider than the panel, so something must have been dropped — and what
    // is drawn must stay inside it. Measured through the same function that laid it out.
    assert!(
        chrome.width(long, 12.0, viewer_ui::chrome::Style::default()) > 260.0,
        "the fixture is not long enough to elide"
    );
    assert!(ink(&list, 26..46) > 40, "the row has glyphs");
    assert_eq!(
        ink(&list, 26..46),
        ink_within(&list, 26..46, 0..260),
        "a glyph was drawn outside the panel"
    );
}

/// §8.11's layer tab: a switch throws, a locked one does not, and a heading is not a layer.
///
/// The locked case is the clause speaking, not a preference. §8.11.4.3 on Table 99's `/Locked`:
/// "[t]he state of a locked group cannot be changed through the user interface of an interactive
/// PDF processor." So the switch is *drawn* — a person is entitled to see the state — and
/// clicking it produces nothing.
#[test]
fn a_layer_switch_throws_unless_the_document_locked_it() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let layers = vec![
        Layer::Collection {
            label: Some("Plans".to_owned()),
            children: vec![Layer::Group {
                group: ObjectId::new(30, 0),
                name: Some("Walls".to_owned()),
                on: true,
                locked: false,
            }],
        },
        Layer::Group {
            group: ObjectId::new(31, 0),
            name: Some("Watermark".to_owned()),
            on: false,
            locked: true,
        },
    ];
    let outline = Outline::default();
    let content = Content {
        outline: &outline,
        layers: &layers,
        attachments: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    // The second of three tabs.
    assert_eq!(
        panel.click((140.0, 8.0), content, 1.0),
        Some(Hit::Redraw),
        "the Layers tab"
    );

    // Row 0 is the collection's heading; its left edge acts on nothing.
    assert_eq!(panel.click((6.0, 36.0), content, 1.0), Some(Hit::Nothing));
    // Row 1 is the group inside it, indented one level, so its switch is further right.
    assert_eq!(
        panel.click((20.0, 56.0), content, 1.0),
        Some(Hit::SetGroup {
            group: ObjectId::new(30, 0),
            on: false,
        }),
        "an unlocked switch offers the opposite of what is on"
    );
    // Row 2 is the locked group at depth zero.
    assert_eq!(
        panel.click((6.0, 76.0), content, 1.0),
        Some(Hit::Nothing),
        "§8.11.4.3 forbids changing a locked group from the interface"
    );

    // And all three rows are drawn, which is the other half: refusing the click must not mean
    // hiding the state.
    let list = panel.draw(&chrome, content, HEIGHT, 1.0);
    for (row, band) in [(0, 26..46), (1, 46..66), (2, 66..86)] {
        assert!(ink(&list, band) > 20, "row {row} is blank");
    }
}

/// §7.11.4's file tab lists what the document embeds, and says so when it embeds nothing.
///
/// An empty list and a list this program failed to fill look identical on a screen, and only one
/// of them is a fact about the file — so the empty case is a sentence rather than blank space,
/// and this checks there is ink either way.
#[test]
fn the_file_tab_names_what_is_embedded_and_says_when_nothing_is() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let outline = Outline::default();
    let empty = Content {
        outline: &outline,
        layers: &[],
        attachments: &[],
    };
    let mut panel = Sidebar::default();
    panel.toggle();
    assert_eq!(
        panel.click((230.0, 8.0), empty, 1.0),
        Some(Hit::Redraw),
        "the Files tab"
    );
    assert!(
        ink(&panel.draw(&chrome, empty, HEIGHT, 1.0), 26..46) > 40,
        "an empty list must say it is empty"
    );

    // A document that does embed one: the row carries the file's name and a click asks for the
    // bytes.
    let Some(bytes) = corpus_bytes("attachment.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let document = pdf_syntax::Document::open(bytes).expect("a corpus document");
    let files = pdf_model::attachment::attachments(&document);
    assert!(!files.is_empty(), "the fixture embeds a file");
    let listed = Content {
        outline: &outline,
        layers: &[],
        attachments: &files,
    };
    assert!(ink(&panel.draw(&chrome, listed, HEIGHT, 1.0), 26..46) > 40);
    // The row's click is §7.11.4 from a person's side: the bytes are inside the document, and
    // the key is the one the `/EmbeddedFiles` tree filed them under.
    assert_eq!(
        panel.click((80.0, 36.0), listed, 1.0),
        Some(Hit::Extract(files[0].name.clone()))
    );
}

/// A corpus document's bytes, or `None` when the submodule is not checked out.
fn corpus_bytes(name: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    std::fs::read(path).ok()
}
