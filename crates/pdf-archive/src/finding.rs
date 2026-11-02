//! What a requirement found, and where.
//!
//! A verdict that says only *failed* sends its reader back into the document to discover
//! where — so every failure carries a witness: the object, the page, or the name that made it
//! one. That is the same discipline `tools/conformance` applies to this project's own claims,
//! and the reason `doc/rfc/0006` argues the validator is the larger part of the value.

use std::collections::BTreeSet;
use std::sync::Arc;

use pdf_syntax::ObjectId;

/// Where in a document a requirement failed.
///
/// `None` in every field is a legitimate witness for a rule about the file as a whole — the
/// header's bytes, the trailer's keys — and says so rather than inventing a location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Where {
    /// The object the failure is in, where it is in one.
    pub object: Option<ObjectId>,
    /// The zero-based page index, where the failure belongs to a page.
    pub page: Option<usize>,
    /// The name of the offending entry, filter, font or colourant.
    pub name: Option<String>,
}

impl Where {
    /// The whole file, for a rule about its shape rather than its contents.
    #[must_use]
    pub const fn file() -> Self {
        Self {
            object: None,
            page: None,
            name: None,
        }
    }

    /// One object.
    #[must_use]
    pub const fn object(id: ObjectId) -> Self {
        Self {
            object: Some(id),
            page: None,
            name: None,
        }
    }

    /// One page, by zero-based index.
    #[must_use]
    pub const fn page(index: usize) -> Self {
        Self {
            object: None,
            page: Some(index),
            name: None,
        }
    }

    /// The same place, with the name of what was wrong there.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// One place a requirement was not met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Where it was found.
    pub place: Where,
    /// What was wrong, in one sentence a person can act on.
    pub what: String,
}

/// What one requirement's predicate found, and how much of it was kept.
///
/// # The bound, and why it is reported rather than applied in silence
///
/// A hostile or merely enormous document can fail one requirement on every object it has, and
/// a report holding a million findings is not a report. So a predicate's findings are capped —
/// and the cap is *announced*, because "this requirement failed 12 times" and "this requirement
/// failed at least 12 times and we stopped counting" are different facts about the document,
/// and only the second one tells the reader the list in front of them is a prefix.
#[derive(Debug, Clone, Default)]
pub struct Findings {
    kept: Vec<Finding>,
    seen: usize,
    /// How many of those named an object, past the bound as well as inside it.
    objects: usize,
    /// Every distinct object those named, past `MAX_KEPT` as well as inside it.
    named: Named,
    /// Objects ISO 19005 section 6.2.2's exemption puts outside this requirement's population.
    exempt: Option<Arc<BTreeSet<ObjectId>>>,
}

/// How many places one requirement reports before it starts counting instead.
const MAX_KEPT: usize = 32;

/// How many distinct objects one requirement's [`Named`] holds before it says it was cut short.
///
/// A remedy rewrites every object a failed requirement names (trap 289: a remedy that walked the
/// thirty-two places a report keeps restated thirty-two of a hundred and fourteen and wrote a
/// file failing the row it answered), so the set is kept past `MAX_KEPT`. It is bounded
/// because a hostile file can fail one requirement at every object it has: 4096 identifiers of
/// eight bytes each, in a tree, is a few hundred kilobytes for one requirement, and past it the
/// remedy is told the list is incomplete rather than handed a prefix.
pub const MAX_NAMED: usize = 4096;

/// How many bytes of entry names one requirement's [`Named::places`] holds before it is cut short.
///
/// A place's name can be a producer's string — a font's or a colourant's — so [`MAX_NAMED`]
/// places alone do not bound the set: four thousand names of a megabyte each are four gigabytes
/// for one requirement. A mebibyte of names is far past any place list a remedy reads (the
/// names it acts on are a dictionary's keys) and is the bound of its own the lifted cap needs
/// (trap 305).
pub const MAX_PLACE_NAME_BYTES: usize = 1 << 20;

/// Every distinct object and every distinct place one requirement's findings named, and whether
/// any place named no object.
///
/// What a remedy iterates where [`Findings::kept`] would hand it a prefix. The object set stops
/// growing at [`MAX_NAMED`] objects and then says so through [`Self::cut_short`]; the place list
/// stops at [`MAX_NAMED`] places or [`MAX_PLACE_NAME_BYTES`] of their names and says so through
/// [`Self::places_cut_short`]. A caller reading either after it was cut short has a prefix again
/// and refuses by name rather than half-remedy.
///
/// The places are what a remedy needs where the object is not the whole of the place — a page and
/// the boundary entry it failed at, a file specification and the `/EF` key under which the
/// failing stream sits — and they are kept in the order they were first found, so that a remedy
/// over them does its work in the order a remedy over [`Findings::kept`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Named {
    objects: BTreeSet<ObjectId>,
    cut_short: bool,
    elsewhere: bool,
    places: Vec<Where>,
    distinct: BTreeSet<Where>,
    name_bytes: usize,
    places_cut_short: bool,
}

