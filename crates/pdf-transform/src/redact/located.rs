//! Shading data located in the region, ISO 32000-2 §8.7.4.5 and §12.5.6.23 (ADR 1363).
//!
//! §12.5.6.23 destroys "that portion of the image data" a region contains, and forbids hiding it:
//! "clipping or image masks shall not be used to hide that data". A shading's marks inside the
//! region are cut away by its clip ([`super::shading`]); what this module decides is which of the
//! shading's **data** are located in the region, and destroys those.
//!
//! # Where a shading's data are located
//!
//! Each shading type says where its values land, and a value is located wherever it colours the
//! plane — the clip does not move it, as a clip does not move an image's samples:
//!
//! - **Axial** (§8.7.4.5.3): a parameter's colour lies along a line that "extends indefinitely
//!   perpendicular to that axis", so no value is located in the region alone. Its marks there are
//!   cut by the clip and nothing is destroyed, whatever function states the colours.
//! - **Radial** (§8.7.4.5.4): a parameter's colour lies on its blend circle — "as if an infinite
//!   number of such circles are painted in turn" — so the values located in the region alone are
//!   those whose circles lie wholly inside it ([`radial_only_here`]). The first and last values
//!   also lie on every circle an `/Extend` entry adds past them.
//! - **Function-based** (§8.7.4.5.2): the colour at every point of the domain is the function's
//!   there, so a sampled function's sample is located over the cells it is interpolated across.
//! - **Meshes** (§8.7.4.5.5 to §8.7.4.5.8): colours at vertices and corners, cut in [`super::mesh`];
//!   a `/Function`'s parameter is located on the pieces that carry it.
//!
//! # What is destroyed, by what a function is
//!
//! A value located in the region alone is destroyed; one also located outside it is the colour of
//! a mark the annotation did not identify, and is kept. A §7.10.2 **sampled** function's samples
//! are cleared to the zero of their integer domain — the image's own rule (ADR 1124) — where every
//! parameter a sample is interpolated over is one the region alone carries; with `/Order` 3 a
//! sample reaches every parameter (a cubic spline is not local), so it is cleared only where they
//! all are. A §7.10.3 **exponential** function is a law with two colours, `/C0` and `/C1`, which
//! any two of its values outside the region determine; so it is zeroed only where every parameter
//! it serves is the region's alone. A §7.10.4 **stitching** function is taken piece by piece
//! through its `/Encode`. A §7.10.5 **calculator** function is a program: its text is not divided
//! by the parameters it serves and is not rewritten, so a one-input program is served by a
//! stitching function written over it (ADR 2528) — the producer's program, as the producer's own
//! object, for every parameter outside the spans the region alone carries, and the constant a
//! sampled function's samples are cleared to for those inside. A function-based shading's
//! two-input program has no stitching to divide it, and the page is refused by name where the
//! region meets its domain.
//!
//! # How the destroyed data are written
//!
//! A shading or pattern whose data this page destroys is written afresh for the page — the
//! producer's dictionary with the destroyed parts replaced — and the resource entry naming it in
//! the stream that painted it is given the new object. The producer's object stays for whatever
//! else names it, so a placement elsewhere that draws it outside the region draws what the producer
//! wrote; two placements on one page naming the same entry take the union of what each destroys,
//! as two placements of one image do (ADR 1277).

use std::cell::Cell;
use std::collections::HashMap;

use pdf_render::Transform;
use pdf_render::geom::Point;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::serialize::flate_encode;

use super::mesh::{self, Placement};
use super::{Walk, mapping};

/// Which resource category a destroyed entry is named in.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum Category {
    /// Table 34's `/Shading`, named by `sh`.
    Shading,
    /// Table 34's `/Pattern`, named by `scn` or `SCN` for a §8.7.4 shading pattern.
    Pattern,
}

impl Category {
    /// The resource dictionary key.
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Shading => "Shading",
            Self::Pattern => "Pattern",
        }
    }
}

/// Which object a destruction is written for: the object an entry names by reference, or a
/// direct entry of one resource dictionary — the page's (`None`) or a form's.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) enum Key {
    /// An indirect object.
    Object(ObjectId),
    /// A direct entry.
    Direct {
        form: Option<ObjectId>,
        category: Category,
        name: Vec<u8>,
    },
}

/// One entry some placement asked to have its located data destroyed, with every placement.
pub(super) struct Request {
    category: Category,
    /// The entry as the resource dictionary states it.
    entry: Object,
    /// Each placement's map from the shading's space into the display list's.
    placements: Vec<Transform>,
}

/// A replacement object, in the source document's terms except where it holds an object of its
/// own, which the writer places and names by its new number.
#[derive(Clone, Debug)]
pub(super) enum Built {
    /// A value carried as the producer wrote it.
    Source(Object),
    /// A dictionary, entry by entry.
    Dict(Vec<(Name, Built)>),
    /// An array, element by element.
    Array(Vec<Built>),
    /// An indirect object of its own, holding the value.
    New(Box<Built>),
    /// A stream: its dictionary, and its data, already `FlateDecode`d.
    Stream(Vec<(Name, Built)>, Vec<u8>),
}

/// Closed intervals of a parameter, kept sorted and merged.
#[derive(Clone, Debug, Default)]
pub(super) struct Intervals(Vec<(f64, f64)>);

impl Intervals {
    /// The union of the given intervals, each put in order.
    pub(super) fn of(items: impl IntoIterator<Item = (f64, f64)>) -> Self {
        let mut items: Vec<(f64, f64)> = items
            .into_iter()
            .map(|(a, b)| (a.min(b), a.max(b)))
            .filter(|(a, b)| a.is_finite() || b.is_finite())
            .collect();
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::with_capacity(items.len());
        for (low, high) in items {
            match merged.last_mut() {
                Some(last) if low <= last.1 => last.1 = last.1.max(high),
                _ => merged.push((low, high)),
            }
        }
        Self(merged)
    }

    /// Whether any interval shares a point with `[low, high]` — touching counts, which keeps a
    /// sample whose weight there is zero: the direction that destroys less, never more.
    pub(super) fn intersects(&self, low: f64, high: f64) -> bool {
        self.0.iter().any(|(a, b)| *a <= high && low <= *b)
    }

