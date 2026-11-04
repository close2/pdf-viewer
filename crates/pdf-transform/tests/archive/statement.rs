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
use pdf_syntax::object::Object;
use pdf_transform::archive::{AttachmentAnswer, Configuration, Listed, StatementPage};
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
pub(super) fn stated(output: &[u8], statement: &StatementPage) -> String {
    statement
        .pages
        .iter()
        .map(|index| text_of_page(output, *index))
        .collect::<String>()
        .split_whitespace()
        .collect()
}

/// Whether the statement's text holds a fact, compared without spaces or line breaks.
pub(super) fn holds_fact(page: &str, fact: &str) -> bool {
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
    assert_eq!(statement.listed.total(), 2);
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
    assert_eq!(
        statement.listed,
        Listed {
            removed_objects: 1,
            ..Listed::default()
        },
        "only the removed file is stated"
    );
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
    a_tagged_document_holding_two_files_under(
        &["<< /Type /StructTreeRoot /K [] >>"],
        String::new(),
        b"1 0 0 rg 10 10 50 50 re f",
    )
}

/// A tagged document holding the two files, its structure as `structure` (objects 6 on) and its
/// one page stating `page` and drawing `content`; the files come after the structure.
fn a_tagged_document_holding_two_files_under(
    structure: &[&str],
    page: String,
    content: &[u8],
) -> Vec<u8> {
    let mut binary = Vec::new();
    let mut names = String::new();
    for (index, (name, data)) in [NOTE, BLOB].into_iter().enumerate() {
        let specification = index
            .saturating_mul(2)
            .saturating_add(6)
            .saturating_add(structure.len());
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
        objects: structure.iter().map(|&body| body.to_owned()).collect(),
        binary_objects: binary,
        page,
        contents: Some((String::new(), content.to_vec())),
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

/// `doc/adr/2672`: a parent tree of any depth gains the statement page's entry in a new leaf.
///
/// §14.7.5.4 makes the page's `/StructParents` "the integer key under which the entry
/// corresponding to the object shall be found in the structural parent tree", and §7.9.7's
/// `/Limits` are what a reader descends by — so each appended page is read back through
/// `pdf_model::structure::ParentTree`, the lookup this reader draws with, and each of its
/// sequences has to find a `P` element. The producer's own page has to find its element still.
#[test]
fn a_parent_tree_with_kids_gains_the_statement_pages_entries_at_any_depth() {
    // Object 6 the root, 7 the producer's element, 8 the parent tree's root, 9 on its nodes. The
    // producer's key is 3 and `/ParentTreeNextKey` says 4.
    let root = "<< /Type /StructTreeRoot /K [7 0 R] /ParentTree 8 0 R /ParentTreeNextKey 4 >>";
    let element = "<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K 0 >>";
    let shapes: [&[&str]; 3] = [
        // A root over one leaf.
        &[
            "<< /Kids [9 0 R] >>",
            "<< /Limits [3 3] /Nums [3 [7 0 R]] >>",
        ],
        // A root over an intermediate node over two leaves.
        &[
            "<< /Kids [9 0 R] >>",
            "<< /Limits [1 3] /Kids [10 0 R 11 0 R] >>",
            "<< /Limits [1 1] /Nums [1 null] >>",
            "<< /Limits [3 3] /Nums [3 [7 0 R]] >>",
        ],
        // A root holding `/Nums` through a reference of its own.
        &["<< /Nums 9 0 R >>", "[3 [7 0 R]]"],
    ];
    for tree in shapes {
        let structure: Vec<&str> = [root, element]
            .into_iter()
            .chain(tree.iter().copied())
            .collect();
        let source = a_tagged_document_holding_two_files_under(
            &structure,
            "/StructParents 3".to_owned(),
            b"/P << /MCID 0 >> BDC 1 0 0 rg 10 10 50 50 re f EMC",
        );
        for target in [Target::Two(Level::A), Target::Four(Flavour::Plain)] {
            let (report, output) = convert_with_plan(
                &source,
                &planned(
                    &if target == Target::Two(Level::A) {
                        row("remedy = \"append-statement\"")
                    } else {
                        format!(
                            "[site.\"{SITE}-in-the-plain-profile\"]\nremedy = \"append-statement\"\n"
                        )
                    },
                    target,
                ),
            );
            let conversion = conversion(&report);
            let output =
                output.unwrap_or_else(|| panic!("{tree:?} {target}: {:?}", conversion.decided));
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{tree:?} {target}: the extended tree is held to the target again"
            );
            let statement = conversion.statement.as_ref().expect("the page is reported");
            let document = Document::open(output.clone()).expect("the output opens");
            let pages = pdf_model::Pages::new(&document);
            let found = |index: usize, mcid: i64| {
                let page = pages.get(index).expect("the page");
                pdf_model::structure::ParentTree::for_page(&document, &page.dict)
                    .element(&document, mcid)
                    .and_then(|element| {
                        document
                            .get_key(&element, "S")
                            .as_name()
                            .map(|name| name.as_bytes().to_vec())
                    })
            };
            assert_eq!(
                found(0, 0).as_deref(),
                Some(&b"P"[..]),
                "{tree:?} {target}: the producer's page"
            );
            assert!(!statement.pages.is_empty());
            for index in &statement.pages {
                let page = pages.get(*index).expect("the statement's page");
                assert!(
                    document.get_key(&page.dict, "StructParents").as_integer() >= Some(4),
                    "{tree:?} {target}: page {index} takes a key from ParentTreeNextKey on"
                );
                assert_eq!(
                    found(*index, 0).as_deref(),
                    Some(&b"P"[..]),
                    "{tree:?} {target}: page {index}'s first sequence finds its element"
                );
            }
            if target == Target::Two(Level::A) {
                let blob = u32::try_from(structure.len() + 8).expect("a small number");
                assert_states(&stated(&output, statement), blob, BLOB);
            }
        }
    }
}

/// `doc/adr/2674`: a producer page the parent tree loses and an appended page share one new leaf.
///
/// The producer's page states no `/StructParents`, so the rebuild gives it the first key from
/// `/ParentTreeNextKey` and the statement's pages take the keys after it; both sets of entries ride
/// in the one leaf the appended pages' extension writes, and the structure tree root and the parent
/// tree's root are rewritten once. Each page is read back through
/// `pdf_model::structure::ParentTree`.
#[test]
fn a_rebuilt_entry_and_the_statement_pages_entries_share_one_new_leaf() {
    let structure = [
        "<< /Type /StructTreeRoot /K [7 0 R] /ParentTree 8 0 R /ParentTreeNextKey 4 >>",
        "<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K 0 >>",
        "<< /Kids [9 0 R] >>",
        "<< /Limits [3 3] /Nums [3 null] >>",
    ];
    let source = a_tagged_document_holding_two_files_under(
        &structure,
        String::new(),
        b"/P << /MCID 0 >> BDC 1 0 0 rg 10 10 50 50 re f EMC",
    );
    for target in [Target::Two(Level::A), Target::Four(Flavour::Plain)] {
        let text = if target == Target::Two(Level::A) {
            row("remedy = \"append-statement\"")
        } else {
            format!("[site.\"{SITE}-in-the-plain-profile\"]\nremedy = \"append-statement\"\n")
        };
        let (report, output) = convert_with_plan(&source, &planned(&text, target));
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| panic!("{target}: {:?}", conversion.decided));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let statement = conversion.statement.as_ref().expect("the page is reported");
        let document = Document::open(output.clone()).expect("the output opens");
        let pages = pdf_model::Pages::new(&document);
        let key_and_found = |index: usize| {
            let page = pages.get(index).expect("the page");
            let element = pdf_model::structure::ParentTree::for_page(&document, &page.dict)
                .element(&document, 0)
                .and_then(|element| {
                    document
                        .get_key(&element, "S")
                        .as_name()
                        .map(|name| name.as_bytes().to_vec())
                });
            (
                document.get_key(&page.dict, "StructParents").as_integer(),
                element,
            )
        };
        assert_eq!(key_and_found(0), (Some(4), Some(b"P".to_vec())), "{target}");
        for index in &statement.pages {
            let (key, element) = key_and_found(*index);
            assert!(
                key >= Some(5),
                "{target}: page {index} after the rebuilt key"
            );
            assert_eq!(
                element.as_deref(),
                Some(&b"P"[..]),
                "{target}: page {index}"
            );
        }
        let catalog = document.catalog().expect("a catalog");
        let root = document.get_key(&catalog, "StructTreeRoot");
        let tree = document.get_key(root.as_dict().expect("the root"), "ParentTree");
        let kids = document
            .get_key(tree.as_dict().expect("the tree"), "Kids")
            .as_array()
            .map(<[Object]>::len);
        assert_eq!(kids, Some(2), "{target}: one new leaf for both");
    }
}

