//! Building a display-list shading from a PDF shading dictionary.
//!
//! The specification's seven types collapse to four here, because several of them describe
//! the same thing in different notations. See [`pdf_render::shading`] for why that grouping
//! loses nothing a backend needs.
//!
//! # Colours are resolved now, not later
//!
//! A shading states its colours as a function of one or two parameters, in some colour
//! space. Both are resolved here: the function is evaluated and the colour space applied,
//! so the display list carries plain RGB. That keeps colour management in one place, and
//! it is also what lets the result be shared and compared — a display list holding a
//! closure could be neither.

use std::collections::BTreeMap;
use std::sync::Arc;

use pdf_render::{Color, ColourGrid, Point, Ramp, Shading, ShadingKind, Transform};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};
use rayon::iter::{IndexedParallelIterator as _, ParallelIterator as _};
use rayon::slice::ParallelSliceMut as _;

use crate::colour::{ColourSpace, Compositing};
use crate::function::{Function, Value};

/// The most cells a function-based shading's grid will carry, whatever the device asks for.
///
/// Type 1 shadings are an arbitrary function of two variables, so unlike the other types
/// they cannot be reduced to a ramp: the display list carries the function's ingredients
/// ([`FunctionColours`]) and a backend asks for the grid its device wants. That request is a
/// number a *magnification* controls — a full page at 16× is hundreds of millions of cells —
/// and each cell is a function evaluation plus a colour conversion, so this bounds the work
/// and the memory the way `image::MAX_MASK_GRID` bounds a mask's raster. ISO 32000-2 §10.7.3
/// says a bound of this kind is the device's to set: "each output device may have internal
/// limits". 2^22 cells is 2048×2048 — sixteen times the old fixed grid per axis — and 64 MB
/// of transient `Color` at the limit.
const MAX_FUNCTION_CELLS: u64 = 1 << 22;

/// Why a shading could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ShadingError {
    /// The `/ShadingType` is not one this crate implements.
    #[error("shading type {kind} is not implemented")]
    UnsupportedType {
        /// The type that was found.
        kind: i64,
    },
    /// The shading is structurally invalid.
    #[error("malformed shading: {detail}")]
    Malformed {
        /// What was wrong.
        detail: String,
    },
}

/// Shadings already built, keyed by the object that states them.
///
/// # Why this exists, with the measurement that asked for it
///
/// A shading's colours cost the whole of its construction: every `/Function` is parsed —
/// which for a type 0 function means inflating a stream and decoding its samples — and a
/// [`Ramp`] is then 256 evaluations of it. None of that depends on *where* the shading is
/// painted, and the same object is commonly painted many times: `bug1721218_reduced.pdf`
/// runs `sh` 3576 times over three function objects, and rebuilding for each one was
/// `Function::parse` 6.7%, `Function::eval` 4.1% and `shading::ramp` 3.2% of a 54 G
/// instruction page. Caching the kind removed all of it (ADR 0069).
///
/// # What is not cached, and why the key is not just an identity
///
/// A shading's `/ColorSpace` may be a *name*, which §8.6.5.1 resolves through the resource
/// dictionary in force — so one object can mean two things under two resource dictionaries.
/// Those are not cached at all, which is exact rather than approximately right: the six
/// named spaces are not cached at all, which is exact rather than approximately right. An
/// array or a stream states the space in the object itself and is cached.
#[derive(Debug, Default)]
pub struct Cache {
    /// The kind and the shading's own matrix, which is `/Matrix` for a type 1 and the
    /// identity for every other type.
    built: BTreeMap<(ObjectId, usize, Compositing), (Arc<ShadingKind>, Transform)>,
    /// A `/ColorSpace` stated as an **indirect object**, parsed once.
    ///
    /// [`Self::built`] cannot help a page of *distinct* shadings, and one exists:
    /// `3129278.pdf` from the `SafeDocs` corpus states **1053 axial shadings**, each its own
    /// object and each naming the same `ICCBased` space, so nothing above was ever hit and
    /// `ColourSpace::parse` inflated and parsed that profile 1053 times — **60% of the page's
    /// 380 G interpretation instructions in `zlib`, and 17% more in `icc::Profile::parse`**.
    ///
    /// Keyed by [`ObjectId`] for [`Self::built`]'s reason turned round: a reference is what
    /// says two shadings mean *one* space, and a space stated inline is the object's own and
    /// costs nothing to parse twice. §8.6.5.1's resolution through the resource dictionary
    /// does not reach here at all — that applies to a space stated as a **name**, which has no
    /// object identity and is not put in this table.
    spaces: BTreeMap<ObjectId, ColourSpace>,
}

