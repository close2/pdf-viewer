//! Which spot colourants a page names: the input to ISO 32000-2 §10.8.3's step a).
//!
//! §10.8.3 asks for the page to be processed "as if separations were to be created for a
//! simulated device that supports subtractive process colourants and possibly spot colours", and
//! leaves which ones to the processor:
//!
//! > The PDF processor determines what process colours and possible spot colours the simulated
//! > device is to have.
//!
//! A separation is one plane, and a plane has to exist before the first mark lands on it — so
//! the colourants are read off the page's resources before a byte of its content stream is
//! interpreted, which is what [`spot_colourants`] does. How many planes they take is
//! [`SpotColourants::spot_planes`]: a raster carries three channels (§11.3.4 composites each
//! component on its own), so `S` colourants are `ceil(S / 3)` planes beside the two
//! [`crate::colour::Plane::PROCESS`] planes the four process components already take. ADR 1281
//! is the design, and what the planes are used for is its section 3.
//!
//! # What counts as a spot colourant
//!
//! A `Separation` space's `name` and a `DeviceN` space's `names`, less what §8.6.6.4 and
//! §8.6.6.5 say is not a colourant of its own:
//!
//! - **`None`** marks nothing — "The special colourant name None shall not produce any visible
//!   output." — so it needs no plane.
//! - **`All`** is every colourant rather than one of them — "The special colourant name All shall
//!   refer collectively to all colourants available on an output device, including those for the
//!   standard process colourants." — so it paints every plane and takes none. In a `DeviceN` the
//!   name is forbidden outright ("The special name All , used by Separation colour spaces, shall
//!   not be used."), and a file stating it there has named no colourant either.
//! - **`Cyan`, `Magenta`, `Yellow` and `Black`** are the process planes' own. §8.6.6.4 excepts
//!   them from the arbitrary names, "which are reserved to name the process colourants of a CMYK
//!   device", and §8.6.6.5 repeats it; the simulated device §10.8.3 describes is exactly such a
//!   device, so the four fold into the process planes rather than taking a plane each.
//! - **An `NChannel` space's process components** belong to Table 71's process colour space, and
//!   §8.6.6.5 draws the line by that dictionary: "Any component not specified in the process
//!   dictionary shall be considered to be a spot colourant." A component `/Components` names is
//!   a process one whatever it is called.
//!
//! # What is walked
//!
//! Every place a colour space reaches this page's marks from: the `/ColorSpace`, `/Pattern`,
//! `/Shading`, `/XObject` and `/Font` resources, a form's and a tiling pattern's and a Type 3
//! font's own `/Resources` beneath them, an `Indexed` space's base and a `Pattern` space's
//! underlying space, and each annotation's normal appearance, which §12.5.5 draws onto the same
//! page. **A soft mask's group is not walked**, and §11.7.3 is why: "In particular, spot colours
//! shall not be available in a transparency group XObject that is used to define a soft mask; the
//! alternate colour space shall always be substituted in that case." What a mask group names
//! therefore needs no plane, and `/ExtGState` is the only road to one.
//!
//! Over-reading costs a plane nothing draws on and under-reading costs a colourant its
//! separation, so the walk takes a resource the content stream may not use rather than parsing
//! the stream to find out: the answer is needed before the stream is run.
//!
//! # The bound
//!
//! Trap 38's question has the answer *the standard states none*: §8.6.6.4 allows colourant names
//! "subject to implementation limits" and §8.6.6.5 lets a `DeviceN` "contain an arbitrary number
//! of colour components". So the walk truncates nothing, and the bound is where planes are made:
//! [`MAX_SPOT_PLANES`], with a page past it named rather than cut. The walk is finite without a
//! budget —
//! every indirect object is entered at most once per role, `Seen` holding which, and it runs
//! from an explicit stack rather than recursion, so a chain of forms each naming the next costs
//! heap rather than the thread's stack.
#![expect(
    clippy::doc_markdown,
    reason = "the module and `nchannel_process` quote §8.6.6.5 and §11.7.3 verbatim, and a \
              quotation may not gain backticks"
)]

