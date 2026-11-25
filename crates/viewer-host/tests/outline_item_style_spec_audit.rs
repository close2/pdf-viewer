//! Specification audit of ISO 32000-2 §12.3.3 (review 2197): an outline item's colour and style
//! reach the row a host draws.
//!
//! Table 151's `/C` is "the colour that shall be used for the outline entry's text", and §12.3.3
//! says `/F` "shall be an integer interpreted as one-bit flags specifying style characteristics
//! for displaying the item" — Table 152's italic and bold. `pdf_model::outline` reads all three;
//! `viewer_host::outline_rows` is where every host's outline panel is built, and a row built from
//! a red, bold, italic item is the row built from a black, plain one.

use pdf_model::outline::{Item, Outline};
use pdf_syntax::ObjectId;
use viewer_host::outline_rows;

/// One top-level item, closed and childless, styled by `italic`, `bold` and `colour`.
fn item(italic: bool, bold: bool, colour: [f32; 3]) -> Outline {
    Outline {
        items: vec![Item {
            id: ObjectId::new(10, 0),
            title: "Chapter".to_owned(),
            title_languages: Vec::new(),
            destination: None,
            open: false,
            italic,
            bold,
            colour,
            children: Vec::new(),
        }],
        stated_count: None,
        language: None,
    }
}

/// 2197-B04. The control is that the row carries what it does carry — a different title is a
/// different row; the failing half is that Table 151's `/C` and Table 152's flags change nothing.
#[test]
fn an_outline_items_colour_and_style_reach_its_row_spec_audit() {
    let plain = outline_rows(&item(false, false, [0.0, 0.0, 0.0]));
    let mut retitled = item(false, false, [0.0, 0.0, 0.0]);
    retitled.items[0].title = "Appendix".to_owned();
    assert_ne!(
        plain,
        outline_rows(&retitled),
        "control: Table 151's /Title reaches the row"
    );

    let styled = outline_rows(&item(true, true, [1.0, 0.0, 0.0]));
    assert_ne!(
        plain, styled,
        "Table 151's /C [1 0 0] and Table 152's italic and bold flags are the text's colour and \
         style, so a host drawing the row has to be told them"
    );
}