impl Cache {
    /// Builds a shading, reusing an earlier build of the same object where it is sound to.
    ///
    /// `transform` maps the shading's own coordinates into the space the caller will draw
    /// in, and is the one part of the answer that is not cached — it is why the same
    /// shading painted twice is two commands.
    ///
    /// # Errors
    ///
    /// See [`ShadingError`]. A failed build is not remembered: it is rare, it is reported
    /// once by the caller's own deduplication, and remembering an error would mean deciding
    /// whether the error is a property of the object or of the moment.
    pub fn build(
        &mut self,
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        transform: Transform,
        smoothness: Option<f32>,
        into: &Compositing,
    ) -> Result<Shading, ShadingError> {
        // §10.7.3's tolerance is part of the key rather than of the object: the same shading
        // painted under two `/SM` values is two sets of colours, and a page that changes it
        // between paintings has said so.
        let resolution = Ramp::resolution_for(smoothness);
        let key = object.as_reference().filter(|_| {
            // A `/ColorSpace` stated as a *name* is the one thing about a shading that is
            // not a property of the object alone: §8.6.5.1 resolves it through the resource
            // dictionary in force, and even the device names go through §8.6.5.6's
            // `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` there. So a named space is
            // not cached at all, which is exact; an array or a stream is the object's own.
            let space =
                dictionary_of(document, object).map(|dict| document.get_key(&dict, "ColorSpace"));
            !matches!(space, Some(Object::Name(_)))
        });
        if let Some(id) = key
            && let Some((kind, own)) = self.built.get(&(id, resolution, into.clone()))
        {
            return Ok(Shading {
                kind: Arc::clone(kind),
                transform: own.then(transform),
            });
        }
        let space = self.space_of(document, object, resources);
        let (kind, own) = kind_of(document, object, resources, resolution, into, space)?;
        let kind = Arc::new(kind);
        if let Some(id) = key {
            self.built
                .insert((id, resolution, into.clone()), (Arc::clone(&kind), own));
        }
        Ok(Shading {
            kind,
            transform: own.then(transform),
        })
    }

    /// This shading's colour space, parsed once where the shading states it by reference.
    ///
    /// `None` where there is nothing to remember — an inline space, or a name — and
    /// [`kind_of`] then parses it itself, which is what it did for every shading before this
    /// table existed.
    fn space_of(
        &mut self,
        document: &Document,
        object: &Object,
        resources: &Dictionary,
    ) -> Option<ColourSpace> {
        let dict = dictionary_of(document, object)?;
        let id = dict.get("ColorSpace")?.as_reference()?;
        if let Some(space) = self.spaces.get(&id) {
            return Some(space.clone());
        }
        let space = ColourSpace::parse(document, &Object::Reference(id), resources)?;
        self.spaces.insert(id, space.clone());
        Some(space)
    }
}

/// The shading dictionary of an object, whether it is a dictionary or a stream.
pub(crate) fn dictionary_of(document: &Document, object: &Object) -> Option<Dictionary> {
    match document.resolve(object) {
        Object::Dictionary(dict) => Some(dict),
        Object::Stream(stream) => Some(stream.dict.clone()),
        _ => None,
    }
}

/// Builds a shading from its dictionary.
///
/// `transform` maps the shading's own coordinates into the space the caller will draw in.
///
/// Callers that paint many shadings should hold a [`Cache`] and use [`Cache::build`]; this
/// is the uncached spelling, kept for callers with one shading to build.
///
/// # Errors
///
/// See [`ShadingError`].
pub fn build(
    document: &Document,
    object: &Object,
    resources: &Dictionary,
    transform: Transform,
) -> Result<Shading, ShadingError> {
    let (kind, own) = kind_of(
        document,
        object,
        resources,
        Ramp::RESOLUTION,
        &Compositing::Device,
        None,
    )?;
    Ok(Shading {
        kind: Arc::new(kind),
        transform: own.then(transform),
    })
}