use std::collections::BTreeSet;

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::colour::{ColourSpace, DeviceSpots, PROCESS_COLOURANTS, Plane, Press, Reading};

/// How many spot planes one page is given: sixteen, forty-eight colourants.
///
/// **The standard states no bound**, so the number is this tree's and says so (trap 38): §8.6.6.4
/// allows colourant names "subject to implementation limits" and §8.6.6.5 lets a `DeviceN`
/// "contain an arbitrary number of colour components". What a plane costs is what bounds it —
/// every plane is one more interpretation of the page's content stream, and in the backends one
/// more raster the size of the page — so the bound is a budget, and it is set against what
/// documents do. `pdf-model --example spot_depth` over the crawl, the first ten pages of every
/// document: the distribution falls away geometrically from one colourant, the largest page of an
/// ordinary document names forty-nine, and the only pages past that are six files of one
/// interpreter's bug report naming 1090 apiece. Sixteen planes carry every page below that one
/// document, at eighteen interpretations with the process pair. ADR 1311 has the numbers.
///
/// **A page past it is not truncated silently.** Its first [`MAX_SPOT_COLOURANTS`] colourants
/// have planes; a mark in any other reverts to its alternate colour space as §11.7.3's second
/// bullet allows, and is named, per mark and by colourant, by
/// `pdf_colour::colour::DeviceSpots::without_a_plane`.
pub const MAX_SPOT_PLANES: usize = 16;

/// How many spot colourants [`MAX_SPOT_PLANES`] carry, [`Plane::COLOURANTS`] to a plane.
pub const MAX_SPOT_COLOURANTS: usize = MAX_SPOT_PLANES * Plane::COLOURANTS;

/// How many samples of step b)'s flat XYZ one spot colourant's curve holds: one per level of the
/// eight-bit tint a spot plane stores, so the curve is read at its own samples.
pub const FLAT_SAMPLES: usize = 256;

/// The spot colourants a page names, in the order its resources name them first.
///
/// What [`spot_colourants`] returns. Each name is held as the bytes the file wrote, because
/// §7.3.5 makes two names one name only on an exact binary match.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpotColourants {
    /// Every spot colourant, once, first-named first.
    names: Vec<Name>,
    /// Where each colourant's appearance is stated, index for index with `names`.
    sources: Vec<Source>,
}

/// The colour space a spot colourant's appearance is read from: ISO 32000-2 §10.8.3 step b)'s
/// separation of that colourant, converted to flat XYZ.
///
/// **The standard states a colourant's appearance only through the alternate of a space that
/// names it**, and a page can name one colourant in several spaces whose alternates disagree. So
/// which one speaks for it is this tree's choice, and it is made in the order the standard gives
/// the colourant most directly: a `Separation` space names the colourant alone (§8.6.6.4), as does
/// the `Separation` space an `NChannel` space's `/Colorants` entry states for it — Table 70:
/// "Each entry in this dictionary shall be an array defining a Separation colour space for that
/// colourant" — and between two of those the first the walk meets decides; failing either, a
/// `DeviceN` space's tint transform evaluated with that component alone, which is the colour the
/// space says the colourant paints on its own. ADR 1317.
#[derive(Debug, Clone, PartialEq)]
enum Source {
    /// A `Separation` space naming the colourant, resolved.
    Separation(Object),
    /// A `DeviceN` space naming the colourant as its component `index` of `count`.
    DeviceN {
        /// The space, resolved.
        space: Object,
        /// Which component is the colourant.
        index: usize,
        /// How many components the space has.
        count: usize,
    },
}

impl Source {
    /// Whether this source states the colourant alone, which ranks it above a `DeviceN`'s.
    fn is_separation(&self) -> bool {
        matches!(self, Self::Separation(_))
    }
}

impl SpotColourants {
    /// The colourants, in the order the walk met them.
    #[must_use]
    pub fn names(&self) -> &[Name] {
        &self.names
    }

    /// How many spot colourants the page names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the page names no spot colourant, which is every page drawn in process colours
    /// alone.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// How many planes these colourants take beside the process ones: `ceil(S / 3)`, one channel
    /// of a raster per colourant (§11.3.4's per-component compositing), and never more than
    /// [`MAX_SPOT_PLANES`].
    #[must_use]
    pub fn spot_planes(&self) -> usize {
        self.names
            .len()
            .min(MAX_SPOT_COLOURANTS)
            .div_ceil(Plane::COLOURANTS)
    }

