//! A valid PDF 2.0 folder hierarchy reaches the same structure through public host rows.

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::{PanelRow, RowAction, collection_rows};

fn pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(<2>a.txt) 4 0 R (<3>b.txt) 5 0 R (<4>c.txt) 6 0 R] >> >> /Collection 10 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Filespec /F (a.txt) /UF (a.txt) /EF << /F 7 0 R >> >>".to_owned(),
        "<< /Type /Filespec /F (b.txt) /UF (b.txt) /EF << /F 8 0 R >> >>".to_owned(),
        "<< /Type /Filespec /F (c.txt) /UF (c.txt) /EF << /F 9 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 5 >>\nstream\nalpha\nendstream".to_owned(),
        "<< /Type /EmbeddedFile /Length 4 >>\nstream\nbeta\nendstream".to_owned(),
        "<< /Type /EmbeddedFile /Length 5 >>\nstream\ngamma\nendstream".to_owned(),
        "<< /Type /Collection /Folders 11 0 R >>".to_owned(),
        // Root has no /Parent; the links below satisfy Table 159 for every non-root folder.
        "<< /Type /Folder /ID 1 /Name (Archive) /Child 12 0 R >>".to_owned(),
        "<< /Type /Folder /ID 2 /Name (Alpha) /Parent 11 0 R /Child 14 0 R /Next 13 0 R >>".to_owned(),
        "<< /Type /Folder /ID 3 /Name (Beta) /Parent 11 0 R >>".to_owned(),
        "<< /Type /Folder /ID 4 /Name (Year) /Parent 12 0 R >>".to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = write!(out, "{number} 0 obj\n{body}\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0123456789ABCDEF0123456789ABCDEF> <0123456789ABCDEF0123456789ABCDEF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn extracted(row: &PanelRow, key: &str) -> bool {
    matches!(&row.action, RowAction::Extract { name } if name.as_str() == key)
}

#[test]
fn valid_parent_child_next_links_reach_the_expected_public_folder_rows() {
    let bytes = pdf();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref PDF 2.0 opens");
    assert!(!parsed.was_recovered(), "no-recovery guard");
    assert!(
        !parsed.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert_eq!(pdf_model::Pages::new(&parsed).len(), 1, "page-tree control");

    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Collection {
        collection,
        initial,
        order,
    } = viewer.query(Query::Collection)
    else {
        panic!("the catalog collection is available to the public consumer");
    };
    let Answer::Attachments(attachments) = viewer.query(Query::Attachments) else {
        panic!("all three embedded files are available before folder placement");
    };
    assert_eq!(attachments.len(), 3);

    let rows = collection_rows(&collection, &initial, &order, &attachments);
    assert_eq!(rows.len(), 1, "the rootless file population is empty");
    assert_eq!(rows[0].label, "Archive");
    assert_eq!(
        rows[0]
            .children
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        ["Alpha", "Beta"],
        "Child names the first sibling; Next names the next sibling"
    );

    let alpha = &rows[0].children[0];
    let beta = &rows[0].children[1];
    let alpha_file = alpha
        .children
        .iter()
        .find(|row| extracted(row, "<2>a.txt"))
        .expect("the ID 2 key is placed under the ID 2 folder");
    assert_eq!(
        alpha_file.label, "a.txt",
        "the user-facing name omits the folder tag"
    );
    let year = alpha
        .children
        .iter()
        .find(|row| row.label == "Year")
        .expect("Alpha's Child is nested below Alpha");
    let nested_file = year
        .children
        .iter()
        .find(|row| extracted(row, "<4>c.txt"))
        .expect("the ID 4 key is nested under Year");
    assert_eq!(nested_file.label, "c.txt");
    let beta_file = beta
        .children
        .iter()
        .find(|row| extracted(row, "<3>b.txt"))
        .expect("the ID 3 key is placed under the ID 3 folder");
    assert_eq!(beta_file.label, "b.txt");
}