/// The output with an incremental update (§7.5.6) writing its parent tree root back to its first
/// kid alone, so that the leaf the conversion added for its appended pages is in no node.
fn withholding_the_appended_pages_leaf(output: &[u8]) -> Vec<u8> {
    let document = Document::open(output.to_vec()).expect("the output opens");
    let catalog = document.catalog().expect("a catalog");
    let root = document.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("a structure tree root");
    let tree = root
        .get("ParentTree")
        .and_then(Object::as_reference)
        .expect("the parent tree root is an object of its own");
    let kids = document.get_key(
        &document.get(tree).as_dict().expect("a node").clone(),
        "Kids",
    );
    let kids = kids.as_array().expect("a /Kids root");
    assert_eq!(kids.len(), 2, "the producer's leaf and the new one");
    let first = kids[0].as_reference().expect("the producer's leaf");
    let trailer = document.trailer();
    let size = trailer
        .get("Size")
        .and_then(Object::as_integer)
        .expect("a size");
    let catalog_id = trailer
        .get("Root")
        .and_then(Object::as_reference)
        .expect("a root");
    let previous = output
        .windows(b"startxref".len())
        .rposition(|window| window == b"startxref")
        .and_then(|at| {
            std::str::from_utf8(output.get(at.saturating_add(b"startxref".len())..)?)
                .ok()?
                .split_whitespace()
                .next()?
                .parse::<usize>()
                .ok()
        })
        .expect("the last startxref");
    let mut planted = output.to_vec();
    planted.push(b'\n');
    let object_at = planted.len();
    planted.extend_from_slice(
        format!(
            "{} {} obj\n<< /Kids [{} {} R] >>\nendobj\n",
            tree.number, tree.generation, first.number, first.generation
        )
        .as_bytes(),
    );
    let xref_at = planted.len();
    planted.extend_from_slice(
        format!(
            "xref\n0 1\n0000000000 65535 f \n{} 1\n{object_at:010} {:05} n \n\
             trailer\n<< /Size {size} /Root {} {} R /Prev {previous} >>\n\
             startxref\n{xref_at}\n%%EOF\n",
            tree.number, tree.generation, catalog_id.number, catalog_id.generation
        )
        .as_bytes(),
    );
    planted
}