    /// The simulated device these colourants describe, beside `press`'s process colourants: the
    /// first [`MAX_SPOT_COLOURANTS`] with a plane each and the rest named as having none.
    #[must_use]
    pub fn device(&self, press: &Press) -> DeviceSpots {
        let (within, beyond) = self
            .names
            .split_at(self.names.len().min(MAX_SPOT_COLOURANTS));
        DeviceSpots::new(within.to_vec(), beyond.to_vec(), press)
    }

    /// How many planes a page naming these colourants is separated into: the process planes and
    /// the spot ones.
    #[must_use]
    pub fn plane_count(&self) -> usize {
        Plane::PROCESS.len().saturating_add(self.spot_planes())
    }

    /// Adds one colourant name unless it is not a spot colourant of its own, and takes `source`
    /// as where its appearance is read unless a source [`Source`] ranks higher is already held.
    fn admit(&mut self, name: &Name, process: &[Name], source: Source) {
        let bytes = name.as_bytes();
        let special = bytes == b"None" || bytes == b"All";
        if special || PROCESS_COLOURANTS.contains(&bytes) || process.contains(name) {
            return;
        }
        if let Some(at) = self.names.iter().position(|held| held == name) {
            if let Some(held) = self.sources.get_mut(at)
                && !held.is_separation()
                && source.is_separation()
            {
                *held = source;
            }
        } else {
            self.names.push(name.clone());
            self.sources.push(source);
        }
    }

    /// ISO 32000-2 §10.8.3 step b) for each of the first `count` colourants: its separation's flat
    /// XYZ relative to the matte's white, sampled at [`FLAT_SAMPLES`] tints — or `None` where a
    /// colourant's space does not parse, whose separation this tree cannot state.
    ///
    /// Read from [`Source`]'s space under `reading` with its tint transform — the colourant's
    /// separation is what the producer says the colourant alone looks like, which is the question
    /// §8.6.6.4's alternate answers, and never the simulation's own answer to it — at the initial
    /// rendering intent (Table 51), because a plane is one raster whatever intents its marks
    /// stated. A device alternate reaches XYZ through this processor's CIE definition of it, which
    /// for `DeviceCMYK` is the page's output intent where it states one (§14.11.5) and the assumed
    /// inks where it does not: the press the process planes are drawn on.
    pub(crate) fn flat_curves(
        &self,
        document: &Document,
        resources: &Dictionary,
        reading: Reading<'_>,
        count: usize,
    ) -> Option<Vec<pdf_render::SpotColourant>> {
        let rendering = crate::icc::Rendering::compensating();
        self.names
            .iter()
            .zip(&self.sources)
            .take(count)
            .map(|(name, source)| {
                let (space, index, arity) = match source {
                    Source::Separation(space) => (space, 0, 1),
                    Source::DeviceN {
                        space,
                        index,
                        count,
                    } => (space, *index, *count),
                };
                let space = ColourSpace::parse_under(document, space, resources, reading)?;
                let mut values = vec![0.0_f32; arity];
                let flat: Vec<[f32; 3]> = (0..FLAT_SAMPLES)
                    .map(|sample| {
                        #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
                        let tint = sample as f32 / (FLAT_SAMPLES - 1) as f32;
                        if let Some(value) = values.get_mut(index) {
                            *value = tint;
                        }
                        space.flat_ratio(&values, rendering)
                    })
                    .collect();
                pdf_render::SpotColourant::new(Arc::from(name.as_bytes()), Arc::from(flat))
            })
            .collect()
    }
}

