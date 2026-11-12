//! A save names its subsets against the names the file already holds (ISO 32000-2 §9.9.2, ADR
//! 2832).
//!
//! A value only a face from this machine can set is saved with that face subset and embedded
//! (ADR 1425), and the subset's six-letter tag is a digest. §9.9.2 requires "different subsets of
//! the same font in the same PDF file" to have different tags, and an incremental update keeps
//! every font the file already held — so a save into a file that already states the name its
//! subset would be given must give it another. The test saves once to learn the name, writes a
//! file that already states it, and saves the same value there.
//!
//! Its own binary, beside `machine_face_written.rs`, because it asks the machine for a face; where
//! the machine offers none covering the value, it says so and stops (ADR 1154).

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_syntax::{Document, Object};

/// The value typed: Arabic, which no compiled-in face draws, so the layout asks the machine.
const TYPED: &str = "\u{633}\u{644}\u{627}\u{645}";

/// One page with one text field, and `extra` objects after it, numbered from 7.
fn one_field(extra: &[String]) -> Vec<u8> {
    let mut body = String::from(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] \
         /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (field) /P 3 0 R /DA (/HeBo 12 Tf 0 g) >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n",
    );
    for (number, object) in (7..).zip(extra) {
        let _ = write!(body, "{number} 0 obj\n{object}\nendobj\n");
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

/// Every Type 0 font's `/BaseFont` that bears §9.9.2's tag — "exactly six uppercase letters" and
/// a plus sign — wherever the objects a file's cross-reference information names hold one, a
/// form's direct `/Resources` included.
fn subset_names(bytes: Vec<u8>) -> BTreeSet<Vec<u8>> {
    fn visit(object: &Object, names: &mut BTreeSet<Vec<u8>>) {
        let dict = match object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            Object::Array(items) => {
                for item in items {
                    visit(item, names);
                }
                return;
            }
            _ => return,
        };
        let type0 = dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|subtype| subtype.as_bytes() == b"Type0");
        if let Some(name) = dict.get("BaseFont").and_then(Object::as_name)
            && type0
        {
            let name = name.as_bytes();
            if name.get(6) == Some(&b'+') && name.iter().take(6).all(u8::is_ascii_uppercase) {
                names.insert(name.to_vec());
            }
        }
        for (_, value) in dict.iter() {
            visit(value, names);
        }
    }
    let document = Document::open(bytes).expect("the file re-opens");
    let mut names = BTreeSet::new();
    for number in document.xref().object_numbers() {
        visit(&document.get(document.defined_id(number)), &mut names);
    }
    names
}

/// The value typed into `file`'s field and saved: the subset names the saved file states that
/// `file` did not.
fn saved_names(file: Vec<u8>) -> BTreeSet<Vec<u8>> {
    let before = subset_names(file.clone());
    let document = Document::open(file).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text(TYPED.to_owned())),
        1
    );
    let written = view.save(&document).expect("the fixture can be written");
    assert!(
        written.unconstructed.is_empty(),
        "{:?}",
        written.unconstructed
    );
    subset_names(written.bytes)
        .difference(&before)
        .cloned()
        .collect()
}

/// A save into a file that already states the name the save's subset would carry gives the
/// subset another tag of the same shape, naming the same face.
///
/// §9.9.2: "the choice of letters is arbitrary, but different subsets of the same font in the
/// same PDF file shall have different tags". The font the file states is a different font
/// object from the one the save writes, and no reader can tell two subsets under one name apart;
/// so the clause's "same PDF file" is the whole file the save leaves, the earlier font included.
/// The control is the first save, into the file without it, which takes the name.
#[test]
fn a_save_does_not_reuse_a_subset_name_the_file_already_holds() {
    let fresh = saved_names(one_field(&[]));
    let Some(taken) = fresh.first().cloned() else {
        println!("skipped: no face on this machine covers the Arabic value");
        return;
    };
    assert_eq!(fresh.len(), 1, "one value, one subset: {fresh:?}");

    let name = String::from_utf8(taken.clone()).expect("a font name the writer wrote is ASCII");
    let earlier =
        format!("<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H >>");
    let resaved = saved_names(one_field(&[earlier]));
    assert_eq!(resaved.len(), 1, "{resaved:?}");
    let written = resaved.first().expect("one name");
    assert_ne!(*written, taken, "the name the file holds is passed over");
    assert_eq!(written.get(6..), taken.get(6..), "the same face");
    assert!(written.iter().take(6).all(u8::is_ascii_uppercase));
}
