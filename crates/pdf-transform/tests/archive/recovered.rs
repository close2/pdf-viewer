//! A document whose catalog or page tree is not stated converts behind the loss
//! `recovered-structure` (`doc/adr/2502`).
//!
//! ISO 32000-2 §7.5.5's Table 15 makes the trailer's `/Root` required and §7.7.2's Table 29 makes
//! the catalog's `/Pages` required; the standard defines no recovery when either fails. The viewer
//! finds the pages by their own `/Type /Page` declaration all the same, and the archive verb
//! writes a catalog and one page tree node listing them, each page the producer's.

use super::*;

/// A whole file of the numbered objects (the first is object 1), with a classic cross-reference
/// table and the trailer entries `trailer` states beside `/Size`.
fn file(objects: &[&str], trailer: &str) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(
            format!("{} 0 obj\n{object}\nendobj\n", index.saturating_add(1)).as_bytes(),
        );
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} {trailer} >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

/// A page naming object 2 as its parent and stating its own box.
const PAGE: &str = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>";

/// The authorisation this loss stands for, and nothing else.
fn authorised() -> Authorisations {
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::RecoveredStructure);
    authorised
}

/// The conversion to `target`, as the caller's refusal or the report and the written file.
fn attempt(
    source: &[u8],
    target: Target,
    authorised: Authorisations,
) -> Result<(Report, Option<Vec<u8>>), pdf_transform::Refusal> {
    let mut plan = plan_from("", target);
    plan.authorised = authorised;
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Archive(plan),
        &[Source::new(source.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    Ok((report, sinks.into_outputs().pop().map(|(_, bytes)| bytes)))
}

/// The conversion to `target` with the loss authorised, held to the target again.
fn recovered(source: &[u8], target: Target) -> (Report, Document, Vec<u8>) {
    let (report, output) = attempt(source, target, authorised()).expect("the conversion applies");
    let output = written(&report, output);
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{target}: what was written is held to the target again"
    );
    let document = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
    (report, document, output)
}

/// The first page of a written document, found by walking its page tree.
fn first_page(document: &Document) -> pdf_syntax::object::Dictionary {
    let tree = pdf_model::Pages::new(document);
    assert!(
        tree.found_by_scanning().is_none(),
        "the output states a page tree that is walked, where the source's was scanned for"
    );
    tree.every()
        .into_iter()
        .next()
        .flatten()
        .expect("a first page")
        .dict
}

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

#[test]
fn a_root_naming_no_dictionary_is_written_as_a_catalog_and_a_page_tree() {
    // Table 15: `/Root` is "( Required; shall be an indirect reference )" to the catalog; here
    // object 1 is an integer, so no catalog is stated and the page is found by its own `/Type`.
    let source = file(
        &["42", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", PAGE],
        "/Root 1 0 R",
    );
    for target in TARGETS {
        let (report, document, output) = recovered(&source, target);
        let note = conversion(&report)
            .recovered_structure
            .as_ref()
            .expect("the report names what was rebuilt");
        assert_eq!(note.pages, 1, "{target}");
        assert_eq!(
            note.catalog,
            pdf_transform::archive::RecoveredCatalog::Written
        );
        assert_eq!(
            note.scanned,
            Some(1),
            "{target}: found by the page's own /Type"
        );
        let page = first_page(&document);
        assert!(
            page.get("MediaBox").is_some(),
            "{target}: the producer's box is carried"
        );
        assert!(
            output
                .windows(b"recovered-structure".len())
                .any(|w| w == b"recovered-structure"),
            "{target}: the file's own xmpMM:History says it"
        );
        let text = conversion(&report).render();
        assert!(text.contains("recovered-structure"), "{target}: {text}");
        let json = format!("{:?}", conversion(&report).to_json());
        assert!(json.contains("recovered_structure"), "{target}: {json}");
        assert!(json.contains("found_by_scanning"), "{target}: {json}");
    }
}

#[test]
fn a_trailer_with_no_root_is_written_with_a_catalog_and_the_page_found_by_scanning() {
    let source = file(&["<< /Type /Pages /Count 1 >>", PAGE], "");
    let (report, document, _) = recovered(&source, Target::Two(Level::B));
    let note = conversion(&report)
        .recovered_structure
        .as_ref()
        .expect("rebuilt");
    assert_eq!(
        note.catalog,
        pdf_transform::archive::RecoveredCatalog::Written
    );
    assert_eq!(note.pages, 1);
    first_page(&document);
}

#[test]
fn a_catalog_whose_pages_entry_dangles_keeps_its_other_entries() {
    // Table 29: `/Pages` is required of the catalog and names the root of the page tree; here it
    // names nothing, and the producer's `/Lang` is still the producer's.
    let source = file(
        &[
            "<< /Type /Catalog /Pages 9 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 >>",
            PAGE,
        ],
        "/Root 1 0 R",
    );
    for target in TARGETS {
        let (report, document, _) = recovered(&source, target);
        let note = conversion(&report)
            .recovered_structure
            .as_ref()
            .expect("rebuilt");
        assert_eq!(
            note.catalog,
            pdf_transform::archive::RecoveredCatalog::Kept,
            "{target}"
        );
        let catalog = document.catalog().expect("a catalog");
        assert_eq!(
            catalog.get("Lang").and_then(pdf_syntax::Object::as_string),
            Some(&b"en-US"[..]),
            "{target}: the producer's /Lang"
        );
        first_page(&document);
    }
}

#[test]
fn a_node_with_no_kids_hands_its_inheritable_entries_to_the_pages_that_name_it() {
    // Table 30 makes `/Kids` required of an intermediate node. §7.7.3.4: an inheritable attribute
    // omitted from a page "shall be inherited from an ancestor node"; the new root states none,
    // so the page is given the value its old parent stated.
    let source = file(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /MediaBox [0 0 612 792] /Rotate 90 /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ],
        "/Root 1 0 R",
    );
    for target in TARGETS {
        let (_, document, _) = recovered(&source, target);
        let page = first_page(&document);
        // The values are named by an object of their own (one per ancestor, not per page), so
        // they are read through the reference.
        let media_box: Vec<f64> = document
            .get_key(&page, "MediaBox")
            .as_array()
            .expect("the box is stated for the page")
            .iter()
            .filter_map(pdf_syntax::Object::as_number)
            .collect();
        assert_eq!(media_box, [0.0, 0.0, 612.0, 792.0], "{target}");
        assert_eq!(
            document.get_key(&page, "Rotate").as_integer(),
            Some(90),
            "{target}"
        );
        let parent = page
            .get("Parent")
            .and_then(pdf_syntax::Object::as_reference);
        let root = document
            .catalog()
            .expect("a catalog")
            .get("Pages")
            .and_then(pdf_syntax::Object::as_reference);
        assert_eq!(parent, root, "{target}: /Parent names the new root");
    }
}

#[test]
fn the_loss_withheld_leaves_the_refusal_where_it_was() {
    let source = file(
        &["42", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>", PAGE],
        "/Root 1 0 R",
    );
    for target in TARGETS {
        let refusal = attempt(&source, target, Authorisations::default())
            .expect_err("nothing authorised, nothing rebuilt");
        assert!(
            matches!(refusal, pdf_transform::Refusal::Reconstructed(_)),
            "{target}: {refusal}"
        );
    }
}

#[test]
fn the_shipped_profiles_authorise_it_and_the_others_do_not() {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../doc/profiles/");
    for (profile, expected) in [
        ("as-if-printed", true),
        ("as-shown", true),
        ("refuse-any-loss", false),
        ("keep-everything", false),
    ] {
        let text = std::fs::read_to_string(format!("{directory}{profile}.toml")).expect("profile");
        for target in TARGETS {
            let configuration =
                pdf_transform::archive::Configuration::read(&text, target).expect("it reads");
            assert_eq!(
                configuration
                    .authorisations(target)
                    .grants(Loss::RecoveredStructure),
                expected,
                "{profile} {target}"
            );
        }
    }
}

/// The path of a document of the corpora under `doc/`, or `None` where the submodule is not
/// checked out (such a test passes and says so, as `doc/environment.md` describes).
fn corpus(path: &str) -> Option<Vec<u8>> {
    let full = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    if let Ok(bytes) = std::fs::read(&full) {
        Some(bytes)
    } else {
        eprintln!("{path} is not checked out; not checked");
        None
    }
}

/// The sentence the refusal of `path` states with the loss authorised.
fn refused_by_name(path: &str) -> Option<String> {
    let source = corpus(path)?;
    match attempt(&source, Target::Two(Level::B), authorised()) {
        Err(pdf_transform::Refusal::Reconstructed(sentence)) => Some(sentence),
        other => panic!(
            "{path}: expected a refusal by name, got {:?}",
            other.map(|(r, _)| r.refused.len())
        ),
    }
}

/// A source whose catalog names no page tree, with one page.
const DANGLING: [&str; 3] = [
    "<< /Type /Catalog /Pages 9 0 R >>",
    "<< /Type /Pages /Count 1 >>",
    PAGE,
];

/// `bytes` written again with ISO 32000-2 §7.6's standard handler, the user password empty.
///
/// The objects keep their numbers and entries, so a catalog that named no page tree still names
/// none; only the strings and streams are ciphertext.
fn encrypted(bytes: &[u8]) -> Vec<u8> {
    use pdf_syntax::serialize::{Assembly, Form, Options};
    let source = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let mut assembly = Assembly::new(vec![&source]);
    for number in 1..=64 {
        let id = ObjectId::new(number, 0);
        if matches!(source.get(id), pdf_syntax::Object::Null) {
            break;
        }
        assembly.copy(0, id).expect("the assembly takes it");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut out = Vec::new();
    pdf_syntax::serialize_encrypted(
        &assembly,
        pdf_syntax::Version { major: 1, minor: 7 },
        Options::new(Form::of(&source)),
        &pdf_syntax::Protection::owner_only("owner"),
        &mut pdf_syntax::SystemEntropy,
        &mut out,
    )
    .expect("the fixture is encrypted");
    out
}

/// The loss of the structure and `more`, authorised.
fn authorised_with(more: &[Loss]) -> Authorisations {
    let mut authorised = authorised();
    for loss in more {
        authorised.authorise(*loss);
    }
    authorised
}

#[test]
fn an_encrypted_source_is_rebuilt_behind_the_encryption_loss_and_the_claim_is_the_source_s() {
    // ISO 32000-2 §7.6.4.2's Table 22 flags are the source's; the rebuilt file is unencrypted and
    // meets ISO 19005's no-encryption rule, so the loss is raised from the source
    // (`doc/adr/1187`, `doc/adr/2523`).
    let source = encrypted(&file(&DANGLING, "/Root 1 0 R"));
    assert!(
        Document::open_with_limits(source.clone(), Limits::DEFAULT)
            .expect("opens")
            .is_encrypted()
    );
    // Part 4 alone: the only handler this tree writes is §7.6.4.4's, a PDF 2.0 construct, and at
    // part 2 the file-header row refuses a 2.0 header, which is a different requirement's answer.
    let target = Target::Four(Flavour::Plain);
    let (report, output) =
        attempt(&source, target, authorised_with(&[Loss::Encryption])).expect("applies");
    let output = written(&report, output);
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{target}"
    );
    let written = Document::open_with_limits(output, Limits::DEFAULT).expect("opens");
    assert!(
        !written.is_encrypted(),
        "{target}: the output states no /Encrypt"
    );
    let converted = conversion(&report);
    assert!(converted.recovered_structure.is_some(), "{target}");
    assert!(
        matches!(
            decision(&report, "file-structure/no-encryption"),
            Decision::Authorised {
                rewrite: Rewrite::EncryptionRemoved,
                ..
            }
        ),
        "{target}"
    );
    assert!(
        !converted
            .conformed
            .contains(&"file-structure/no-encryption"),
        "{target}: the source did not meet it"
    );
    assert!(
        converted.protection.is_some(),
        "{target}: the claim the source's encryption made is reported"
    );
    // The loss withheld, it stops the conversion as it does for a file that is not rebuilt.
    let (report, output) = attempt(&source, target, authorised()).expect("applies");
    assert!(output.is_none(), "{target}");
    assert!(matches!(
        decision(&report, "file-structure/no-encryption"),
        Decision::Unauthorised { .. }
    ));
}

/// The widget, appearance and signature value of a signed field, as objects 4, 5 and 6, beside a
/// page that lists the widget (the layout of `a_signed_form`, without the lock and permissions).
const SIGNED: [&str; 6] = [
    "<< /Type /Catalog /Pages 9 0 R /AcroForm << /Fields [4 0 R] /SigFlags 3 >> >>",
    "<< /Type /Pages /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Annots [4 0 R] >>",
    "<< /Type /Annot /Subtype /Widget /FT /Sig /T (Approval) /F 4 /Rect [10 10 90 90] \
     /AP << /N 5 0 R >> /V 6 0 R >>",
    "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length 0 >>\nstream\n\nendstream",
    "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached \
     /ByteRange [0 100 200 100] /Contents <0102> /Name (Test Signer) /M (D:20260912120000Z) >>",
];

#[test]
fn a_signed_source_is_rebuilt_and_each_signature_is_reported_against_the_source() {
    // §12.8.1: a signature is a statement about the bytes it covered, which are the source's. The
    // rebuilt file's are others, so the report is read from the source and the value is removed
    // from what is written (`doc/adr/2523`).
    let source = file(&SIGNED, "/Root 1 0 R");
    for target in TARGETS {
        let (report, output) = attempt(
            &source,
            target,
            authorised_with(&[Loss::SignatureAssertion]),
        )
        .expect("applies");
        let output = written(&report, output);
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target}"
        );
        let converted = conversion(&report);
        assert!(converted.recovered_structure.is_some(), "{target}");
        let signed = converted
            .signatures
            .as_ref()
            .expect("the signature is reported");
        assert!(
            matches!(signed.decision, Decision::Authorised { .. }),
            "{target}: {:?}",
            signed.decision
        );
        assert_eq!(signed.each.len(), 1, "{target}");
        assert_eq!(
            signed.each[0].name.as_deref(),
            Some("Test Signer"),
            "{target}"
        );
        assert!(
            signed.changed >= 1,
            "{target}: the value was removed from the file written"
        );
        assert!(
            !output.windows(11).any(|window| window == b"Test Signer"),
            "{target}: no signature value is left in the output"
        );
        // The loss withheld, the signature's question stops the conversion.
        let (report, output) = attempt(&source, target, authorised()).expect("applies");
        assert!(output.is_none(), "{target}");
        assert!(matches!(
            conversion(&report)
                .signatures
                .as_ref()
                .expect("reported")
                .decision,
            Decision::Unauthorised { .. }
        ));
    }
}