/// ISO 32000-2 §10.8.3 step a)'s separations of one page: the page processed "as if separations
/// were to be created for a simulated device that supports subtractive process colourants and
/// possibly spot colours".
///
/// One display list per plane of the simulated device, each the whole page interpreted once with
/// every colour resolved to that plane's colourants (`pdf_colour::colour::DeviceSpots`): the two
/// process planes, in which a spot colourant with a plane paints §11.7.3's "additive value of
/// 1.0", and [`SpotColourants::spot_planes`] spot planes, three colourants to a plane.
///
/// **The planes are the page's display list**: the interpretation's list is the chromatic process
/// plane, carrying the black one and the spot planes (`pdf_render::DisplayList::set_separated`),
/// and a backend draws all of them and puts them together by steps b) to d)
/// (`pdf_render::separation`). This is what the model says *about* them — which colourants have a
/// plane and which a mark painted without one — and [`plane`] reads a plane off the list. It is
/// here only where a reader asked for the simulation and the page names a spot colourant, so no
/// other page carries or pays for it. ADRs 1311, 1317.
#[derive(Debug, Clone, PartialEq)]
pub struct Separation {
    /// The colourants with a spot plane, in plane order.
    colourants: Vec<Name>,
    /// How many planes the page was separated into, the process pair included.
    plane_count: usize,
    /// The colourants a mark painted past [`MAX_SPOT_COLOURANTS`], each once, by name.
    without_a_plane: Vec<Name>,
}

impl Separation {
    /// A page's separation: its colourants, how many planes it was made in, and what had none.
    pub(crate) fn new(
        colourants: Vec<Name>,
        plane_count: usize,
        without_a_plane: Vec<Name>,
    ) -> Self {
        Self {
            colourants,
            plane_count,
            without_a_plane,
        }
    }

    /// The spot colourants that have a plane: colourant `3 × n + c` is channel `c` of
    /// `Plane::Spot(n)`.
    #[must_use]
    pub fn colourants(&self) -> &[Name] {
        &self.colourants
    }

    /// How many planes the page was separated into: the process pair and the spot planes.
    #[must_use]
    pub fn plane_count(&self) -> usize {
        self.plane_count
    }

    /// The spot colourants a mark on this page painted that have no plane — the page named more
    /// than [`MAX_SPOT_COLOURANTS`] — which reverted to their alternate colour space, by name.
    #[must_use]
    pub fn without_a_plane(&self) -> &[Name] {
        &self.without_a_plane
    }
}

/// One plane of a separated page's display list, or `None` where `list` is not separated or the
/// simulated device has no such plane.
///
/// The chromatic plane is `list` itself — its own commands are that plane's — the black plane is
/// its companion, and spot plane `n` is the `n`th of its [`pdf_render::SpotSeparation::planes`].
#[must_use]
pub fn plane(list: &pdf_render::DisplayList, plane: Plane) -> Option<&pdf_render::DisplayList> {
    let separation = list.separation()?;
    match plane {
        Plane::Chromatic => Some(list),
        Plane::Black => list.black(),
        Plane::Spot(index) => separation.planes().get(index),
    }
}

/// What an object is being read *as*, which decides what is looked for inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    /// A resource dictionary (§7.8.3, Table 34).
    Resources,
    /// A colour space, by name or by array (§8.6).
    ColourSpace,
    /// A pattern resource: a tiling pattern's stream or a shading pattern's dictionary (§8.7).
    Pattern,
    /// A shading dictionary or stream (§8.7.4.3).
    Shading,
    /// An external object: an image's colour space or a form's resources (§8.8, §8.10).
    XObject,
    /// A font, of which only Type 3's own resources can name a colour space (§9.6.4).
    Font,
    /// An annotation's `/AP` normal appearance, a stream or a dictionary of them (§12.5.5).
    Appearance,
}

/// Which indirect objects have been entered, and in which role.
///
/// Per role rather than per object because one object can honestly be reached as two things —
/// a stream that is both a pattern's and a form's is two readings — and what the set is for is
/// that neither reading is made twice, which is what bounds the walk against a cycle.
type Seen = BTreeSet<(ObjectId, Role)>;