/// The half of a shading that depends on the object alone: its colours and its own matrix.
///
/// # Errors
///
/// See [`ShadingError`].
fn kind_of(
    document: &Document,
    object: &Object,
    resources: &Dictionary,
    resolution: usize,
    into: &Compositing,
    space: Option<ColourSpace>,
) -> Result<(ShadingKind, Transform), ShadingError> {
    let resolved = document.resolve(object);
    let dict = match &resolved {
        Object::Dictionary(dict) => dict.clone(),
        Object::Stream(stream) => stream.dict.clone(),
        _ => {
            return Err(ShadingError::Malformed {
                detail: "not a dictionary or stream".to_owned(),
            });
        }
    };

    let kind = document
        .get_key(&dict, "ShadingType")
        .as_integer()
        .ok_or_else(|| ShadingError::Malformed {
            detail: "no /ShadingType".to_owned(),
        })?;

    let space = match space {
        Some(space) => space,
        None => ColourSpace::parse(document, &document.get_key(&dict, "ColorSpace"), resources)
            .ok_or_else(|| ShadingError::Malformed {
                detail: "unsupported /ColorSpace".to_owned(),
            })?,
    };

    let (kind, own) = match kind {
        // Only a type 1 shading has a `/Matrix`, which places its domain rectangle within
        // the shading's own space. It composes ahead of the caller's transform rather than
        // being carried separately, so the display list needs only one transform per
        // shading.
        1 => (
            function_based(document, &dict, &space, into)?,
            matrix_of(document, &dict, "Matrix"),
        ),
        2 => (
            axial(document, &dict, &space, resolution, into)?,
            Transform::IDENTITY,
        ),
        3 => (
            radial(document, &dict, &space, resolution, into)?,
            Transform::IDENTITY,
        ),
        4..=7 => (
            mesh(document, &resolved, &dict, &space, kind, resolution, into)?,
            Transform::IDENTITY,
        ),
        other => return Err(ShadingError::UnsupportedType { kind: other }),
    };

    Ok((kind, own))
}

/// ISO 32000-2 §8.7.4.3 Table 77's `/BBox`, if the shading dictionary states one.
///
/// > An array of four numbers giving the left, bottom, right, and top coordinates,
/// > respectively, of the shading's bou nding box. The coordinates shall be interpreted in
/// > the shading's target coordinate space. If present, this bounding box shall be applied
/// > as a temporary clipping boundary when the shading is painted, in addition to the
/// > current clipping path and any other clipping boundaries in effect at that time.
///
/// Returned as four numbers rather than applied here, for two reasons the clause states. It
/// is in the shading's *target* space — the space the caller paints into — where everything
/// else in this module is in the shading's own; and it is a **clip**, which is the
/// interpreter's to compose, not a property of the gradient. NOTE 2 is a reminder that it
/// is a clip like any other: a `BBox` of zero height or width "will still paint one pixel".
#[must_use]
pub fn bbox_of(document: &Document, object: &Object) -> Option<[f32; 4]> {
    let resolved = document.resolve(object);
    let dict = match &resolved {
        Object::Dictionary(dict) => dict.clone(),
        Object::Stream(stream) => stream.dict.clone(),
        _ => return None,
    };
    let array = document.get_key(&dict, "BBox");
    let values: Vec<f32> = array
        .as_array()?
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .collect();
    <[f32; 4]>::try_from(values.as_slice()).ok()
}

/// Reads `/Coords` as a fixed number of values.
fn coords(document: &Document, dict: &Dictionary, expected: usize) -> Option<Vec<f32>> {
    let array = document.get_key(dict, "Coords");
    let items = array.as_array()?;
    if items.len() < expected {
        return None;
    }
    Some(
        items
            .iter()
            .take(expected)
            .filter_map(|item| document.resolve(item).as_number().map(narrow))
            .collect(),
    )
}

/// Reads `/Extend`, which defaults to no extension at either end.
fn extend(document: &Document, dict: &Dictionary) -> (bool, bool) {
    let array = document.get_key(dict, "Extend");
    let Some(items) = array.as_array() else {
        return (false, false);
    };
    let at = |index: usize| {
        items
            .get(index)
            .map(|item| document.resolve(item))
            .is_some_and(|item| matches!(item, Object::Boolean(true)))
    };
    (at(0), at(1))
}

/// Reads `/Domain`, which defaults to the unit interval.
fn domain(document: &Document, dict: &Dictionary) -> (f32, f32) {
    let array = document.get_key(dict, "Domain");
    let Some(items) = array.as_array() else {
        return (0.0, 1.0);
    };
    let at = |index: usize, fallback: f32| {
        items
            .get(index)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_number())
            .map_or(fallback, narrow)
    };
    (at(0, 0.0), at(1, 1.0))
}

/// Samples a shading's colour function across its domain into a ramp.
fn ramp(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    resolution: usize,
    into: &Compositing,
) -> Result<Ramp, ShadingError> {
    let functions =
        Function::parse_group(document, &document.get_key(dict, "Function")).map_err(|e| {
            ShadingError::Malformed {
                detail: e.to_string(),
            }
        })?;
    if functions.is_empty() {
        return Err(ShadingError::Malformed {
            detail: "no /Function".to_owned(),
        });
    }
    let (low, high) = domain(document, dict);
    let breaks = breakpoints_over(&functions, low, high);

    Ok(Ramp::sample_across_at(resolution, &breaks, |t| {
        let parameter = low + t * (high - low);
        colour_from(&functions, &[parameter], space, into)
    }))
}