#[test]
fn a_page_whose_dictionary_does_not_parse_whole_is_refused_by_name() {
    if let Some(sentence) = refused_by_name("doc/pdf.js/test/pdfs/poppler-742-0-fuzzed.pdf") {
        assert!(sentence.contains("does not parse whole"), "{sentence}");
    }
}

#[test]
fn a_page_written_directly_in_its_kids_is_given_an_object_and_inherits_from_its_parent() {
    // Table 30 makes /Kids "[a]n array of indirect references", so a page written inside it is
    // the producer's departure; the rebuild gives it the object the table asks for and reads
    // §7.7.3.4's inheritable entries from the node its /Parent names.
    let source = file(
        &[
            "<< /Kids [<< /Type /Page /Parent 1 0 R /Contents 2 0 R >>] \
             /MediaBox [0 0 200 300] /Resources << >> >>",
            "<< /Length 12 >>\nstream\n0 0 10 10 re f\nendstream",
        ],
        "/Root << /Pages 1 0 R >>",
    );
    for target in TARGETS {
        let (report, document, _) = recovered(&source, target);
        assert!(
            conversion(&report).recovered_structure.is_some(),
            "{target}"
        );
        let page = first_page(&document);
        let media = document.get_key(&page, "MediaBox");
        assert!(
            media.as_array().is_some_and(|boxes| boxes.len() == 4),
            "{target}: the page states the /MediaBox it inherited"
        );
        let pages = pdf_model::Pages::new(&document);
        assert_eq!(pages.len(), 1, "{target}");
        assert!(
            pages.get(0).and_then(|page| page.id).is_some(),
            "{target}: the page is an object of its own"
        );
    }
}