impl Named {
    /// The objects named, in identifier order.
    #[must_use]
    pub const fn objects(&self) -> &BTreeSet<ObjectId> {
        &self.objects
    }

    /// Whether an object past [`MAX_NAMED`] distinct ones was named and is not in the set.
    #[must_use]
    pub const fn cut_short(&self) -> bool {
        self.cut_short
    }

    /// Whether a place named no object: a page, a content stream's operator or the file.
    #[must_use]
    pub const fn elsewhere(&self) -> bool {
        self.elsewhere
    }

    /// Every distinct place named, in the order each was first found.
    #[must_use]
    pub fn places(&self) -> &[Where] {
        &self.places
    }

    /// Whether a place past [`MAX_NAMED`] distinct ones, or past [`MAX_PLACE_NAME_BYTES`] of
    /// their names, was found and is not in [`Self::places`].
    #[must_use]
    pub const fn places_cut_short(&self) -> bool {
        self.places_cut_short
    }

    fn record(&mut self, place: &Where) {
        match place.object {
            Some(id) if self.objects.len() < MAX_NAMED || self.objects.contains(&id) => {
                self.objects.insert(id);
            }
            Some(_) => self.cut_short = true,
            None => self.elsewhere = true,
        }
        if self.places_cut_short || self.distinct.contains(place) {
            return;
        }
        let name_bytes = place.name.as_ref().map_or(0, String::len);
        if self.places.len() >= MAX_NAMED
            || self.name_bytes.saturating_add(name_bytes) > MAX_PLACE_NAME_BYTES
        {
            self.places_cut_short = true;
            return;
        }
        self.name_bytes = self.name_bytes.saturating_add(name_bytes);
        self.distinct.insert(place.clone());
        self.places.push(place.clone());
    }
}

impl Findings {
    /// The same, for a requirement section 6.2.2's exemption narrows.
    ///
    /// **A narrowing of the population, not a filter over the findings**, and the difference is
    /// [`Self::truncated`]: a report keeps the first `MAX_KEPT` places and counts the rest, so
    /// a filter applied afterwards could only ever see a prefix and would have to guess about
    /// everything past it. An exempt object dropped here is never *seen*, which is what the
    /// clause says — the resource is not used for rendering, so the requirement has nothing to
    /// be met or failed about there.
    ///
    /// Only a finding that names an object is dropped. A requirement that reports a page or the
    /// file for a fault whose subject is an exempt object keeps its finding, which over-reports
    /// rather than under-reports — the direction `doc/todo/62` section 3 argues every unit of
    /// risk in this work lies against.
    #[must_use]
    pub fn exempting(exempt: Arc<BTreeSet<ObjectId>>) -> Self {
        Self {
            kept: Vec::new(),
            seen: 0,
            objects: 0,
            named: Named::default(),
            exempt: Some(exempt),
        }
    }

    /// Records a place the requirement was not met.
    pub fn record(&mut self, place: Where, what: impl Into<String>) {
        if let Some(exempt) = self.exempt.as_ref()
            && place.object.is_some_and(|id| exempt.contains(&id))
        {
            return;
        }
        self.seen = self.seen.saturating_add(1);
        if place.object.is_some() {
            self.objects = self.objects.saturating_add(1);
        }
        self.named.record(&place);
        if self.kept.len() < MAX_KEPT {
            self.kept.push(Finding {
                place,
                what: what.into(),
            });
        }
    }

    /// The places kept, in the order they were found.
    #[must_use]
    pub fn kept(&self) -> &[Finding] {
        &self.kept
    }

    /// Every distinct object the places named, past the bound on places as well as inside it.
    #[must_use]
    pub const fn named(&self) -> &Named {
        &self.named
    }

    /// How many places the requirement was not met, including those past the bound.
    #[must_use]
    pub const fn seen(&self) -> usize {
        self.seen
    }

    /// Whether any place this requirement reported names an object.
    ///
    /// **What makes the unreferenced-resource exemption affordable.** `crate::check` narrows a
    /// failing requirement's population by running its predicate a second time against
    /// [`Self::exempting`], and a requirement whose places are pages or the file itself can have
    /// nothing withdrawn — [`Self::exempting`] drops a finding by its object and by nothing else.
    /// Asking this first skips the second run for every such row. It counts past the bound as
    /// well as inside it, because the answer has to be about what the predicate *found* rather
    /// than about the prefix a report keeps.
    ///
    /// The measurement that put it here: a full PDF/A-4 report over ISO 32000-2's own
    /// specification, whose dearest failing requirement reports a place per page over 1023 pages
    /// and names no object at any of them.
    #[must_use]
    pub const fn named_an_object(&self) -> bool {
        self.objects > 0
    }