    /// The parts of these intervals inside `[low, high]`.
    fn within(&self, low: f64, high: f64) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(a, b)| {
                    let (a, b) = (a.max(low), b.min(high));
                    (a <= b).then_some((a, b))
                })
                .collect(),
        )
    }

    /// Each interval carried by an increasing or decreasing affine map.
    fn mapped(&self, map: impl Fn(f64) -> f64) -> Self {
        Self::of(self.0.iter().map(|(a, b)| (map(*a), map(*b))))
    }

    /// `self` less `other`, as closed intervals: each piece keeps the boundary it shares with
    /// `other`, so a value on the boundary counts as kept.
    fn less(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        for (low, high) in &self.0 {
            let mut from = *low;
            let mut covered = false;
            for (a, b) in &other.0 {
                if *b < from {
                    continue;
                }
                if *a > *high {
                    break;
                }
                if *a > from {
                    out.push((from, *a));
                }
                from = from.max(*b);
                if from >= *high {
                    covered = true;
                    break;
                }
            }
            if !covered {
                out.push((from, *high));
            }
        }
        Self::of(out)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Walk<'_> {
    /// Asks for a `sh` operand's located data to be destroyed at this placement; the entry is the
    /// one `/Resources /Shading` names.
    pub(super) fn request_shading(&mut self, name: &[u8], entry: &Object) {
        self.request(Category::Shading, name, entry, self.ctm);
    }

    /// Asks for a shading pattern's located data to be destroyed at a painted path's placement.
    ///
    /// §8.7.3.1: a pattern's `/Matrix` "maps pattern space to the default coordinate space of
    /// the pattern's parent content stream", which is the space the stream running now began in.
    pub(super) fn request_pattern(&mut self, name: &[u8], entry: &Object, pattern: &Dictionary) {
        let matrix = super::stated_matrix(self.document, pattern);
        self.request(
            Category::Pattern,
            name,
            entry,
            matrix.then(self.stream_base),
        );
    }

    fn request(&mut self, category: Category, name: &[u8], entry: &Object, to_display: Transform) {
        let key = match entry {
            Object::Reference(id) => Key::Object(*id),
            _ => Key::Direct {
                form: self.forms_open.last().copied(),
                category,
                name: name.to_vec(),
            },
        };
        if let Some((_, request)) = self.located.iter_mut().find(|(known, _)| *known == key) {
            request.placements.push(to_display);
        } else {
            self.located.push((
                key.clone(),
                Request {
                    category,
                    entry: entry.clone(),
                    placements: vec![to_display],
                },
            ));
        }
        if !self
            .added_overrides
            .iter()
            .any(|(known, named, _)| *known == category && named.as_slice() == name)
        {
            self.added_overrides.push((category, name.to_vec(), key));
        }
    }

    /// Builds each requested entry's replacement, or `None` where nothing of it is located in the
    /// region alone; or the refusal by name.
    pub(super) fn finish_located(&self) -> Result<HashMap<Key, Built>, String> {
        let mut out = HashMap::new();
        for (key, request) in &self.located {
            let built = match request.category {
                Category::Shading => self.build_shading(&request.entry, &request.placements)?,
                Category::Pattern => self.build_pattern(&request.entry, &request.placements)?,
            };
            if let Some(built) = built {
                out.insert(key.clone(), built);
            }
        }
        Ok(out)
    }

    fn build_pattern(
        &self,
        entry: &Object,
        placements: &[Transform],
    ) -> Result<Option<Built>, String> {
        let object = self.document.resolve(entry);
        let Some(pattern) = object.as_dict() else {
            return Ok(None);
        };
        let Some(shading) = pattern.get("Shading") else {
            return Ok(None);
        };
        let Some(built) = self.build_shading(shading, placements)? else {
            return Ok(None);
        };
        Ok(Some(wrap(entry, rebuilt(pattern, "Shading", built))))
    }

    fn build_shading(
        &self,
        entry: &Object,
        placements: &[Transform],
    ) -> Result<Option<Built>, String> {
        let object = self.document.resolve(entry);
        let dict = match &object {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            _ => return Ok(None),
        };
        match self.document.get_key(&dict, "ShadingType").as_integer() {
            Some(1) => self.build_function_based(entry, &dict, placements),
            Some(2) => Ok(None),
            Some(3) => self.build_radial(entry, &dict, placements),
            Some(4..=7) => self.build_mesh(entry, placements),
            kind => Err(format!(
                "§8.7.4.3: a shading of /ShadingType {} meets the region, whose data this build \
                 does not locate; the page is refused",
                kind.map_or_else(|| "absent".to_owned(), |kind| kind.to_string())
            )),
        }
    }

    /// A radial shading: the values whose circles lie wholly inside the region, destroyed.
    fn build_radial(
        &self,
        entry: &Object,
        dict: &Dictionary,
        placements: &[Transform],
    ) -> Result<Option<Built>, String> {
        let coords = super::numbers(self.document, dict, "Coords").unwrap_or_default();
        let [x0, y0, r0, x1, y1, r1] = <[f32; 6]>::try_from(coords.as_slice())
            .map_err(|_| {
                "§8.7.4.5.4: a radial shading meeting the region does not state its six \
                 /Coords; the page is refused"
                    .to_owned()
            })?
            .map(f64::from);
        if r0 == 0.0 && r1 == 0.0 {
            return Ok(None);
        }
        let domain = pair(self.document, dict, "Domain").unwrap_or((0.0, 1.0));
        let extend = extend(self.document, dict);
        let circles = Circles {
            centre: (kurbo::Point::new(x0, y0), kurbo::Point::new(x1, y1)),
            radius: (r0, r1),
            extend,
        };
        let regions = self.region_bounds();
        let mut only_here: Vec<(f64, f64)> = Vec::new();
        let mut kept_ends: Vec<f64> = Vec::new();
        for placement in placements {
            let (spans, escapes) = radial_only_here(&circles, *placement, &regions);
            only_here.extend(spans);
            if escapes.0 {
                kept_ends.push(0.0);
            }
            if escapes.1 {
                kept_ends.push(1.0);
            }
        }
        if only_here.is_empty() {
            return Ok(None);
        }
        let to_t = |s: f64| s.mul_add(domain.1 - domain.0, domain.0);
        let used = Intervals::of([(domain.0, domain.1)]);
        let only_here = Intervals::of(only_here).mapped(to_t);
        let kept = Intervals::of(
            used.less(&only_here)
                .0
                .into_iter()
                .chain(kept_ends.into_iter().map(|s| (to_t(s), to_t(s)))),
        );
        let Some(function) = dict.get("Function") else {
            return Ok(None);
        };
        let Some(built) = self.build_function_tree(function, &used, &kept)? else {
            return Ok(None);
        };
        Ok(Some(wrap(entry, rebuilt(dict, "Function", built))))
    }

    /// A function-based shading: the samples whose cells lie wholly inside the region, cleared.
    fn build_function_based(
        &self,
        entry: &Object,
        dict: &Dictionary,
        placements: &[Transform],
    ) -> Result<Option<Built>, String> {
        let domain = super::numbers(self.document, dict, "Domain")
            .and_then(|values| <[f32; 4]>::try_from(values.as_slice()).ok())
            .map_or([0.0, 1.0, 0.0, 1.0], |values| values.map(f64::from));
        let matrix = super::stated_matrix(self.document, dict);
        let to_display: Vec<Transform> = placements.iter().map(|to| matrix.then(*to)).collect();
        let Some(function) = dict.get("Function") else {
            return Ok(None);
        };
        let built = match self.document.resolve(function) {
            Object::Array(items) => {
                let mut any = false;
                let mut out = Vec::with_capacity(items.len());
                for item in &items {
                    let one = self.build_function_2in(item, domain, &to_display)?;
                    any |= one.is_some();
                    out.push(one.unwrap_or_else(|| Built::Source(item.clone())));
                }
                any.then_some(Built::Array(out))
            }
            _ => self.build_function_2in(function, domain, &to_display)?,
        };
        Ok(built.map(|built| wrap(entry, rebuilt(dict, "Function", built))))
    }

    /// One 2-in function of a function-based shading.
    fn build_function_2in(
        &self,
        entry: &Object,
        domain: [f64; 4],
        to_display: &[Transform],
    ) -> Result<Option<Built>, String> {
        let object = self.document.resolve(entry);
        let dict = match &object {
            Object::Stream(stream) => stream.dict.clone(),
            Object::Dictionary(dict) => dict.clone(),
            _ => return Ok(None),
        };
        let regions = self.region_bounds();
        match self.document.get_key(&dict, "FunctionType").as_integer() {
            Some(0) => {
                let Some(sampled) = Sampled::read(self.document, &object) else {
                    return Err(sampled_unreadable());
                };
                let cleared = sampled.cleared(|support| {
                    let [Some(x), Some(y)] = [support.first(), support.get(1)] else {
                        return false;
                    };
                    let (x0, x1) = (x.0.max(domain[0]), x.1.min(domain[1]));
                    let (y0, y1) = (y.0.max(domain[2]), y.1.min(domain[3]));
                    if x0 > x1 || y0 > y1 {
                        return false;
                    }
                    to_display.iter().any(|to| {
                        let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                            .map(|(x, y)| to.apply(Point::new(narrow(x), narrow(y))));
                        let [bx0, by0, bx1, by1] = box_of(&corners);
                        regions.iter().any(|region| {
                            bx0 >= region[0]
                                && bx1 <= region[2]
                                && by0 >= region[1]
                                && by1 <= region[3]
                        })
                    })
                });
                Ok(cleared.map(|data| Sampled::stream(&dict, &data)))
            }
            Some(4) => {
                let meets = to_display.iter().any(|to| {
                    let corners = [
                        (domain[0], domain[2]),
                        (domain[1], domain[2]),
                        (domain[1], domain[3]),
                        (domain[0], domain[3]),
                    ]
                    .map(|(x, y)| to.apply(Point::new(narrow(x), narrow(y))));
                    let [bx0, by0, bx1, by1] = box_of(&corners);
                    regions.iter().any(|region| {
                        bx0 < region[2] && bx1 > region[0] && by0 < region[3] && by1 > region[1]
                    })
                });
                if meets {
                    return Err(two_input_calculator_refusal());
                }
                Ok(None)
            }
            kind => Err(format!(
                "§8.7.4.5.2: a function-based shading meeting the region names a function of \
                 /FunctionType {}, which is not one of two inputs; the page is refused",
                kind.map_or_else(|| "absent".to_owned(), |kind| kind.to_string())
            )),
        }
    }

    /// A shading's `/Function`, whole: [`Self::build_function`] under the room the viewer's own
    /// function reader leaves the rewritten tree (ADR 2528).
    ///
    /// The reader builds at most [`pdf_model::function::MAX_FUNCTIONS`] functions for one root and
    /// counts a function once for every time a reference to it is followed, so a calculator
    /// function that an outer stitching function names in k pieces is k nodes, each of which can be
    /// stitched on its own. Past the budget the written tree would not draw anywhere, outside the
    /// region too; `room` is what the producer's own tree leaves, spent by every stitching function
    /// written over a calculator.
    fn build_function_tree(
        &self,
        function: &Object,
        used: &Intervals,
        kept: &Intervals,
    ) -> Result<Option<Built>, String> {
        let mut nodes = 0;
        self.count_function_nodes(function, 0, &mut nodes);
        let room = Cell::new(pdf_model::function::MAX_FUNCTIONS.saturating_sub(nodes));
        self.build_function(function, used, kept, &room)
    }

    /// The functions the reader would build for `entry`: one for each dictionary or stream, each
    /// time it is named, with a stitching function's pieces beneath it. The count stops once it
    /// passes the reader's budget, and at the depth the reader refuses.
    fn count_function_nodes(&self, entry: &Object, depth: usize, nodes: &mut usize) {
        if *nodes > pdf_model::function::MAX_FUNCTIONS || depth > MAX_FUNCTION_DEPTH {
            return;
        }
        let object = self.document.resolve(entry);
        if let Object::Array(items) = &object {
            for item in items {
                self.count_function_nodes(item, depth, nodes);
            }
            return;
        }
        let dict = match &object {
            Object::Stream(stream) => &stream.dict,
            Object::Dictionary(dict) => dict,
            _ => return,
        };
        *nodes = nodes.saturating_add(1);
        if self.document.get_key(dict, "FunctionType").as_integer() == Some(3)
            && let Some(pieces) = self.document.get_key(dict, "Functions").as_array()
        {
            for piece in pieces {
                self.count_function_nodes(piece, depth.saturating_add(1), nodes);
            }
        }
    }

    /// A 1-in function's values that `kept` does not reach, destroyed where every parameter a
    /// piece of the function serves is one of them; `used` is what the shading passes it.
    fn build_function(
        &self,
        entry: &Object,
        used: &Intervals,
        kept: &Intervals,
        room: &Cell<usize>,
    ) -> Result<Option<Built>, String> {
        let object = self.document.resolve(entry);
        if let Object::Array(items) = &object {
            let mut any = false;
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let one = self.build_function(item, used, kept, room)?;
                any |= one.is_some();
                out.push(one.unwrap_or_else(|| Built::Source(item.clone())));
            }
            return Ok(any.then_some(Built::Array(out)));
        }
        let dict = match &object {
            Object::Stream(stream) => stream.dict.clone(),
            Object::Dictionary(dict) => dict.clone(),
            _ => return Ok(None),
        };
        let domain = pair(self.document, &dict, "Domain").unwrap_or((0.0, 1.0));
        let used = used.within(domain.0.min(domain.1), domain.0.max(domain.1));
        if used.is_empty() {
            return Ok(None);
        }
        match self.document.get_key(&dict, "FunctionType").as_integer() {
            Some(0) => {
                let Some(sampled) = Sampled::read(self.document, &object) else {
                    return Err(sampled_unreadable());
                };
                let cleared = sampled.cleared(|support| {
                    support.first().is_some_and(|(low, high)| {
                        used.intersects(*low, *high) && !kept.intersects(*low, *high)
                    })
                });
                Ok(cleared.map(|data| Sampled::stream(&dict, &data)))
            }
            Some(2) => {
                if used
                    .0
                    .iter()
                    .any(|(low, high)| kept.intersects(*low, *high))
                {
                    return Ok(None);
                }
                let zeros = |key: &str, default: usize| {
                    let count = self
                        .document
                        .get_key(&dict, key)
                        .as_array()
                        .map_or(default, <[Object]>::len);
                    Built::Array(vec![Built::Source(Object::Integer(0)); count])
                };
                let mut entries: Vec<(Name, Built)> = dict
                    .iter()
                    .filter(|(key, _)| !matches!(key.as_bytes(), b"C0" | b"C1"))
                    .map(|(key, value)| (key.clone(), Built::Source(value.clone())))
                    .collect();
                entries.push((Name::new(&b"C0"[..]), zeros("C0", 1)));
                entries.push((Name::new(&b"C1"[..]), zeros("C1", 1)));
                Ok(Some(wrap(entry, Built::Dict(entries))))
            }
            Some(3) => self.build_stitching(entry, &dict, domain, (&used, kept), room),
            Some(4) => self.build_calculator(entry, &dict, (&used, kept), room),
            kind => Err(format!(
                "§7.10: a shading meeting the region names a function of /FunctionType {}; the \
                 page is refused",
                kind.map_or_else(|| "absent".to_owned(), |kind| kind.to_string())
            )),
        }
    }

    /// §7.10.5: a calculator function serving parameters the region alone carries, stitched.
    ///
    /// The producer's program is not divided and not rewritten: a §7.10.4 stitching function whose
    /// `/Bounds` cut the region-only parameter spans out serves the program, unchanged, to every
    /// parameter outside them and a constant (ADR 1124's) to those inside (ADR 2528).
    fn build_calculator(
        &self,
        entry: &Object,
        dict: &Dictionary,
        (used, kept): (&Intervals, &Intervals),
        room: &Cell<usize>,
    ) -> Result<Option<Built>, String> {
        let region_only = Intervals(
            used.less(kept)
                .0
                .into_iter()
                .filter(|(low, high)| high > low)
                .collect(),
        );
        if region_only.is_empty() {
            return Ok(None);
        }
        let domain = match pair(self.document, dict, "Domain") {
            Some((low, high)) if low < high => (low, high),
            _ => {
                return Err(calculator_unstitchable(
                    "does not state two increasing /Domain numbers",
                ));
            }
        };
        let outputs =
            super::numbers(self.document, dict, "Range").map_or(0, |range| range.len() / 2);
        if outputs == 0 || outputs > MAX_CALCULATOR_OUTPUTS {
            return Err(calculator_unstitchable(
                "states no /Range of between one and sixty-four outputs",
            ));
        }
        let pieces = stitch_plan(domain, &region_only)?;
        // The stitching function and its pieces replace the one node the program was.
        let added = pieces.len();
        let left = room.get().checked_sub(added).ok_or_else(|| {
            calculator_unstitchable(
                "would grow the function tree past the budget the viewer's function reader builds",
            )
        })?;
        room.set(left);
        let range = dict.get("Range").cloned().unwrap_or(Object::Null);
        let reals = |values: &[f64]| {
            Built::Array(
                values
                    .iter()
                    .map(|value| Built::Source(Object::Real(*value)))
                    .collect(),
            )
        };
        let constant = || {
            let zeros = || Built::Array(vec![Built::Source(Object::Integer(0)); outputs]);
            Built::Dict(vec![
                (key("FunctionType"), Built::Source(Object::Integer(2))),
                (key("Domain"), reals(&[0.0, 1.0])),
                (key("C0"), zeros()),
                (key("C1"), zeros()),
                (key("N"), Built::Source(Object::Integer(1))),
                (key("Range"), Built::Source(range.clone())),
            ])
        };
        let mut encode = Vec::with_capacity(pieces.len().saturating_mul(2));
        for (index, piece) in pieces.iter().enumerate() {
            if piece.cleared {
                encode.extend([0.0, 1.0]);
            } else {
                // The piece's own span, so the program is handed the input it was handed before.
                let end = pieces
                    .get(index.saturating_add(1))
                    .map_or(domain.1, |next| next.start);
                encode.extend([piece.start, end]);
            }
        }
        let bounds: Vec<f64> = pieces.iter().skip(1).map(|piece| piece.start).collect();
        let functions = pieces
            .iter()
            .map(|piece| {
                if piece.cleared {
                    constant()
                } else {
                    Built::Source(entry.clone())
                }
            })
            .collect();
        Ok(Some(wrap(
            entry,
            Built::Dict(vec![
                (key("FunctionType"), Built::Source(Object::Integer(3))),
                (key("Domain"), reals(&[domain.0, domain.1])),
                (key("Functions"), Built::Array(functions)),
                (key("Bounds"), reals(&bounds)),
                (key("Encode"), reals(&encode)),
                (key("Range"), Built::Source(range)),
            ]),
        )))
    }

    /// §7.10.4: each piece of a stitching function, taken through its `/Encode`.
    fn build_stitching(
        &self,
        entry: &Object,
        dict: &Dictionary,
        domain: (f64, f64),
        (used, kept): (&Intervals, &Intervals),
        room: &Cell<usize>,
    ) -> Result<Option<Built>, String> {
        let functions = self.document.get_key(dict, "Functions");
        let Some(functions) = functions.as_array() else {
            return Ok(None);
        };
        let bounds = super::numbers(self.document, dict, "Bounds").unwrap_or_default();
        let encode = super::numbers(self.document, dict, "Encode").unwrap_or_default();
        let mut edges = vec![domain.0];
        edges.extend(bounds.iter().map(|value| f64::from(*value)));
        edges.push(domain.1);
        let mut any = false;
        let mut out = Vec::with_capacity(functions.len());
        for (index, function) in functions.iter().enumerate() {
            let (Some(low), Some(high)) = (edges.get(index), edges.get(index.saturating_add(1)))
            else {
                out.push(Built::Source(function.clone()));
                continue;
            };
            let (e0, e1) = (
                encode
                    .get(index.saturating_mul(2))
                    .map_or(0.0, |value| f64::from(*value)),
                encode
                    .get(index.saturating_mul(2).saturating_add(1))
                    .map_or(1.0, |value| f64::from(*value)),
            );
            let map = |x: f64| {
                if high > low {
                    (x - low) / (high - low) * (e1 - e0) + e0
                } else {
                    e0
                }
            };
            let sub_used = used.within(*low, *high).mapped(map);
            let sub_kept = kept.within(*low, *high).mapped(map);
            let one = if sub_used.is_empty() {
                None
            } else {
                self.build_function(function, &sub_used, &sub_kept, room)?
            };
            any |= one.is_some();
            out.push(one.unwrap_or_else(|| Built::Source(function.clone())));
        }
        if !any {
            return Ok(None);
        }
        Ok(Some(wrap(
            entry,
            rebuilt(dict, "Functions", Built::Array(out)),
        )))
    }

    /// A mesh cut by every placement, written afresh with its `/Function`'s region-only values
    /// destroyed.
    fn build_mesh(
        &self,
        entry: &Object,
        placements: &[Transform],
    ) -> Result<Option<Built>, String> {
        let object = self.document.resolve(entry);
        let Some(stream) = object.as_stream() else {
            return Err(
                "§8.7.4.5.5: a mesh shading meeting the region is not a stream; the page is \
                 refused"
                    .to_owned(),
            );
        };
        let mut read = mesh::read(self.document, stream)?;
        let used = Intervals::of(read.parameter_spans());
        let regions = self.region_bounds();
        let mut changed = false;
        for to_display in placements {
            let placement = Placement {
                to_display: mapping(*to_display),
                regions: regions.clone(),
            };
            changed |= if read.kind <= 5 {
                read.cut_triangles(&placement)?
            } else {
                read.cut_patches(&placement)?
            };
        }
        if !changed {
            return Ok(None);
        }
        let encoded = read.encode(&stream.dict);
        for to_display in placements {
            if !encoded.margin_holds(mapping(*to_display).norm()) {
                return Err(
                    "§7.3.3: a coordinate of the cut mesh is large enough that writing it on a \
                     32-bit grid and reading it at single precision could move it inside the \
                     region; the page is refused"
                        .to_owned(),
                );
            }
        }
        let mut entries: Vec<(Name, Built)> = encoded
            .dict
            .iter()
            .map(|(key, value)| (key.clone(), Built::Source(value.clone())))
            .collect();
        if let Some(function) = stream.dict.get("Function") {
            let kept = Intervals::of(read.parameter_spans());
            if let Some(built) = self.build_function_tree(function, &used, &kept)? {
                entries.retain(|(key, _)| key.as_bytes() != b"Function");
                entries.push((Name::new(&b"Function"[..]), built));
            }
        }
        entries.push((
            Name::new(&b"Filter"[..]),
            Built::Source(Object::Name(Name::new(&b"FlateDecode"[..]))),
        ));
        let data = flate_encode(&encoded.data, 6).ok_or_else(|| {
            "§12.5.6.23: the cut mesh could not be re-encoded; the page is refused".to_owned()
        })?;
        Ok(Some(Built::New(Box::new(Built::Stream(entries, data)))))
    }
}