#[test]
fn a_direct_page_naming_no_parent_states_the_values_the_reader_resolved_for_it() {
    // The nodes above the page are not named by the page, but the `/Kids` that holds it is the way
    // the reader reached it, and §7.7.3.4's inheritable entries it applied (`/MediaBox`, `/Rotate`)
    // are stated on the page the rebuild writes.
    let source = file(
        &[
            "<< /Kids [<< /Type /Page /Contents 2 0 R >>] /MediaBox [0 0 200 300] /Rotate 90 \
             /Resources << >> >>",
            "<< /Length 12 >>\nstream\n0 0 10 10 re f\nendstream",
        ],
        "/Root << /Pages 1 0 R >>",
    );
    for target in TARGETS {
        let (report, document, _) = recovered(&source, target);
        assert!(
            conversion(&report).recovered_structure.is_some(),
            "{target}"
        );
        let page = first_page(&document);
        let media = document.get_key(&page, "MediaBox");
        assert_eq!(
            media.as_array().map(<[_]>::len),
            Some(4),
            "{target}: the page states the /MediaBox the reader applied"
        );
        assert_eq!(
            document.get_key(&page, "Rotate").as_integer(),
            Some(90),
            "{target}"
        );
        let pages = pdf_model::Pages::new(&document);
        let drawn = pages.get(0).expect("the page");
        assert!(
            (drawn.media_box[2] - 200.0).abs() < f32::EPSILON
                && (drawn.media_box[3] - 300.0).abs() < f32::EPSILON,
            "{target}: {:?}",
            drawn.media_box
        );
    }
}

