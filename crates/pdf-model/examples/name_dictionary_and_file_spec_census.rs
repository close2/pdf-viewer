//! Three entries two tables define and nothing in this tree consumed: `/Names /AP`, `/Thumb`, `/EP`.
//!
//! §7.7.4's Table 32 gives the name dictionary an `/AP` tree — "[a] name tree mapping name
//! strings to annotation appearance streams (see 12.5.5, "Appearance streams")" — and §7.11.3's
//! Table 43 gives a file specification a `/Thumb` and an `/EP`. This counts the population behind
//! each, because a reader built for an entry no document states is a reader nothing calibrates
//! (trap 8), and a fixture is then the only witness there is.
//!
//! The fourth column is the one §12.5.5 decides rather than counts. The clause ends its
//! appearance-dictionary paragraph with a prohibition:
//!
//! > For convenience in managing appearance streams that are used repeatedly, the AP entry in a
//! > PDF document's name dictionary ( see 7.7.4, "Name dictionary") may contain a name tree
//! > mapping name strings to appearance streams. The name strings have no standard meanings; no
//! > PDF objects may refer to appearance streams by name.
//!
//! So an annotation whose `/AP` is a *name* is not a document using the tree; it is a document
//! breaking that sentence. The count is here so that the claim is measured rather than assumed.
//!
//! Every counter is calibrated by planting what it looks for (trap 13): a hand-built §7.6.7
//! wrapper carrying an `/EP`, a `/Thumb`, an `/AP` tree and one annotation whose `/AP` is a name
//! scores 2, 2, 1 and 1, which is what makes the zeros below sentences about the corpus rather
//! than about this file.
//!
//! ```sh
//! cargo run --release -p pdf-model --example name_dictionary_and_file_spec_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Dictionary, Document, Object, tree};
use rayon::prelude::*;

/// How many documents are named per finding before the list is cut.
const MAX_NAMED: usize = 16;

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents opened at all.
    opened: usize,
    /// Documents whose catalog states a `/Names` dictionary.
    with_name_dictionary: usize,
    /// Documents whose name dictionary states an `/AP` tree that resolves to a dictionary.
    with_ap_tree: usize,
    /// Names that tree holds, summed.
    named_appearances: usize,
    /// Of those, the ones whose value resolves to a stream — an appearance stream, as Table 32
    /// says the tree maps to.
    named_appearance_streams: usize,
    /// Annotations whose `/AP` is a name rather than Table 170's dictionary, which §12.5.5's
    /// last sentence forbids any object from being.
    annotations_naming_an_appearance: usize,
    /// File specification dictionaries reached at all — **sightings**, not distinct objects: a
    /// specification filed in the `/EmbeddedFiles` tree *and* named by a catalog `/AF` array is
    /// counted twice, which is how §7.6.7 requires an encrypted payload's to be written.
    specifications: usize,
    /// Of those, the ones stating Table 43's `/Thumb`.
    with_thumb: usize,
    /// Of those, the ones whose `/Thumb` resolves to a stream, which is the type the table gives.
    with_thumb_stream: usize,
    /// Of those, the ones stating Table 43's `/EP`.
    with_ep: usize,
    /// Of those, the ones whose `/EP` states Table 28's required `/Subtype`.
    with_ep_subtype: usize,
}

impl Counts {
    /// Adds another document's totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.with_name_dictionary = self
            .with_name_dictionary
            .saturating_add(other.with_name_dictionary);
        self.with_ap_tree = self.with_ap_tree.saturating_add(other.with_ap_tree);
        self.named_appearances = self
            .named_appearances
            .saturating_add(other.named_appearances);
        self.named_appearance_streams = self
            .named_appearance_streams
            .saturating_add(other.named_appearance_streams);
        self.annotations_naming_an_appearance = self
            .annotations_naming_an_appearance
            .saturating_add(other.annotations_naming_an_appearance);
        self.specifications = self.specifications.saturating_add(other.specifications);
        self.with_thumb = self.with_thumb.saturating_add(other.with_thumb);
        self.with_thumb_stream = self
            .with_thumb_stream
            .saturating_add(other.with_thumb_stream);
        self.with_ep = self.with_ep.saturating_add(other.with_ep);
        self.with_ep_subtype = self.with_ep_subtype.saturating_add(other.with_ep_subtype);
    }

    /// Whether this document is worth naming in the output.
    fn notable(&self) -> bool {
        self.with_ap_tree > 0
            || self.annotations_naming_an_appearance > 0
            || self.with_thumb > 0
            || self.with_ep > 0
    }
}

/// Every file specification dictionary a document reaches by a route this tree already walks.
///
/// The `/EmbeddedFiles` tree, §14.13's `/AF` arrays on the catalog and on every page, and every
/// annotation's `/FS`. Not a closed set — a specification can hang off an action or a `/RF` — but
/// it is the set a person's attachment list is built from, which is where a `/Thumb` or an `/EP`
/// would be shown.
fn specifications(document: &Document) -> Vec<Dictionary> {
    let mut out = Vec::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let names = document.get_key(&catalog, "Names");
    if let Some(names) = names.as_dict() {
        let embedded = document.get_key(names, "EmbeddedFiles");
        if let Some(embedded) = embedded.as_dict() {
            for (_, value) in tree::name_pairs(embedded, &|object| document.resolve(object)) {
                push_spec(document, &value, &mut out);
            }
        }
    }
    associated(document, &catalog, &mut out);
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        associated(document, &page.dict, &mut out);
        let annots = document.get_key(&page.dict, "Annots");
        let Some(annotations) = annots.as_array() else {
            continue;
        };
        for entry in annotations {
            let resolved = document.resolve(entry);
            let Some(annotation) = resolved.as_dict() else {
                continue;
            };
            push_spec(document, &document.get_key(annotation, "FS"), &mut out);
        }
    }
    out
}

