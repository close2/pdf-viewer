//! A page whose media box ISO 19005-2 section 6.1.13 holds out of range, restated in larger units.
//!
//! # Why the size the limit measures is a count of units
//!
//! Section 6.1.13 holds each page boundary ISO 32000-2 §14.11.2 describes to between 3 and 14 400
//! *units* in either direction, and the unit is not fixed. §7.7.3.3's Table 31 gives a page a
//! `/UserUnit`, "a positive number that shall give the size of default user space units, in
//! multiples of 1⁄72 inch", and §8.3.2.3 makes the default 1⁄72 inch only where the entry is
//! absent. So a page twice the limit across is a page whose boxes count half as many units twice
//! as large: the same sheet, stated in other units.
//!
//! # The construction: the producer's operators bracketed, nothing of theirs edited
//!
//! Dividing every box by a factor `k` and multiplying `/UserUnit` by it leaves the page the same
//! physical size, but the producer's content still counts in the old units. Its operators keep
//! their bytes and their place; a stream before them concatenates the matrix `[1/k 0 0 1/k 0 0]`
//! with `cm` under a `q`, and a stream after them closes every state they left open and that `q`
//! (§8.4.2's balance across the `/Contents` array). It is ADR 1123's bracket around a producer's
//! own streams, with §8.3.4's scaling where that bracket placed an appearance: every mark lands
//! where it landed, measured in inches, and no mark is added (`doc/adr/2888`).
//!
//! `k` is a power of two, the smallest that brings every box on the page inside the range, so
//! that each coordinate divided by it is exact in binary arithmetic and the boxes written are the
//! producer's numbers with the exponent moved.
//!
//! # What the bracket does not reach, refused by name
//!
//! The cost of the construction is that **only the page's content stream is inside the `cm`**.
//! Everything else stated in the page's default user space keeps counting old units in a space
//! whose unit has grown, and is refused rather than restated:
//!
//! - an annotation's `/Rect` and every coordinate after it (§12.5.2);
//! - a pattern in the page's resources, whose matrix §8.7.2 maps "to the default (initial)
//!   coordinate space of the page", which the bracket's `cm` does not change;
//! - an article bead (`/B`, §12.4.3) and a viewport (`/VP`, §12.9);
//! - an explicit destination naming the page with a coordinate (§12.3.2.2's Table 149);
//! - a structure element's `/BBox` layout attribute (§14.8.5.4.3), which is stated in the
//!   coordinates of the page it is on and is refused wherever it is, since which page is not
//!   read here.
//!
//! # A media box with a side of no length is restated as the box this reader draws
//!
//! A side of no length has no factor at all: no unit counts it as three. But such a box is no
//! rectangle a page can be shown on, and `pdf_model::Page` reads it as no entry, so the page is
//! drawn on the media box its ancestry states or, where none does, on
//! [`pdf_model::Page::DEFAULT_MEDIA_BOX`] — a choice of this reader's that ISO 32000-2 does not
//! make, since §7.7.3.3's Table 31 requires the entry and states no default. That box is written
//! on the page, with each other box stating no area written as the box this reader computes for
//! it, §14.11.2.1's default; the unit and the content stay as they are, so nothing a reader of
//! this viewer saw moves (`doc/adr/3142`). Where the box drawn is itself outside the range, the
//! page keeps [`super::boundaries::THE_MEDIA_BOX_IS_THE_PAGE`].

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::Page;
use pdf_model::page::Boundary;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::decision::Because;
use super::prepare::Spare;
use super::preserve;

/// The smallest page boundary ISO 19005-2 section 6.1.13 admits, in units.
const SMALLEST: f64 = 3.0;
/// The largest page boundary ISO 19005-2 section 6.1.13 admits, in units.
const LARGEST: f64 = 14_400.0;
/// The five boundaries §14.11.2 describes, in the order a report names them.
const BOXES: [&str; 5] = ["MediaBox", "CropBox", "BleedBox", "TrimBox", "ArtBox"];
/// How far the factor's exponent is searched each way: `2^32` units of one point is past any
/// sheet, and a box needing more has sides too far apart for any one factor.
const MOST_DOUBLINGS: i32 = 32;
/// How many objects the walk for coordinates in the page's units visits before it stops.
const MOST_NODES: usize = 4_000_000;
/// How deep the walk descends into one object's direct arrays and dictionaries.
const MOST_DEPTH: usize = 64;
/// The explicit destinations Table 149 states with a coordinate in the page's space.
const PLACED_DESTINATIONS: [&[u8]; 6] = [b"XYZ", b"FitH", b"FitV", b"FitR", b"FitBH", b"FitBV"];