/// Where a shading's functions jump, as fractions of the interval `low` to `high`.
///
/// §8.7.4.5.3 makes the colour at a point whatever the function says it is, and a type 3
/// function with two equal `/Bounds` says one colour up to a point and another after it — a
/// step, which a table of evenly spaced samples cannot hold. [`Ramp::sample_across_at`] puts a
/// pair of stops at each of these.
///
/// The interval is the shading's own parameter: `/Domain` for an axial or radial shading, and
/// for a mesh the `/Decode` pair Table 81 gives the parametric value. A zero-width interval
/// states one colour and has nowhere for a break to sit.
pub(crate) fn breakpoints_over(functions: &[Function], low: f32, high: f32) -> Vec<f32> {
    let span = high - low;
    if span.abs() <= f32::EPSILON {
        return Vec::new();
    }
    functions
        .iter()
        .flat_map(Function::breakpoints)
        .map(|at| (at - low) / span)
        .collect()
}

/// Evaluates a shading's functions at a point and converts the result to RGB.
///
/// A shading gives either one function producing every component or one function per
/// component; both are handled by concatenating the outputs.
///
/// This is the once-only form, for the handful of points a ramp or an opacity question asks
/// about. A grid asks per cell and uses [`colour_into`] with a buffer of its own.
fn colour_from(
    functions: &[Function],
    inputs: &[f32],
    space: &ColourSpace,
    into: &Compositing,
) -> Color {
    colour_into(functions, inputs, space, into, &mut Components::default())
}

/// The buffers one thread reuses across a grid of [`colour_into`] calls.
///
/// Two of them for the outputs because a shading's `/Function` may be a *group*: the group's
/// outputs are concatenated in `all`, and `one` is where a member writes before being appended.
/// A group of one — which is what almost every shading has — writes straight into `all` and
/// never touches `one`.
#[derive(Default)]
struct Components {
    /// Every component of one cell's colour, in the order the group produces them.
    all: Vec<f32>,
    /// One member function's outputs, when the group has more than one member.
    one: Vec<f32>,
    /// A §7.10.5 program's operand stack, which holds typed values rather than components and
    /// so cannot be either of the buffers above (ADR 0371). Untouched by every other function
    /// type.
    stack: Vec<Value>,
}

/// [`colour_from`] reusing the caller's buffers, which is what a device-resolution grid needs.
///
/// Same colour, same bits; the difference is that a million cells no longer mean a million
/// allocations. ADR 0364.
fn colour_into(
    functions: &[Function],
    inputs: &[f32],
    space: &ColourSpace,
    into: &Compositing,
    scratch: &mut Components,
) -> Color {
    if let [only] = functions {
        only.eval_into(inputs, &mut scratch.all, &mut scratch.stack);
    } else {
        scratch.all.clear();
        for function in functions {
            function.eval_into(inputs, &mut scratch.one, &mut scratch.stack);
            scratch.all.extend_from_slice(&scratch.one);
        }
    }
    into.paint(space, &scratch.all, true)
}

fn axial(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    resolution: usize,
    into: &Compositing,
) -> Result<ShadingKind, ShadingError> {
    let coords = coords(document, dict, 4).ok_or_else(|| ShadingError::Malformed {
        detail: "an axial shading needs four /Coords".to_owned(),
    })?;
    Ok(ShadingKind::Axial {
        start: Point::new(coords[0], coords[1]),
        end: Point::new(coords[2], coords[3]),
        ramp: ramp(document, dict, space, resolution, into)?,
        extend: extend(document, dict),
    })
}

fn radial(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    resolution: usize,
    into: &Compositing,
) -> Result<ShadingKind, ShadingError> {
    let coords = coords(document, dict, 6).ok_or_else(|| ShadingError::Malformed {
        detail: "a radial shading needs six /Coords".to_owned(),
    })?;
    // A negative radius is not a circle. Refusing beats drawing a mirrored gradient.
    if coords[2] < 0.0 || coords[5] < 0.0 {
        return Err(ShadingError::Malformed {
            detail: "a radial shading has a negative radius".to_owned(),
        });
    }
    Ok(ShadingKind::Radial {
        start: Point::new(coords[0], coords[1]),
        start_radius: coords[2],
        end: Point::new(coords[3], coords[4]),
        end_radius: coords[5],
        ramp: ramp(document, dict, space, resolution, into)?,
        extend: extend(document, dict),
    })
}