/// Pushes `object` if it resolves to a dictionary.
fn push_spec(document: &Document, object: &Object, out: &mut Vec<Dictionary>) {
    if let Some(dict) = document.resolve(object).as_dict() {
        out.push(dict.clone());
    }
}

/// Pushes every specification in a dictionary's §14.13 `/AF` array.
fn associated(document: &Document, dictionary: &Dictionary, out: &mut Vec<Dictionary>) {
    let files = document.get_key(dictionary, "AF");
    let Some(files) = files.as_array() else {
        return;
    };
    for entry in files {
        push_spec(document, entry, out);
    }
}

/// Annotations whose `/AP` is a name, over every page.
fn appearances_named(document: &Document) -> usize {
    let pages = pdf_model::Pages::new(document);
    let mut named = 0_usize;
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annots = document.get_key(&page.dict, "Annots");
        let Some(annotations) = annots.as_array() else {
            continue;
        };
        for entry in annotations {
            let resolved = document.resolve(entry);
            let Some(annotation) = resolved.as_dict() else {
                continue;
            };
            if document.get_key(annotation, "AP").as_name().is_some() {
                named = named.saturating_add(1);
            }
        }
    }
    named
}

/// One document's tally.
fn measure(document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        annotations_naming_an_appearance: appearances_named(document),
        ..Counts::default()
    };
    if let Ok(catalog) = document.catalog() {
        let names = document.get_key(&catalog, "Names");
        if let Some(names) = names.as_dict() {
            counts.with_name_dictionary = 1;
            let appearances = document.get_key(names, "AP");
            if let Some(root) = appearances.as_dict() {
                counts.with_ap_tree = 1;
                for (_, value) in tree::name_pairs(root, &|object| document.resolve(object)) {
                    counts.named_appearances = counts.named_appearances.saturating_add(1);
                    if value.as_stream().is_some() {
                        counts.named_appearance_streams =
                            counts.named_appearance_streams.saturating_add(1);
                    }
                }
            }
        }
    }
    for specification in specifications(document) {
        counts.specifications = counts.specifications.saturating_add(1);
        let thumb = document.get_key(&specification, "Thumb");
        if !matches!(thumb, Object::Null) {
            counts.with_thumb = counts.with_thumb.saturating_add(1);
            if thumb.as_stream().is_some() {
                counts.with_thumb_stream = counts.with_thumb_stream.saturating_add(1);
            }
        }
        let payload = document.get_key(&specification, "EP");
        if let Some(payload) = payload.as_dict() {
            counts.with_ep = counts.with_ep.saturating_add(1);
            if document.get_key(payload, "Subtype").as_name().is_some() {
                counts.with_ep_subtype = counts.with_ep_subtype.saturating_add(1);
            }
        }
    }
    counts
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        println!("usage: name_dictionary_and_file_spec_census <file.pdf> [<file.pdf> ...]");
        return;
    }

    let measured: Vec<(String, Counts)> = paths
        .par_iter()
        .filter_map(|path| {
            let bytes = std::fs::read(path).ok()?;
            let document = Document::open(bytes).ok()?;
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            Some((name, measure(&document)))
        })
        .collect();

    let mut total = Counts::default();
    let mut named: Vec<String> = Vec::new();
    for (name, counts) in &measured {
        if counts.notable() && named.len() < MAX_NAMED {
            named.push(format!(
                "  {name}: /Names /AP {}, appearances named {}, /AP-as-name {}, /Thumb {}, /EP {}",
                counts.with_ap_tree,
                counts.named_appearances,
                counts.annotations_naming_an_appearance,
                counts.with_thumb,
                counts.with_ep
            ));
        }
        total.absorb(counts);
    }

    println!(
        "{} of {} document(s) opened; {} state a /Names dictionary",
        total.opened,
        paths.len(),
        total.with_name_dictionary
    );
    println!(
        "  §7.7.4 Table 32 /AP: {} document(s) state the tree, holding {} name(s), {} of which \
         resolve to a stream",
        total.with_ap_tree, total.named_appearances, total.named_appearance_streams
    );
    println!(
        "  §12.5.5's prohibition: {} annotation(s) state an /AP that is a name",
        total.annotations_naming_an_appearance
    );
    println!(
        "  {} file specification sighting(s); §7.11.3 Table 43 /Thumb on {} ({} a stream), /EP \
         on {} ({} stating Table 28's required /Subtype)",
        total.specifications,
        total.with_thumb,
        total.with_thumb_stream,
        total.with_ep,
        total.with_ep_subtype
    );
    for line in named {
        println!("{line}");
    }
}
