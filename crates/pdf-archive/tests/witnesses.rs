//! Real documents veraPDF fails and this checker once passed, each held to the row it fails.
//!
//! A gap shared by the checker and the converter is invisible to both: the converter writes the
//! file, the checker passes it, and only a second reading — veraPDF's, at a merge — says otherwise.
//! Each entry below is the source document of such a finding, and the test asserts that the row
//! the gap belonged to now fails it at both targets. The documents are in the read corpora under
//! `doc/` — `doc/corpora` and pdf.js's `doc/pdf.js` — which are not part of a checkout, so the test
//! is ignored. An absent optional repository is named and skipped; a missing or unreadable named
//! witness beneath a present repository fails. A fixture pins each row in the crate's own tests;
//! this is the evidence that the fixture's shape is the real file's.
//!
//! Every clause is cited and none is quoted: `crate`'s module comment says why.

use std::path::Path;

use pdf_archive::{Flavour, Level, Outcome, Target, check};
use pdf_syntax::Document;

/// A witness: the document's path under `doc/`, and the rows it must fail.
struct Witness {
    /// Optional repository root under `doc/`; corpora are checked out independently.
    repository: &'static str,
    path: &'static str,
    rows: &'static [&'static str],
    why: &'static str,
}

static WITNESSES: &[Witness] = &[
    Witness {
        repository: "corpora/pdfbox",
        path: "corpora/pdfbox/pdfbox/src/test/resources/org/apache/pdfbox/pdmodel/\
               PDFBOX-6040-nodeloop.pdf",
        rows: &[
            "file-structure/page-tree-root-states-no-parent",
            "file-structure/page-tree-kids-name-dictionaries",
        ],
        why: "ISO 32000-2 section 7.7.3.2's Table 30, at section 5.1 of both parts: the root names \
              itself as its parent and lists a kid the file does not hold (ADR 2558, ADR 2583)",
    },
    Witness {
        repository: "corpora/format-corpus",
        path: "corpora/format-corpus/pdf-handbuilt-test-corpus/T02-03_006_page-object-missing.pdf",
        rows: &[
            "file-structure/page-tree-kids-name-dictionaries",
            "file-structure/page-tree-holds-a-page-it-counts",
        ],
        why: "ISO 32000-2 section 7.7.3.2's Table 30, at section 5.1 of both parts: the root counts \
              one page and its one kid is placed where another object sits, so the file holds \
              none of the pages it states (doc/questions/A775, ADR 2670)",
    },
    Witness {
        repository: "pdf.js",
        path: "pdf.js/test/pdfs/issue7544.pdf",
        rows: &["fonts/font-dictionary-states-type-font"],
        why: "ISO 32000-2 section 9.7.4.1's Table 115, at ISO 19005-2 section 6.2.11.2 and ISO \
              19005-4 section 6.2.10.2: the Type 0 font's one descendant states no Type, and its \
              place in DescendantFonts makes it the CIDFont all the same (ADR 2608)",
    },
    Witness {
        repository: "corpora/format-corpus",
        path: "corpora/format-corpus/pdf-handbuilt-test-corpus/\
               T02-04-01_005_resources-subtype-missing.pdf",
        rows: &[
            "fonts/font-dictionary-states-its-subtype",
            "fonts/font-programs-embedded",
        ],
        why: "ISO 32000-2 section 9.6.2.1's Table 109, at ISO 19005-2 section 6.2.11.2 and ISO \
              19005-4 section 6.2.10.2: the font the page shows states no Subtype, so no table \
              describes it, and it embeds no program (ADR 3409)",
    },
    Witness {
        repository: "pdf.js",
        path: "pdf.js/test/pdfs/bug1068432.pdf",
        rows: &["fonts/compact-font-reals-state-a-number"],
        why: "Adobe Technical Note #5176 section 4 through ISO 32000-2 section 9.9's Table 124, at \
              ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2: the Top DICT states \
              UnderlinePosition and UnderlineThickness as reals with no digit (ADR 3426)",
    },
    Witness {
        repository: "pdf.js",
        path: "pdf.js/test/pdfs/bug1308536.pdf",
        rows: &["fonts/compact-font-reals-state-a-number"],
        why: "the same clauses: the Private DICT states reals with no digit before the exponent \
              or the end (ADR 3426)",
    },
    Witness {
        repository: "pdf.js",
        path: "pdf.js/test/pdfs/bug866395.pdf",
        rows: &["fonts/font-programs-embedded"],
        why: "ISO 32000-2 section 9.9's Table 124, at ISO 19005-2 section 6.2.11.4.1 and ISO \
              19005-4 section 6.2.10.4.1: the /FontFile3 decodes to no bytes, so the file holds \
              no program for the font the page shows (ADR 3427)",
    },
    Witness {
        repository: "pdf.js",
        path: "pdf.js/test/pdfs/bug920426.pdf",
        rows: &["fonts/encoding-cmap-maps-codes-to-cids"],
        why: "ISO 32000-2 section 9.7.5.4 c), at ISO 19005-2 section 6.2.11.3.3 and ISO 19005-4 \
              section 6.2.10.3.3: the Type 0 font's embedded Encoding CMap maps every code with \
              beginbfchar (ADR 3428)",
    },
];

#[test]
// not a gate: the witnesses are documents under doc/corpora and doc/pdf.js, which no checkout
// carries; the list is run by hand when a checker row changes its reading, beside the veraPDF
// comparison (ADRs 2582, 2583).
#[ignore = "needs doc/corpora and doc/pdf.js, which are not part of a checkout"]
fn each_witness_fails_the_row_its_gap_belonged_to() {
    let documents = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut read = 0_usize;
    for witness in WITNESSES {
        let repository = documents.join(witness.repository);
        if !repository.is_dir() {
            println!(
                "optional repository not checked out, not judged: {}",
                repository.display()
            );
            continue;
        }
        let path = documents.join(witness.path);
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "declared witness {} could not be read: {error}",
                path.display()
            )
        });
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