/// Reads one of the four mesh types into triangles.
fn mesh(
    document: &Document,
    object: &Object,
    dict: &Dictionary,
    space: &ColourSpace,
    kind: i64,
    resolution: usize,
    into: &Compositing,
) -> Result<ShadingKind, ShadingError> {
    let stream = object.as_stream().ok_or_else(|| ShadingError::Malformed {
        detail: "a mesh shading must be a stream".to_owned(),
    })?;

    // A mesh may state colours directly or as a single parameter through a function; the
    // reader needs to know which, so `/Function` is optional here rather than required.
    let functions = match document.get_key(dict, "Function") {
        Object::Null => Vec::new(),
        object => {
            Function::parse_group(document, &object).map_err(|e| ShadingError::Malformed {
                detail: e.to_string(),
            })?
        }
    };

    let (triangles, ramp) = crate::mesh::read(
        document, stream, kind, space, &functions, resolution, into,
    )
    .ok_or_else(|| ShadingError::Malformed {
        detail: format!("the type {kind} mesh stream could not be read"),
    })?;

    Ok(ShadingKind::Mesh {
        triangles: triangles.into(),
        ramp,
    })
}

fn function_based(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    into: &Compositing,
) -> Result<ShadingKind, ShadingError> {
    let functions =
        Function::parse_group(document, &document.get_key(dict, "Function")).map_err(|e| {
            ShadingError::Malformed {
                detail: e.to_string(),
            }
        })?;
    if functions.is_empty() {
        return Err(ShadingError::Malformed {
            detail: "no /Function".to_owned(),
        });
    }

    // A type 1 shading's `/Domain` is a rectangle rather than an interval.
    let array = document.get_key(dict, "Domain");
    let values: Vec<f32> = array
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| document.resolve(item).as_number().map(narrow))
                .collect()
        })
        .unwrap_or_default();
    let rectangle = <[f32; 4]>::try_from(values.as_slice()).unwrap_or([0.0, 1.0, 0.0, 1.0]);

    // A conversion's alpha is the space's rather than the value's — the one colour space
    // whose colours are not opaque, §8.6.6.4's `/None` colourant, discards its output for
    // *every* tint — so one evaluation at the domain's corner answers §11.4.6's opacity
    // question for the whole domain, without the function being evaluated anywhere else
    // before a device asks for its grid.
    let [x0, _, y0, _] = rectangle;
    let opaque = colour_from(&functions, &[x0, y0], space, into).a >= 1.0;

    let program = device_program(&functions, space, into, rectangle);

    Ok(ShadingKind::Sampled {
        domain: rectangle,
        source: pdf_render::DeferredColours::new(Arc::new(FunctionColours {
            functions,
            space: space.clone(),
            into: into.clone(),
            domain: rectangle,
            opaque,
        })),
        program,
    })
}

/// The same colours as a program a device can evaluate, or `None` — and every `None` here is a
/// page that draws exactly as it did before ADR 0376, from the grid above.
///
/// **This is where the two paths are made to answer the same question**, and each condition is
/// a place they would otherwise part. The device is handed a list, a domain rectangle and a
/// `Range`; the producer above evaluates the same function and then converts its outputs to a
/// device colour. So the device path is available exactly where that conversion is nothing:
///
/// - **One function, not a group.** §7.10.5.3's `/Function` may be `n` functions of one output
///   each instead of one of `n`. Both are drawn by the producer; only the second is a single
///   program, and stitching `n` programs into one is arithmetic this tree would be inventing.
/// - **§11.4.7's ordinary compositing target** ([`Compositing::Device`]). A `/Luminosity` mask
///   group weighs the colour into ink and a four-component page paints half of a `DeviceCMYK`
///   blend; neither is a colour the program's own outputs are.
/// - **`DeviceGray` or `DeviceRGB`.** These are the two spaces where a component *is* a device
///   component: §8.6.4.2 and §8.6.4.3 name no transformation, and [`ColourSpace::to_rgb`]'s
///   arms for them are the identity plus the unit-interval clamp below. Every other space —
///   `DeviceCMYK` included, which ADR 0263 converts through §10.4.2.5 — is a conversion, and a
///   conversion is not something to restate in a shader.
/// - **A `/Range` of the space's own width.** §7.10.5.3 makes `/Range` required for a type 4
///   function and makes a count that differs from the outputs an error; a width that differs
///   from the space's would make the colour a different colour.
///
/// The bounds handed over are each intersected with `[0, 1]`, which is not a narrowing: the
/// producer clips to `/Range` (§7.10.1) and then [`ColourSpace::to_rgb`] clamps each component
/// to the unit interval, and clamping into `[lo, hi]` and then into `[0, 1]` is clamping into
/// the intersection. §8.7.4.5.2's Table 78 explicitly allows a wider declared range — "[i]f the
/// value returned by the function for a given colour component is out of range, it shall be
/// adjusted to the nearest valid value" — so a document that declares one keeps drawing, on
/// either path, with the same arithmetic.
fn device_program(
    functions: &[Function],
    space: &ColourSpace,
    into: &Compositing,
    rectangle: [f32; 4],
) -> Option<pdf_render::ShadingProgram> {
    if *into != Compositing::Device {
        return None;
    }
    let [function] = functions else {
        return None;
    };
    let bounds = function.range_bounds()?;
    // A bound that is not an interval of numbers is not a clip a device can apply, and the
    // producer's own `clamp` would answer something a reader would have to derive.
    let clip = |index: usize| -> Option<[f32; 2]> {
        let (low, high) = bounds.get(index).copied()?;
        (low.is_finite() && high.is_finite() && low <= high)
            .then(|| [low.clamp(0.0, 1.0), high.clamp(0.0, 1.0)])
    };
    let range = match (space, bounds.len()) {
        (ColourSpace::Gray, 1) => pdf_render::ProgramRange::Gray(clip(0)?),
        (ColourSpace::Rgb, 3) => pdf_render::ProgramRange::Rgb([clip(0)?, clip(1)?, clip(2)?]),
        _ => return None,
    };
    Some(pdf_render::ShadingProgram::new(
        function.device_program(rectangle)?,
        range,
    ))
}

