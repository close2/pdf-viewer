//! Audit 2260 of ISO 32000-2 Annexes D, F and I against `pdf-transform`: what a reader takes from
//! a linearization parameter dictionary the file's length no longer matches, and the bytes a
//! writer that formats a text string by hand gives a character Table D.3 does not put there.
//!
//! Each test pairs a control that passes on main with the case the audit found.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]

use pdf_archive::{Level, Target};
use pdf_model::page_label::PageLabels;
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{ArchivePlan, Authorisations, FormAnswers, Loss};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::range::Selection;
use pdf_transform::tool::ToolOutputs;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Refusal, Source, apply};

/// The plan the archive tests build, converting source 0 to `target` with `authorised`.
fn plan(target: Target, authorised: Authorisations) -> ArchivePlan {
    let config = pdf_transform::archive::Configuration::read("", target).expect("it reads");
    ArchivePlan {
        source: 0,
        names: "out.pdf".parse().expect("a pattern"),
        target,
        authorised,
        profile: None,
        press: None,
        substitute_fonts: true,
        supplied_fonts: std::collections::BTreeMap::new(),
        cjk_pack: None,
        lgc_pack: None,
        departures: Vec::new(),
        claim_conformance: false,
        forms: FormAnswers::default(),
        derivations: config.derivations(target),
        supplies: config.supplies(target),
        preservations: config.preservations(target),
        annotation_flags: config.annotation_flags,
        resolutions: config.resolutions(target),
        tool_outputs: ToolOutputs::new(),
        external_data: std::collections::BTreeMap::new(),
        clock: pdf_transform::archive::Clock::System,
    }
}

/// A file whose first object is a linearization parameter dictionary stating three pages, whose
/// trailer names no `/Root` (so the archive verb rebuilds its structure), holding one page.
///
/// `/L` is written as ten digits so that the file's length does not depend on its value; where
/// `true_length` is set it is the file's own length, and otherwise one byte more.
fn source(true_length: bool) -> Vec<u8> {
    let build = |length: usize| {
        let objects = [
            format!("<< /Linearized 1 /N 3 /L {length:010} >>"),
            "<< /Type /Pages /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>".to_owned(),
        ];
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for offset in offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!("trailer\n<< /Size 4 >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        bytes
    };
    let length = build(0).len();
    let bytes = build(if true_length { length } else { length + 1 });
    assert_eq!(bytes.len(), length, "the length does not move with /L");
    bytes
}

/// The archive conversion to PDF/A-2b with `recovered-structure` authorised.
fn convert(source: Vec<u8>) -> Result<(), Refusal> {
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::RecoveredStructure);
    let sinks = MemorySinks::new();
    apply(
        &Plan::Archive(plan(Target::Two(Level::B), authorised)),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .map(|_| ())
}

/// Table F.1, `/L`: "It shall be exactly equal to the actual length of the PDF file. A mismatch
/// indicates that the file is not linearized and shall be treated as ordinary PDF file, ignoring
/// linearization information." `/N` is linearization information; in a file whose `/L` does not
/// match, it states nothing, and the one page the reader reaches is every page the file states
/// (the root's `/Count` is 1).
#[test]
fn a_linearization_dictionary_whose_length_mismatches_states_no_page_count_spec_audit() {
    // Control: `/L` is the file's length, the file is linearized, and its `/N` of 3 is a count
    // the one page the reader reaches falls short of — refused by name.
    match convert(source(true)) {
        Err(Refusal::Reconstructed(sentence)) => assert!(
            sentence.contains("linearization parameter dictionary's /N"),
            "control: {sentence}"
        ),
        other => panic!("control: refused for the count, got {other:?}"),
    }

    // `/L` is one byte off: the dictionary is to be ignored, so no count exceeds the page held.
    if let Err(refusal) = convert(source(false)) {
        panic!(
            "Table F.1: a mismatched /L makes the file ordinary and its /N is ignored, so the \
             one page held is all the file states — but the conversion refused: {refusal}"
        );
    }
}

/// A one-page file whose `/PageLabels` gives page 0 the prefix `prefix`, a string's bytes.
fn labelled(prefix: &str) -> Vec<u8> {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /PageLabels << /Nums [0 << /P {prefix} >>] >> >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Resources << >> >>".to_owned(),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

/// The label of page 0 as a reader finds it in `bytes`.
fn first_label(bytes: Vec<u8>) -> Option<String> {
    let document = Document::open_with_limits(bytes, Limits::DEFAULT).expect("it opens");
    PageLabels::read(&document).label(0)
}

/// The merge of `source`'s one page, alone.
fn merged(source: Vec<u8>) -> Vec<u8> {
    let sinks = MemorySinks::new();
    let plan = Plan::Merge(MergePlan {
        inputs: vec![Input {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
        }],
        collate: false,
        information: Vec::new(),
        names: "merged.pdf".parse().expect("a pattern"),
    });
    apply(
        &plan,
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("it merges");
    sinks.into_outputs().remove(0).1
}

/// §7.9.2.2's text string is `PDFDocEncoding` where it has no Unicode prefix, and Annex D's
/// Table D.3 gives code 0x1F the character U+02DC SMALL TILDE, codes 0x18 to 0x1F being the
/// eight accents (Errata Collection 3's Issue #562 restores 0x18 and 0x19). A label that holds
/// U+001F, which a UTF-16BE string can state, is therefore not one `PDFDocEncoding` can carry,
/// and a writer that puts the character's ASCII byte into an unprefixed string has written a
/// tilde. `merge` writes each page's label again (`merge::text_string`, shared with `split`).
#[test]
fn a_label_holding_a_control_character_survives_a_merge_spec_audit() {
    // Control: a label with a character outside ASCII is carried exactly.
    let accented = labelled("<FEFF004100E9>");
    assert_eq!(first_label(accented.clone()).as_deref(), Some("A\u{e9}"));
    assert_eq!(
        first_label(merged(accented)).as_deref(),
        Some("A\u{e9}"),
        "control: a non-ASCII label is written in UTF-16BE and reads back"
    );

    let control = labelled("<FEFF0041001F>");
    assert_eq!(first_label(control.clone()).as_deref(), Some("A\u{1f}"));
    assert_eq!(
        first_label(merged(control)).as_deref(),
        Some("A\u{1f}"),
        "Table D.3: byte 0x1F is U+02DC in PDFDocEncoding, so a U+001F written as that byte \
         reads back as a tilde"
    );
}