/// The two circles of a radial shading and its `/Extend`, in the shading's space.
struct Circles {
    centre: (kurbo::Point, kurbo::Point),
    radius: (f64, f64),
    extend: (bool, bool),
}

/// The values of `s` in `[0, 1]` whose blend circles lie wholly inside one region box, under one
/// placement, and whether the circles an `/Extend` entry adds past each end leave every box — in
/// which case that end's value is also located outside the region.
///
/// A circle maps to an ellipse whose box is its centre's image plus or minus the radius times the
/// length of each row of the linear map, and centre and radius are both affine in `s` (§8.7.4.5.4's
/// parametric equations), so "the circle's box lies inside the region's" is four affine
/// inequalities in `s`: an interval.
fn radial_only_here(
    circles: &Circles,
    to_display: Transform,
    regions: &[[f64; 4]],
) -> (Vec<(f64, f64)>, (bool, bool)) {
    let map = mapping(to_display);
    let (c0, c1) = circles.centre;
    let (r0, r1) = circles.radius;
    let across = map.a.hypot(map.c);
    let up = map.b.hypot(map.d);
    let start = map.apply(c0);
    let end = map.apply(c1);
    let mut spans = Vec::new();
    // Where the extension past each end stops: at the radius's zero on the side it shrinks
    // towards, and nowhere on the side it grows.
    let low_extension = if r1 > r0 {
        -r0 / (r1 - r0)
    } else {
        f64::NEG_INFINITY
    };
    let high_extension = if r0 > r1 {
        1.0 + r1 / (r0 - r1)
    } else {
        f64::INFINITY
    };
    let mut low_escapes = circles.extend.0;
    let mut high_escapes = circles.extend.1;
    for region in regions {
        // Each side as `alpha + beta · s ≥ 0`.
        let sides = [
            (
                start.x - r0 * across - region[0],
                (end.x - start.x) - (r1 - r0) * across,
            ),
            (
                region[2] - start.x - r0 * across,
                -(end.x - start.x) - (r1 - r0) * across,
            ),
            (
                start.y - r0 * up - region[1],
                (end.y - start.y) - (r1 - r0) * up,
            ),
            (
                region[3] - start.y - r0 * up,
                -(end.y - start.y) - (r1 - r0) * up,
            ),
        ];
        let mut low = f64::NEG_INFINITY;
        let mut high = f64::INFINITY;
        for (alpha, beta) in sides {
            if beta > 0.0 {
                low = low.max(-alpha / beta);
            } else if beta < 0.0 {
                high = high.min(-alpha / beta);
            } else if alpha < 0.0 {
                high = f64::NEG_INFINITY;
            }
        }
        if low <= high {
            if low.max(0.0) <= high.min(1.0) {
                spans.push((low.max(0.0), high.min(1.0)));
            }
            if low <= low_extension && high >= 0.0 {
                low_escapes = false;
            }
            if high >= high_extension && low <= 1.0 {
                high_escapes = false;
            }
        }
    }
    (spans, (low_escapes, high_escapes))
}