/// ISO 32000-2 §8.7.4.5.2's function of two variables, evaluated at the grid a device asks.
///
/// > In Type 1 (function-based) shadings, the colour at every point in the domain is defined
/// > by a specified mathematical function.
///
/// *Every point* has no resolution, so the display list carries this — the function group,
/// the colour space and the compositing target, everything an evaluation needs and nothing
/// borrowed from the document — and a backend asks for the grid its device wants through
/// `pdf_render::ColoursAtDeviceScale`. Self-contained for ADR 0210's reason: `Document`
/// caches behind `RefCell` and is not `Sync`, and a display list is drawn on every core.
struct FunctionColours {
    /// The shading's `/Function` group: one 2-in n-out function, or n 2-in 1-out ones.
    functions: Vec<Function>,
    /// The shading's colour space, resolved when the shading was built.
    space: ColourSpace,
    /// What the colours are being composited into (ADR 0220).
    into: Compositing,
    /// The domain rectangle the grid covers, as `[x0, x1, y0, y1]`.
    domain: [f32; 4],
    /// Whether every colour the space can produce is opaque; see `function_based`.
    opaque: bool,
}

impl std::fmt::Debug for FunctionColours {
    /// The shape of the source, never the function's samples: a type 0 function carries its
    /// whole sample stream, and a display list is printed by `Command`'s own derive.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionColours")
            .field("functions", &self.functions.len())
            .field("domain", &self.domain)
            .finish_non_exhaustive()
    }
}

/// A rectangular run of cells of a lattice: which cells of it are actually evaluated.
///
/// `pdf_render::Patch` states the block as a fraction of the domain, because that is the form
/// a device's own rectangle arrives in; this is the same block in cells, cut against the
/// lattice `cells_within_budget` settled on. The two are separate types on purpose — the
/// budget is this device's answer to §10.7.3's "internal limits" and no caller can name the
/// lattice before it has been applied.
#[derive(Debug, Clone, Copy)]
struct Block {
    /// The lattice the cells sit on. **What decides a cell's centre**, whatever the block is.
    lattice: pdf_render::Grid,
    /// The block's first column and row within that lattice.
    origin: (u32, u32),
    /// The block's own extent in cells, at least one each way.
    extent: pdf_render::Grid,
}

/// The cells of a `cells`-wide axis that the fraction `low..=high` of it needs.
///
/// Snapped outward to whole cells — a cell partly covered is a cell that has to be
/// evaluated — and then one cell further on each side, which is `Patch::within`'s stated
/// margin for the bilinear filter both backends read the grid with. The whole axis where the
/// fraction is not one: a caller that cannot say what the target reaches gets what every
/// caller got before ADR 0408.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    reason = "each product is a fraction of 0..=1 times a cell count bounded by \
              MAX_FUNCTION_CELLS, and the rounding is done in i64 — which no u32 can \
              overflow — before it is clamped into the 0..=cells a u32 holds"
)]
fn cells_touched(low: f32, high: f32, cells: u32) -> (u32, u32) {
    if !low.is_finite() || !high.is_finite() || high < low || cells == 0 {
        return (0, cells);
    }
    let at = |value: f32, round: fn(f32) -> f32| round(value * cells as f32) as i64;
    let first = (at(low, f32::floor) - 1).clamp(0, i64::from(cells) - 1) as u32;
    let last = (at(high, f32::ceil) + 1).clamp(0, i64::from(cells)) as u32;
    (first, last.max(first.saturating_add(1)))
}

