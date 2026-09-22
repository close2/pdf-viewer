//! The page boundary entries ISO 19005-2 section 6.1.13's limit is failed at, removed.
//!
//! # Why a removal answers a limit at all
//!
//! Section 6.1.13's last requirement holds *any* of the page boundaries ISO 32000-2 §14.11.2
//! describes to between 3 and 14 400 units in either direction, and §14.11.2 describes five. Of
//! those, §7.7.3.3's Table 31 makes `/MediaBox` **required** and the crop, bleed, trim and art
//! boxes **optional**, and §14.11.2.1 gives each optional one a default that is another box in
//! the same file: the crop box's "default value is the page's media box", and the bleed, trim
//! and art boxes' "default value is the page's crop box". So removing an out-of-range optional
//! entry is not an edit to what the page says — it is the page saying it the way Table 31
//! admits — and no mark on it moves.
//!
//! # Whether it is free, which is a second question the clause answers
//!
//! §14.11.2.1:
//!
//! > If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the
//! > media box, a processor shall treat the box as its intersection with the media box.
//!
//! An over-sized entry is therefore **already** its intersection with the media box to every
//! conforming processor. Where that intersection is what the default would give, removing the
//! entry changes nothing any reader computes and the removal is mechanical. Where the two differ
//! — a box too *small* to meet the limit, or an over-sized one a narrower crop box stands behind
//! — a reader would clip, trim or place the page's content differently afterwards, and the
//! removal costs [`Loss::PageBoundary`], which the caller authorises. `doc/adr/1210` is the
//! argument; the predicate is [`removal`].
//!
//! # The media box keeps the catalogue's original *none*
//!
//! Table 31 requires it, so there is nothing to fall back to: the only ways to bring a media box
//! inside the limit are rescaling the page, which moves every mark on it, and tiling it into
//! several pages, which composes pages nobody produced. Both are on the far side of ADR 0816's
//! fence, and a document failing at its media box is refused by name.
//!
//! # An entry an ancestor states governs every page beneath it
//!
//! Table 31 marks `/MediaBox` and `/CropBox` inheritable and §7.7.3.4 says where an inheritable
//! entry is looked for, so a page may fail this requirement at a value an ancestor wrote. The
//! removal is made where the entry *is*, and the cost is then asked of **every** page that
//! resolves through it: where the removal moves what a reader computes for a page whose own
//! boundaries met the limit, that page is paying for a sibling's failure, which is ADR 0947's
//! first rule the wrong way round — so it is refused by name rather than made.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::Outcome;
use pdf_model::page::Boundary;
use pdf_model::{Page, Pages};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
const SITE: &str = "implementation-limits/page-boundary-sizes";

/// The four optional boundaries, with the entry each one is written under.
///
/// `/MediaBox` is deliberately absent: Table 31 requires it, so it is not removable and is
/// refused by name rather than passed over silently.
const OPTIONAL: [(&str, Boundary); 4] = [
    ("CropBox", Boundary::Crop),
    ("BleedBox", Boundary::Bleed),
    ("TrimBox", Boundary::Trim),
    ("ArtBox", Boundary::Art),
];

/// One entry a conversion takes off a page, with what a reader computed before and after.
#[derive(Debug, Clone, PartialEq)]
pub struct RemovedBoundary {
    /// The page, counting from one, as a report for a person states it.
    pub page: usize,
    /// The boundary this row is about — `CropBox`, `BleedBox`, `TrimBox` or `ArtBox`.
    pub entry: &'static str,
    /// Whether the conversion removed *this* entry, or the rectangle moved because the entry it
    /// defaults to was removed.
    ///
    /// §14.11.2.1 makes the bleed, trim and art boxes default to the crop box, so removing an
    /// out-of-range crop box can move what a reader computes for a boundary whose own entry is
    /// absent and untouched. That is still a cost of this removal, and the report names it as
    /// what it is rather than as an entry that went.
    pub removed: bool,
    /// The rectangle a conforming reader computed for that boundary before the removal.
    pub was: [f32; 4],
    /// The rectangle it computes for that boundary afterwards, from §14.11.2.1's default.
    pub now: [f32; 4],
}