/// §7.10.2's sample table, as far as clearing samples needs it.
struct Sampled {
    size: Vec<usize>,
    domain: Vec<(f64, f64)>,
    encode: Vec<(f64, f64)>,
    outputs: usize,
    bits: usize,
    cubic: bool,
    data: Vec<u8>,
}

/// The most samples a function's table is read to before the page is refused, the reader's own
/// bound on a table (ADR 0343's resource bound on functions).
const MAX_SAMPLES: usize = 1 << 22;

impl Sampled {
    fn read(document: &pdf_syntax::Document, object: &Object) -> Option<Self> {
        let stream = object.as_stream()?;
        let dict = &stream.dict;
        let size: Vec<usize> = super::numbers(document, dict, "Size")?
            .iter()
            .map(|value| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a count, checked positive and bounded below"
                )]
                let count = *value as usize;
                count
            })
            .collect();
        let domain = pairs(&super::numbers(document, dict, "Domain")?);
        if size.len() != domain.len() || size.contains(&0) {
            return None;
        }
        let outputs = super::numbers(document, dict, "Range")?.len() / 2;
        let bits = usize::try_from(document.get_key(dict, "BitsPerSample").as_integer()?).ok()?;
        if !matches!(bits, 1 | 2 | 4 | 8 | 12 | 16 | 24 | 32) || outputs == 0 {
            return None;
        }
        let encode = super::numbers(document, dict, "Encode").map_or_else(
            || {
                size.iter()
                    .map(|count| {
                        #[expect(clippy::cast_precision_loss, reason = "bounded by MAX_SAMPLES")]
                        let last = count.saturating_sub(1) as f64;
                        (0.0, last)
                    })
                    .collect()
            },
            |values| pairs(&values),
        );
        let total = size
            .iter()
            .try_fold(outputs, |acc, count| acc.checked_mul(*count))?;
        if total > MAX_SAMPLES {
            return None;
        }
        let data = document.decoded_stream_data(stream)?.to_vec();
        if data.len() < total.checked_mul(bits)?.div_ceil(8) {
            return None;
        }
        let cubic = document.get_key(dict, "Order").as_integer() == Some(3);
        Some(Self {
            size,
            domain,
            encode,
            outputs,
            bits,
            cubic,
            data,
        })
    }

    /// The input interval sample `k` of dimension `dimension` is interpolated across: its two
    /// neighbours' positions under `/Encode`, reaching the domain's end at the first and last
    /// sample (whose positions clamp every input beyond them), or the whole domain for `/Order` 3.
    fn support(&self, dimension: usize, k: usize) -> (f64, f64) {
        let (d0, d1) = self.domain[dimension];
        let whole = (d0.min(d1), d0.max(d1));
        let (e0, e1) = self.encode.get(dimension).copied().unwrap_or((0.0, 1.0));
        let last = self.size[dimension].saturating_sub(1);
        if self.cubic
            || (e1 - e0).abs() <= f64::EPSILON
            || (d1 - d0).abs() <= f64::EPSILON
            || last == 0
        {
            return whole;
        }
        #[expect(clippy::cast_precision_loss, reason = "bounded by MAX_SAMPLES")]
        let (low, high) = ((k as f64) - 1.0, (k as f64) + 1.0);
        let to_input = |e: f64| (e - e0) / (e1 - e0) * (d1 - d0) + d0;
        let low_is_end = k == 0;
        let high_is_end = k == last;
        let (a, b) = (to_input(low), to_input(high));
        let (mut from, mut to) = (a.min(b), a.max(b));
        // The end samples are also every input clamped onto them.
        let increasing = (e1 - e0) * (d1 - d0) > 0.0;
        if (low_is_end && increasing) || (high_is_end && !increasing) {
            from = whole.0;
        }
        if (high_is_end && increasing) || (low_is_end && !increasing) {
            to = whole.1;
        }
        (from.max(whole.0), to.min(whole.1))
    }

    /// The table with every sample `clear` answers for zeroed, or `None` where it answers for
    /// none.
    fn cleared(&self, clear: impl Fn(&[(f64, f64)]) -> bool) -> Option<Vec<u8>> {
        let mut data = self.data.clone();
        let count: usize = self.size.iter().product();
        let mut any = false;
        let mut support = Vec::with_capacity(self.size.len());
        for index in 0..count {
            support.clear();
            let mut rest = index;
            for (dimension, size) in self.size.iter().enumerate() {
                support.push(self.support(dimension, rest.checked_rem(*size).unwrap_or(0)));
                rest = rest.checked_div(*size).unwrap_or(0);
            }
            if !clear(&support) {
                continue;
            }
            any = true;
            let first = index.saturating_mul(self.outputs).saturating_mul(self.bits);
            for bit in first..first.saturating_add(self.outputs.saturating_mul(self.bits)) {
                if let Some(byte) = data.get_mut(bit / 8) {
                    *byte &= !(0x80u8 >> (bit % 8));
                }
            }
        }
        any.then_some(data)
    }

    /// The function written afresh: the producer's dictionary with the table re-encoded.
    fn stream(dict: &Dictionary, data: &[u8]) -> Built {
        let mut entries: Vec<(Name, Built)> = dict
            .iter()
            .filter(|(key, _)| {
                !matches!(
                    key.as_bytes(),
                    b"Filter" | b"DecodeParms" | b"DP" | b"Length"
                )
            })
            .map(|(key, value)| (key.clone(), Built::Source(value.clone())))
            .collect();
        entries.push((
            Name::new(&b"Filter"[..]),
            Built::Source(Object::Name(Name::new(&b"FlateDecode"[..]))),
        ));
        let encoded = flate_encode(data, 6).unwrap_or_default();
        Built::New(Box::new(Built::Stream(entries, encoded)))
    }
}

