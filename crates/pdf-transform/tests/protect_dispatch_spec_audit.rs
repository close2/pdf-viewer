//! Public whole-file transforms preserve explicitly supplied output credentials and flags.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture and assertions"
)]

use pdf_model::Pages;
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams};
use pdf_syntax::{Document, Limits, Object, Version};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::pages::PagesPlan;
use pdf_transform::range::Selection;
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{
    Access, Budget, MemorySinks, Plan, Policy, Protect, Secret, Source, apply_protected,
};
use std::collections::BTreeSet;

fn literal_two_page_pdf() -> Vec<u8> {
    let first = "BT /F1 12 Tf 20 100 Td (FIRST) Tj ET";
    let second = "BT /F1 12 Tf 20 100 Td (SECOND) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{first}\nendstream", first.len() + 1),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{second}\nendstream", second.len() + 1),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn encrypted_source(clear: &[u8]) -> Vec<u8> {
    let document =
        Document::open_with_limits(clear.to_vec(), Limits::DEFAULT).expect("fixture opens");
    assert!(!document.was_recovered(), "source fixture xref is exact");
    let mut assembly = Assembly::new(vec![&document]);
    for number in 1..=7 {
        let id = pdf_syntax::ObjectId::new(number, 0);
        assert!(
            !matches!(document.get(id), Object::Null),
            "fixture object {number} exists"
        );
        assembly.copy(0, id).expect("fixture object copies");
    }
    assembly.set_root(pdf_syntax::ObjectId::new(1, 0));
    let mut encrypted = Vec::new();
    pdf_syntax::serialize_encrypted(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::of(&document)),
        &pdf_syntax::Protection::owner_only("source-owner"),
        &mut pdf_syntax::SystemEntropy,
        &mut encrypted,
    )
    .expect("source encryption succeeds");
    encrypted
}

fn page_text(bytes: Vec<u8>, password: &str) -> (bool, bool, Vec<String>) {
    let document = Document::open_with_password(bytes, Limits::DEFAULT, password)
        .expect("credential opens output");
    let permissions = document.permissions().expect("output permissions");
    let page_tree = Pages::new(&document);
    assert!(!document.was_recovered(), "output has an exact xref");
    let mut text = Vec::new();
    for index in 0..page_tree.len() {
        let page = page_tree.get(index).expect("page exists");
        let interpreted = pdf_model::interpret(&document, &page);
        assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
        text.push(interpreted.text);
    }
    (permissions.owner, permissions.copy, text)
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "four public verbs share one input and credential/content assertion matrix"
)]
fn every_public_whole_file_verb_applies_new_credentials_and_access_flags() {
    let source_bytes = encrypted_source(&literal_two_page_pdf());
    let selection = || "1-2".parse::<Selection>().expect("valid selection");
    let plans = [
        Plan::Split(SplitPlan {
            source: 0,
            pages: selection(),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("pattern"),
        }),
        Plan::Merge(MergePlan {
            inputs: vec![Input {
                source: 0,
                pages: selection(),
            }],
            collate: false,
            information: Vec::new(),
            names: "merged.pdf".parse().expect("pattern"),
        }),
        Plan::Pages(PagesPlan {
            source: 0,
            edits: Vec::new(),
            names: "pages.pdf".parse().expect("pattern"),
        }),
        Plan::Optimize(OptimizePlan {
            source: 0,
            names: "optimized.pdf".parse().expect("pattern"),
            prune: true,
            object_streams: ObjectStreams::Disable,
            streams: Streams::Carry,
            linearize: false,
            images: None,
        }),
    ];
    let protect = Protect {
        user_password: Secret::from("new-reader".to_owned()),
        owner_password: Secret::from("new-owner".to_owned()),
        access: Access {
            copy: false,
            ..Access::ALL
        },
        encrypt_metadata: true,
    };
    for plan in plans {
        let mut seen = BTreeSet::new();
        let sinks = MemorySinks::new();
        let source = Source::with_password(
            source_bytes.clone(),
            Secret::from("source-owner".to_owned()),
        );
        apply_protected(
            &plan,
            &[&source],
            &sinks,
            &Policy::default(),
            &Budget::default(),
            Some(&protect),
        )
        .expect("the whole-file verb applies with Protect");
        let outputs = sinks.into_outputs();
        let expected_count = if matches!(plan, Plan::Split(_)) { 2 } else { 1 };
        assert_eq!(outputs.len(), expected_count, "{plan:?}");

        for (_, output) in outputs {
            assert!(
                Document::open_with_limits(output.clone(), Limits::DEFAULT).is_err(),
                "no password must fail: {plan:?}"
            );
            assert!(
                Document::open_with_password(output.clone(), Limits::DEFAULT, "wrong").is_err(),
                "wrong password must fail: {plan:?}"
            );
            assert!(
                Document::open_with_password(output.clone(), Limits::DEFAULT, "source-owner")
                    .is_err(),
                "source credential must not carry over: {plan:?}"
            );
            let (owner, copy, text) = page_text(output.clone(), "new-reader");
            assert!(
                !owner,
                "the user credential is not the owner credential: {plan:?}"
            );
            assert!(
                !copy,
                "the caller's Table 22 copy flag is propagated: {plan:?}"
            );
            assert!(
                text.iter()
                    .any(|page| page.contains("FIRST") || page.contains("SECOND")),
                "visible text survives: {plan:?} {text:?}"
            );
            for page in &text {
                if page.contains("FIRST") {
                    seen.insert("FIRST");
                }
                if page.contains("SECOND") {
                    seen.insert("SECOND");
                }
            }
            let (owner, _, owner_text) = page_text(output, "new-owner");
            assert!(owner, "the new owner credential has owner access: {plan:?}");
            assert_eq!(
                owner_text, text,
                "both credentials see the same page content: {plan:?}"
            );
        }
        assert_eq!(
            seen,
            BTreeSet::from(["FIRST", "SECOND"]),
            "each verb preserves both page texts: {plan:?}"
        );
    }
}