    /// Whether the list is a prefix of what was found rather than the whole of it.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.seen > MAX_KEPT
    }

    /// Whether the requirement was met.
    #[must_use]
    pub const fn met(&self) -> bool {
        self.seen == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{Findings, MAX_NAMED, MAX_PLACE_NAME_BYTES, Where};
    use pdf_syntax::ObjectId;

    fn id(number: usize) -> ObjectId {
        ObjectId::new(u32::try_from(number).expect("a small object number"), 0)
    }

    #[test]
    fn every_object_named_is_kept_past_the_places_and_up_to_its_own_bound() {
        // Trap 289: a remedy over the kept places restated 32 of 114 objects. The set holds
        // all 114, and at MAX_NAMED distinct objects it is still whole.
        let mut findings = Findings::default();
        for number in 1..=114 {
            findings.record(Where::object(id(number)), "wrong");
        }
        assert_eq!(findings.kept().len(), 32);
        assert_eq!(findings.named().objects().len(), 114);
        assert!(!findings.named().cut_short() && !findings.named().elsewhere());

        let mut at_the_bound = Findings::default();
        for number in 1..=MAX_NAMED {
            at_the_bound.record(Where::object(id(number)), "wrong");
        }
        // An object already held is named again past the bound without cutting the set short.
        at_the_bound.record(Where::object(id(1)), "wrong again");
        assert_eq!(at_the_bound.named().objects().len(), MAX_NAMED);
        assert!(!at_the_bound.named().cut_short());

        at_the_bound.record(Where::object(id(MAX_NAMED + 1)), "one more");
        assert_eq!(at_the_bound.named().objects().len(), MAX_NAMED);
        assert!(
            at_the_bound.named().cut_short(),
            "the set says it is a prefix"
        );
    }

    #[test]
    fn every_distinct_place_is_kept_in_the_order_found_up_to_its_own_bounds() {
        // A page and the entry it failed at: the place, not its object, is the population.
        let mut findings = Findings::default();
        for page in 0..40 {
            findings.record(Where::page(page).named("BleedBox"), "too small");
            findings.record(Where::page(page).named("BleedBox"), "reported twice");
            findings.record(Where::page(page).named("TrimBox"), "too small");
        }
        assert_eq!(findings.kept().len(), 32);
        let places = findings.named().places();
        assert_eq!(places.len(), 80);
        assert_eq!(places[0], Where::page(0).named("BleedBox"));
        assert_eq!(places[1], Where::page(0).named("TrimBox"));
        assert_eq!(places[79], Where::page(39).named("TrimBox"));
        assert!(!findings.named().places_cut_short());

        let mut at_the_bound = Findings::default();
        for page in 0..MAX_NAMED {
            at_the_bound.record(Where::page(page), "wrong");
        }
        at_the_bound.record(Where::page(0), "a place already held");
        assert_eq!(at_the_bound.named().places().len(), MAX_NAMED);
        assert!(!at_the_bound.named().places_cut_short());
        at_the_bound.record(Where::page(MAX_NAMED), "one more");
        assert_eq!(at_the_bound.named().places().len(), MAX_NAMED);
        assert!(at_the_bound.named().places_cut_short());

        // The names' bytes are a bound of their own: a producer's string can be long.
        let mut by_bytes = Findings::default();
        let long = "N".repeat(MAX_PLACE_NAME_BYTES / 2);
        by_bytes.record(Where::page(0).named(long.clone()), "wrong");
        by_bytes.record(Where::page(1).named(long.clone()), "wrong");
        assert!(!by_bytes.named().places_cut_short());
        by_bytes.record(Where::page(2).named("N"), "one byte past");
        assert_eq!(by_bytes.named().places().len(), 2);
        assert!(by_bytes.named().places_cut_short());
    }

    #[test]
    fn a_place_past_the_kept_ones_that_names_no_object_is_still_seen() {
        let mut findings = Findings::default();
        for number in 1..=40 {
            findings.record(Where::object(id(number)), "wrong");
        }
        findings.record(Where::page(0), "in a content stream");
        assert!(
            findings
                .kept()
                .iter()
                .all(|kept| kept.place.object.is_some())
        );
        assert!(findings.named().elsewhere());
    }

    #[test]
    fn a_requirement_nothing_reported_is_met() {
        let findings = Findings::default();
        assert!(findings.met());
        assert!(!findings.truncated());
        assert_eq!(findings.seen(), 0);
    }

    #[test]
    fn past_the_bound_the_count_keeps_going_and_says_so() {
        let mut findings = Findings::default();
        for index in 0..100 {
            findings.record(Where::page(index), "wrong");
        }
        assert!(!findings.met());
        assert_eq!(
            findings.seen(),
            100,
            "the count is of what the document did"
        );
        assert_eq!(
            findings.kept().len(),
            32,
            "the list is of what a report can hold"
        );
        assert!(
            findings.truncated(),
            "and the difference between them is announced"
        );
    }
}