impl RemovedBoundary {
    /// Whether this removal changes the region a conforming reader computes.
    ///
    /// The whole of `doc/adr/1210`'s predicate, read off the two rectangles: equal means the
    /// entry carried no information a reader used, which is what makes the removal mechanical.
    #[must_use]
    pub fn costs_nothing(&self) -> bool {
        same_rectangle(self.was, self.now)
    }
}

/// Whether two effective boundaries are the same rectangle.
///
/// **Exact equality is the question, not an approximation of it.** Both sides come out of the
/// same reader doing the same arithmetic over the same `/MediaBox` — `pdf_model::Page` once with
/// the entry and once without it — so a difference of any size is a difference the entry made,
/// and a tolerance here would let a removal move a page by whatever the tolerance was. There is
/// no measurement being compared and nothing to round.
#[expect(
    clippy::float_cmp,
    reason = "the two rectangles are the same computation with and without one dictionary entry, \
              so the question is whether the entry changed anything at all; a tolerance would \
              silently admit a removal that moved the page by less than it"
)]
fn same_rectangle(left: [f32; 4], right: [f32; 4]) -> bool {
    left == right
}

/// The boundary entries this conversion removes, and what each one costs.
#[derive(Debug, Default)]
pub(super) struct Boundaries {
    /// The keys each page object loses, in the source's own numbering.
    pub(super) at: BTreeMap<ObjectId, Vec<&'static str>>,
    /// What went, per entry, for the report.
    pub(super) done: Vec<RemovedBoundary>,
}

impl Boundaries {
    /// Whether every removal this conversion makes leaves a reader computing what it computed.
    ///
    /// The decision between [`super::Decision::Mechanical`] and an authorised loss, asked once
    /// over the whole document: a conversion is one act, so a file with one free removal and one
    /// costly one costs the caller an authorisation.
    pub(super) fn costs_nothing(&self) -> bool {
        self.done.iter().all(RemovedBoundary::costs_nothing)
    }
}

/// Every page boundary entry the requirement was failed at, with the removal's cost.
///
/// **The population is the validator's findings rather than a walk of this crate's**, which is
/// the rule every remedy here follows: which page fails at which entry is `pdf_archive`'s
/// reading of section 6.1.13, and a second walk would be a second reading of ISO 19005. What
/// *is* walked here is the page tree, and only for a document that failed: an entry a page
/// inherits is written on an ancestor, so which pages a removal reaches is a question about the
/// tree rather than about the finding.
pub(super) fn removal(
    document: &Document,
    report: &pdf_archive::Report,
) -> Result<Boundaries, Because> {
    let Some(judgement) = report.failures().find(|judgement| judgement.id == SITE) else {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    };
    let Outcome::Failed { places, .. } = &judgement.outcome else {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    };
    let tree = Pages::new(document);
    // Which object *states* each failing entry, which for §7.7.3.3's two inheritable boundaries
    // may be an ancestor of the page the finding names.
    let mut holders: BTreeMap<ObjectId, BTreeSet<&'static str>> = BTreeMap::new();
    // Which page failed at which entry, so that a removal reaching a page that failed nothing
    // can be told from one a page asked for.
    let mut failed: BTreeMap<usize, BTreeSet<&'static str>> = BTreeMap::new();
    for finding in places {
        let Some(index) = finding.place.page else {
            return Err(Because::NotBuiltYet(NOT_REPORTED_AT_A_PAGE));
        };
        let Some(entry) = finding.place.name.as_deref() else {
            return Err(Because::NotBuiltYet(NOT_REPORTED_AT_AN_ENTRY));
        };
        if entry == "MediaBox" {
            return Err(Because::TheFence(THE_MEDIA_BOX_IS_THE_PAGE));
        }
        let Some((key, _)) = OPTIONAL.iter().find(|(key, _)| *key == entry) else {
            return Err(Because::NotBuiltYet(NOT_REPORTED_AT_AN_ENTRY));
        };
        let page = tree.get(index).ok_or(Because::NotBuiltYet(NO_SUCH_PAGE))?;
        let holder = states(document, &page, key).ok_or(Because::NotBuiltYet(NOT_STATED))?;
        holders.entry(holder).or_default().insert(key);
        failed.entry(index).or_default().insert(key);
    }
    let mut out = Boundaries::default();
    for index in 0..tree.len() {
        let Some(page) = tree.get(index) else {
            return Err(Because::NotBuiltYet(NO_SUCH_PAGE));
        };
        // Which of the removals this page's boundaries resolve through — its own entries, and
        // the ancestors' entries §7.7.3.4 puts in force for it.
        let mut reaching: BTreeSet<&'static str> = BTreeSet::new();
        for (holder, keys) in &holders {
            for key in keys {
                if states(document, &page, key) == Some(*holder) {
                    reaching.insert(key);
                }
            }
        }
        if reaching.is_empty() {
            continue;
        }
        let mut after = flattened(&page);
        for key in &reaching {
            after.remove(key);
        }
        let after = tree.detached(&after);
        for (key, boundary) in OPTIONAL {
            let was = page.boundary(boundary);
            let now = after.boundary(boundary);
            let removed = reaching.contains(&key);
            if same_rectangle(was, now) && !removed {
                continue;
            }
            // **A page that failed nothing may not be changed**, `doc/adr/0947`'s first rule.
            // An entry on an ancestor governs every page beneath it, so a removal that moves
            // what a reader computes for a page whose own boundaries all met the limit is paid
            // for by a page that asked for nothing, and that is refused by name rather than
            // made. A page that *did* fail takes the consequences of its own removal — including
            // §14.11.2.1's other three boxes following the crop box it defaulted to — and those
            // are the [`super::Loss::PageBoundary`] the caller authorises.
            if !same_rectangle(was, now) && !failed.contains_key(&index) {
                return Err(Because::TheFence(THE_BOX_IS_AN_ANCESTOR_S));
            }
            out.done.push(RemovedBoundary {
                page: index.saturating_add(1),
                entry: key,
                removed,
                was,
                now,
            });
        }
    }
    for (holder, keys) in holders {
        out.at.entry(holder).or_default().extend(keys);
    }
    if out.at.is_empty() {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    }
    Ok(out)
}

