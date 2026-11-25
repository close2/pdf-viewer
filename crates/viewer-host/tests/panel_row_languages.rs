//! The languages an outline row and a layer row are in: ISO 32000-2 §14.9.2.1's catalog `/Lang`,
//! overridden by §7.9.2.2.2's escape sequences inside the title or name from where each stands.
//!
//! §14.9.2.1 extends the catalog's language past the content streams:
//!
//! > This applies to both content within content streams and any text strings, including text
//! > strings not included in the structure hierarchy such as, for example, entries in metadata,
//! > outline entries and names for optional content groups.
//!
//! and §7.9.2.2.2 puts an escape inside the string over it:
//!
//! > Escape sequences may appear anywhere in a Unicode text string to indicate the language in
//! > which subsequent text shall be written.
//!
//! The fixture's three escaped strings are UTF-8 text strings (§7.9.2.2.1's `EF BB BF` prefix):
//! a group name `"Plan "` then `en` and `"Walls"`, a collection label opening with `en`, and an
//! outline title opening with `fr`.

#![expect(
    clippy::panic,
    reason = "a test fails by panicking on an answer of the wrong shape"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::{PanelRow, layer_rows, outline_rows};

/// A one-page document with an outline of one item and an `/Order` of a group and a labelled
/// collection holding a second, whose catalog states `language` where it is `Some`.
fn document(language: Option<&str>) -> Vec<u8> {
    let lang = language.map_or(String::new(), |tag| format!(" /Lang ({tag})"));
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /Outlines 5 0 R{lang} /OCProperties << \
             /OCGs [7 0 R 8 0 R] /D << /Order [7 0 R [<EFBBBF1B656E1B4C6179657273> 8 0 R]] >> \
             >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Outlines /First 6 0 R /Last 6 0 R /Count 1 >>".to_owned(),
        "<< /Title <EFBBBF1B66721B4368617069747265> /Parent 5 0 R /Dest [3 0 R /Fit] >>".to_owned(),
        "<< /Type /OCG /Name (Grundriss) >>".to_owned(),
        "<< /Type /OCG /Name <EFBBBF506C616E201B656E1B57616C6C73> >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn opened(language: Option<&str>) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: document(language).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn runs(list: &[(usize, &str)]) -> Vec<(usize, String)> {
    list.iter()
        .map(|(start, tag)| (*start, (*tag).to_owned()))
        .collect()
}

/// The layer panel's three rows: the plain name, the heading, and the escaped name under it.
fn layer_panel(language: Option<&str>) -> (PanelRow, PanelRow, PanelRow) {
    let Answer::Layers(layers) = opened(language).query(Query::Layers) else {
        panic!("the catalog states an /Order");
    };
    let rows = layer_rows(&layers);
    let [plain, heading] = rows.as_slice() else {
        panic!("a group and a collection at the top, got {rows:?}");
    };
    let [escaped] = heading.children.as_slice() else {
        panic!("the collection holds one group");
    };
    (plain.clone(), heading.clone(), escaped.clone())
}

/// §14.9.2.1's "names for optional content groups": a layer row carries the catalog's `/Lang`,
/// and a §7.9.2.2.2 escape overrides it from where it stands — the whole label where it opens
/// the string, the bytes after it where it stands inside.
#[test]
fn a_layer_rows_label_is_in_the_documents_language_until_an_escape_says_otherwise() {
    let (plain, heading, escaped) = layer_panel(Some("de-DE"));
    assert_eq!(plain.label, "Grundriss");
    assert_eq!(plain.languages, runs(&[(0, "de-DE")]));
    assert_eq!(heading.label, "Layers");
    assert_eq!(heading.languages, runs(&[(0, "en")]));
    assert_eq!(escaped.label, "Plan Walls");
    assert_eq!(escaped.languages, runs(&[(0, "de-DE"), (5, "en")]));
}

/// The control: a document that states no language leaves a plain name with none, and the
/// escapes still say theirs.
#[test]
fn a_layer_row_in_a_document_with_no_language_carries_only_its_escapes() {
    let (plain, heading, escaped) = layer_panel(None);
    assert_eq!(plain.languages, runs(&[]));
    assert_eq!(heading.languages, runs(&[(0, "en")]));
    assert_eq!(escaped.languages, runs(&[(5, "en")]));
}

/// §7.9.2.2.2 in an outline title: the escape that opens the title is its language, over the
/// catalog's.
#[test]
fn an_outline_titles_own_escape_overrides_the_documents_language() {
    let Answer::Outline(outline) = opened(Some("de-DE")).query(Query::Outline) else {
        panic!("the catalog states an outline");
    };
    let rows = outline_rows(&outline);
    let [row] = rows.as_slice() else {
        panic!("one item, got {rows:?}");
    };
    assert_eq!(row.label, "Chapitre");
    assert_eq!(row.languages, runs(&[(0, "fr")]));
}
