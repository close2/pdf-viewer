//! §14.3.2's *object-level* metadata: a `/Metadata` stream on something other than the catalog.
//!
//! §14.3.2 attaches metadata to an object "through the Metadata entry in a stream or dictionary
//! representing the object" (Table 348), and NOTE 1 names the components it is most likely to sit
//! on — an ICC profile stream, a font file stream, a form `XObject`, an image, a shading. This
//! answers **how often the corpus states one, on what kind of object, and whether `Xmp::read`
//! reads it back** — the same reader §14.3.2 uses for the catalog, which takes a dictionary
//! rather than a document precisely so that any of Table 348's carriers can be handed to it.
//!
//! The point of the count is trap 13's calibration: the §14.6.2 and §14.3.2 rows say object-level
//! metadata has a reader but no in-scope consumer, and the honest denominator for "no consumer is
//! owed" is how many documents put one where a consumer would look — a form `XObject`, an image, an
//! annotation — versus the carriers (`/Metadata` on a catalog or a page) that are already read.
//!
//! ```sh
//! cargo run --release -p pdf-model --example object_metadata_census -- --pdfjs
//! cargo run --release -p pdf-model --example object_metadata_census -- --crawl
//! ```
//!
//! The walk is the whole object graph rather than the carriers §14.3.2 lists, because a
//! `/Metadata` entry means the same thing wherever it hangs and enumerating the carriers would
//! measure this program's reach instead of the corpus's contents (the reason `associated_file_
//! census` walks the graph too).

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use pdf_model::xmp::Xmp;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

/// How a `/Metadata`-bearing dictionary is classified, by what the object it describes is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Carrier {
    /// The document catalog — §7.7.2's document-level packet, read by `Xmp::document`.
    Catalog,
    /// A page or the page tree — read by the properties panel and `pdf-vfs`.
    Page,
    /// A form `XObject` (`/Type /XObject`, `/Subtype /Form`) — object-level, no in-scope consumer.
    FormXObject,
    /// An image `XObject` (`/Subtype /Image`) — object-level.
    Image,
    /// An annotation (`/Type /Annot`) — object-level.
    Annotation,
    /// Any other stream or dictionary: a font file, an ICC profile, a shading, an embedded file.
    Other,
}

impl Carrier {
    /// The label a total is printed under.
    fn label(self) -> &'static str {
        match self {
            Carrier::Catalog => "catalog (document-level, read)",
            Carrier::Page => "page (read)",
            Carrier::FormXObject => "form XObject (object-level)",
            Carrier::Image => "image XObject (object-level)",
            Carrier::Annotation => "annotation (object-level)",
            Carrier::Other => "other stream/dictionary (object-level)",
        }
    }

    /// The six categories, in report order.
    const ALL: [Carrier; 6] = [
        Carrier::Catalog,
        Carrier::Page,
        Carrier::FormXObject,
        Carrier::Image,
        Carrier::Annotation,
        Carrier::Other,
    ];
}

/// What one document contributes: for each carrier, how many `/Metadata` streams and how many of
/// those `Xmp::read` reads back.
#[derive(Default, Clone)]
struct Counts {
    /// One `(stated, read)` pair per [`Carrier`], indexed by its position in [`Carrier::ALL`].
    seen: [(usize, usize); 6],
}

impl Counts {
    /// Records one `/Metadata`-bearing dictionary and whether it read back.
    fn tally(&mut self, carrier: Carrier, read: bool) {
        let slot = Carrier::ALL.iter().position(|c| *c == carrier).unwrap_or(5);
        self.seen[slot].0 = self.seen[slot].0.saturating_add(1);
        if read {
            self.seen[slot].1 = self.seen[slot].1.saturating_add(1);
        }
    }

    /// Whether this document states any object-level `/Metadata` — the residue's denominator.
    fn states_object_level(&self) -> bool {
        self.seen[2..].iter().any(|(stated, _)| *stated > 0)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scope: &[&str] = if args.iter().any(|a| a == "--crawl") {
        &["corpus-cache/safedocs/cc-main-2021-31"]
    } else if args.iter().any(|a| a == "--pdfjs") {
        &["doc/pdf.js/test/pdfs"]
    } else {
        &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"]
    };
    let mut files = Vec::new();
    for relative in scope {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    eprintln!("{} PDF(s) in the population", files.len());

    let measured: Vec<(String, Counts)> = files
        .par_iter()
        .map(|path| {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (label, measure(path))
        })
        .collect();

    println!("{} PDF(s) read", measured.len());
    for (position, carrier) in Carrier::ALL.iter().enumerate() {
        let stated: usize = measured.iter().map(|(_, c)| c.seen[position].0).sum();
        let read: usize = measured.iter().map(|(_, c)| c.seen[position].1).sum();
        println!("  {}: {stated} stated, {read} read back", carrier.label());
    }

    let witnesses: Vec<&(String, Counts)> = measured
        .iter()
        .filter(|(_, counts)| counts.states_object_level())
        .collect();
    println!(
        "{} document(s) state an object-level /Metadata (form XObject, image, annotation or other)",
        witnesses.len()
    );
    for (label, counts) in witnesses {
        let detail: Vec<String> = Carrier::ALL
            .iter()
            .enumerate()
            .skip(2)
            .filter(|(position, _)| counts.seen[*position].0 > 0)
            .map(|(position, carrier)| {
                format!(
                    "{} {}={} read",
                    counts.seen[position].0,
                    carrier.label(),
                    counts.seen[position].1
                )
            })
            .collect();
        println!("  {label}: {}", detail.join("; "));
    }
}

/// Reads one document's whole object graph, tallying every `/Metadata`-bearing dictionary.
fn measure(path: &Path) -> Counts {
    let mut counts = Counts::default();
    let Ok(bytes) = std::fs::read(path) else {
        return counts;
    };
    let Ok(document) = Document::open(bytes) else {
        return counts;
    };
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        // §14.3.2: the entry names a metadata *stream*, so a `/Metadata` that resolves to
        // anything else is not one and is passed over.
        if document.get_key(dict, "Metadata").as_stream().is_none() {
            continue;
        }
        let carrier = classify(&document, dict);
        // `Xmp::read` is the reader §14.3.2 uses for the catalog; it takes a dictionary so that
        // any Table 348 carrier can be handed to it. `Some(Ok(_))` is a packet read back.
        let read = matches!(Xmp::read(&document, dict), Some(Ok(_)));
        counts.tally(carrier, read);
    }
    counts
}

/// Classifies a `/Metadata`-bearing dictionary by the object it describes.
fn classify(document: &Document, dict: &Dictionary) -> Carrier {
    let type_name = document.get_key(dict, "Type");
    match type_name.as_name().map(Name::as_bytes) {
        Some(b"Catalog") => return Carrier::Catalog,
        Some(b"Pages" | b"Page") => return Carrier::Page,
        Some(b"Annot") => return Carrier::Annotation,
        Some(b"XObject") | None => {}
        Some(_) => return Carrier::Other,
    }
    match document
        .get_key(dict, "Subtype")
        .as_name()
        .map(Name::as_bytes)
    {
        Some(b"Form") => Carrier::FormXObject,
        Some(b"Image") => Carrier::Image,
        _ => Carrier::Other,
    }
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}