impl Block {
    /// The cells of `lattice` that `within` touches, with the filter margin `Patch` asks for.
    fn of(lattice: pdf_render::Grid, within: [f32; 4]) -> Self {
        let (left, right) = cells_touched(within[0], within[1], lattice.width);
        let (top, bottom) = cells_touched(within[2], within[3], lattice.height);
        Self {
            lattice,
            origin: (left, top),
            extent: pdf_render::Grid {
                width: right.saturating_sub(left).max(1),
                height: bottom.saturating_sub(top).max(1),
            },
        }
    }

    /// The part of `domain` this block covers, in the shading's own coordinates.
    ///
    /// A block that reaches an edge of the lattice takes the domain's own bound there rather
    /// than computing one: `x0 + 1.0 * (x1 - x0)` is not `x1` in `f32` for most pairs, and an
    /// unclipped grid has to be placed by exactly the numbers it was placed by before.
    fn covers(&self, domain: [f32; 4]) -> [f32; 4] {
        #[expect(
            clippy::cast_precision_loss,
            reason = "cell indices are bounded by MAX_FUNCTION_CELLS"
        )]
        fn edge(index: u32, cells: u32, low: f32, high: f32) -> f32 {
            if index == 0 {
                return low;
            }
            if index >= cells {
                return high;
            }
            low + (index as f32 / cells.max(1) as f32) * (high - low)
        }

        let [x0, x1, y0, y1] = domain;
        let (left, top) = self.origin;
        [
            edge(left, self.lattice.width, x0, x1),
            edge(
                left.saturating_add(self.extent.width),
                self.lattice.width,
                x0,
                x1,
            ),
            edge(top, self.lattice.height, y0, y1),
            edge(
                top.saturating_add(self.extent.height),
                self.lattice.height,
                y0,
                y1,
            ),
        ]
    }
}

impl FunctionColours {
    /// One row of the block, at the cell centres §10.7.4's half-pixel rule puts them at.
    ///
    /// A row is the parallel unit as well as the serial one, so that the two paths run the
    /// same arithmetic in the same order and a raster cannot depend on how many threads drew
    /// it. Each call brings its own [`Components`] and reuses it across the row.
    ///
    /// **The centre is taken from the cell's index in the lattice, never in the block.** That
    /// one line is what makes the clip exact: cell `(i, j)` of the lattice is the same
    /// coordinate, in the same `f32` bits, whether the block is the whole grid or a corner of
    /// it. ADR 0408.
    fn row(&self, row: usize, block: Block, out: &mut [Color]) {
        let [x0, x1, y0, y1] = self.domain;
        #[expect(
            clippy::cast_precision_loss,
            reason = "cell indices are bounded by MAX_FUNCTION_CELLS, far inside f32's \
                      exact integer range"
        )]
        let centre = |index: u32, cells: u32| -> f32 {
            (2.0 * index as f32 + 1.0) / (2.0 * cells.max(1) as f32)
        };
        let row = block
            .origin
            .1
            .saturating_add(u32::try_from(row).unwrap_or(u32::MAX));
        let y = y0 + centre(row, block.lattice.height) * (y1 - y0);
        let mut scratch = Components::default();
        for (column, cell) in out.iter_mut().enumerate() {
            let column = block
                .origin
                .0
                .saturating_add(u32::try_from(column).unwrap_or(u32::MAX));
            let x = x0 + centre(column, block.lattice.width) * (x1 - x0);
            *cell = colour_into(
                &self.functions,
                &[x, y],
                &self.space,
                &self.into,
                &mut scratch,
            );
        }
    }
}

impl pdf_render::ColoursAtDeviceScale for FunctionColours {
    /// The function evaluated at each cell's centre, over the block the target can sample.
    ///
    /// The centre is §10.7.4's rule for reading a raster back — "the point whose coordinate
    /// values have fractional parts of one-half" — applied in the writing direction: when the
    /// grid is the device's own, each device pixel then carries the function's value at that
    /// pixel's centre, which is as close to "the colour at every point" as a raster gets.
    ///
    /// Row by row, and the rows across rayon's pool where [`rows_in_parallel`] says so. ADR 0364
    /// has the measurement; what it cost to divide is one pass filling the grid before writing it,
    /// because a slice has to exist before it can be handed out in pieces. That is a memset
    /// against the function evaluations it carries, and it is not measurable beside them.
    ///
    /// **The budget is applied to the lattice and not to the block**, which is the one place a
    /// reader might expect the opposite. Bounding the block would be bounding the work; bounding
    /// the lattice is bounding *where the samples fall*, and a magnified page has to keep the
    /// lattice it would have had unclipped or the clip stops being exact. It costs nothing —
    /// the block is never larger — and it leaves a magnified shading exactly as coarse as it
    /// was before, which is a fidelity question ADR 0408 section 6 keeps separate from this one.
    fn colours(&self, patch: pdf_render::Patch) -> ColourGrid {
        let block = Block::of(cells_within_budget(patch.grid), patch.within);
        let width = block.extent.width as usize;
        let total = width.saturating_mul(block.extent.height as usize);
        let mut pixels = vec![Color::TRANSPARENT; total];

        if rows_in_parallel(total) {
            pixels
                .par_chunks_mut(width.max(1))
                .enumerate()
                .for_each(|(row, out)| self.row(row, block, out));
        } else {
            for (row, out) in pixels.chunks_mut(width.max(1)).enumerate() {
                self.row(row, block, out);
            }
        }

        ColourGrid {
            width: block.extent.width,
            height: block.extent.height,
            pixels: pixels.into(),
            covers: block.covers(self.domain),
        }
    }

