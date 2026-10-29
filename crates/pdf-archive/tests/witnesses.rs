//! Real documents veraPDF fails and this checker once passed, each held to the row it fails.
//!
//! A gap shared by the checker and the converter is invisible to both: the converter writes the
//! file, the checker passes it, and only a second reading — veraPDF's, at a merge — says otherwise.
//! Each entry below is the source document of such a finding, and the test asserts that the row
//! the gap belonged to now fails it at both targets. The documents are in the read corpora under
//! `doc/corpora`, which is not part of a checkout, so the test is ignored and a missing file is
//! named rather than failed. A fixture pins each row in the crate's own tests; this is the
//! evidence that the fixture's shape is the real file's.
//!
//! Every clause is cited and none is quoted: `crate`'s module comment says why.

use std::path::Path;

use pdf_archive::{Flavour, Level, Outcome, Target, check};
use pdf_syntax::Document;

/// A witness: the document under `doc/corpora`, and the rows it must fail.
struct Witness {
    path: &'static str,
    rows: &'static [&'static str],
    why: &'static str,
}

static WITNESSES: &[Witness] = &[Witness {
    path: "pdfbox/pdfbox/src/test/resources/org/apache/pdfbox/pdmodel/PDFBOX-6040-nodeloop.pdf",
    rows: &[
        "file-structure/page-tree-root-states-no-parent",
        "file-structure/page-tree-kids-name-dictionaries",
    ],
    why: "ISO 32000-2 section 7.7.3.2's Table 30, at section 5.1 of both parts: the root names \
          itself as its parent and lists a kid the file does not hold (ADR 2558, ADR 2583)",
}];

#[test]
// not a gate: the witnesses are documents under doc/corpora, which no checkout carries; the list is
// run by hand when a checker row changes its reading, beside the veraPDF comparison (ADRs 2582, 2583).
#[ignore = "needs doc/corpora, which is not part of a checkout"]
fn each_witness_fails_the_row_its_gap_belonged_to() {
    let corpora = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/corpora");
    let mut read = 0_usize;
    for witness in WITNESSES {
        let path = corpora.join(witness.path);
        let Ok(bytes) = std::fs::read(&path) else {
            println!("not here, not judged: {}", path.display());
            continue;
        };
        let document = Document::open(bytes)
            .unwrap_or_else(|error| panic!("{} does not open: {error}", witness.path));
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let failed: Vec<&str> = check(&document, target)
                .judgements
                .into_iter()
                .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
                .map(|judgement| judgement.id)
                .collect();
            for row in witness.rows {
                assert!(
                    failed.contains(row),
                    "{} must fail {row} under {target:?} — {}; what failed was {failed:?}",
                    witness.path,
                    witness.why
                );
            }
        }
        read = read.saturating_add(1);
    }
    println!("{read} of {} witnesses read", WITNESSES.len());
}