/// The spot colourants `page` names, read off its resources and its annotations' appearances.
///
/// See the module documentation for which names count and which objects are walked. The result
/// is ordered by first appearance in a walk whose order is the resource dictionaries' own key
/// order, so the same page always gives the same planes in the same order.
#[must_use]
pub fn spot_colourants(document: &Document, page: &crate::Page) -> SpotColourants {
    let mut found = SpotColourants::default();
    let mut seen = Seen::new();
    let mut roots: Vec<(Object, Role)> =
        vec![(Object::Dictionary(page.resources.clone()), Role::Resources)];
    if let Some(annotations) = document.get_key(&page.dict, "Annots").as_array() {
        for annotation in annotations {
            let annotation = document.resolve(annotation);
            if let Some(annotation) = annotation.as_dict()
                && let Some(normal) = document.get_key(annotation, "AP").as_dict()
                && let Some(entry) = normal.get("N")
            {
                roots.push((entry.clone(), Role::Appearance));
            }
        }
    }
    // A stack pops the last thing pushed, so each object's children go on in reverse and come
    // off in the order the object states them: the walk is depth-first in document order, which
    // is what makes "first named" mean the same thing on every run.
    let mut stack: Vec<(Object, Role)> = roots.into_iter().rev().collect();
    let mut children = Vec::new();
    while let Some((object, role)) = stack.pop() {
        if let Some(id) = object.as_reference()
            && !seen.insert((id, role))
        {
            continue;
        }
        let object = document.resolve(&object);
        visit(document, &object, role, &mut children, &mut found);
        stack.extend(children.drain(..).rev());
    }
    found
}

/// Reads one resolved object in its role, admitting the colourants it names and stacking what
/// it reaches onto `stack` in the order it states them.
fn visit(
    document: &Document,
    object: &Object,
    role: Role,
    stack: &mut Vec<(Object, Role)>,
    found: &mut SpotColourants,
) {
    match role {
        Role::Resources => {
            let Some(resources) = object.as_dict() else {
                return;
            };
            for (category, role) in [
                ("ColorSpace", Role::ColourSpace),
                ("Pattern", Role::Pattern),
                ("Shading", Role::Shading),
                ("XObject", Role::XObject),
                ("Font", Role::Font),
            ] {
                if let Some(entries) = document.get_key(resources, category).as_dict() {
                    stack.extend(entries.iter().map(|(_, value)| (value.clone(), role)));
                }
            }
        }
        Role::ColourSpace => colour_space(document, object, stack, found),
        Role::Pattern => {
            let Some(pattern) = object.as_dict() else {
                return;
            };
            // §8.7.3's tiling pattern is a content stream with resources of its own; §8.7.4's
            // shading pattern names a shading. A dictionary states which by `/PatternType`,
            // and reading both keys where either is present costs nothing a wrong guess would
            // not.
            push_entry(pattern, "Resources", Role::Resources, stack);
            push_entry(pattern, "Shading", Role::Shading, stack);
        }
        Role::Shading => {
            if let Some(shading) = object.as_dict() {
                push_entry(shading, "ColorSpace", Role::ColourSpace, stack);
            }
        }
        Role::XObject => {
            let Some(xobject) = object.as_dict() else {
                return;
            };
            match document.get_key(xobject, "Subtype").as_name() {
                // §8.9.5 Table 87's `/ColorSpace`. An image's own `/SMask` is a mask and is
                // not walked, for §11.7.3's reason in the module documentation.
                Some(subtype) if subtype.as_bytes() == b"Image" => {
                    push_entry(xobject, "ColorSpace", Role::ColourSpace, stack);
                }
                Some(subtype) if subtype.as_bytes() == b"Form" => {
                    push_entry(xobject, "Resources", Role::Resources, stack);
                }
                _ => {}
            }
        }
        Role::Font => {
            if let Some(font) = object.as_dict()
                && document
                    .get_key(font, "Subtype")
                    .as_name()
                    .is_some_and(|subtype| subtype.as_bytes() == b"Type3")
            {
                push_entry(font, "Resources", Role::Resources, stack);
            }
        }
        Role::Appearance => match object {
            // §12.5.5: a normal appearance is a form, or a dictionary of forms keyed by
            // appearance state.
            Object::Stream(stream) => push_entry(&stream.dict, "Resources", Role::Resources, stack),
            Object::Dictionary(states) => {
                stack.extend(
                    states
                        .iter()
                        .map(|(_, value)| (value.clone(), Role::Appearance)),
                );
            }
            _ => {}
        },
    }
}