/// One page this conversion restated in larger units, as the report names it.
#[derive(Debug, Clone, PartialEq)]
pub struct RestatedPage {
    /// The page, counting from one.
    pub page: usize,
    /// How many of the old units one new unit is: every box is divided by it, `/UserUnit`
    /// multiplied by it.
    pub factor: f64,
    /// The `/UserUnit` the page stated, 1 where it stated none (§7.7.3.3's Table 31).
    pub user_unit_was: f64,
    /// The `/UserUnit` written.
    pub user_unit_now: f64,
    /// Each box the page stated or inherited, with its rectangle before and after.
    pub boxes: Vec<(&'static str, [f64; 4], [f64; 4])>,
}

/// What one page object takes.
#[derive(Debug, Clone)]
pub(super) struct Restatement {
    /// Each box written on the page itself, in the new units.
    pub(super) boxes: Vec<(&'static str, [f64; 4])>,
    /// The `/UserUnit` written, `None` where the page keeps the unit it states.
    pub(super) user_unit: Option<f64>,
    /// The `q … cm` stream before the producer's content, absent where the page has none.
    pub(super) prepend: Option<ObjectId>,
    /// The stream after the producer's content, closing what it left open and the prepended `q`.
    pub(super) closing: Option<ObjectId>,
    /// The stream after a content that leaves nothing open — a photograph that replaced the
    /// producer's — which closes the prepended `q` alone.
    pub(super) closing_balanced: Option<ObjectId>,
}

/// Every page this conversion restates.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// What each page object takes, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Restatement>,
    /// The streams the brackets add.
    pub(super) written: Vec<(ObjectId, Object)>,
    /// What changed, per page, for the report.
    pub(super) done: Vec<RestatedPage>,
}

/// The power of two that brings every side inside section 6.1.13's range, or `None` where none
/// does.
///
/// The smallest change of unit is taken: a page too large is counted in units 2, 4, 8 … times the
/// old, a page too small in units ½, ¼ … of it, and the first that fits every side is the answer.
/// A side of no length, or sides further apart than the range's own ratio of 4 800, fit none.
#[must_use]
pub(super) fn factor(sides: &[f64]) -> Option<f64> {
    let largest = sides.iter().copied().fold(0.0_f64, f64::max);
    let smallest = sides.iter().copied().fold(f64::INFINITY, f64::min);
    if !(smallest > 0.0 && largest.is_finite()) {
        return None;
    }
    let fits = |k: f64| largest / k <= LARGEST && smallest / k >= SMALLEST;
    // Too large is counted in larger units, too small in smaller; a page cannot be both and fit.
    let step = if largest > LARGEST { 2.0 } else { 0.5 };
    let mut k = 1.0_f64;
    for _ in 0..MOST_DOUBLINGS {
        if fits(k) {
            return Some(k);
        }
        if (step > 1.0 && largest / k <= LARGEST) || (step < 1.0 && smallest / k >= SMALLEST) {
            return None;
        }
        k *= step;
    }
    fits(k).then_some(k)
}

/// The pages (zero-based) that failed at their media box, restated, or why they cannot be.
pub(super) fn restate(
    document: &Document,
    failing: &BTreeSet<usize>,
    pages: &[Option<Page>],
    spare: &mut Spare,
) -> Result<Restated, Because> {
    let mut out = Restated::default();
    let mut ids: BTreeSet<ObjectId> = BTreeSet::new();
    let mut closing_balanced: Option<ObjectId> = None;
    for index in failing {
        let page = pages
            .get(*index)
            .and_then(Option::as_ref)
            .ok_or(Because::NotBuiltYet(NO_SUCH_PAGE))?;
        let id = page.id.ok_or(Because::NotBuiltYet(NO_SUCH_PAGE))?;
        let stated: Vec<(&'static str, [f64; 4])> = BOXES
            .iter()
            .filter_map(|key| stated_rectangle(document, page, key).map(|rect| (*key, rect)))
            .collect();
        if stated
            .iter()
            .any(|(key, rect)| *key == "MediaBox" && !has_area(*rect))
        {
            let drawn = as_drawn(page, &stated)?;
            out.at.insert(
                id,
                Restatement {
                    boxes: drawn.iter().map(|(key, _, now)| (*key, *now)).collect(),
                    user_unit: None,
                    prepend: None,
                    closing: None,
                    closing_balanced: None,
                },
            );
            let user_unit = stated_user_unit(document, page)?;
            out.done.push(RestatedPage {
                page: index.saturating_add(1),
                factor: 1.0,
                user_unit_was: user_unit,
                user_unit_now: user_unit,
                boxes: drawn,
            });
            continue;
        }
        let sides: Vec<f64> = stated
            .iter()
            .flat_map(|(_, rect)| [(rect[2] - rect[0]).abs(), (rect[3] - rect[1]).abs()])
            .collect();
        let k = factor(&sides).ok_or(Because::TheFence(
            super::boundaries::THE_MEDIA_BOX_IS_THE_PAGE,
        ))?;
        placed_on_the_page(document, page)?;
        let user_unit_was = stated_user_unit(document, page)?;
        let (prepend, closing) = match page.dict.get("Contents") {
            None | Some(Object::Null) => (None, None),
            Some(Object::Reference(_) | Object::Array(_)) => {
                let (content, issues) = page.content_with_report(document);
                if !issues.is_empty() {
                    return Err(Because::NotBuiltYet(CONTENT_DID_NOT_DECODE));
                }
                let depths = preserve::bracket_depths(document, &content, &page.resources)?;
                let prepend_at = spare.take(document).ok_or(Because::NotBuiltYet(NO_SPARE))?;
                let closing_at = spare.take(document).ok_or(Because::NotBuiltYet(NO_SPARE))?;
                let scale = operand(1.0 / k);
                let prepend = format!("q\n{scale} 0 0 {scale} 0 0 cm\n");
                out.written.push((prepend_at, stream(prepend.as_bytes())?));
                out.written.push((
                    closing_at,
                    stream("Q\n".repeat(depths.open.saturating_add(1)).as_bytes())?,
                ));
                (Some(prepend_at), Some(closing_at))
            }
            Some(_) => return Err(Because::NotBuiltYet(preserve::CONTENTS_NOT_REFERENCED)),
        };
        if prepend.is_some() && closing_balanced.is_none() {
            let at = spare.take(document).ok_or(Because::NotBuiltYet(NO_SPARE))?;
            out.written.push((at, stream(b"Q\n")?));
            closing_balanced = Some(at);
        }
        let boxes: Vec<(&'static str, [f64; 4], [f64; 4])> = stated
            .iter()
            .map(|(key, rect)| (*key, *rect, rect.map(|side| side / k)))
            .collect();
        out.at.insert(
            id,
            Restatement {
                boxes: boxes.iter().map(|(key, _, now)| (*key, *now)).collect(),
                user_unit: Some(user_unit_was * k),
                prepend,
                closing,
                closing_balanced: prepend.and(closing_balanced),
            },
        );
        out.done.push(RestatedPage {
            page: index.saturating_add(1),
            factor: k,
            user_unit_was,
            user_unit_now: user_unit_was * k,
            boxes,
        });
        ids.insert(id);
    }
    if let Some(reason) = coordinates_in_page_units(document, &ids) {
        return Err(Because::NotBuiltYet(reason));
    }
    Ok(out)
}

impl Restatement {
    /// The page's dictionary in the new units: its boxes, its `/UserUnit`, and its `/Contents`
    /// bracketed. `photographed` says the content is a photograph's, which leaves nothing open.
    pub(super) fn write(&self, out: &mut Dictionary, photographed: bool) {
        for (key, rect) in &self.boxes {
            out.insert(
                Name::new(key.as_bytes()),
                Object::Array(rect.iter().map(|side| Object::Real(*side)).collect()),
            );
        }
        if let Some(unit) = self.user_unit {
            out.insert(Name::new(&b"UserUnit"[..]), Object::Real(unit));
        }
        let closing = if photographed {
            self.closing_balanced
        } else {
            self.closing
        };
        let (Some(prepend), Some(closing)) = (self.prepend, closing) else {
            return;
        };
        // §7.7.3.3's Table 31 admits a single stream or an array of them; a single one is flattened
        // to the array by reference, so no producer byte moves.
        let mut contents = vec![Object::Reference(prepend)];
        match out.get("Contents") {
            Some(Object::Array(parts)) => contents.extend(parts.iter().cloned()),
            Some(other) => contents.push(other.clone()),
            None => {}
        }
        contents.push(Object::Reference(closing));
        out.insert(Name::new(&b"Contents"[..]), Object::Array(contents));
    }
}

/// The `/UserUnit` a page states, 1 where it states none (§7.7.3.3's Table 31).
fn stated_user_unit(document: &Document, page: &Page) -> Result<f64, Because> {
    match page.dict.get("UserUnit") {
        None => Ok(1.0),
        Some(stated) => document
            .resolve(stated)
            .as_number()
            .filter(|unit| *unit > 0.0 && unit.is_finite())
            .ok_or(Because::NotBuiltYet(USER_UNIT_UNREADABLE)),
    }
}

/// Whether a rectangle as written has a side of some length.
fn has_area(rect: [f64; 4]) -> bool {
    (rect[2] - rect[0]).abs() > 0.0 && (rect[3] - rect[1]).abs() > 0.0
}

/// One box a restated page states: its key, its rectangle as written, and the one written now.
type BoxBeforeAndAfter = (&'static str, [f64; 4], [f64; 4]);

/// Each box a page whose media box has a side of no length states, before and after: the media
/// box and every other box stating no area written as the one this reader computes for the page,
/// every box with an area as it is written (`doc/adr/3142`).
///
/// The rectangles written are `pdf_model::Page`'s own, so the page a reader of this viewer was
/// shown is the page the output states; one still outside section 6.1.13's range at the page's
/// own unit keeps [`super::boundaries::THE_MEDIA_BOX_IS_THE_PAGE`].
fn as_drawn(
    page: &Page,
    stated: &[(&'static str, [f64; 4])],
) -> Result<Vec<BoxBeforeAndAfter>, Because> {
    let mut drawn = Vec::with_capacity(stated.len());
    for (key, rect) in stated {
        let now = if has_area(*rect) {
            *rect
        } else {
            let boundary = match *key {
                "CropBox" => Boundary::Crop,
                "BleedBox" => Boundary::Bleed,
                "TrimBox" => Boundary::Trim,
                "ArtBox" => Boundary::Art,
                _ => Boundary::Media,
            };
            page.boundary(boundary).map(f64::from)
        };
        drawn.push((*key, *rect, now));
    }
    let sides: Vec<f64> = drawn
        .iter()
        .flat_map(|(_, _, rect)| [(rect[2] - rect[0]).abs(), (rect[3] - rect[1]).abs()])
        .collect();
    if sides.iter().all(|side| (SMALLEST..=LARGEST).contains(side)) {
        Ok(drawn)
    } else {
        Err(Because::TheFence(
            super::boundaries::THE_MEDIA_BOX_IS_THE_PAGE,
        ))
    }
}

/// Refuses a page that states something in its default user space the bracket's `cm` would not
/// carry into the new units.
fn placed_on_the_page(document: &Document, page: &Page) -> Result<(), Because> {
    let non_empty_array = |key: &str| {
        document
            .get_key(&page.dict, key)
            .as_array()
            .is_some_and(|items| !items.is_empty())
    };
    if non_empty_array("Annots") {
        return Err(Because::NotBuiltYet(ANNOTATIONS_IN_PAGE_UNITS));
    }
    if non_empty_array("B") || page.dict.get("VP").is_some() {
        return Err(Because::NotBuiltYet(BEADS_IN_PAGE_UNITS));
    }
    let patterns = document.get_key(&page.resources, "Pattern");
    if patterns
        .as_dict()
        .is_some_and(|patterns| !patterns.is_empty())
    {
        return Err(Because::NotBuiltYet(PATTERNS_IN_PAGE_UNITS));
    }
    Ok(())
}

/// The rectangle a page states or inherits for one boundary, as written.
///
/// §7.7.3.4 resolves `/MediaBox` and `/CropBox` up `/Parent`; the other three are the page's own.
/// The value is the one the file states, as section 6.1.13 measures it, not the one §14.11.2.1
/// intersects with the media box.
fn stated_rectangle(document: &Document, page: &Page, key: &str) -> Option<[f64; 4]> {
    let mut node = page.dict.clone();
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    for _ in 0..MOST_DEPTH {
        if let Some(value) = node.get(key) {
            let value = document.resolve(value);
            let items = value.as_array()?;
            let numbers: Vec<f64> = items
                .iter()
                .map(|item| document.resolve(item).as_number())
                .collect::<Option<_>>()?;
            return numbers.try_into().ok();
        }
        if !["MediaBox", "CropBox"].contains(&key) {
            return None;
        }
        let parent = node.get("Parent").and_then(Object::as_reference)?;
        if !seen.insert(parent) {
            return None;
        }
        node = document.get(parent).as_dict().cloned()?;
    }
    None
}

/// The first thing anywhere in the document that states a coordinate in a restated page's
/// default user space: an explicit destination naming the page with one (Table 149), or a layout
/// attribute's `/BBox` (§14.8.5.4.3). `None` where nothing does.
fn coordinates_in_page_units(
    document: &Document,
    pages: &BTreeSet<ObjectId>,
) -> Option<&'static str> {
    if pages.is_empty() {
        return None;
    }
    let mut budget = MOST_NODES;
    for number in document.xref().object_numbers() {
        let object = document.get(document.defined_id(number));
        let found = match &object {
            Object::Stream(stream) => walk(
                &Object::Dictionary(stream.dict.clone()),
                pages,
                0,
                &mut budget,
            ),
            other => walk(other, pages, 0, &mut budget),
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// [`coordinates_in_page_units`] over one object's direct parts.
fn walk(
    object: &Object,
    pages: &BTreeSet<ObjectId>,
    depth: usize,
    budget: &mut usize,
) -> Option<&'static str> {
    *budget = budget.checked_sub(1)?;
    if *budget == 0 || depth > MOST_DEPTH {
        return Some(TOO_LARGE_TO_WALK);
    }
    match object {
        Object::Array(items) => {
            let names_a_page = items
                .first()
                .and_then(Object::as_reference)
                .is_some_and(|at| pages.contains(&at));
            let placed = items
                .get(1)
                .and_then(Object::as_name)
                .is_some_and(|name| PLACED_DESTINATIONS.contains(&name.as_bytes()));
            if names_a_page && placed && items.iter().skip(2).any(|it| it.as_number().is_some()) {
                return Some(DESTINATION_IN_PAGE_UNITS);
            }
            items
                .iter()
                .find_map(|item| walk(item, pages, depth.saturating_add(1), budget))
        }
        Object::Dictionary(dict) => {
            let layout = dict
                .get("O")
                .and_then(Object::as_name)
                .is_some_and(|owner| owner.as_bytes() == b"Layout");
            if layout && dict.get("BBox").is_some() {
                return Some(LAYOUT_IN_PAGE_UNITS);
            }
            dict.iter()
                .find_map(|(_, value)| walk(value, pages, depth.saturating_add(1), budget))
        }
        _ => None,
    }
}

/// A number as `pdf_syntax::write` spells a real: the shortest decimal that reads back the same.
fn operand(value: f64) -> String {
    let mut written = Vec::new();
    pdf_syntax::write::object(&Object::Real(value), &mut written);
    String::from_utf8_lossy(&written).into_owned()
}

/// A flate-encoded content stream holding exactly `data`.
fn stream(data: &[u8]) -> Result<Object, Because> {
    preserve::content_stream(data).ok_or(Because::NotBuiltYet(NO_SPARE))
}

/// Why no object number could be found for a bracket's stream.
const NO_SPARE: &str = "no unused object number could be found for the streams that bracket a \
     page restated in larger units";

/// Why a page this reader cannot reach is not restated.
const NO_SUCH_PAGE: &str = "this page's media box is outside the range ISO 19005-2 section \
     6.1.13 admits, and the page is not one this reader reaches as an object of its own, so \
     there is no dictionary to restate in larger units";

/// Why a `/UserUnit` this reader cannot read stops the restatement.
const USER_UNIT_UNREADABLE: &str = "this page states a /UserUnit that is not a positive number, \
     which ISO 32000-2 \u{a7}7.7.3.3's Table 31 requires it to be, so the unit the page is \
     restated from is not known";

/// Why content that does not decode stops the restatement.
const CONTENT_DID_NOT_DECODE: &str = "part of this page's content does not decode, so the \
     graphics states it leaves open are not known and the stream closing the restatement's \
     bracket cannot be written balanced (ISO 32000-2 \u{a7}8.4.2)";

/// Why a page with annotations is not restated.
const ANNOTATIONS_IN_PAGE_UNITS: &str = "this page's media box is outside the range ISO \
     19005-2 section 6.1.13 admits, and restating it in larger units (ISO 32000-2 \u{a7}7.7.3.3's \
     /UserUnit) brackets its content stream alone: its annotations' rectangles (\u{a7}12.5.2) \
     would keep counting the old units, so the page is not restated";

/// Why a page with beads or viewports is not restated.
const BEADS_IN_PAGE_UNITS: &str = "this page's media box is outside the range ISO 19005-2 \
     section 6.1.13 admits, and restating it in larger units brackets its content stream alone: \
     its article beads (ISO 32000-2 \u{a7}12.4.3) or viewports (\u{a7}12.9) would keep counting \
     the old units, so the page is not restated";

/// Why a page whose resources hold a pattern is not restated.
const PATTERNS_IN_PAGE_UNITS: &str = "this page's media box is outside the range ISO 19005-2 \
     section 6.1.13 admits, and restating it in larger units brackets its content stream with a \
     cm: a pattern in the page's resources is placed by a matrix ISO 32000-2 \u{a7}8.7.2 maps to \
     the page's default coordinate space, which the cm does not change, so the page is not \
     restated";

/// Why a destination with a coordinate on a restated page stops the restatement.
const DESTINATION_IN_PAGE_UNITS: &str = "a destination names a page whose media box is outside \
     the range ISO 19005-2 section 6.1.13 admits with a coordinate in that page's units (ISO \
     32000-2 Table 149), which a restatement in larger units would leave counting the old ones";

/// Why a layout attribute's `/BBox` stops the restatement.
const LAYOUT_IN_PAGE_UNITS: &str = "the structure tree states a layout attribute's BBox (ISO \
     32000-2 \u{a7}14.8.5.4.3), in the units of a page this reader does not identify here, so a \
     page restated in larger units could leave it counting the old ones";

/// Why a document too large to search for coordinates stops the restatement.
const TOO_LARGE_TO_WALK: &str = "the document is past the bound on the search for coordinates \
     stated in a restated page's units, so whether a destination or attribute counts them is not \
     known";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factor_is_the_smallest_power_of_two_that_fits() {
        // ISO 19005-2 section 6.1.13: every side between 3 and 14 400 units.
        assert_eq!(factor(&[12_608.0, 16_806.0]), Some(2.0));
        assert_eq!(factor(&[100.0, 57_601.0]), Some(8.0));
        assert_eq!(factor(&[2.0, 100.0]), Some(0.5));
        assert_eq!(factor(&[612.0, 792.0]), Some(1.0));
        // A side of no length has no unit that measures it at three.
        assert_eq!(factor(&[0.0, 600.0]), None);
        // 20 000 needs a factor of 2 and 4 then falls under 3: sides further apart than 4 800 to 1.
        assert_eq!(factor(&[20_000.0, 4.0]), None);
    }
}
