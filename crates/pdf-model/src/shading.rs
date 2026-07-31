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

use pdf_render::{Color, Point, Ramp, Shading, ShadingKind, Transform};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::colour::ColourSpace;
use crate::function::Function;

/// Samples across each axis of a function-based shading.
///
/// Type 1 shadings are an arbitrary function of two variables, so unlike the other types
/// they cannot be reduced to a ramp. This grid is what the display list carries instead.
const FUNCTION_GRID: u32 = 128;

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
    built: BTreeMap<(ObjectId, usize), (Arc<ShadingKind>, Transform)>,
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
            && let Some((kind, own)) = self.built.get(&(id, resolution))
        {
            return Ok(Shading {
                kind: Arc::clone(kind),
                transform: own.then(transform),
            });
        }
        let (kind, own) = kind_of(document, object, resources, resolution)?;
        let kind = Arc::new(kind);
        if let Some(id) = key {
            self.built
                .insert((id, resolution), (Arc::clone(&kind), own));
        }
        Ok(Shading {
            kind,
            transform: own.then(transform),
        })
    }
}

/// The shading dictionary of an object, whether it is a dictionary or a stream.
fn dictionary_of(document: &Document, object: &Object) -> Option<Dictionary> {
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
    let (kind, own) = kind_of(document, object, resources, Ramp::RESOLUTION)?;
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

    let space = ColourSpace::parse(document, &document.get_key(&dict, "ColorSpace"), resources)
        .ok_or_else(|| ShadingError::Malformed {
            detail: "unsupported /ColorSpace".to_owned(),
        })?;

    let (kind, own) = match kind {
        // Only a type 1 shading has a `/Matrix`, which places its domain rectangle within
        // the shading's own space. It composes ahead of the caller's transform rather than
        // being carried separately, so the display list needs only one transform per
        // shading.
        1 => (
            function_based(document, &dict, &space)?,
            matrix_of(document, &dict, "Matrix"),
        ),
        2 => (
            axial(document, &dict, &space, resolution)?,
            Transform::IDENTITY,
        ),
        3 => (
            radial(document, &dict, &space, resolution)?,
            Transform::IDENTITY,
        ),
        4..=7 => (
            mesh(document, &resolved, &dict, &space, kind)?,
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

    // Where the function jumps, in the shading's own parameter. §8.7.4.5.3 makes the colour at
    // a point whatever the function says it is, and a type 3 function with two equal `/Bounds`
    // says one colour up to a point and another after it — a step, which a table of evenly
    // spaced samples cannot hold. `Ramp::sample_across` puts a pair of stops at each of these.
    let span = high - low;
    let mut breaks: Vec<f32> = Vec::new();
    if span.abs() > f32::EPSILON {
        for function in &functions {
            breaks.extend(
                function
                    .breakpoints()
                    .into_iter()
                    .map(|at| (at - low) / span),
            );
        }
    }

    Ok(Ramp::sample_across_at(resolution, &breaks, |t| {
        let parameter = low + t * (high - low);
        colour_from(&functions, &[parameter], space)
    }))
}

/// Evaluates a shading's functions at a point and converts the result to RGB.
///
/// A shading gives either one function producing every component or one function per
/// component; both are handled by concatenating the outputs.
fn colour_from(functions: &[Function], inputs: &[f32], space: &ColourSpace) -> Color {
    let mut components: Vec<f32> = Vec::new();
    for function in functions {
        components.extend(function.eval(inputs));
    }
    space.to_rgb(&components)
}

fn axial(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    resolution: usize,
) -> Result<ShadingKind, ShadingError> {
    let coords = coords(document, dict, 4).ok_or_else(|| ShadingError::Malformed {
        detail: "an axial shading needs four /Coords".to_owned(),
    })?;
    Ok(ShadingKind::Axial {
        start: Point::new(coords[0], coords[1]),
        end: Point::new(coords[2], coords[3]),
        ramp: ramp(document, dict, space, resolution)?,
        extend: extend(document, dict),
    })
}

fn radial(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
    resolution: usize,
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
        ramp: ramp(document, dict, space, resolution)?,
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

    let triangles =
        crate::mesh::read(document, stream, kind, space, &functions).ok_or_else(|| {
            ShadingError::Malformed {
                detail: format!("the type {kind} mesh stream could not be read"),
            }
        })?;

    Ok(ShadingKind::Mesh {
        triangles: triangles.into(),
    })
}

fn function_based(
    document: &Document,
    dict: &Dictionary,
    space: &ColourSpace,
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

    let [x0, x1, y0, y1] = rectangle;
    let mut pixels = Vec::with_capacity(
        usize::try_from(FUNCTION_GRID)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(FUNCTION_GRID).unwrap_or(0)),
    );
    for row in 0..FUNCTION_GRID {
        for column in 0..FUNCTION_GRID {
            let fx = fraction(column);
            let fy = fraction(row);
            let x = x0 + fx * (x1 - x0);
            let y = y0 + fy * (y1 - y0);
            pixels.push(colour_from(&functions, &[x, y], space));
        }
    }

    Ok(ShadingKind::Sampled {
        domain: rectangle,
        width: FUNCTION_GRID,
        height: FUNCTION_GRID,
        pixels: pixels.into(),
    })
}

/// Position of a grid index across the domain, in `0.0..=1.0`.
fn fraction(index: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "FUNCTION_GRID is a small constant, exactly representable"
    )]
    {
        index as f32 / FUNCTION_GRID.saturating_sub(1).max(1) as f32
    }
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
