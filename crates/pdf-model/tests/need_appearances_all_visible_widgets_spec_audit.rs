//! Table 224's flag applies to all visible widgets. The no-AP source has the required flag true;
//! the control source provides AP for every visible widget with the flag false. PDF 1.7 is used
//! because the note makes AP mandatory for all widgets only in PDF 2.0 and later.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref PDF fixtures with explanatory failures"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_syntax::{Document, Object, ObjectId};

fn pdf(untouched_has_ap: bool) -> Document {
    let flag = if untouched_has_ap { "false" } else { "true" };
    let old = "BT /Helv 10 Tf 1 1 Td (old) Tj ET";
    let kept = "BT /Helv 10 Tf 1 1 Td (kept) Tj ET";
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] /NeedAppearances {flag} /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 8 0 R >> >> >> >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [5 0 R 7 0 R] >>".to_owned(),
        "<< /FT /Tx /T (edited) /V (old) /Kids [5 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /P 3 0 R /Rect [10 10 90 30] /F 4 /AP << /N 9 0 R >> >>".to_owned(),
        "<< /FT /Tx /T (untouched) /V (kept) /Kids [7 0 R] >>".to_owned(),
        if untouched_has_ap {
            "<< /Type /Annot /Subtype /Widget /Parent 6 0 R /P 3 0 R /Rect [100 10 190 30] /F 4 /AP << /N 10 0 R >> >>".to_owned()
        } else {
            "<< /Type /Annot /Subtype /Widget /Parent 6 0 R /P 3 0 R /Rect [100 10 190 30] /F 4 >>".to_owned()
        },
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 80 20] /Resources << /Font << /Helv 8 0 R >> >> /Length {} >>\nstream\n{old}\nendstream", old.len()),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 80 20] /Resources << /Font << /Helv 8 0 R >> >> /Length {} >>\nstream\n{kept}\nendstream", kept.len()),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        offsets.len() + 1
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    Document::open(bytes.into_bytes()).expect("complete PDF with exact xref")
}

fn saved(untouched_has_ap: bool) -> Document {
    let source = pdf(untouched_has_ap);
    assert!(!source.was_recovered(), "source xref is exact");
    let mut view = ViewState::of(&source);
    assert!(view.set_field(&source, "edited", &Entered::Text("changed".to_owned())) > 0);
    let written = view.save(&source).expect("the changed field saves");
    let output = Document::open(written.bytes).expect("saved update reads back");
    assert!(!output.was_recovered(), "saved update xref is exact");
    assert_eq!(
        output.get_key(
            output
                .get(ObjectId::new(4, 0))
                .as_dict()
                .expect("edited field"),
            "V"
        ),
        Object::String(b"changed".to_vec().into()),
        "the edit was actually serialized"
    );
    assert_eq!(
        output.get_key(
            output
                .get(ObjectId::new(6, 0))
                .as_dict()
                .expect("untouched field"),
            "V"
        ),
        Object::String(b"kept".to_vec().into()),
        "untouched field value stays intact"
    );
    output
}

fn form(document: &Document) -> pdf_syntax::Dictionary {
    let catalog = document.catalog().expect("catalog");
    document
        .get_key(&catalog, "AcroForm")
        .as_dict()
        .cloned()
        .expect("form")
}

fn assert_normal_appearance(document: &Document, widget: u32) {
    let widget = document.get(ObjectId::new(widget, 0));
    let appearance = document.get_key(widget.as_dict().expect("widget"), "AP");
    let normal = document.get_key(appearance.as_dict().expect("appearance dictionary"), "N");
    assert!(
        normal.as_stream().is_some(),
        "text widget has a normal appearance stream"
    );
}

#[test]
fn need_appearances_true_is_retained_for_an_untouched_visible_widget_without_ap() {
    let saved = saved(false);
    let untouched = saved.get(ObjectId::new(7, 0));
    let appearance = saved.get_key(untouched.as_dict().expect("untouched widget"), "AP");
    if appearance.is_null() {
        assert_eq!(
            saved.get_key(&form(&saved), "NeedAppearances"),
            Object::Boolean(true),
            "remaining missing appearance requires the document-wide flag"
        );
    } else {
        // Regenerating the missing appearance also satisfies the writer's duty.
        assert_normal_appearance(&saved, 7);
    }
    assert_normal_appearance(&saved, 5);
}

#[test]
fn need_appearances_is_not_set_when_every_visible_widget_has_an_appearance() {
    let saved = saved(true);
    assert_ne!(
        saved.get_key(&form(&saved), "NeedAppearances"),
        Object::Boolean(true)
    );
    for id in [5, 7] {
        assert_normal_appearance(&saved, id);
    }
}