/// The object that states one boundary entry for a page: the page itself, or an ancestor.
///
/// §7.7.3.3's Table 31 marks `/MediaBox` and `/CropBox` inheritable and the bleed, trim and art
/// boxes not, and §7.7.3.4 states the rule the search follows — an inheritable entry is looked
/// for on the page and then up its `/Parent` chain, and the first node stating it is the one
/// whose value is in force. A non-inheritable entry is on the page or nowhere.
fn states(document: &Document, page: &Page, key: &str) -> Option<ObjectId> {
    if page.dict.get(key).is_some() {
        return page.id;
    }
    if !INHERITABLE.contains(&key) {
        return None;
    }
    let mut node = page.dict.clone();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    for _ in 0..MAX_ANCESTRY {
        let at = node.get("Parent").and_then(Object::as_reference)?;
        if !seen.insert(at) {
            return None;
        }
        node = document.get(at).as_dict().cloned()?;
        if node.get(key).is_some() {
            return Some(at);
        }
    }
    None
}

/// The two boundaries §7.7.3.3's Table 31 marks inheritable.
///
/// > attributes that are not explicitly identified in the table as inheritable shall not be
/// > inherited
///
/// So the bleed, trim and art boxes are read from the page object alone, and a page stating none
/// of them has §14.11.2.1's default rather than an ancestor's value.
const INHERITABLE: [&str; 2] = ["MediaBox", "CropBox"];

/// How far up §7.7.3.2's page tree an inheritable entry is looked for.
///
/// `pdf_syntax::Limits::DEFAULT`'s `max_depth`, which is the depth the parser admitted the tree
/// at: a `/Parent` chain longer than that is one no page in the document was read through.
const MAX_ANCESTRY: usize = 256;

/// The page's dictionary with the two inheritable boundaries written into it.
///
/// **What makes the after-picture the same reader's as the before-picture.** `pdf_model::Page`
/// is this tree's one reading of §14.11.2.1 — its five boxes are already defaulted and
/// intersected with the media box — so the way to ask what a reader computes *after* a removal
/// is to ask it again of the page without the entry, rather than to write a second reading of
/// the clause here. That needs the ancestry §7.7.3.4 supplies to be on the dictionary itself,
/// because [`Pages::detached`] reads a dictionary with no ancestry: `/MediaBox` and `/CropBox`
/// are Table 31's two inheritable boundaries, and both are written as the effective values the
/// page already resolved to, which the clause makes identical to what the ancestry states.
fn flattened(page: &Page) -> Dictionary {
    let mut dict = page.dict.clone();
    dict.insert(Name::new(&b"MediaBox"[..]), rectangle(page.media_box));
    dict.insert(Name::new(&b"CropBox"[..]), rectangle(page.crop_box));
    dict
}

