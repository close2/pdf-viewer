//! An embedded file this target does not admit, laid out as pages where this reader can lay it
//! out, and each file its own answer (`doc/adr/2536`).
//!
//! ISO 19005-2 section 6.8 admits an embedded file only where it conforms to a part of ISO 19005,
//! so a text file and a binary one both fail `embedded-files/embedded-file-is-itself-pdfa`. One row
//! answers both: `preserve` by appended page, under `doc/adr/1014`'s permission (a page of content
//! the file holds), with `on-failure = "discard"` for the file no page can carry. The outcome is
//! per object, so the report, its JSON and `xmpMM:History` name each file with its own answer.

use super::*;
use pdf_transform::archive::{AttachmentAnswer, AttachmentOutcome};

/// The row every test here answers the site with, `on-failure` given.
fn appended(on_failure: &str) -> String {
    format!(
        "[site.\"embedded-files/embedded-file-is-itself-pdfa\"]\nremedy = \"preserve\"\n\
         placement = \"append\"\non-failure = \"{on_failure}\"\n"
    )
}

/// A PDF/A-2 page painting in `DeviceRGB` whose name tree holds the given files, each `(name, bytes)`.
///
/// Objects 6 and up: each file is a §7.11.3 file specification followed by its §7.11.4 embedded
/// file stream, so file `n` is specification `6 + 2n` and stream `7 + 2n`. None of the files is a
/// PDF, so each fails ISO 19005-2 section 6.8.
pub(super) fn a_document_holding(files: &[(&str, &[u8])]) -> Vec<u8> {
    a_document_painting(files, b"1 0 0 rg 10 10 50 50 re f")
}

/// The same, with the page's content stream given.
fn a_document_painting(files: &[(&str, &[u8])], contents: &[u8]) -> Vec<u8> {
    let mut names = String::new();
    let mut binary = Vec::new();
    for (index, (name, data)) in files.iter().enumerate() {
        let specification = index.saturating_mul(2).saturating_add(6);
        let _ = write!(names, "({name}) {specification} 0 R ");
        binary.push(
            format!(
                "<< /Type /Filespec /F ({name}) /UF ({name}) /EF << /F {} 0 R >> >>",
                specification.saturating_add(1)
            )
            .into_bytes(),
        );
        binary.push(stream(
            &format!(
                "/Type /EmbeddedFile /Subtype /application#2Foctet-stream /Length {}",
                data.len()
            ),
            data,
        ));
    }
    Conforming {
        catalog: format!("/Names << /EmbeddedFiles << /Names [{names}] >> >>"),
        binary_objects: binary,
        // DeviceRGB, so that the conversion gives the file the shipped output intent a page of
        // text needs (Table 51's initial DeviceGray, ISO 19005-2 section 6.2.4.3).
        contents: Some((String::new(), contents.to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// The text file the tests lay out, and the binary one no page can carry.
pub(super) const NOTE: (&str, &[u8]) = ("note.txt", b"Kept as a page of this document.\n");
pub(super) const BLOB: (&str, &[u8]) = ("blob.bin", b"\x00\x01\x02\xff\xfe binary");

pub(super) fn pages_of(output: &[u8]) -> usize {
    let document = Document::open(output.to_vec()).expect("the output opens");
    pdf_model::Pages::new(&document).len()
}

#[test]
fn two_attachments_take_two_answers_and_each_is_named() {
    let target = Target::Two(Level::B);
    let source = a_document_holding(&[NOTE, BLOB]);
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("discard"), target));
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| {
        panic!(
            "both files are answered, so the conversion writes: {:?}",
            conversion.decided
        )
    });
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{:?}",
        conversion.decided
    );
    let outcomes = &conversion.attachments;
    assert_eq!(outcomes.len(), 2, "one outcome per file: {outcomes:?}");
    let note = outcomes
        .iter()
        .find(|outcome| outcome.object.number == 6)
        .expect("the text file's outcome");
    let AttachmentAnswer::AppendedAsPages(pages) = &note.answer else {
        panic!("the text file is laid out as pages: {note:?}");
    };
    assert_eq!(pages, &vec![1], "on the page after the document's own one");
    assert_eq!(note.loss(), "attachment-as-pages");
    let blob = outcomes
        .iter()
        .find(|outcome| outcome.object.number == 8)
        .expect("the binary file's outcome");
    assert!(
        matches!(&blob.answer, AttachmentAnswer::Removed(because) if because.contains("not UTF-8")),
        "the binary file is removed and says why: {blob:?}"
    );
    assert_eq!(blob.loss(), "embedded-file");
    // The source's one page and the note's one appended page: the pages a source states,
    // counted against the pages written.
    assert_eq!(pages_of(&source), 1);
    assert_eq!(pages_of(&output), 2);
    // Both answers reach the JSON and the file's own history, each by name.
    let json = report.to_json().render();
    assert!(json.contains("\"answer\": \"appended-as-pages\""), "{json}");
    assert!(json.contains("\"answer\": \"removed\""), "{json}");
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("the embedded file note.txt (object 6 0) is laid out on page(s) 2"),
        "xmpMM:History names the laid-out file"
    );
    assert!(
        text.contains("the embedded file blob.bin (object 8 0) is removed (embedded-file)"),
        "xmpMM:History names the removed file"
    );
}

