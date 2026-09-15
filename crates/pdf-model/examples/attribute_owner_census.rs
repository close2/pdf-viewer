//! §14.8.5.3's ranking, measured: how often two owners state one attribute, and who owns them.
//!
//! The clause ranks attribute objects into three bands before any value is read, and the ranking
//! only ever *changes* an answer where two admitted objects state the same attribute name in
//! different bands. This counts that population, because a ranking no document exercises is a
//! different kind of work from one that decides what a screen reader says — and neither of those
//! two sentences can be written from the clause alone.
//!
//! It also counts the owners themselves, split the way §14.7.4.2's closing paragraph splits them:
//! an `NSO` object's owner is its namespace name, that name is a Table 376 owner value or it is
//! not, and where it is not the only namespaces this program processes are §14.8.6.1's two
//! standard structure ones. Every other owner is *refused by name* and the refusal is printed.
//!
//! ```sh
//! cargo run --release -p pdf-model --example attribute_owner_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::structure::{AttributeObject, Child, Owner, Priority, Tree};
use pdf_syntax::Document;

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents with a structure tree at all.
    tagged: usize,
    /// Elements the walk reached.
    elements: usize,
    /// Elements carrying at least one attribute object, by either of §14.7.6's routes.
    with_attributes: usize,
    /// Attribute objects read, refused ones included.
    objects: usize,
    /// Objects in §14.8.5.3's band 1 — an owner that is none of the five, whose format is ours.
    band_owner: usize,
    /// Objects in band 2 — the five PDF-native owners, through the element's `/A`.
    band_standard: usize,
    /// Objects in band 3 — the class map.
    band_class: usize,
    /// Objects whose owner this program does not process, which are consulted for nothing.
    refused: usize,
    /// Elements whose admitted objects carry **more than one** distinct owner.
    several_owners: usize,
    /// Elements where two admitted objects state the same attribute name.
    same_attribute_twice: usize,
    /// Elements where they state it in **different bands**, which is where the rank decides.
    rank_decides: usize,
    /// Objects whose `/O` is `NSO`.
    namespace_owned: usize,
    /// …of those, in one of §14.8.6.1's two standard structure namespaces.
    namespace_standard: usize,
    /// …of those, whose namespace name is one of Table 376's own owner values.
    namespace_equivalent: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.tagged = self.tagged.saturating_add(other.tagged);
        self.elements = self.elements.saturating_add(other.elements);
        self.with_attributes = self.with_attributes.saturating_add(other.with_attributes);
        self.objects = self.objects.saturating_add(other.objects);
        self.band_owner = self.band_owner.saturating_add(other.band_owner);
        self.band_standard = self.band_standard.saturating_add(other.band_standard);
        self.band_class = self.band_class.saturating_add(other.band_class);
        self.refused = self.refused.saturating_add(other.refused);
        self.several_owners = self.several_owners.saturating_add(other.several_owners);
        self.same_attribute_twice = self
            .same_attribute_twice
            .saturating_add(other.same_attribute_twice);
        self.rank_decides = self.rank_decides.saturating_add(other.rank_decides);
        self.namespace_owned = self.namespace_owned.saturating_add(other.namespace_owned);
        self.namespace_standard = self
            .namespace_standard
            .saturating_add(other.namespace_standard);
        self.namespace_equivalent = self
            .namespace_equivalent
            .saturating_add(other.namespace_equivalent);
    }
}

/// The attribute names an object states, which is every entry Table 360 does not reserve.
fn attribute_names(document: &Document, object: &AttributeObject) -> Vec<String> {
    let mut names = Vec::new();
    for (key, _) in object.dict.iter() {
        let name = String::from_utf8_lossy(key.as_bytes()).into_owned();
        if object.get(document, &name).is_some() {
            names.push(name);
        }
    }
    names
}