/// The most pieces a stitching function written over a calculator function may have: each
/// cleared piece carries a constant of its own, so the figure bounds what a page of many
/// `/Redact` annotations over one shading makes the rewrite hold.
const MAX_CALCULATOR_PIECES: usize = 1024;

/// How deep a stitching function nests before the viewer's function reader refuses it
/// (`pdf-colour`'s `MAX_STITCH_DEPTH`): the count of a tree stops there.
const MAX_FUNCTION_DEPTH: usize = 8;

/// The most outputs a calculator function's `/Range` may state for its constant to be written:
/// the viewer's function reader reads no function of more, so a tree written for more could not be
/// drawn, and a `/Range` of a million entries would otherwise be copied into every cleared piece.
const MAX_CALCULATOR_OUTPUTS: usize = pdf_model::function::MAX_VALUES;

/// One piece of the stitching function written over a calculator function.
#[derive(Clone, Copy, Debug, PartialEq)]
struct StitchPiece {
    /// Where the piece begins: the `/Domain`'s start for the first, a `/Bounds` entry for the rest.
    start: f64,
    /// Whether the piece serves the constant rather than the producer's program.
    cleared: bool,
}

/// The pieces of the §7.10.4 stitching function that clears `cleared` from `domain` and leaves
/// the rest to the producer's program.
///
/// §7.10.4 gives `/Bounds` as increasing numbers and each piece as "closed on the left and open
/// on the right", so a cleared span's left end is cleared too, and its right end is the next
/// piece's: of a span's two ends, which are parameters the kept span beside it also carries, the
/// left end is destroyed with the span and no parameter the region alone carries is left. The
/// bounds are numbers a reader narrows to `f32`; a program piece that narrowing makes empty is
/// merged into the cleared span beside it, and a cleared span it makes empty is refused rather
/// than left standing. Table 41 gives the last piece one exception, which this plan does not use:
/// "the last interval, shall always be closed on the right", so the domain's end is served by the
/// last piece whichever kind it is.
fn stitch_plan(domain: (f64, f64), cleared: &Intervals) -> Result<Vec<StitchPiece>, String> {
    let mut wanted = Vec::with_capacity(cleared.0.len().saturating_mul(2).saturating_add(1));
    let mut cursor = domain.0;
    for (low, high) in &cleared.0 {
        if *low > cursor {
            wanted.push(StitchPiece {
                start: cursor,
                cleared: false,
            });
        }
        wanted.push(StitchPiece {
            start: *low,
            cleared: true,
        });
        cursor = *high;
    }
    if cursor < domain.1 {
        wanted.push(StitchPiece {
            start: cursor,
            cleared: false,
        });
    }
    let sliver =
        || calculator_unstitchable("has a region-only span narrower than a number is read to");
    let mut pieces: Vec<StitchPiece> = Vec::with_capacity(wanted.len());
    for piece in wanted {
        while let Some(last) = pieces.last() {
            if narrow(piece.start) > narrow(last.start) {
                break;
            }
            if last.cleared {
                return Err(sliver());
            }
            pieces.pop();
        }
        if pieces
            .last()
            .is_some_and(|last| last.cleared == piece.cleared)
        {
            continue;
        }
        let start = if pieces.is_empty() {
            domain.0
        } else {
            piece.start
        };
        pieces.push(StitchPiece {
            start,
            cleared: piece.cleared,
        });
    }
    if pieces.len() > 1
        && pieces
            .last()
            .is_some_and(|last| narrow(last.start) >= narrow(domain.1))
    {
        if pieces.last().is_some_and(|last| last.cleared) {
            return Err(sliver());
        }
        pieces.pop();
    }
    if !pieces.iter().any(|piece| piece.cleared) {
        return Err(sliver());
    }
    if pieces.len() > MAX_CALCULATOR_PIECES {
        return Err(calculator_unstitchable(
            "has more region-only spans than the stitching function written for them may hold",
        ));
    }
    Ok(pieces)
}