/// `doc/adr/2673`: the converter's validator fails an output whose appended pages' leaf is withheld.
///
/// ADR 2672's withheld leaf planted on purpose: the output of the one-leaf shape above, with an
/// incremental update (§7.5.6) writing the parent tree root back to the producer's one kid, so the
/// statement page's `/StructParents` key is in no node. §14.7.5.4's "[t]he tree shall contain an
/// entry for each ... content stream containing at least one marked-content sequence that is a
/// content item" is what fails, at the row section 5.1 of both parts carries; the output itself
/// still conforms.
#[test]
fn an_output_whose_appended_pages_leaf_is_withheld_fails_the_parent_tree_row() {
    const ROW: &str = "logical-structure/content-items-find-their-elements-through-the-parent-tree";
    let structure = [
        "<< /Type /StructTreeRoot /K [7 0 R] /ParentTree 8 0 R /ParentTreeNextKey 4 >>",
        "<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K 0 >>",
        "<< /Kids [9 0 R] >>",
        "<< /Limits [3 3] /Nums [3 [7 0 R]] >>",
    ];
    let source = a_tagged_document_holding_two_files_under(
        &structure,
        "/StructParents 3".to_owned(),
        b"/P << /MCID 0 >> BDC 1 0 0 rg 10 10 50 50 re f EMC",
    );
    for target in [Target::Two(Level::A), Target::Four(Flavour::Plain)] {
        let configuration = if target == Target::Two(Level::A) {
            row("remedy = \"append-statement\"")
        } else {
            format!("[site.\"{SITE}-in-the-plain-profile\"]\nremedy = \"append-statement\"\n")
        };
        let (report, output) = convert_with_plan(&source, &planned(&configuration, target));
        let output =
            output.unwrap_or_else(|| panic!("{target}: {:?}", conversion(&report).decided));
        let failed = |bytes: &[u8]| {
            holds(bytes, target)
                .judgements
                .into_iter()
                .filter(|judgement| judgement.id == ROW)
                .flat_map(|judgement| match judgement.outcome {
                    Outcome::Failed { places, .. } => {
                        places.into_iter().map(|finding| finding.what).collect()
                    }
                    _ => Vec::new(),
                })
                .collect::<Vec<String>>()
        };
        assert_eq!(
            failed(&output),
            Vec::<String>::new(),
            "{target}: the output"
        );

        let planted = withholding_the_appended_pages_leaf(&output);
        // One finding per appended page, each naming the key no node holds.
        let withheld = failed(&planted);
        let statement = conversion(&report)
            .statement
            .as_ref()
            .expect("the page is reported");
        assert_eq!(
            withheld.len(),
            statement.pages.len(),
            "{target}: {withheld:?}"
        );
        assert!(
            withheld
                .iter()
                .all(|what| what.contains("no entry under this StructParents key")),
            "{target}: {withheld:?}"
        );
    }
}