#[test]
fn a_source_the_reader_draws_no_page_of_is_refused_by_name() {
    let source = file(
        &["42", "<< /Type /Pages /Kids [] /Count 0 >>"],
        "/Root 1 0 R",
    );
    let refusal = attempt(&source, Target::Two(Level::B), authorised())
        .expect_err("no page, nothing to write");
    let pdf_transform::Refusal::Reconstructed(sentence) = refusal else {
        panic!("another variant: {refusal}");
    };
    assert!(sentence.contains("draws no page"), "{sentence}");
}

#[test]
fn a_parent_cycle_ends_and_an_ancestor_past_the_bound_is_not_reached() {
    // The walk up a page's `/Parent` chain is bounded, and a cycle ends it at once: the page's
    // own box is the one carried, and the value 500 x 700 stated past the bound is not.
    let mut objects = vec![
        "<< /Type /Catalog /Pages 9 0 R >>".to_owned(),
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 200 300] >>".to_owned(),
        "<< /Parent 4 0 R >>".to_owned(),
        "<< /Parent 3 0 R >>".to_owned(),
    ];
    // A second page under a chain of one hundred nodes, the box stated by the last.
    objects.push("<< /Type /Page /Parent 6 0 R >>".to_owned());
    for index in 0..100 {
        let parent = if index == 99 {
            String::new()
        } else {
            format!("/Parent {} 0 R", 7 + index)
        };
        let boxed = if index == 99 {
            "/MediaBox [0 0 500 700]"
        } else {
            ""
        };
        objects.push(format!("<< /Type /Pages {parent} {boxed} >>"));
    }
    let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
    let source = file(&objects, "/Root 1 0 R");
    let (report, document, _) = recovered(&source, Target::Two(Level::B));
    let note = conversion(&report)
        .recovered_structure
        .as_ref()
        .expect("rebuilt");
    assert_eq!(note.pages, 2);
    let pages = pdf_model::Pages::new(&document);
    let boxes: Vec<Vec<f64>> = pages
        .every()
        .into_iter()
        .flatten()
        .map(|page| page.media_box.iter().map(|&edge| f64::from(edge)).collect())
        .collect();
    assert_eq!(boxes[0], [0.0, 0.0, 200.0, 300.0], "the page's own box");
    assert_ne!(
        boxes[1],
        [0.0, 0.0, 500.0, 700.0],
        "a box one hundred ancestors up is not reached"
    );
}

