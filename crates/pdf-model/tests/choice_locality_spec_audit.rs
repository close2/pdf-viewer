//! Table 234's /TI and /I belong to the terminal choice field. A separate widget's /Parent hop
//! must reach that field, without turning the values into inheritance from a nonterminal group.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "small exact-xref document and public field-reader assertions"
)]

use std::fmt::Write as _;

use pdf_model::form::{ChoiceControl, Control, fields};
use pdf_model::view::ViewState;
use pdf_syntax::Document;

fn pdf(
    parent_top: Option<u8>,
    parent_selection: Option<u8>,
    terminal_top: Option<u8>,
    terminal_selection: Option<u8>,
) -> Document {
    let parent_top = parent_top.map_or(String::new(), |value| format!(" /TI {value}"));
    let parent_selection = parent_selection.map_or(String::new(), |value| format!(" /I [{value}]"));
    let terminal_top = terminal_top.map_or(String::new(), |value| format!(" /TI {value}"));
    let terminal_selection =
        terminal_selection.map_or(String::new(), |value| format!(" /I [{value}]"));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [8 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 5 0 R /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 7 0 R /P 3 0 R /Rect [20 20 280 60] /F 4 >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        format!(
            "<< /FT /Ch /Ff 2097152 /T (choice) /Parent 8 0 R /Kids [4 0 R] \
             /Opt [[(c0)(Twin)] [(c1)(Twin)]] /V (Twin){terminal_top}{terminal_selection} >>"
        ),
        format!(
            "<< /FT /Ch /Ff 2097152 /T (group) /Kids [7 0 R] \
             /Opt [[(p0)(Twin)] [(p1)(Twin)]] /V (Twin){parent_top}{parent_selection} >>"
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 9\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    Document::open(bytes.into_bytes()).expect("complete PDF with exact xref")
}

fn choice(
    parent_top: Option<u8>,
    parent_selection: Option<u8>,
    terminal_top: Option<u8>,
    terminal_selection: Option<u8>,
) -> ChoiceControl {
    let document = pdf(
        parent_top,
        parent_selection,
        terminal_top,
        terminal_selection,
    );
    assert!(!document.was_recovered(), "the exact xref is authoritative");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let view = ViewState::of(&document);
    let fields = fields(&document, &page, &view);
    assert_eq!(fields.len(), 1, "one terminal choice field: {fields:?}");
    let Control::Choice(choice) = fields.into_iter().next().expect("one field").control else {
        panic!("expected a choice field")
    };
    choice
}

#[test]
fn top_index_is_taken_from_terminal_through_widget_but_not_from_nonterminal_ancestor() {
    assert_eq!(choice(None, None, None, None).top, 0, "Table 234 default");
    assert_eq!(
        choice(Some(0), None, Some(1), None).top,
        1,
        "the separate widget reaches its own terminal field's /TI"
    );
    assert_eq!(
        choice(Some(1), None, None, None).top,
        0,
        "a group-level /TI must not replace the terminal list's default of 0"
    );
}

#[test]
fn selected_index_is_unchanged_by_nonterminal_ancestor_i_and_terminal_i_remains_local() {
    let local_i = choice(None, None, None, Some(1));
    let local_i_with_conflicting_ancestor = choice(None, Some(0), None, Some(1));
    assert_eq!(local_i.selected, vec![1], "terminal /I positive control");
    assert_eq!(
        local_i_with_conflicting_ancestor.selected, local_i.selected,
        "terminal /I is read through the separate widget and wins over a group-level /I"
    );

    let no_ancestor_i = choice(None, None, None, None);
    let ancestor_i = choice(None, Some(1), None, None);
    assert_eq!(
        ancestor_i.selected, no_ancestor_i.selected,
        "identical terminal /Opt and /V must have the same interpretation without ancestor /I"
    );
}

fn options_pdf(parent_opt: bool, terminal_opt: bool) -> Document {
    let parent_opt = if parent_opt {
        " /Opt [(Parent first) (Parent second)]"
    } else {
        ""
    };
    let terminal_opt = if terminal_opt {
        " /Opt [(Child first) (Child second)] /V (Child second)"
    } else {
        ""
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [8 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 5 0 R /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 7 0 R /P 3 0 R /Rect [20 20 280 60] /F 4 >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        format!("<< /FT /Ch /T (choice) /Parent 8 0 R /Kids [4 0 R]{terminal_opt} >>"),
        format!("<< /FT /Ch /T (group) /Kids [7 0 R]{parent_opt} >>"),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 9\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    Document::open(bytes.into_bytes()).expect("complete PDF with exact xref")
}

fn labels(parent_opt: bool, terminal_opt: bool) -> Vec<String> {
    let document = options_pdf(parent_opt, terminal_opt);
    assert!(!document.was_recovered(), "the exact xref is authoritative");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let view = ViewState::of(&document);
    let fields = fields(&document, &page, &view);
    assert_eq!(fields.len(), 1, "one terminal choice field: {fields:?}");
    let Control::Choice(choice) = &fields[0].control else {
        panic!("expected a choice field")
    };
    choice
        .options
        .iter()
        .map(|option| option.label.clone())
        .collect()
}

#[test]
fn choice_opt_stays_with_the_dictionary_that_states_it() {
    let absent = labels(false, false);
    assert!(absent.is_empty(), "no terminal Opt baseline");
    let own = labels(true, true);
    assert_eq!(
        own,
        ["Child first", "Child second"],
        "terminal Opt positive control"
    );
    let inherited = labels(true, false);
    assert_eq!(
        inherited, absent,
        "Table 234 leaves Opt optional but does not designate it inheritable; its absence recommends presenting no choices"
    );
}
