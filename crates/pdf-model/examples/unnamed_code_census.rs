//! What the codes ISO 32000-2 §9.10.2 cannot name are made of, by which method could have named
//! them.
//!
//! ADR 0311 gave the refusal a voice and left it as one number: the codes a corpus page shows that
//! no method of the clause can name. A number cannot say whether a reader lost them to the answer
//! the clause states in its own words — "there is no way to determine what the character code
//! represents" — or to a route the clause states and this program does not walk, and those two
//! have different consequences (`CLAUDE.md` principle 5). This is that split, over whatever
//! documents are named on the command line.
//!
//! The population is the corpus gate's own: **page one**, and only where the page **reports
//! nothing**, because a document whose font already says "no outline for any code this page shows"
//! is not silent about it. `pdf-model/tests/corpus.rs` prints the total this breaks down.
//!
//! ```sh
//! cargo run --profile gates -p pdf-model --example unnamed_code_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! `PDFVIEWER_TRACE_UNNAMED_CODE=1` on any of the runs above names every code on stderr with the
//! font it was shown in, which is where the glyph name behind an `UnlistedName` is legible — the
//! one variant where what the name *is* decides whose gap it is.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::content::UnnamedCodes;
use pdf_syntax::Document;

/// One column of the census: what to call a [`pdf_font::NamingGap`] variant, and how to read its
/// count out of a page's tally.
///
/// One row of [`CAUSES`]. Named because the table is walked three times and the pair is easier to
/// read with a name on it than as the type it is.
///
/// A table rather than six blocks because every column is printed the same way, and because the
/// order is the clause's own priority order — `/ToUnicode` first, the glyph name second, the
/// character collection third, and what is left over where none of the three applied.
type Cause = (&'static str, fn(&UnnamedCodes) -> usize);

const CAUSES: [Cause; 6] = [
    ("a mapping that answers with no characters", |counts| {
        counts.empty_mapping
    }),
    ("a /ToUnicode that omits the code", |counts| {
        counts.incomplete_to_unicode
    }),
    ("a glyph name neither list holds", |counts| {
        counts.unlisted_name
    }),
    ("a collection with nothing for the CID", |counts| {
        counts.unnamed_cid
    }),
    ("an Identity ordering and no /ToUnicode", |counts| {
        counts.unaddressable_cid
    }),
    ("a glyph selected by code that nothing names", |counts| {
        counts.unnamed_glyph
    }),
];

/// How many documents are named under each cause.
const WORST: usize = 10;

fn main() {
    let mut documents = 0_usize;
    let mut silent = 0_usize;
    let mut total = UnnamedCodes::default();
    let mut by_document: Vec<(String, UnnamedCodes)> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            continue;
        };
        let interpretation = pdf_model::interpret(&document, &page);
        if !interpretation.is_complete() {
            continue;
        }
        silent = silent.saturating_add(1);
        let counts = interpretation.codes_without_a_character;
        if counts.total() == 0 {
            continue;
        }
        total = sum(total, counts);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        by_document.push((name, counts));
    }

    println!(
        "{documents} document(s) opened, {silent} page ones that report nothing, {} of those \
         showing a code §9.10.2 could not name",
        by_document.len()
    );
    println!(
        "{} code(s) over {} document(s), by which method could have named them:",
        total.total(),
        by_document.len()
    );
    for (label, read) in CAUSES {
        let count = read(&total);
        if count == 0 {
            println!("  {count:6}  {label}");
            continue;
        }
        println!("  {count:6}  {label}");
        let mut worst: Vec<(&str, usize)> = by_document
            .iter()
            .map(|(name, counts)| (name.as_str(), read(counts)))
            .filter(|(_, count)| *count > 0)
            .collect();
        worst.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        println!("          in {} document(s):", worst.len());
        for (name, count) in worst.iter().take(WORST) {
            println!("          {count:6} {name}");
        }
    }

    let mut heaviest = by_document.clone();
    heaviest.sort_by(|a, b| b.1.total().cmp(&a.1.total()).then_with(|| a.0.cmp(&b.0)));
    println!("the heaviest documents, and what each is made of:");
    for (name, counts) in heaviest.iter().take(WORST) {
        let split: Vec<String> = CAUSES
            .iter()
            .filter_map(|(label, read)| {
                let count = read(counts);
                (count > 0).then(|| format!("{count} {label}"))
            })
            .collect();
        println!("  {:6} {name}: {}", counts.total(), split.join("; "));
    }
}

/// Adds one page's tally to a running one, field by field.
///
/// Written out rather than derived so that a field added to [`UnnamedCodes`] fails to compile
/// here — a census that silently drops a population is worse than no census.
fn sum(left: UnnamedCodes, right: UnnamedCodes) -> UnnamedCodes {
    let UnnamedCodes {
        empty_mapping,
        incomplete_to_unicode,
        unlisted_name,
        unnamed_cid,
        unaddressable_cid,
        unnamed_glyph,
    } = right;
    UnnamedCodes {
        empty_mapping: left.empty_mapping.saturating_add(empty_mapping),
        incomplete_to_unicode: left
            .incomplete_to_unicode
            .saturating_add(incomplete_to_unicode),
        unlisted_name: left.unlisted_name.saturating_add(unlisted_name),
        unnamed_cid: left.unnamed_cid.saturating_add(unnamed_cid),
        unaddressable_cid: left.unaddressable_cid.saturating_add(unaddressable_cid),
        unnamed_glyph: left.unnamed_glyph.saturating_add(unnamed_glyph),
    }
}