/// The profile `append-statement` is named for at the presentation sites, as it ships.
const STATE_EVERY_LOSS: &str = include_str!("../../../../doc/profiles/state-every-loss.toml");

/// A page whose two graphics states hold all three halftone entries ISO 19005 section 6.2.5
/// forbids: `G0` a threshold halftone and a halftone origin, `G1` a halftone naming itself.
fn three_halftone_entries(target: Target) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        resources: "/ExtGState << /G0 6 0 R /G1 8 0 R >>".to_owned(),
        objects: vec![
            "<< /Type /ExtGState /HT 7 0 R /HTO [0 0] >>".to_owned(),
            "<< /Type /Halftone /HalftoneType 6 /Width 2 /Height 2 /Length 4 >>\nstream\n@P`p\nendstream"
                .to_owned(),
            "<< /Type /ExtGState /HT 9 0 R >>".to_owned(),
            "<< /Type /Halftone /HalftoneType 1 /HalftoneName (Default) /Frequency 60 /Angle 45 \
             /SpotFunction /Round >>"
                .to_owned(),
        ],
        contents: Some((
            String::new(),
            b"/G0 gs 0 0 1 rg 10 10 50 50 re f /G1 gs 60 60 20 20 re f".to_vec(),
        )),
        ..base
    }
    .build()
}

/// Every entry the statement lists, read back against the source: the object it names states
/// the key, and the value the page gives is that key's value as the source writes it.
///
/// `doc/adr/1014` section 3's check on an appended page, done from the page's text alone, so a
/// statement naming a key or a value the source does not hold fails here. Returns how many
/// entries it read.
fn read_back_against(source: &[u8], page: &str) -> usize {
    let document = Document::open_with_limits(source.to_vec(), Limits::DEFAULT).expect("it opens");
    let mut read: usize = 0;
    let lines: Vec<&str> = page.lines().map(str::trim).collect();
    for (index, line) in lines.iter().enumerate() {
        let Some(object) = line
            .strip_prefix("Object: graphics state ")
            .or_else(|| line.strip_prefix("Object: halftone "))
        else {
            continue;
        };
        let number: u32 = object
            .split(' ')
            .next()
            .and_then(|number| number.parse().ok())
            .expect("an object number");
        let line_after = |by: usize| {
            lines
                .get(index.saturating_add(by))
                .copied()
                .unwrap_or_default()
        };
        let key = line_after(1)
            .strip_prefix("Entry removed: ")
            .or_else(|| line_after(1).strip_prefix("Entry written Default: "))
            .unwrap_or_else(|| panic!("the entry follows the object: {page}"));
        let value = line_after(2)
            .strip_prefix("Value in the source: ")
            .unwrap_or_else(|| panic!("the value follows the entry: {page}"));
        let held = document.get(ObjectId::new(number, 0));
        let dict = match &held {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            other => panic!("object {number} is {other:?}"),
        };
        let stated = dict
            .get(key)
            .unwrap_or_else(|| panic!("the source's object {number} states no {key}: {page}"));
        let mut written = Vec::new();
        pdf_syntax::write::object(stated, &mut written);
        assert_eq!(
            value,
            String::from_utf8_lossy(&written),
            "object {number}'s {key}"
        );
        read = read.saturating_add(1);
    }
    read
}