/// A dictionary key.
fn key(name: &str) -> Name {
    Name::new(name.as_bytes())
}

/// The refusal for a calculator function whose region-only values cannot be stitched out.
fn calculator_unstitchable(why: &str) -> String {
    format!(
        "§7.10.5: a calculator function serves parameters the region alone carries, and one that \
         {why} cannot have them cut out by a stitching function (§7.10.4); the page is refused"
    )
}

/// The refusal for a two-input calculator function under a region.
///
/// A §7.10.4 stitching function has one input, so no stitching divides a function of two; the
/// program is the producer's text and a rewrite of it would be one this build authored.
fn two_input_calculator_refusal() -> String {
    "§7.10.5: a function-based shading's two-input calculator function serves a domain the region \
     meets, and a stitching function (§7.10.4) divides one input only — a program rewritten to \
     omit the region's values would be one this build authored; the page is refused"
        .to_owned()
}

/// The refusal for a sample table that does not read.
fn sampled_unreadable() -> String {
    "§7.10.2: a sampled function whose values the region carries does not state a table this \
     build reads; the page is refused rather than its samples left in place"
        .to_owned()
}

/// A dictionary with one entry replaced.
fn rebuilt(dict: &Dictionary, key: &str, value: Built) -> Built {
    let mut entries: Vec<(Name, Built)> = dict
        .iter()
        .filter(|(name, _)| name.as_bytes() != key.as_bytes())
        .map(|(name, entry)| (name.clone(), Built::Source(entry.clone())))
        .collect();
    entries.push((Name::new(key.as_bytes()), value));
    Built::Dict(entries)
}