/// Stacks `dict`'s `key` to be read in `role`, unresolved, so that [`Seen`] sees its reference.
fn push_entry(dict: &Dictionary, key: &str, role: Role, stack: &mut Vec<(Object, Role)>) {
    if let Some(value) = dict.get(key) {
        stack.push((value.clone(), role));
    }
}

/// Reads one colour space: §8.6.6.4's `Separation`, §8.6.6.5's `DeviceN`, and the two families
/// that carry another space inside them.
fn colour_space(
    document: &Document,
    object: &Object,
    stack: &mut Vec<(Object, Role)>,
    found: &mut SpotColourants,
) {
    let Some(items) = object.as_array() else {
        // A name is a device or pattern family, or `/DeviceN`'s shorthand for nothing: no
        // colourant of its own.
        return;
    };
    let family = items.first().map(|family| document.resolve(family));
    let Some(family) = family.as_ref().and_then(Object::as_name) else {
        return;
    };
    match family.as_bytes() {
        b"Separation" => {
            if let Some(name) = items.get(1).map(|name| document.resolve(name))
                && let Some(name) = name.as_name()
            {
                found.admit(name, &[], Source::Separation(object.clone()));
            }
        }
        b"DeviceN" => {
            let attributes = items.get(4).map(|attributes| document.resolve(attributes));
            let process = attributes
                .as_ref()
                .map(|attributes| nchannel_process(document, attributes))
                .unwrap_or_default();
            let colorants = attributes
                .as_ref()
                .and_then(Object::as_dict)
                .map(|attributes| document.get_key(attributes, "Colorants"));
            let names = items.get(1).map(|names| document.resolve(names));
            let names = names
                .as_ref()
                .and_then(Object::as_array)
                .unwrap_or_default();
            for (index, name) in names.iter().enumerate() {
                let Some(name) = document.resolve(name).as_name().cloned() else {
                    continue;
                };
                // Table 70's `/Colorants` entry is the colourant's own `Separation` space, which
                // [`Source`] ranks first.
                let own = colorants
                    .as_ref()
                    .and_then(Object::as_dict)
                    .and_then(|colorants| colorants.get_by_name(&name))
                    .map(|entry| document.resolve(entry))
                    .filter(|entry| {
                        entry
                            .as_array()
                            .and_then(|entry| entry.first())
                            .map(|family| document.resolve(family))
                            .is_some_and(|family| {
                                family
                                    .as_name()
                                    .is_some_and(|family| family.as_bytes() == b"Separation")
                            })
                    });
                let source = own.map_or_else(
                    || Source::DeviceN {
                        space: object.clone(),
                        index,
                        count: names.len(),
                    },
                    Source::Separation,
                );
                found.admit(&name, &process, source);
            }
        }
        // §8.6.6.3's base space and §8.6.6.2's underlying space are each a colour space in
        // their own right, and either may be a `Separation` or a `DeviceN`.
        b"Indexed" | b"Pattern" => {
            if let Some(inner) = items.get(1) {
                stack.push((inner.clone(), Role::ColourSpace));
            }
        }
        _ => {}
    }
}

/// Table 71's process `/Components` of an `NChannel` space, or nothing for any other `DeviceN`.
///
/// §8.6.6.5 gives the process dictionary its meaning under the `NChannel` subtype alone — "A
/// value of DeviceN for the Subtype entry, or no value, shall mean that only the previous
/// features shall be supported" — so a plain `DeviceN`'s dictionary names no process component
/// and only the four reserved names fold.
fn nchannel_process(document: &Document, attributes: &Object) -> Vec<Name> {
    let Some(attributes) = attributes.as_dict() else {
        return Vec::new();
    };
    if document
        .get_key(attributes, "Subtype")
        .as_name()
        .is_none_or(|subtype| subtype.as_bytes() != b"NChannel")
    {
        return Vec::new();
    }
    let process = document.get_key(attributes, "Process");
    let Some(process) = process.as_dict() else {
        return Vec::new();
    };
    document
        .get_key(process, "Components")
        .as_array()
        .unwrap_or_default()
        .iter()
        .filter_map(|component| document.resolve(component).as_name().cloned())
        .collect()
}