/// The statement's text with its lines kept, for [`read_back_against`].
///
/// A line the layout broke at the measure is joined back to the field it continues, which is
/// every line that opens with no field label of the statement's own.
fn stated_lines(output: &[u8], statement: &StatementPage) -> String {
    let labels = [
        "Requirement:",
        "Asks:",
        "Loss:",
        "Object:",
        "Entry ",
        "Value in the source:",
        "In the ",
        "Tried before:",
        "Not admitted",
        "Removed in",
        "Page:",
        "Font:",
        "Fonts not drawn:",
        "Content stream requirements",
        "Structure tree leaves",
        "Optional content groups",
    ];
    let mut out = String::new();
    for line in statement
        .pages
        .iter()
        .map(|index| text_of_page(output, *index))
        .collect::<Vec<_>>()
        .join("\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let line = line
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
            .trim();
        if labels.iter().any(|label| line.starts_with(label)) {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

#[test]
fn each_halftone_entry_the_profile_takes_is_stated_and_reads_back_against_the_source() {
    // ISO 19005-4 section 6.2.5 forbids all three entries; state-every-loss answers each site with
    // try = ["append-statement", "discard"], so the output loses them and says so on one page.
    let target = Target::Four(Flavour::Plain);
    let source = three_halftone_entries(target);
    let plan = planned(STATE_EVERY_LOSS, target);
    let (report, output) = convert_with_plan(&source, &plan);
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let statement = conversion.statement.as_ref().expect("the page is reported");
    assert_eq!(
        statement.sites,
        [
            "graphics/no-halftone-name",
            "graphics/halftone-type-is-one-or-five",
            "graphics/no-halftone-origin-in-a-graphics-state",
        ]
    );
    // The source's one page, then the statement.
    assert_eq!(pages_of(&source), 1);
    assert_eq!(statement.pages.first(), Some(&1));
    assert_eq!(pages_of(&output), 1 + statement.pages.len());
    assert_eq!(statement.listed.total(), 3);
    let page = stated_lines(&output, statement);
    assert_eq!(read_back_against(&source, &page), 3, "{page}");
    let unbroken: String = page.split_whitespace().collect();
    for fact in [
        "Object: halftone 9 0",
        "Entry removed: HalftoneName",
        "Object: graphics state 6 0",
        "Entry written Default: HT",
        "Value in the source: 7 0 R",
        "Entry removed: HTO",
        "Value in the source: [0 0]",
        "In the resources of page(s): 1",
        "Loss: halftone-name,",
    ] {
        assert!(
            holds_fact(&unbroken, fact),
            "the page states {fact:?}:\n{page}"
        );
    }
    let written = String::from_utf8_lossy(&output);
    assert!(!written.contains("HalftoneName") && !written.contains("/HTO"));
}

#[test]
fn a_halftone_name_is_stated_at_part_two() {
    // ISO 19005-2 section 6.2.5 forbids HalftoneName and a threshold halftone; HTO is part 4's
    // alone, so at part 2 that entry stays and is not listed.
    let target = Target::Two(Level::B);
    let source = three_halftone_entries(target);
    let plan = planned(STATE_EVERY_LOSS, target);
    let (report, output) = convert_with_plan(&source, &plan);
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let statement = conversion.statement.as_ref().expect("the page is reported");
    assert_eq!(statement.listed.total(), 2);
    let page = stated_lines(&output, statement);
    assert_eq!(read_back_against(&source, &page), 2, "{page}");
    let unbroken: String = page.split_whitespace().collect();
    assert!(
        holds_fact(&unbroken, "(ISO 19005-2 section 6.2.5)"),
        "{page}"
    );
}

#[test]
fn a_halftone_statement_that_cannot_be_composed_says_why_beside_the_discard() {
    // `doc/rfc/0014` section 3: a page that cannot be composed hands each object to the next
    // step, and the report names each object's step. Seventeen graphics states with a halftone
    // origin, each naming a halftone with a name, are thirty-four entries at two rows, two more
    // than a statement lists, so no page is composed, `discard` takes the entries, and the report
    // says why no page lists them (`doc/adr/2591`).
    let target = Target::Four(Flavour::Plain);
    let states: usize = 17;
    let source = Conforming {
        resources: format!(
            "/ExtGState << {} >>",
            (0..states)
                .map(|n| format!("/G{n} {} 0 R", 6 + 2 * n))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        objects: (0..states)
            .flat_map(|n| {
                [
                    format!("<< /Type /ExtGState /HTO [0 0] /HT {} 0 R >>", 7 + 2 * n),
                    format!(
                        "<< /Type /Halftone /HalftoneType 1 /HalftoneName (D{n}) /Frequency 60 \
                         /Angle 45 /SpotFunction /Round >>"
                    ),
                ]
            })
            .collect(),
        contents: Some((
            String::new(),
            (0..states)
                .fold(String::new(), |mut out, n| {
                    let _ = write!(out, "/G{n} gs ");
                    out
                })
                .into_bytes(),
        )),
        ..Conforming::default()
    }
    .build();
    let plan = planned(STATE_EVERY_LOSS, target);
    let (report, output) = convert_with_plan(&source, &plan);
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert!(conversion.statement.is_none(), "no page is composed");
    assert_eq!(pages_of(&output), pages_of(&source));
    let sites: Vec<&str> = conversion.unstated.iter().map(|row| row.site).collect();
    assert_eq!(
        sites,
        [
            "graphics/no-halftone-origin-in-a-graphics-state",
            "graphics/no-halftone-name",
        ],
        "in the validator's order: {:?}",
        conversion.unstated
    );
    let rendered = conversion.render();
    for unstated in &conversion.unstated {
        assert_eq!(unstated.then, pdf_transform::archive::RemedyKind::Discard);
        assert!(
            unstated.because.contains("thirty-two objects"),
            "the composition's own sentence: {}",
            unstated.because
        );
        assert!(
            rendered.contains(&format!("not stated: {}", unstated.site)),
            "{rendered}"
        );
    }
    let json = conversion.to_json().render();
    assert!(json.contains("thirty-two objects"), "{json}");
}

/// The font row's site, whose `discard` carries the font bundle (`doc/adr/2600`).
const FONT_SITE: &str = "fonts/font-programs-embedded";

/// Every photographed page and every face the font row's entries name, read back against the
/// report: each `Page:` the statement gives is a page the report says was photographed, with the
/// report's loss words in its order and the report's fonts; each `Font:` is a face the report says
/// was embedded with those losses. Returns how many entries it read, so a caller can hold the count
/// to the report's own.
fn fonts_read_back_against(conversion: &pdf_transform::archive::Conversion, page: &str) -> usize {
    let mut read: usize = 0;
    let lines: Vec<&str> = page.lines().map(str::trim).collect();
    for (index, line) in lines.iter().enumerate() {
        let following = lines
            .iter()
            .skip(index.saturating_add(1))
            .take_while(|line| !line.starts_with("Requirement:"));
        let losses: Vec<&str> = following
            .clone()
            .filter_map(|line| line.strip_prefix("Loss: "))
            .map(|line| line.split(',').next().unwrap_or_default())
            .collect();
        if let Some(stated) = line.strip_prefix("Page: ") {
            let number: usize = stated
                .split(',')
                .next()
                .and_then(|number| number.parse().ok())
                .expect("a page number");
            let photographed = conversion
                .photographed
                .iter()
                .find(|photographed| photographed.page == number)
                .unwrap_or_else(|| {
                    panic!("the statement names page {number}, which the report does not: {page}")
                });
            let words: Vec<&str> = photographed.losses().into_iter().map(Loss::word).collect();
            assert_eq!(losses, words, "page {number}'s losses: {page}");
            let fonts = following
                .clone()
                .find_map(|line| line.strip_prefix("Fonts not drawn: "))
                .map(|fonts| fonts.split(',').map(str::trim).collect::<Vec<_>>())
                .unwrap_or_default();
            assert_eq!(fonts, photographed.fonts, "page {number}'s fonts: {page}");
            read = read.saturating_add(1);
        } else if let Some(stated) = line.strip_prefix("Font: ") {
            let face = conversion
                .substituted
                .iter()
                .find(|face| {
                    let named = format!("{} (resource {})", face.requested, face.resource);
                    stated.split_whitespace().collect::<String>()
                        == named.split_whitespace().collect::<String>()
                })
                .unwrap_or_else(|| {
                    panic!("the statement names the font {stated}, which the report does not")
                });
            assert_eq!(
                losses,
                face.losses.iter().map(Loss::word).collect::<Vec<_>>()
            );
            read = read.saturating_add(1);
        }
    }
    read
}

#[test]
fn each_page_the_font_row_photographs_is_stated_and_reads_back_against_the_report() {
    // ISO 32000-2 §9.7.5.2: an Identity CMap shall not be used with a non-embedded font, and the
    // fixture's Adobe-Identity font has no program and no ToUnicode map, so the font row's one
    // remedy is the photograph (A570). state-every-loss answers the row with
    // try = ["append-statement", "discard"], so the page is photographed and the statement page
    // names it with its loss.
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let source = photographed::a_page_no_reader_can_read("", b"", "", &[]);
        let plan = planned(STATE_EVERY_LOSS, target);
        let (report, output) = convert_with_plan(&source, &plan);
        let conversion = conversion(&report);
        let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        if let Some(directory) = std::env::var_os("FONT_STATEMENT_ARTIFACTS") {
            let path =
                std::path::Path::new(&directory).join(format!("font-statement-{target:?}.pdf"));
            std::fs::write(path, &output).expect("the artifact is written");
        }
        assert_eq!(conversion.photographed.len(), 1, "{}", conversion.render());
        let statement = conversion
            .statement
            .as_ref()
            .unwrap_or_else(|| panic!("the page is reported: {}", conversion.render()));
        assert_eq!(statement.sites, [FONT_SITE], "at {target}");
        assert_eq!(statement.listed.total(), 1);
        assert_eq!(statement.pages.first(), Some(&1));
        assert_eq!(pages_of(&output), 1 + statement.pages.len());
        assert!(
            conversion.unstated.iter().all(|row| row.site != FONT_SITE),
            "{:?}",
            conversion.unstated
        );
        let page = stated_lines(&output, statement);
        assert_eq!(fonts_read_back_against(conversion, &page), 1, "{page}");
        let unbroken: String = page.split_whitespace().collect();
        for fact in [
            "Requirement: fonts/font-programs-embedded",
            "Page: 1, photographed at 300 dpi",
            "Loss: text-as-image,",
            "Fonts not drawn: Arial",
            "Tried before: no step",
        ] {
            assert!(
                holds_fact(&unbroken, fact),
                "the page states {fact:?}:\n{page}"
            );
        }
    }
}

#[test]
fn a_photographed_page_of_a_layered_document_is_stated_with_the_groups_it_fixed() {
    // A717: the photograph of a page of a document stating optional content is drawn in the
    // configuration this viewer shows (ISO 32000-2 §8.11.4.3's /D, whose /OFF array turns Hidden
    // off), and the groups its content named are named with the state drawn. The page holds an
    // operator §8.2's Table 50 does not define, so it is photographed under page-as-image.
    let target = Target::Four(Flavour::Plain);
    let source = photographed::a_layered_document_with_a_fault_on_its_first_page(target);
    let plan = planned(STATE_EVERY_LOSS, target);
    let (report, output) = convert_with_plan(&source, &plan);
    let conversion = conversion(&report);
    let output = output.unwrap_or_else(|| panic!("{}", conversion.render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    if let Some(directory) = std::env::var_os("FONT_STATEMENT_ARTIFACTS") {
        let path = std::path::Path::new(&directory).join("font-statement-layered.pdf");
        std::fs::write(path, &output).expect("the artifact is written");
    }
    let statement = conversion.statement.as_ref().expect("the page is reported");
    let page = stated_lines(&output, statement);
    let listed = conversion.photographed.len().saturating_add(
        conversion
            .substituted
            .iter()
            .filter(|face| !face.losses.is_empty())
            .count(),
    );
    assert!(listed > 0, "{}", conversion.render());
    assert_eq!(fonts_read_back_against(conversion, &page), listed, "{page}");
    let unbroken: String = page.split_whitespace().collect();
    for fact in [
        "Page: 1, photographed at 300 dpi",
        "Loss: page-as-image,",
        "Loss: optional-content-of-photographed-pages,",
        "Optional content groups fixed as drawn: \"Shown\" on, \"Hidden\" off",
        "Content stream requirements answered: graphics/only-operators-the-base-standard-defines",
    ] {
        assert!(
            holds_fact(&unbroken, fact),
            "the page states {fact:?}:\n{page}"
        );
    }
}