/// A rectangle as §7.9.5's array of four numbers.
fn rectangle(rect: [f32; 4]) -> Object {
    Object::Array(
        rect.iter()
            .map(|side| Object::Real(f64::from(*side)))
            .collect(),
    )
}

/// Why nothing is removed though the target binds the requirement.
const NOTHING_FAILED: &str = "no page in this document states a boundary entry outside the \
     range ISO 19005-2 section 6.1.13 admits, so there is nothing to remove";

/// Why a finding that names no page stops the removal.
const NOT_REPORTED_AT_A_PAGE: &str = "this requirement was failed somewhere this reader cannot \
     place on a page, so which page object would lose an entry cannot be worked out";

/// Why a finding that names no entry stops the removal.
const NOT_REPORTED_AT_AN_ENTRY: &str = "this requirement was failed at a page boundary this \
     reader cannot name, so which of ISO 32000-2 \u{a7}14.11.2's five boxes would be removed \
     cannot be worked out";

/// Why a page this reader cannot find stops the removal.
const NO_SUCH_PAGE: &str = "this requirement was failed at a page this reader cannot reach \
     through the page tree, so there is no object to take an entry out of";

/// Why a boundary no object in the ancestry states is not removed.
const NOT_STATED: &str = "this requirement was failed at a page boundary no object in the \
     failing page's own dictionary or its ancestry states, so there is no entry to remove. ISO \
     32000-2 \u{a7}7.7.3.4 is where an inheritable entry is looked for, and a boundary reached \
     through neither route is one this reader cannot place";

/// Why a failing media box is refused, which is the catalogue's original *none*.
pub(super) const THE_MEDIA_BOX_IS_THE_PAGE: &str = "this document states a MediaBox outside the \
     range ISO 19005-2 section 6.1.13 admits. ISO 32000-2 \u{a7}7.7.3.3's Table 31 makes that \
     entry required and \u{a7}14.11.2.1 gives it no default, so there is nothing to fall back \
     to: the only ways to bring it inside the limit are rescaling the page, which moves every \
     mark on it, and tiling it into several pages, which composes pages nobody produced. Both \
     are on the far side of ADR 0816's fence. ISO 19005-4 states no implementation-limits \
     subclause at all, so a large-format drawing can be PDF/A-4 and cannot be PDF/A-2 - ask for \
     PDF/A-4";

/// Why a boundary a page inherits is not removed where the removal would cost anything.
const THE_BOX_IS_AN_ANCESTOR_S: &str = "the page boundary this document fails the limit at is \
     one the page inherits: ISO 32000-2 \u{a7}7.7.3.3's Table 31 makes MediaBox and CropBox \
     inheritable, so the entry is written on an ancestor and governs every page beneath it. \
     \u{a7}14.11.2.1's default does not land on the same rectangle for all of those pages, so \
     removing it would change what a reader shows for a page that failed nothing - which is the \
     one thing this conversion will not do on another page's behalf. Removing the entry from the \
     pages that need it, or stating it on each page, resolves it";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_removal_that_lands_on_the_same_rectangle_costs_nothing() {
        // ISO 32000-2 §14.11.2.1: an over-sized crop box is already its intersection with
        // the media box, and the default for an absent crop box is the media box — so the two
        // rectangles are the same one and the entry carried nothing a reader used.
        let free = RemovedBoundary {
            page: 1,
            entry: "CropBox",
            removed: true,
            was: [0.0, 0.0, 4.0, 4.0],
            now: [0.0, 0.0, 4.0, 4.0],
        };
        assert!(free.costs_nothing());
        let costly = RemovedBoundary {
            page: 1,
            entry: "TrimBox",
            removed: true,
            was: [0.0, 0.0, 2.0, 2.0],
            now: [0.0, 0.0, 400.0, 400.0],
        };
        assert!(!costly.costs_nothing());
        let boundaries = Boundaries {
            at: BTreeMap::new(),
            done: vec![free.clone(), costly],
        };
        assert!(!boundaries.costs_nothing());
        let boundaries = Boundaries {
            at: BTreeMap::new(),
            done: vec![free],
        };
        assert!(boundaries.costs_nothing());
    }
}