/// A source whose only page carries an entry of `depth` arrays nested in one another.
fn nested_entry(depth: usize) -> Vec<u8> {
    let deep = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Deep {deep} >>");
    file(&["42", "<< /Type /Pages /Count 1 >>", &page], "/Root 1 0 R")
}

#[test]
fn a_value_nested_to_the_parsers_last_depth_is_carried_and_one_deeper_is_refused() {
    // The parser admits a value nested to 255 arrays inside a page dictionary and no deeper; the
    // value at 255 is carried, and at 256 the page's dictionary does not parse whole, which is
    // refused by name instead of followed. (`carry`'s own bound is the walk's backstop for a
    // value no parser built.)
    let (report, output) =
        attempt(&nested_entry(255), Target::Two(Level::B), authorised()).expect("it applies");
    let output = written(&report, output);
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("opens");
    assert!(
        first_page(&document).get("Deep").is_some(),
        "the deep entry is carried"
    );
    let refusal = attempt(&nested_entry(256), Target::Two(Level::B), authorised())
        .expect_err("a page that cannot be carried is not rebuilt");
    let pdf_transform::Refusal::Reconstructed(sentence) = refusal else {
        panic!("another variant: {refusal}");
    };
    assert!(sentence.contains("does not parse whole"), "{sentence}");
}