/// A rebuilt value placed as an object of its own where the producer's was one.
fn wrap(entry: &Object, built: Built) -> Built {
    match (entry, built) {
        (_, built @ Built::New(_)) => built,
        (Object::Reference(_), built) => Built::New(Box::new(built)),
        (_, built) => built,
    }
}

/// A two-number entry.
fn pair(document: &pdf_syntax::Document, dict: &Dictionary, key: &str) -> Option<(f64, f64)> {
    let values = super::numbers(document, dict, key)?;
    match values.as_slice() {
        [a, b, ..] => Some((f64::from(*a), f64::from(*b))),
        _ => None,
    }
}

/// Numbers taken two at a time.
fn pairs(values: &[f32]) -> Vec<(f64, f64)> {
    values
        .chunks_exact(2)
        .map(|pair| (f64::from(pair[0]), f64::from(pair[1])))
        .collect()
}

/// Table 80's `/Extend`.
fn extend(document: &pdf_syntax::Document, dict: &Dictionary) -> (bool, bool) {
    let values = document.get_key(dict, "Extend");
    let flag = |index: usize| {
        values
            .as_array()
            .and_then(|items| items.get(index))
            .is_some_and(|item| matches!(document.resolve(item), Object::Boolean(true)))
    };
    (flag(0), flag(1))
}

