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
use viewer_ui::chrome::{Chrome, Hit, Panel};

/// A window this test pretends to have: wide enough for the panel, tall enough for the rows.
const WIDTH: u32 = 400;
/// How tall the pretend window is.
const HEIGHT: u32 = 300;

/// An item with a destination naming an object, which is §12.3.2.2's first form.
fn item(title: &str, page: u32, children: Vec<Item>) -> Item {
    Item {
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
/// The background is a light grey and the text is black, so "ink" is anything well below it.
/// Counting rather than comparing to a golden image: a golden would pin the shape of Liberation
/// Sans, and what is being checked is that glyphs reached the raster at all.
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
            if raster.data.get(at).is_some_and(|red| *red < 100) {
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
    let mut panel = Panel::default();

    let hidden = panel.draw(&chrome, &outline, HEIGHT, 1.0);
    assert_eq!(ink(&hidden, 0..HEIGHT), 0, "a hidden panel draws nothing");

    panel.toggle();
    let shown = panel.draw(&chrome, &outline, HEIGHT, 1.0);
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
    let mut panel = Panel::default();
    assert_eq!(
        panel.click((10.0, 40.0), &outline, 1.0),
        None,
        "a hidden panel owns no clicks"
    );
    panel.toggle();

    // Outside it: the page's, and the panel says so by answering nothing.
    assert_eq!(panel.click((300.0, 40.0), &outline, 1.0), None);
    // In the heading: inside the panel, on nothing that acts. Swallowed rather than passed on,
    // because a click falling through would start a selection on a page nobody can see.
    assert_eq!(panel.click((40.0, 8.0), &outline, 1.0), Some(Hit::Nothing));

    // The first row's title.
    let followed = panel.click((80.0, 36.0), &outline, 1.0);
    let Some(Hit::Follow(target)) = followed else {
        panic!("a click on a title follows its destination, not {followed:?}");
    };
    assert_eq!(
        target,
        viewer_core::PageTarget::Destination(
            outline.items[0]
                .destination
                .expect("the fixture states one")
        )
    );

    // The first row's disclosure triangle, which is the left fourteen pixels.
    assert_eq!(
        panel.click((6.0, 36.0), &outline, 1.0),
        Some(Hit::Toggle),
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
    let mut panel = Panel::default();
    panel.toggle();

    let three_rows = ink(&panel.draw(&chrome, &outline, HEIGHT, 1.0), 26..90);
    assert_eq!(
        panel.click((6.0, 36.0), &outline, 1.0),
        Some(Hit::Toggle),
        "chapter one closes"
    );
    let two_rows = ink(&panel.draw(&chrome, &outline, HEIGHT, 1.0), 26..90);
    assert!(
        two_rows < three_rows,
        "the closed subtree is still being drawn: {two_rows} against {three_rows}"
    );

    // Chapter two is now the second visible row, and clicking *it* must reach chapter two's
    // destination rather than the section's.
    let followed = panel.click((80.0, 56.0), &outline, 1.0);
    let Some(Hit::Follow(target)) = followed else {
        panic!("the second visible row follows nothing: {followed:?}");
    };
    assert_eq!(
        target,
        viewer_core::PageTarget::Destination(
            outline.items[1]
                .destination
                .expect("the fixture states one")
        ),
        "the row below a closed subtree is chapter two"
    );

    assert_eq!(
        panel.click((6.0, 36.0), &outline, 1.0),
        Some(Hit::Toggle),
        "chapter one opens again"
    );
    assert_eq!(
        ink(&panel.draw(&chrome, &outline, HEIGHT, 1.0), 26..90),
        three_rows,
        "reopening restores exactly what closing hid"
    );
}

/// A title longer than the panel is cut with an ellipsis rather than drawn over the page.
#[test]
fn a_long_title_is_elided_to_the_width_available() {
    let chrome = Chrome::new().expect("§9.6.2.2's fourteen are compiled in");
    let long = "Annex Q (normative) Method for determining something at very great length";
    let mut panel = Panel::default();
    panel.toggle();
    let outline = Outline {
        items: vec![item(long, 10, Vec::new())],
        stated_count: None,
    };
    let list = panel.draw(&chrome, &outline, HEIGHT, 1.0);
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