#[test]
fn a_file_no_page_can_carry_stops_a_row_whose_on_failure_is_stop() {
    let target = Target::Two(Level::B);
    let source = a_document_holding(&[NOTE, BLOB]);
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("stop"), target));
    assert!(output.is_none(), "the binary file stops the conversion");
    let refused = decision(&report, "embedded-files/embedded-file-is-itself-pdfa");
    assert!(
        matches!(refused, Decision::Refused(_)),
        "the site is refused with the sentence naming on-failure: {refused:?}"
    );
}

#[test]
fn every_text_file_is_laid_out_and_each_is_its_own_page() {
    let target = Target::Two(Level::B);
    let second: (&str, &[u8]) = ("second.txt", b"A second file.\n");
    let source = a_document_holding(&[NOTE, second]);
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("stop"), target));
    let output = output.expect("both files are text, so stop is never reached");
    let pages: Vec<Vec<usize>> = conversion(&report)
        .attachments
        .iter()
        .map(|outcome| match &outcome.answer {
            AttachmentAnswer::AppendedAsPages(pages) => pages.clone(),
            AttachmentAnswer::Removed(because) => panic!("{because}"),
        })
        .collect();
    assert_eq!(pages, vec![vec![1], vec![2]]);
    assert_eq!(pages_of(&output), 3);
}

#[test]
fn the_seventeenth_file_takes_on_failure_rather_than_a_layout() {
    // The bound is checked before any file is decoded: sixteen are laid out and the rest take
    // the row's on-failure, each named.
    let target = Target::Two(Level::B);
    let names: Vec<String> = (0..17).map(|index| format!("file{index:02}.txt")).collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .map(|name| (name.as_str(), b"one line\n".as_slice()))
        .collect();
    let source = a_document_holding(&files);
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("discard"), target));
    let output = output.expect("the seventeenth is discarded, so the conversion writes");
    let outcomes = &conversion(&report).attachments;
    assert_eq!(outcomes.len(), 17);
    let removed: Vec<&AttachmentOutcome> = outcomes
        .iter()
        .filter(|outcome| matches!(outcome.answer, AttachmentAnswer::Removed(_)))
        .collect();
    assert_eq!(removed.len(), 1, "{outcomes:?}");
    assert!(
        matches!(&removed[0].answer, AttachmentAnswer::Removed(because)
            if because.contains("MOST_ATTACHMENTS_AS_PAGES"))
    );
    assert_eq!(pages_of(&output), 17);
}

#[test]
fn a_file_whose_page_cannot_be_composed_takes_on_failure_too() {
    // A page that paints nothing in a device space gets no output intent from this conversion, and
    // a page of text needs one (Table 51's initial DeviceGray), so the composition refuses. With
    // `discard` that refusal is the file's outcome, named with the composition's own sentence, and
    // the document is written without it.
    let target = Target::Two(Level::B);
    let source = a_document_painting(&[NOTE], b"");
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("discard"), target));
    let output = output.expect("the file is discarded, so the conversion writes");
    let outcomes = &conversion(&report).attachments;
    assert_eq!(outcomes.len(), 1);
    assert!(
        matches!(&outcomes[0].answer, AttachmentAnswer::Removed(because)
            if because.contains("output intent")),
        "{outcomes:?}"
    );
    assert_eq!(pages_of(&output), 1);
    // And with `stop` the same document is refused rather than losing the file.
    let (_, output) = convert_with_plan(&source, &plan_from(&appended("stop"), target));
    assert!(output.is_none());
}