/// The box of four points.
fn box_of(points: &[Point; 4]) -> [f64; 4] {
    points.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |[x0, y0, x1, y1], point| {
            let (x, y) = (f64::from(point.x), f64::from(point.y));
            [x0.min(x), y0.min(y), x1.max(x), y1.max(y)]
        },
    )
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a shading coordinate is a page coordinate, inside f32"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::{Intervals, MAX_CALCULATOR_PIECES, StitchPiece, stitch_plan};

    fn piece(start: f64, cleared: bool) -> StitchPiece {
        StitchPiece { start, cleared }
    }

    /// §7.10.4's Table 41: the pieces are in order, each begins at a `/Bounds` entry, and the first
    /// at the `/Domain`'s start — a span at either end of the domain leaves no program piece
    /// there, and one that is the whole domain is a single piece.
    #[test]
    fn the_pieces_follow_the_spans_cleared() {
        let plan =
            |spans: &[(f64, f64)]| stitch_plan((0.0, 1.0), &Intervals::of(spans.iter().copied()));
        assert_eq!(
            plan(&[(0.25, 0.5)]).expect("a plan"),
            [piece(0.0, false), piece(0.25, true), piece(0.5, false)]
        );
        assert_eq!(
            plan(&[(0.0, 0.25)]).expect("a plan"),
            [piece(0.0, true), piece(0.25, false)]
        );
        assert_eq!(
            plan(&[(0.5, 1.0)]).expect("a plan"),
            [piece(0.0, false), piece(0.5, true)]
        );
        assert_eq!(plan(&[(0.0, 1.0)]).expect("a plan"), [piece(0.0, true)]);
    }

    /// A reader narrows each number to `f32`: a program piece it makes empty between two spans is
    /// folded into them, and a span it makes empty is refused rather than left standing.
    #[test]
    fn a_piece_narrower_than_a_number_is_read_to_is_folded_or_refused() {
        let folded = stitch_plan((0.0, 1.0), &Intervals::of([(0.1, 0.3), (0.3 + 1e-12, 0.6)]))
            .expect("a plan");
        assert_eq!(
            folded,
            [piece(0.0, false), piece(0.1, true), piece(0.6, false)]
        );
        let sliver = stitch_plan((0.0, 1.0), &Intervals::of([(0.5, 0.5 + 1e-12)]));
        assert!(sliver.is_err(), "{sliver:?}");
    }

    /// The bound on the pieces: a plan of exactly the bound is made and one more is refused.
    #[test]
    fn the_pieces_have_a_bound() {
        let spans = |count: usize| {
            Intervals::of((0..count).map(|k| {
                let low = f64::from(u32::try_from(2 * k).expect("small")) * 1e-3;
                (low, low + 1e-3)
            }))
        };
        let at_the_bound = stitch_plan((0.0, 2.0), &spans(MAX_CALCULATOR_PIECES / 2));
        assert_eq!(at_the_bound.expect("a plan").len(), MAX_CALCULATOR_PIECES);
        let past = stitch_plan((0.0, 2.0), &spans(MAX_CALCULATOR_PIECES / 2 + 1));
        assert!(past.is_err(), "{past:?}");
    }
}
