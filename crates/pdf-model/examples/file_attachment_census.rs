//! §12.5.6.15's `/FS`, counted against §7.7.4's `/EmbeddedFiles` tree.
//!
//! A file attachment annotation "contains a reference to a file, which typically shall be
//! embedded in the PDF file", and Table 187 makes `/FS` **required**. §7.11.4.1 gives an
//! embedded file two homes — a file specification's own `/EF`, and the document-wide
//! `/EmbeddedFiles` name tree — so an annotation's file is only reachable from a list built
//! out of that tree when the document happens to file it there as well.
//!
//! This counts the population that decides whether reading `/FS` shows a person a file they
//! could not otherwise reach: annotations of subtype `FileAttachment`, how many name an
//! embedded stream, how many of those streams the name tree also names, and how many state
//! Table 172's `/Contents` beside a `/Desc` in the file specification — which is the pair
//! §12.5.6.15's one `shall` decides between.
//!
//! ```sh
//! cargo run --release -p pdf-model --example file_attachment_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object, Stream};

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Annotations seen at all.
    seen: usize,
    /// `/Subtype /FileAttachment` annotations.
    attachments: usize,
    /// Of those, the ones whose `/FS` resolves to a dictionary.
    with_specification: usize,
    /// Of those, the ones whose specification carries an `/EF` stream.
    embedded: usize,
    /// Of those, the ones whose stream the `/EmbeddedFiles` tree also names.
    also_in_the_tree: usize,
    /// Of those, the ones stating Table 172's `/Contents`.
    with_contents: usize,
    /// Of those, the ones whose specification also states Table 43's `/Desc`.
    with_contents_and_desc: usize,
    /// Of those, the ones where the two texts differ — where the `shall` decides a value.
    contents_differs_from_desc: usize,
    /// Files the `/EmbeddedFiles` tree names, for scale.
    in_the_tree: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.seen = self.seen.saturating_add(other.seen);
        self.attachments = self.attachments.saturating_add(other.attachments);
        self.with_specification = self
            .with_specification
            .saturating_add(other.with_specification);
        self.embedded = self.embedded.saturating_add(other.embedded);
        self.also_in_the_tree = self.also_in_the_tree.saturating_add(other.also_in_the_tree);
        self.with_contents = self.with_contents.saturating_add(other.with_contents);
        self.with_contents_and_desc = self
            .with_contents_and_desc
            .saturating_add(other.with_contents_and_desc);
        self.contents_differs_from_desc = self
            .contents_differs_from_desc
            .saturating_add(other.contents_differs_from_desc);
        self.in_the_tree = self.in_the_tree.saturating_add(other.in_the_tree);
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();
    // The document whose page walk took longest, and what it took cold and warm.
    let mut slowest = (String::new(), 0_u128, 0_u128);

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        // What the walk costs, which is the question `viewer_core`'s attachment query asks: it
        // reads every page's `/Annots` on demand, and a thousand-page document is where that
        // shows. Five runs, median, for ADR 0222's reason about wall clocks on this machine.
        let mut runs: Vec<u128> = Vec::with_capacity(5);
        for _ in 0..5_u8 {
            let began = std::time::Instant::now();
            let files = every_attached_file(&document);
            runs.push(began.elapsed().as_micros());
            drop(files);
        }
        // The first run is the one a launch path would pay: every page object is parsed for the
        // first time. The rest answer out of `Document`'s object cache.
        let cold = runs.first().copied().unwrap_or_default();
        let mut warm = runs.clone();
        warm.sort_unstable();
        let median = warm.get(2).copied().unwrap_or_default();
        if cold > slowest.1 {
            slowest = (
                path.rsplit('/').next().unwrap_or(&path).to_owned(),
                cold,
                median,
            );
        }
        let counts = document_counts(&document);
        if counts.attachments > 0 {
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            lines.push(format!(
                "  {name}: {} /FileAttachment ({} with a specification, {} embedding a file, \
                 {} of those also in the /EmbeddedFiles tree, {} stating /Contents)",
                counts.attachments,
                counts.with_specification,
                counts.embedded,
                counts.also_in_the_tree,
                counts.with_contents
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened, {} annotation(s)", total.seen);
    println!("  {} of subtype /FileAttachment", total.attachments);
    println!(
        "  {} state an /FS that resolves to a dictionary, {} of which embed a file",
        total.with_specification, total.embedded
    );
    println!(
        "  {} of those embedded files are also named by /Names /EmbeddedFiles ({} files there \
         in all)",
        total.also_in_the_tree, total.in_the_tree
    );
    println!(
        "  {} state /Contents, {} of those beside a /Desc, {} of which differ",
        total.with_contents, total.with_contents_and_desc, total.contents_differs_from_desc
    );
    println!(
        "  the slowest walk of every page's /Annots was {} at {} µs cold, {} µs warm",
        slowest.0, slowest.1, slowest.2
    );
    for line in &lines {
        println!("{line}");
    }
}

/// Every file the document's file attachment annotations name, which is what a document-wide
/// *list* of them would cost a panel: one walk of the page tree and every `/Annots` array on it.
///
/// `viewer_core` deliberately does not build this — see ADR 0295 and `Query::Attachments` — and
/// this is the measurement that decided it, which is why the walk lives here rather than there.
fn every_attached_file(document: &Document) -> Vec<pdf_model::attachment::Attachment> {
    let mut out = Vec::new();
    // One walk of the page tree, not one per page: `Pages::get` descends from the root for
    // each index, which cost 870 ms warm over ISO 32000-2's 1023 pages where this costs 14.
    let mut pages: Vec<(usize, pdf_syntax::ObjectId)> = pdf_model::Pages::new(document)
        .indices()
        .into_iter()
        .map(|(id, index)| (index, id))
        .collect();
    pages.sort_unstable();
    for (_, id) in pages {
        let page = document.get(id);
        let Some(page) = page.as_dict() else {
            continue;
        };
        let annotations = document.get_key(page, "Annots");
        let Some(annotations) = annotations.as_array() else {
            continue;
        };
        for entry in annotations {
            let resolved = document.resolve(entry);
            let Some(annotation) = resolved.as_dict() else {
                continue;
            };
            if document
                .get_key(annotation, "Subtype")
                .as_name()
                .map(pdf_syntax::Name::as_bytes)
                != Some(b"FileAttachment")
            {
                continue;
            }
            out.extend(pdf_model::attachment::of_annotation(document, annotation));
        }
    }
    out
}

/// Walks every annotation on every page of one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let tree: Vec<Arc<Stream>> = pdf_model::attachment::attachments(document)
        .into_iter()
        .map(|file| file.stream)
        .collect();
    counts.in_the_tree = tree.len();

    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            counts.seen = counts.seen.saturating_add(1);
            let subtype = document.get_key(annotation, "Subtype");
            if subtype.as_name().map(pdf_syntax::Name::as_bytes) != Some(b"FileAttachment") {
                continue;
            }
            counts.attachments = counts.attachments.saturating_add(1);
            let specification = document.get_key(annotation, "FS");
            let Some(specification) = specification.as_dict() else {
                continue;
            };
            counts.with_specification = counts.with_specification.saturating_add(1);
            let Some(file) = pdf_model::attachment::of_annotation(document, annotation) else {
                continue;
            };
            counts.embedded = counts.embedded.saturating_add(1);
            if tree.iter().any(|named| Arc::ptr_eq(named, &file.stream)) {
                counts.also_in_the_tree = counts.also_in_the_tree.saturating_add(1);
            }
            let Some(contents) = text(document, annotation, "Contents") else {
                continue;
            };
            counts.with_contents = counts.with_contents.saturating_add(1);
            if let Some(description) = text(document, specification, "Desc") {
                counts.with_contents_and_desc = counts.with_contents_and_desc.saturating_add(1);
                if description != contents {
                    counts.contents_differs_from_desc =
                        counts.contents_differs_from_desc.saturating_add(1);
                }
            }
        }
    }
    counts
}

/// One text-string entry, empty read as absent.
fn text(document: &Document, dict: &Dictionary, key: &str) -> Option<String> {
    match document.get_key(dict, key) {
        Object::String(bytes) => {
            let text = pdf_syntax::text_string(&bytes);
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}