    fn is_opaque(&self) -> bool {
        self.opaque
    }
}

/// Whether a grid of `cells` is worth dividing across rayon's pool.
///
/// **This is rasterisation-side work and not interpretation.** Interpreting a content stream
/// is sequential by construction — each operator reads the graphics state the one before it
/// left — so nothing in `content.rs` is a candidate for a pool. A grid of cells is the
/// opposite shape and is the shape `image::band_pixels` already divides: each cell is a pure
/// function of its own two coordinates, so a row boundary changes which thread computes a
/// value and never which value is computed. That is `doc/habits.md`'s question — *what does
/// a parallel unit's answer depend on* — answered before the division rather than after.
///
/// Two conditions, both measured (ADR 0364):
///
/// - **A grid smaller than [`PARALLEL_CELLS`] stays on one thread.** A shading covering a
///   swatch resolves to a few hundred cells, and a fork-join round trip costs more than the
///   evaluations it divides.
/// - **A caller already on a rayon worker keeps the grid**, which is `colour::build_ink_table`'s
///   rule for a different reason. `render-cpu` splits a page into strips across the pool and
///   builds this shading's pattern inside each; dividing again would fork a job per strip into
///   a pool with no idle thread to take it, and the page is already using every core.
fn rows_in_parallel(cells: usize) -> bool {
    cells >= PARALLEL_CELLS
        && rayon::current_num_threads() >= 2
        && rayon::current_thread_index().is_none()
}

/// The smallest grid worth evaluating on more than one thread.
///
/// A 64×64 tile, and **it was chosen against the clock rather than with it**. In wall clock the
/// division wins at every size measured — 600 renders of one type 4 program, at load 4.5 on this
/// machine's 24 cores, went 0.419 s → 0.194 at 400 cells and 45.778 → 18.561 at 40 000 — so a
/// threshold read off that column alone would be zero. The column it is read off instead is
/// processor time: at 400 cells the division buys 1.0 s of clock for **5.6×** the processor time
/// (2.061 s of user time against 8.696 + 2.927), and it buys that only where there is an idle core
/// to spend it on. Re-run at load 45 the 4096-cell arm read 9.145 s serial against **11.944 s**
/// divided, the division losing outright. A viewer is not the only thing on a person's machine and
/// a page may hold many shadings where the measurement held one, so below a tile the grid is a few
/// milliseconds of work and stays where it is. ADR 0364 has the whole table.
const PARALLEL_CELLS: usize = 1 << 12;

/// The grid a request will actually be answered at: no finer than asked, no more than fits.
///
/// The request is a number a magnification controls, so [`MAX_FUNCTION_CELLS`] bounds it
/// the way `image::MAX_MASK_GRID` bounds a deferred mask — halving both axes until the
/// product fits, so the grid keeps the shape of the request.
fn cells_within_budget(grid: pdf_render::Grid) -> pdf_render::Grid {
    let mut cells = pdf_render::Grid {
        width: grid.width.max(1),
        height: grid.height.max(1),
    };
    while u64::from(cells.width).saturating_mul(u64::from(cells.height)) > MAX_FUNCTION_CELLS
        && (cells.width > 1 || cells.height > 1)
    {
        cells.width = (cells.width / 2).max(1);
        cells.height = (cells.height / 2).max(1);
    }
    cells
}

/// Reads a six-number matrix, defaulting to the identity.
pub fn matrix_of(document: &Document, dict: &Dictionary, key: &str) -> Transform {
    let array = document.get_key(dict, key);
    let Some(items) = array.as_array() else {
        return Transform::IDENTITY;
    };
    let values: Vec<f32> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .collect();
    match <[f32; 6]>::try_from(values.as_slice()) {
        Ok(matrix) => Transform::new(
            matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5],
        ),
        Err(_) => Transform::IDENTITY,
    }
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a shading coordinate outside f32's range is not a coordinate"
    )]
    {
        value as f32
    }
}
