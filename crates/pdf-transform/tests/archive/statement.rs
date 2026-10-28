//! `append-statement`: the loss `discard` takes, then one appended page stating it (`doc/rfc/0014`
//! section 4, `A706`).
//!
//! Every value on the page is computed from the source — the object's number, its name, its decoded
//! length, its media type, the SHA-256 of its bytes — so each test reads the page back and checks
//! it against the file it was made from, which is `doc/adr/1014` section 3's check on every
//! appended page. ISO 19005-2 section 6.8 admits an embedded file only where it conforms to a part
//! of ISO 19005, so both of the files these documents hold fail it.

use super::attachments_as_pages::{BLOB, NOTE, a_document_holding, pages_of};
use super::*;
use pdf_transform::archive::{AttachmentAnswer, Configuration, StatementPage};
use sha2::{Digest as _, Sha256};

const SITE: &str = "embedded-files/embedded-file-is-itself-pdfa";

fn row(keys: &str) -> String {
    format!("[site.\"{SITE}\"]\n{keys}\n")
}

/// The plan a configuration makes, with the authorisations its `discard` steps stand for — what
/// `profiles.rs` hands the converter for a shipped profile.
fn planned(text: &str, target: Target) -> ArchivePlan {
    let config = Configuration::read(text, target).expect("it reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// The text a page of the output shows, zero-based.
fn text_of_page(output: &[u8], index: usize) -> String {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the page exists");
    pdf_model::interpret(&document, &page).text
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// The statement's text over all its pages, with every space and line break taken out.
///
/// `doc/adr/1025` section 4 sets an appended page's text verbatim and breaks a line at the measure
/// wherever it falls, inside a word or a digest, so a fact is compared with the breaks removed.
fn stated(output: &[u8], statement: &StatementPage) -> String {
    statement
        .pages
        .iter()
        .map(|index| text_of_page(output, *index))
        .collect::<String>()
        .split_whitespace()
        .collect()
}

/// Whether the statement's text holds a fact, compared without spaces or line breaks.
fn holds_fact(page: &str, fact: &str) -> bool {
    page.contains(&fact.split_whitespace().collect::<String>())
}

/// Every fact the page states about one file, computed here from the bytes the source holds.
fn assert_states(page: &str, specification: u32, (name, bytes): (&str, &[u8])) {
    for fact in [
        format!("Object: file specification {specification} 0"),
        format!("Name: {name}"),
        format!("Decoded length: {} bytes", bytes.len()),
        format!("SHA-256: {}", sha256(bytes)),
        "Media type: application/octet-stream".to_owned(),
        format!("Requirement: {SITE} (ISO 19005-2 section 6.8)"),
    ] {
        assert!(holds_fact(page, &fact), "the page states {fact:?}:\n{page}");
    }
}

#[test]
fn the_statement_is_its_own_page_and_states_each_removed_file_from_the_source() {
    let target = Target::Two(Level::B);
    let source = a_document_holding(&[NOTE, BLOB]);
    let plan = planned(&row("remedy = \"append-statement\""), target);
    let (report, output) = convert_with_plan(&source, &plan);
    let conversion = conversion(&report);
    let output = output
        .unwrap_or_else(|| panic!("the loss is stated, so it writes: {:?}", conversion.decided));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    // The source's one page and the statement: the pages a source states, counted against the
    // pages written.
    assert_eq!(pages_of(&source), 1);
    let statement = conversion.statement.as_ref().expect("the page is reported");
    assert_eq!(
        statement.pages.first(),
        Some(&1),
        "after the document's own page"
    );
    assert_eq!(pages_of(&output), 1 + statement.pages.len());
    assert_eq!(statement.objects, 2);
    assert_eq!(
        conversion.attachments.len(),
        2,
        "{:?}",
        conversion.attachments
    );
    for outcome in &conversion.attachments {
        assert!(
            matches!(outcome.answer, AttachmentAnswer::Removed(_)),
            "{outcome:?}"
        );
        assert_eq!(outcome.stated_on, Some(1), "{outcome:?}");
    }
    let page = stated(&output, statement);
    assert!(
        holds_fact(&page, "Removed in the conversion to PDF/A-2b"),
        "{page}"
    );
    assert_states(&page, 6, NOTE);
    assert_states(&page, 8, BLOB);
    assert!(holds_fact(&page, "Tried before: no step"), "{page}");
    // The report's JSON and the file's own history name the page.
    let json = report.to_json().render();
    assert!(json.contains("\"statement\""), "{json}");
    assert!(json.contains("\"stated-on\": 2"), "{json}");
    let written = String::from_utf8_lossy(&output);
    assert!(
        written.contains("the statement of what this conversion removed is on page(s) 2"),
        "xmpMM:History names the page"
    );
}

/// The plant of the round's claim: two files that must take two answers, reported as two.
#[test]
fn a_list_lays_out_one_file_and_states_the_other() {
    let target = Target::Two(Level::B);
    let text = row(r#"try = ["preserve:attach", "preserve:append", "append-statement"]"#);
    let config = Configuration::read(&text, target).expect("the list reads");
    assert_eq!(
        config.skipped().len(),
        1,
        "preserve:attach is passed over at 2b"
    );
    let source = a_document_holding(&[NOTE, BLOB]);
    let (report, output) = convert_with_plan(&source, &planned(&text, target));
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("both answered: {:?}", conversion.decided));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let statement = conversion.statement.as_ref().expect("the page is reported");
    let note = conversion
        .attachments
        .iter()
        .find(|outcome| outcome.object.number == 6)
        .expect("the note's outcome");
    let AttachmentAnswer::AppendedAsPages(laid_out) = &note.answer else {
        panic!("the note is laid out as pages: {note:?}");
    };
    assert_eq!(laid_out.first(), Some(&1), "{note:?}");
    // The source's page, the note's pages and the statement's, and nothing else.
    assert_eq!(
        pages_of(&output),
        1 + laid_out.len() + statement.pages.len()
    );
    assert_eq!(statement.objects, 1, "only the removed file is stated");
    assert_eq!(
        note.stated_on, None,
        "a file laid out as pages is not stated as removed"
    );
    let blob = conversion
        .attachments
        .iter()
        .find(|outcome| outcome.object.number == 8)
        .expect("the blob's outcome");
    assert!(
        matches!(blob.answer, AttachmentAnswer::Removed(_)),
        "{blob:?}"
    );
    assert_eq!(blob.stated_on, statement.pages.first().copied(), "{blob:?}");
    let page = stated(&output, statement);
    assert_states(&page, 8, BLOB);
    assert!(
        !page.contains("note.txt"),
        "only the removed file is stated:\n{page}"
    );
    assert!(holds_fact(&page, "Tried before: preserve:append"), "{page}");
    assert!(
        holds_fact(&page, "Not admitted at this target: preserve:attach"),
        "{page}"
    );
}

#[test]
fn a_statement_past_its_page_bound_is_refused_by_name() {
    let target = Target::Two(Level::B);
    // Twelve files on the test document's 200-unit page, whose margins leave a few lines to a page:
    // under the object and the length bounds, past sixteen pages once set at 9 units on 12, and
    // under the sixty-four pages every preservation is held to.
    let names: Vec<String> = (0..12).map(|index| format!("f{index}.bin")).collect();
    let files: Vec<(&str, &[u8])> = names.iter().map(|name| (name.as_str(), BLOB.1)).collect();
    let source = a_document_holding(&files);
    let (report, output) = convert_with_plan(
        &source,
        &planned(&row("remedy = \"append-statement\""), target),
    );
    let decided = format!("{:?}", conversion(&report).decided);
    assert!(
        output.is_none(),
        "the loss is not taken unstated: {:?}",
        conversion(&report).statement
    );
    assert!(decided.contains("MOST_STATEMENT_PAGES"), "{decided}");
}

/// A tagged document holding the two files: Table 354's smallest root as object 6, the files
/// after it (specifications 7 and 9).
fn a_tagged_document_holding_two_files() -> Vec<u8> {
    let mut binary = Vec::new();
    let mut names = String::new();
    for (index, (name, data)) in [NOTE, BLOB].into_iter().enumerate() {
        let specification = index.saturating_mul(2).saturating_add(7);
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
        catalog: format!(
            "/MarkInfo << /Marked true >> /StructTreeRoot 6 0 R \
             /Names << /EmbeddedFiles << /Names [{names}] >> >>"
        ),
        objects: vec!["<< /Type /StructTreeRoot /K [] >>".to_owned()],
        binary_objects: binary,
        contents: Some((String::new(), b"1 0 0 rg 10 10 50 50 re f".to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// `A58`: a Level A target describes every appended page, the statement's included.
#[test]
fn a_level_a_target_describes_the_statement_page_in_the_structure_tree() {
    let target = Target::Two(Level::A);
    let source = a_tagged_document_holding_two_files();
    let (report, output) = convert_with_plan(
        &source,
        &planned(&row("remedy = \"append-statement\""), target),
    );
    let conversion = conversion(&report);
    let output =
        output.unwrap_or_else(|| panic!("the statement is described: {:?}", conversion.decided));
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "the validator holds the page's structure entries to Level A"
    );
    let statement = conversion.statement.as_ref().expect("the page is reported");
    let document = Document::open(output.clone()).expect("the output opens");
    let pages = pdf_model::Pages::new(&document);
    for index in &statement.pages {
        let page = pages.get(*index).expect("the statement's page");
        assert!(
            document
                .get_key(&page.dict, "StructParents")
                .as_integer()
                .is_some(),
            "page {index} states its key in the parent tree"
        );
    }
    assert_states(&stated(&output, statement), 9, BLOB);
}
