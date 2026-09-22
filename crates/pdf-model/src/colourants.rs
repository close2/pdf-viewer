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
//! of colour components". So nothing here truncates. The walk is finite without a budget —
//! every indirect object is entered at most once per role, `Seen` holding which, and it runs
//! from an explicit stack rather than recursion, so a chain of forms each naming the next costs
//! heap rather than the thread's stack.
#![expect(
    clippy::doc_markdown,
    reason = "the module and `nchannel_process` quote §8.6.6.5 and §11.7.3 verbatim, and a \
              quotation may not gain backticks"
)]

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::colour::Plane;

/// The four names §8.6.6.4 and §8.6.6.5 reserve to the process colourants of a CMYK device.
const PROCESS_NAMES: [&[u8]; 4] = [b"Cyan", b"Magenta", b"Yellow", b"Black"];

/// The spot colourants a page names, in the order its resources name them first.
///
/// What [`spot_colourants`] returns. Each name is held as the bytes the file wrote, because
/// §7.3.5 makes two names one name only on an exact binary match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpotColourants {
    /// Every spot colourant, once, first-named first.
    names: Vec<Name>,
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
    /// of a raster per colourant (§11.3.4's per-component compositing).
    #[must_use]
    pub fn spot_planes(&self) -> usize {
        self.names.len().div_ceil(Plane::COLOURANTS)
    }

    /// How many planes a page naming these colourants is separated into: the process planes and
    /// the spot ones.
    #[must_use]
    pub fn plane_count(&self) -> usize {
        Plane::PROCESS.len().saturating_add(self.spot_planes())
    }

    /// Adds one colourant name unless it is not a spot colourant of its own, or is already here.
    fn admit(&mut self, name: &Name, process: &[Name]) {
        let bytes = name.as_bytes();
        let special = bytes == b"None" || bytes == b"All";
        if special
            || PROCESS_NAMES.contains(&bytes)
            || process.contains(name)
            || self.names.contains(name)
        {
            return;
        }
        self.names.push(name.clone());
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
                found.admit(name, &[]);
            }
        }
        b"DeviceN" => {
            let process = items
                .get(4)
                .map(|attributes| document.resolve(attributes))
                .map(|attributes| nchannel_process(document, &attributes))
                .unwrap_or_default();
            let names = items.get(1).map(|names| document.resolve(names));
            for name in names
                .as_ref()
                .and_then(Object::as_array)
                .unwrap_or_default()
            {
                if let Some(name) = document.resolve(name).as_name() {
                    found.admit(name, &process);
                }
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