/// [`NOTE`] and [`BLOB`], each named by a file attachment annotation on the page as well.
///
/// Objects 10 and 11 are §12.5.6.15's annotations naming specifications 6 and 8, both drawing the
/// one appearance stream, object 12.
fn a_page_whose_annotations_name_both() -> Vec<u8> {
    let mut binary = Vec::new();
    for (index, (name, data)) in [NOTE, BLOB].iter().enumerate() {
        let specification = index.saturating_mul(2).saturating_add(6);
        binary.push(
            format!(
                "<< /Type /Filespec /F ({name}) /UF ({name}) /EF << /F {} 0 R >> >>",
                specification.saturating_add(1)
            )
            .into_bytes(),
        );
        binary.push(stream(
            &format!(
                "/Type /EmbeddedFile /Subtype /application#2Foctet-stream /Length {}",
                data.len()
            ),
            data,
        ));
    }
    for (specification, x, right) in [(6, 10, 30), (8, 40, 60)] {
        binary.push(
            format!(
                "<< /Type /Annot /Subtype /FileAttachment /Rect [{x} 100 {right} 120] /F 4 \
                 /FS {specification} 0 R /Name /PushPin /Contents (attached) /AP << /N 12 0 R >> >>"
            )
            .into_bytes(),
        );
    }
    binary.push(stream(
        "/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >>",
        b"0 0 1 rg 0 0 20 20 re f",
    ));
    Conforming {
        catalog: "/Names << /EmbeddedFiles << /Names [(note.txt) 6 0 R (blob.bin) 8 0 R] >> >>"
            .to_owned(),
        page: "/Annots [10 0 R 11 0 R]".to_owned(),
        binary_objects: binary,
        contents: Some((String::new(), b"1 0 0 rg 10 10 50 50 re f".to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// The annotation dictionaries on the output's first page, through its `/Annots`.
fn annotations_of(output: &[u8]) -> Vec<pdf_syntax::object::Dictionary> {
    let document = Document::open(output.to_vec()).expect("the output opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the source's page");
    let annots = document.get_key(&page.dict, "Annots");
    annots
        .as_array()
        .expect("the page keeps its annotations")
        .iter()
        .filter_map(|entry| document.resolve(entry).as_dict().cloned())
        .collect()
}

#[test]
fn an_annotation_naming_a_file_laid_out_as_pages_links_to_them_at_part_two() {
    // ISO 19005-2 section 6.3.1 admits a FileAttachment annotation, so it stays on its page; its
    // file is not PDF/A (section 6.8), and §12.5.6.15's Table 184 makes `/FS` required. The file
    // goes to pages, and the annotation becomes a §12.5.6.5 link to the first of them, keeping the
    // producer's appearance — or a link to nothing where its file was removed (`doc/adr/2537`).
    let target = Target::Two(Level::B);
    let source = a_page_whose_annotations_name_both();
    let (report, output) = convert_with_plan(&source, &plan_from(&appended("discard"), target));
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| {
        panic!(
            "the annotations no longer name the files, so the conversion writes: {:?}",
            conversion.decided
        )
    });
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{:?}",
        conversion.decided
    );
    assert_eq!(pages_of(&output), 2, "the note's page is appended");
    let annotations = annotations_of(&output);
    assert_eq!(annotations.len(), 2, "both annotations stay on the page");
    let document = Document::open(output.clone()).expect("the output opens");
    let appended = pdf_model::Pages::new(&document)
        .get(1)
        .expect("the appended page")
        .id
        .expect("reached through the tree");
    for annotation in &annotations {
        assert_eq!(
            annotation
                .get("Subtype")
                .and_then(|subtype| subtype.as_name())
                .map(|name| name.as_bytes().to_vec()),
            Some(b"Link".to_vec()),
            "each is a link now: {annotation:?}"
        );
        assert!(annotation.get("FS").is_none(), "and names no file");
        assert_eq!(
            annotation
                .get("Border")
                .and_then(|border| border.as_array().map(<[_]>::len)),
            Some(3),
            "and states the zero border its producer did not state"
        );
        assert!(
            annotation.get("AP").is_some(),
            "and keeps the producer's appearance"
        );
    }
    let destinations: Vec<_> = annotations
        .iter()
        .filter_map(|annotation| {
            annotation
                .get("Dest")
                .and_then(|dest| dest.as_array().map(<[_]>::to_vec))
        })
        .collect();
    assert_eq!(
        destinations.len(),
        1,
        "only the laid-out file has a page to link to"
    );
    assert_eq!(
        destinations[0]
            .first()
            .and_then(pdf_syntax::Object::as_reference),
        Some(appended)
    );
    // The outcome per file names its annotation, in the report and in the file's history.
    let note = conversion
        .attachments
        .iter()
        .find(|outcome: &&AttachmentOutcome| outcome.object.number == 6)
        .expect("the note's outcome");
    assert_eq!(note.annotations.len(), 1, "{note:?}");
    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("keep their appearance as a link to page 2 and name no file"),
        "xmpMM:History names the annotation's new destination"
    );
    assert!(
        text.contains("keep their appearance as a link to nothing and name no file"),
        "and the one whose file was removed"
    );
    assert!(report.to_json().render().contains("\"annotations\": ["));
}

#[test]
fn an_annotation_naming_a_file_still_refuses_where_no_page_answers_the_row() {
    // Without the pages, nothing answers the annotation's `/FS`, and the refusal is the old one.
    let target = Target::Two(Level::B);
    let source = a_page_whose_annotations_name_both();
    let row = "[site.\"embedded-files/embedded-file-is-itself-pdfa\"]\nremedy = \"discard\"\n";
    let (report, output) = convert_with_plan(&source, &plan_from(row, target));
    assert!(output.is_none(), "{:?}", conversion(&report).decided);
}
