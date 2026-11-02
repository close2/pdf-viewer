//! The reader's substitute for text under a `/Font` entry that is null, at both of its answers.
//!
//! ISO 32000-2 §9.3.1's Table 103 makes `Tf`'s operand "the name of a font resource in the Font
//! subdictionary of the current resource dictionary", and §7.3.9 makes an entry there that refers
//! to an object the file does not define the null object: "An indirect object reference (see
//! 7.3.10, "Indirect objects") to a nonexistent object shall be treated the same as a null
//! object." There is no font dictionary, so §9.5 has nothing to choose a substitute from, and
//! whether the text is drawn at all is the reader's to say (A35, A738, ADR 2612). These tests hold
//! the two answers apart: by default the text draws nothing, and under the setting it draws in
//! §9.6.2.2's Helvetica at that face's own advances, the page's report naming the entry either
//! way.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the fixture's lengths and \
              offsets are a few hundred bytes, where no sum can overflow"
)]

use std::fmt::Write as _;

use pdf_model::view::ViewState;
use pdf_render::Command;
use pdf_syntax::Document;

/// The sentence the interpreter says for `/F1`, at both answers.
const NULL_ENTRY: &str = "the /Font entry F1 is stated and is not a font dictionary";

/// What the setting adds to it.
const SUBSTITUTED: &str = "its text is drawn in Helvetica at that face's own advances";

/// A one-page document whose `/F1` refers to object 9, which the file does not define, whose `/F2`
/// is an integer, and whose `/Null` graphics state selects object 9 through Table 57's `/Font`.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 9 0 R /F2 5 0 R /F3 6 0 R >> \
         /ExtGState << /Null 7 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n12\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         7 0 obj\n<< /Type /ExtGState /Font [9 0 R 24] >>\nendobj\n",
        content.len() + 1,
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

/// Page one interpreted with the substitute asked for or not.
fn interpreted(content: &str, substitute: bool) -> pdf_model::content::Interpretation {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = ViewState::of(&document);
    assert_eq!(
        state.set_null_font_substitute(substitute),
        substitute,
        "the answer moves only when it differs from the default, which is off"
    );
    pdf_model::content::interpret_with(&document, &page, &state)
}

/// The x translation of every fill the page draws, in painting order.
fn fills(interpretation: &pdf_model::content::Interpretation) -> Vec<f32> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some(transform.e),
            _ => None,
        })
        .collect()
}

/// Every report the page carries, as one string.
fn reports(interpretation: &pdf_model::content::Interpretation) -> String {
    format!("{:?}", interpretation.unsupported)
}

/// Off, which is every host that says nothing: the text draws nothing and the page says why.
#[test]
fn by_default_text_under_a_null_entry_draws_nothing_and_is_reported() {
    let drawn = interpreted("BT /F1 24 Tf 10 10 Td (AA) Tj ET", false);
    assert!(fills(&drawn).is_empty(), "no face was asked for");
    let said = reports(&drawn);
    assert!(
        said.contains(NULL_ENTRY),
        "§7.3.9's absence is said: {said}"
    );
    assert!(
        !said.contains(SUBSTITUTED),
        "nothing was substituted: {said}"
    );
}

/// On: the substitute draws each glyph at Helvetica's own advance, and the report is kept.
///
/// `Helvetica.afm` gives `A` a width of 667, so at 24 points §9.4.4's displacement between two
/// `A`s is 667 / 1000 × 24 = 16.008 text-space units, which on this page's identity matrices is
/// the distance between the two fills.
#[test]
fn under_the_setting_the_text_draws_in_the_substitute_at_its_own_advances_and_is_reported() {
    let drawn = interpreted("BT /F1 24 Tf 10 10 Td (AA) Tj ET", true);
    let at = fills(&drawn);
    assert_eq!(at.len(), 2, "one fill a glyph: {at:?}");
    assert!(
        (at[1] - at[0] - 16.008).abs() < 1e-3,
        "the second A stands one Helvetica advance after the first: {at:?}"
    );
    assert_eq!(
        drawn.text.trim_end(),
        "AA",
        "the substitute's text is read too"
    );
    let said = reports(&drawn);
    assert!(
        said.contains(NULL_ENTRY),
        "the entry is still named: {said}"
    );
    assert!(said.contains(SUBSTITUTED), "and the guess is said: {said}");
}

/// Table 57's `/Font` reaches the same nothing by its object, and the setting answers it alike.
#[test]
fn the_graphics_states_font_entry_takes_the_same_answer() {
    let content = "BT /Null gs 10 10 Td (A) Tj ET";
    let off = interpreted(content, false);
    assert!(fills(&off).is_empty());
    assert!(reports(&off).contains("the /Font entry object 9 0 is stated"));
    let on = interpreted(content, true);
    assert_eq!(fills(&on).len(), 1);
    assert!(reports(&on).contains(SUBSTITUTED), "{}", reports(&on));
}

/// Only the null object is substituted: an entry that names something which is there and is not
/// a font dictionary is another fault, and the setting does not reach it.
#[test]
fn an_entry_that_is_not_null_is_not_substituted() {
    let drawn = interpreted("BT /F2 24 Tf 10 10 Td (A) Tj ET", true);
    assert!(fills(&drawn).is_empty(), "an integer is not a null entry");
    let said = reports(&drawn);
    assert!(said.contains("the /Font entry F2 is stated"), "{said}");
    assert!(!said.contains(SUBSTITUTED), "{said}");
}

/// A page whose fonts are all there draws and reports the same at both answers.
#[test]
fn a_page_with_every_font_present_is_the_same_at_both_answers() {
    let content = "BT /F3 24 Tf 10 10 Td (AA) Tj /F3 24 Tf (A) Tj ET";
    let off = interpreted(content, false);
    let on = interpreted(content, true);
    assert_eq!(fills(&off), fills(&on));
    assert_eq!(reports(&off), reports(&on));
    assert!(off.unsupported.is_empty(), "{}", reports(&off));
}

/// A page that selects the name on every line loads the face once and says the sentence once.
#[test]
fn a_name_selected_on_every_line_is_reported_once() {
    let mut content = String::from("BT ");
    for line in 0..200 {
        let _ = write!(content, "/F1 12 Tf 10 {} Td (A) Tj ", line % 5);
    }
    content.push_str("ET");
    let drawn = interpreted(&content, true);
    assert_eq!(fills(&drawn).len(), 200);
    assert_eq!(reports(&drawn).matches(NULL_ENTRY).count(), 1);
}