/// Walks one document's structure tree, reading every element's attribute objects.
fn census(document: &Document, refusals: &mut BTreeMap<String, usize>) -> Counts {
    let mut counts = Counts::default();
    let Some(tree) = Tree::of(document) else {
        return counts;
    };
    counts.tagged = 1;
    for (_, child) in tree.walk(document).items {
        let Child::Element(dict) = child else {
            continue;
        };
        counts.elements = counts.elements.saturating_add(1);
        let attached = tree.attributes(document, &dict);
        if attached.is_empty() {
            continue;
        }
        counts.with_attributes = counts.with_attributes.saturating_add(1);
        counts.objects = counts.objects.saturating_add(attached.len());
        let mut owners: BTreeSet<String> = BTreeSet::new();
        // Which band each attribute name has been stated in so far on this element, so that a
        // second statement can be told from a second statement *in another band*.
        let mut stated: BTreeMap<String, Priority> = BTreeMap::new();
        let mut twice = false;
        let mut decides = false;
        for object in &attached {
            if object.kind == Owner::Namespace {
                counts.namespace_owned = counts.namespace_owned.saturating_add(1);
                if let Some(space) = object.namespace.as_ref() {
                    if space.is_standard() {
                        counts.namespace_standard = counts.namespace_standard.saturating_add(1);
                    } else if Owner::read(&space.name).is_pdf_native() {
                        counts.namespace_equivalent = counts.namespace_equivalent.saturating_add(1);
                    }
                }
            }
            let Some(band) = object.priority() else {
                counts.refused = counts.refused.saturating_add(1);
                let named = refusals.entry(object.owner_as_written()).or_insert(0);
                *named = named.saturating_add(1);
                continue;
            };
            match band {
                Priority::Owner => counts.band_owner = counts.band_owner.saturating_add(1),
                Priority::Standard => {
                    counts.band_standard = counts.band_standard.saturating_add(1);
                }
                Priority::Class => counts.band_class = counts.band_class.saturating_add(1),
            }
            owners.insert(object.owner_as_written());
            for name in attribute_names(document, object) {
                if let Some(held) = stated.insert(name, band) {
                    twice = true;
                    if held != band {
                        decides = true;
                    }
                }
            }
        }
        if owners.len() > 1 {
            counts.several_owners = counts.several_owners.saturating_add(1);
        }
        if twice {
            counts.same_attribute_twice = counts.same_attribute_twice.saturating_add(1);
        }
        if decides {
            counts.rank_decides = counts.rank_decides.saturating_add(1);
        }
    }
    counts
}

fn main() {
    let mut total = Counts::default();
    let mut documents = 0usize;
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let before = refusals.clone();
        let counts = census(&document, &mut refusals);
        // A document is named where it holds any of the three rare things, because the day one
        // appears is the day somebody wants to open it.
        if counts.several_owners > 0 || counts.namespace_owned > 0 || refusals != before {
            println!(
                "{path}: {} elements with attributes, {} under several owners, \
                 {} NSO objects, {} refused",
                counts.with_attributes,
                counts.several_owners,
                counts.namespace_owned,
                counts.refused,
            );
        }
        total.absorb(&counts);
    }
    println!("\n{documents} opened, {} tagged", total.tagged);
    println!(
        "{} elements, {} of them carrying {} attribute objects",
        total.elements, total.with_attributes, total.objects
    );
    println!(
        "bands: {} owner (1), {} standard (2), {} class map (3), {} refused",
        total.band_owner, total.band_standard, total.band_class, total.refused
    );
    println!(
        "{} elements carry attributes under more than one owner; \
         {} state one attribute twice, {} of those in different bands",
        total.several_owners, total.same_attribute_twice, total.rank_decides
    );
    println!(
        "{} attribute objects are owned by a namespace: {} a standard structure namespace, \
         {} a Table 376 owner value",
        total.namespace_owned, total.namespace_standard, total.namespace_equivalent
    );
    for (owner, seen) in &refusals {
        println!("refused by name: {owner} ({seen} object(s))");
    }
}