#[test]
fn a_direct_value_every_page_inherits_is_written_once_and_named_by_each() {
    // §7.7.3.4: inheritable values are inherited "as-is". One hundred pages under a node with a
    // direct `/Resources` of two thousand entries would be one hundred copies of it if each page
    // were given its own; the rebuild places the value once and each page names that object.
    let mut entries = String::new();
    for index in 0..2000 {
        write!(entries, "/G{index} << /CA 1 >> ").expect("a string takes the text");
    }
    let node = format!("<< /Type /Pages /Resources << /ExtGState << {entries}>> >> /Count 100 >>");
    let mut objects = vec!["<< /Type /Catalog /Pages 99999 0 R >>".to_owned(), node];
    objects.extend(
        (0..100).map(|_| "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>".to_owned()),
    );
    let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
    let source = file(&objects, "/Root 1 0 R");
    let (_, document, output) = recovered(&source, Target::Two(Level::B));
    assert!(
        output.len() < source.len() * 3,
        "{} bytes written from {}: the value was copied per page",
        output.len(),
        source.len()
    );
    let tree = pdf_model::Pages::new(&document);
    let named: BTreeSet<_> = tree
        .every()
        .into_iter()
        .flatten()
        .map(|page| {
            page.dict
                .get("Resources")
                .and_then(pdf_syntax::Object::as_reference)
        })
        .collect();
    assert_eq!(
        named.len(),
        1,
        "one object is every page's resources: {named:?}"
    );
    assert!(
        named.iter().all(Option::is_some),
        "named, not stated inline"
    );
}

#[test]
fn a_source_stating_more_pages_than_the_reader_reaches_is_refused_by_name() {
    // Annex F.3.3's Table F.1 makes `/N` of a linearization parameter dictionary the number of
    // pages in the document, and Table 30 makes a page tree node's `/Count` the number of leaves.
    // A source stating more than the reader drew would be written as the pages drawn and nothing
    // said of the rest.
    let linearized = file(
        &[
            "<< /Linearized 1 /N 3 /L 100 >>",
            "<< /Type /Pages /Count 1 >>",
            PAGE,
        ],
        "",
    );
    let counted = file(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 5 >>",
            PAGE,
        ],
        "/Root 1 0 R",
    );
    for (source, whose) in [
        (linearized, "linearization parameter dictionary's /N"),
        (counted, "page tree root's /Count"),
    ] {
        let refusal = attempt(&source, Target::Two(Level::B), authorised())
            .expect_err("pages the reader did not reach are not dropped silently");
        let pdf_transform::Refusal::Reconstructed(sentence) = refusal else {
            panic!("another variant: {refusal}");
        };
        assert!(sentence.contains(whose), "{sentence}");
    }
}
