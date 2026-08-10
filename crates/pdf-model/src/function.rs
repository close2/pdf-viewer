//! PDF functions.
//!
//! A function maps *m* numbers to *n* numbers, and PDF uses them wherever a value has to
//! vary continuously: the colour along a shading, the tint transform of a `Separation`
//! space, a transfer function, a soft mask's alpha. They are the prerequisite for
//! shadings, which is why they live here rather than inside them.
//!
//! Four kinds exist and all four are used in practice. Sampled functions (type 0) are the
//! most common by some margin, exponential (2) next, then stitching (3), then the
//! PostScript calculator (4).
//!
//! # Everything is clamped, twice
//!
//! The specification requires inputs to be clipped to `/Domain` and outputs to `/Range`
//! before and after evaluation. That is not defensive tidiness — it is what keeps a
//! malformed or hostile function from producing infinities and `NaN`s that then propagate
//! into geometry. Every constructor here records those bounds and every evaluation applies
//! them.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object};

/// Most values a function may take or return — the *dimensionality*, m and n.
///
/// The specification sets no limit. This one bounds what a single evaluation can allocate
/// and is far above any real function: colour spaces top out at a handful of components.
///
/// **It is not a bound on §7.10.4's k**, and applying it to one cost this tree four shadings
/// in a real document; [`MAX_FUNCTIONS`] is that bound and says why the two are different.
const MAX_VALUES: usize = 64;

/// How many functions one call into the parser may build, and therefore §7.10.4's largest k.
///
/// §7.10.4's Table 41 makes `/Functions` "[a]n array of k , 1-input functions that shall make
/// up the stitching function" and bounds k **nowhere**: the only value it singles out is the
/// small one, "[t]he value of k may be 1". A 256-stop gradient is written as k = 255, and
/// `2750009.pdf` in the `SafeDocs` sample is exactly that — four shadings, 255 subfunctions
/// apiece, refused whole because [`MAX_VALUES`] was being read as a bound on k when its own
/// documentation says it bounds a component count. §7.10.4 settles that from the other side
/// too: a type 3 function's "Domain shall be of size 2 (that is, 𝑚  =  1 )", so the quantity
/// `MAX_VALUES` bounds is *fixed at one* here while k is free.
///
/// So the bound is a resource budget rather than a reading of the clause, which
/// `CLAUDE.md`'s third principle requires of pathological content. It is a budget for the
/// whole tree one root builds, not a per-array limit, because a bound on breadth alone
/// leaves `breadth ^ depth` reachable. 4096 is sixteen times the largest k seen in a real
/// file, and a `Function` is 120 bytes on this target, so the ceiling is 480 KiB of
/// functions for one shading — checked by `tests/hostile_functions.rs` rather than recalled.
const MAX_FUNCTIONS: usize = 4096;

/// How deep §7.10.4's stitching functions nest, on the way in and on the way out.
///
/// Nesting is legal — a subfunction may itself be a stitching function — and the standard
/// bounds it nowhere, so a `/Functions` array naming its own object recurses for ever. That
/// is not hypothetical: a 720-byte document doing it overflowed the stack of every program
/// in this tree until the four-hundred-and-twenty-fifth session, and
/// `tests/hostile_functions.rs` is the regression test `CLAUDE.md` requires of a crasher.
///
/// One constant for the build and for [`Function::breakpoints`]' walk, so that the two
/// cannot disagree about how deep a chain can be.
const MAX_STITCH_DEPTH: usize = 8;

/// What one call into the parser may spend, shared by every function it builds.
///
/// Threaded rather than counted per array: `/Functions` may hold stitching functions, so the
/// only bound that holds is one on the whole tree.
struct Budget {
    /// How many more functions may be built before the tree is refused.
    remaining: usize,
    /// How many stitching functions are open above the one being built.
    depth: usize,
}

impl Budget {
    /// A fresh budget for one root.
    const fn new() -> Self {
        Self {
            remaining: MAX_FUNCTIONS,
            depth: 0,
        }
    }

    /// Charges one function to the budget.
    fn spend(&mut self) -> Result<(), FunctionError> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| FunctionError::Malformed {
                detail: format!("more than {MAX_FUNCTIONS} functions in one chain"),
            })?;
        Ok(())
    }
}

/// Why a function could not be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FunctionError {
    /// The dictionary does not describe a function this crate implements.
    #[error("function type {kind} is not implemented")]
    UnsupportedType {
        /// The `/FunctionType` that was found.
        kind: i64,
    },
    /// The function is structurally invalid.
    #[error("malformed function: {detail}")]
    Malformed {
        /// What was wrong.
        detail: String,
    },
}

/// A PDF function, ready to evaluate.
#[derive(Debug, Clone)]
pub struct Function {
    /// Input bounds, as pairs. Inputs are clipped to these before anything else.
    domain: Vec<(f32, f32)>,
    /// Output bounds, as pairs. Required for types 0 and 4, optional otherwise.
    range: Option<Vec<(f32, f32)>>,
    kind: Kind,
}

/// The four function types.
#[derive(Debug, Clone)]
enum Kind {
    /// Type 0: values sampled on a grid, interpolated multilinearly.
    Sampled(Box<Sampled>),
    /// Type 2: `C0 + x^N * (C1 - C0)`.
    Exponential {
        c0: Vec<f32>,
        c1: Vec<f32>,
        exponent: f32,
    },
    /// Type 3: a series of functions covering consecutive sub-domains.
    Stitching {
        functions: Vec<Function>,
        bounds: Vec<f32>,
        encode: Vec<(f32, f32)>,
    },
    /// Type 4: a small PostScript expression.
    PostScript(Arc<[Instruction]>),
}

/// A type 0 sampled function.
#[derive(Debug, Clone)]
struct Sampled {
    /// Samples per input dimension.
    size: Vec<usize>,
    /// How many outputs each sample carries.
    outputs: usize,
    /// Sample values, already decoded to their `/Decode` range and flattened.
    ///
    /// Laid out with the first input varying fastest, as the specification requires.
    samples: Vec<f32>,
    /// Maps each input from its domain onto sample indices.
    encode: Vec<(f32, f32)>,
}

impl Function {
    /// Builds a function from a dictionary or stream.
    ///
    /// # Errors
    ///
    /// See [`FunctionError`].
    pub fn parse(document: &Document, object: &Object) -> Result<Self, FunctionError> {
        Self::parse_within(document, object, &mut Budget::new())
    }

    /// The body of [`Self::parse`], carrying the budget every nested function shares.
    fn parse_within(
        document: &Document,
        object: &Object,
        budget: &mut Budget,
    ) -> Result<Self, FunctionError> {
        budget.spend()?;
        let resolved = document.resolve(object);
        let dict = match &resolved {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            _ => {
                return Err(FunctionError::Malformed {
                    detail: "not a dictionary or stream".to_owned(),
                });
            }
        };

        let domain = pairs(document, &dict, "Domain", MAX_VALUES).ok_or_else(|| {
            FunctionError::Malformed {
                detail: "no /Domain".to_owned(),
            }
        })?;
        let range = pairs(document, &dict, "Range", MAX_VALUES);

        let kind = document
            .get_key(&dict, "FunctionType")
            .as_integer()
            .ok_or_else(|| FunctionError::Malformed {
                detail: "no /FunctionType".to_owned(),
            })?;

        let kind = match kind {
            0 => Kind::Sampled(Box::new(Self::parse_sampled(
                document,
                &resolved,
                &dict,
                &domain,
                range.as_deref(),
            )?)),
            2 => Self::parse_exponential(document, &dict)?,
            3 => Self::parse_stitching(document, &dict, &domain, budget)?,
            4 => Self::parse_postscript(document, &resolved)?,
            other => return Err(FunctionError::UnsupportedType { kind: other }),
        };

        Ok(Self {
            domain,
            range,
            kind,
        })
    }

    /// Builds the one-or-many form `/Function` takes in shadings.
    ///
    /// A shading may give a single function producing every colour component, or an array
    /// of single-output functions, one per component. Callers should not care which.
    ///
    /// # Errors
    ///
    /// See [`FunctionError`].
    pub fn parse_group(document: &Document, object: &Object) -> Result<Vec<Self>, FunctionError> {
        // One budget for the whole group rather than one per element: the array's own length
        // is somebody else's number, so a group of a million one-output functions would be
        // as unbounded as a stitching function naming a million.
        let mut budget = Budget::new();
        let resolved = document.resolve(object);
        match &resolved {
            Object::Array(items) => items
                .iter()
                .map(|item| Self::parse_within(document, item, &mut budget))
                .collect(),
            _ => Ok(vec![Self::parse_within(document, &resolved, &mut budget)?]),
        }
    }

    /// Where this function may be discontinuous, as values in its own input domain.
    ///
    /// §7.10.4's type 3 function "defines a stitching of the subdomains of several 1-input
    /// functions", and its `/Bounds` are exactly where one sub-function stops and the next
    /// begins — so a bound is the only place the standard lets a 1-input function jump. Two
    /// equal bounds make a subdomain of zero width, which is how a producer writes a *step*:
    /// `issue10572.pdf` does it twelve times to draw twenty-four hard stripes.
    ///
    /// A caller sampling this function into a table needs the list so that it can put a sample
    /// on each side of every jump; without it a step is averaged into a gradient as wide as
    /// the sampling interval. Types 0, 2 and 4 declare no discontinuity: a sampled function
    /// interpolates between its samples, an exponential one is smooth, and a PostScript
    /// program's branches are not declared anywhere a reader could find them.
    ///
    /// Nested stitching functions are followed, because the jump a sub-function contains is a
    /// jump of the whole: each is mapped back through `/Encode` and the subdomain it belongs
    /// to. Only the first input is considered — `/Bounds` applies to 1-input functions, which
    /// is all §7.10.4 defines.
    #[must_use]
    pub fn breakpoints(&self) -> Vec<f32> {
        fn walk(function: &Function, depth: usize, out: &mut Vec<f32>) {
            let Kind::Stitching {
                functions,
                bounds,
                encode,
            } = &function.kind
            else {
                return;
            };
            // [`MAX_STITCH_DEPTH`] is the same constant `parse_within` builds to, so the walk
            // cannot stop short of a chain the parser was willing to construct.
            if depth > MAX_STITCH_DEPTH {
                return;
            }
            let (domain_low, domain_high) = function.domain.first().copied().unwrap_or((0.0, 1.0));
            out.extend(bounds.iter().copied());

            for (index, sub) in functions.iter().enumerate() {
                // The subdomain this sub-function covers, which is what `eval_stitching`
                // selects with.
                let low = if index == 0 {
                    domain_low
                } else {
                    bounds
                        .get(index.saturating_sub(1))
                        .copied()
                        .unwrap_or(domain_low)
                };
                let high = bounds.get(index).copied().unwrap_or(domain_high);
                let (encode_low, encode_high) = encode.get(index).copied().unwrap_or((0.0, 1.0));

                let mut inner = Vec::new();
                walk(sub, depth.saturating_add(1), &mut inner);
                for at in inner {
                    // Undo the `/Encode` mapping, which is what `eval_stitching` applies on
                    // the way in. An encode range of zero width maps the whole subdomain to
                    // one point, so nothing inside it has a position of its own.
                    if (encode_high - encode_low).abs() <= f32::EPSILON {
                        continue;
                    }
                    let fraction = (at - encode_low) / (encode_high - encode_low);
                    out.push(low + fraction * (high - low));
                }
            }
        }

        let mut out = Vec::new();
        walk(self, 0, &mut out);
        out.retain(|at| at.is_finite());
        out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        out.dedup();
        out
    }

    /// Evaluates the function, clipping inputs to the domain and outputs to the range.
    #[must_use]
    pub fn eval(&self, inputs: &[f32]) -> Vec<f32> {
        let mut clipped: Vec<f32> = Vec::with_capacity(self.domain.len().min(MAX_VALUES));
        for (index, (low, high)) in self.domain.iter().enumerate().take(MAX_VALUES) {
            let value = inputs.get(index).copied().unwrap_or(*low);
            clipped.push(clamp(value, *low, *high));
        }

        let mut outputs = match &self.kind {
            Kind::Sampled(sampled) => sampled.eval(&clipped),
            Kind::Exponential { c0, c1, exponent } => {
                let x = clipped.first().copied().unwrap_or(0.0);
                let factor = if (*exponent - 1.0).abs() < f32::EPSILON {
                    x
                } else {
                    x.powf(*exponent)
                };
                c0.iter()
                    .zip(c1.iter())
                    .map(|(start, end)| start + factor * (end - start))
                    .collect()
            }
            Kind::Stitching {
                functions,
                bounds,
                encode,
            } => self.eval_stitching(functions, bounds, encode, &clipped),
            Kind::PostScript(program) => evaluate_postscript(program, &clipped),
        };

        if let Some(range) = &self.range {
            outputs.truncate(range.len());
            // A function whose program produced too few values still has to answer with
            // the number its range promises, or the caller reads a colour component that
            // is not there.
            while outputs.len() < range.len() {
                outputs.push(0.0);
            }
            for (value, (low, high)) in outputs.iter_mut().zip(range.iter()) {
                *value = clamp(*value, *low, *high);
            }
        }
        outputs
    }

    /// How many values this function returns, when that is known before evaluating.
    #[must_use]
    pub fn outputs(&self) -> Option<usize> {
        match (&self.range, &self.kind) {
            (Some(range), _) => Some(range.len()),
            (None, Kind::Exponential { c0, .. }) => Some(c0.len()),
            (None, Kind::Stitching { functions, .. }) => functions.first().and_then(Self::outputs),
            _ => None,
        }
    }

    /// Selects the sub-function covering an input and re-maps the input onto its domain.
    fn eval_stitching(
        &self,
        functions: &[Function],
        bounds: &[f32],
        encode: &[(f32, f32)],
        clipped: &[f32],
    ) -> Vec<f32> {
        let x = clipped.first().copied().unwrap_or(0.0);
        let (domain_low, domain_high) = self.domain.first().copied().unwrap_or((0.0, 1.0));

        // The sub-domain containing x: the first bound above it, or the last function.
        let index = bounds
            .iter()
            .position(|bound| x < *bound)
            .unwrap_or(bounds.len());
        let low = if index == 0 {
            domain_low
        } else {
            bounds
                .get(index.saturating_sub(1))
                .copied()
                .unwrap_or(domain_low)
        };
        let high = bounds.get(index).copied().unwrap_or(domain_high);

        let (encode_low, encode_high) = encode.get(index).copied().unwrap_or((0.0, 1.0));
        let mapped = interpolate(x, low, high, encode_low, encode_high);

        functions
            .get(index)
            .map(|function| function.eval(&[mapped]))
            .unwrap_or_default()
    }

    fn parse_exponential(document: &Document, dict: &Dictionary) -> Result<Kind, FunctionError> {
        let c0 = numbers(document, dict, "C0", MAX_VALUES).unwrap_or_else(|| vec![0.0]);
        let c1 = numbers(document, dict, "C1", MAX_VALUES).unwrap_or_else(|| vec![1.0]);
        if c0.len() != c1.len() || c0.is_empty() || c0.len() > MAX_VALUES {
            return Err(FunctionError::Malformed {
                detail: format!("/C0 has {} values and /C1 has {}", c0.len(), c1.len()),
            });
        }
        let exponent = document.get_key(dict, "N").as_number().map_or(1.0, narrow);
        Ok(Kind::Exponential { c0, c1, exponent })
    }

    fn parse_stitching(
        document: &Document,
        dict: &Dictionary,
        domain: &[(f32, f32)],
        budget: &mut Budget,
    ) -> Result<Kind, FunctionError> {
        if budget.depth >= MAX_STITCH_DEPTH {
            return Err(FunctionError::Malformed {
                detail: format!("stitching functions nested deeper than {MAX_STITCH_DEPTH}"),
            });
        }
        let array = document.get_key(dict, "Functions");
        let items = array.as_array().ok_or_else(|| FunctionError::Malformed {
            detail: "no /Functions".to_owned(),
        })?;
        if items.len() > budget.remaining {
            return Err(FunctionError::Malformed {
                detail: format!("{} stitched functions", items.len()),
            });
        }
        budget.depth = budget.depth.saturating_add(1);
        let functions = items
            .iter()
            .map(|item| Self::parse_within(document, item, budget))
            .collect::<Result<Vec<_>, _>>();
        budget.depth = budget.depth.saturating_sub(1);
        let functions = functions?;

        // §7.10.4's `/Bounds` holds k − 1 numbers and its `/Encode` 2 × k, so both scale with
        // the number of subfunctions rather than with a component count — which is why
        // [`MAX_FUNCTIONS`] is what bounds them here and [`MAX_VALUES`] bounds `/Domain`.
        let bounds = numbers(document, dict, "Bounds", MAX_FUNCTIONS).unwrap_or_default();
        let encode = pairs(document, dict, "Encode", MAX_FUNCTIONS).unwrap_or_else(|| {
            // The specification requires /Encode, but a missing one is far better treated
            // as the identity than as a reason to drop the whole shading.
            vec![(0.0, 1.0); functions.len()]
        });

        if bounds.len().saturating_add(1) != functions.len() {
            return Err(FunctionError::Malformed {
                detail: format!(
                    "{} functions need {} bounds, found {}",
                    functions.len(),
                    functions.len().saturating_sub(1),
                    bounds.len()
                ),
            });
        }
        // Bounds must ascend and lie inside the domain, or the search below picks
        // sub-functions that were never meant to apply.
        let (low, high) = domain.first().copied().unwrap_or((0.0, 1.0));
        if bounds.windows(2).any(|pair| pair[0] > pair[1])
            || bounds.iter().any(|bound| *bound < low || *bound > high)
        {
            return Err(FunctionError::Malformed {
                detail: "/Bounds are not ascending within /Domain".to_owned(),
            });
        }

        Ok(Kind::Stitching {
            functions,
            bounds,
            encode,
        })
    }

    fn parse_sampled(
        document: &Document,
        object: &Object,
        dict: &Dictionary,
        domain: &[(f32, f32)],
        range: Option<&[(f32, f32)]>,
    ) -> Result<Sampled, FunctionError> {
        /// Bounds the sample grid, so a hostile `/Size` cannot ask for a huge allocation.
        const MAX_SAMPLES: usize = 1 << 22;

        let range = range.ok_or_else(|| FunctionError::Malformed {
            detail: "a sampled function needs /Range".to_owned(),
        })?;
        let stream = object.as_stream().ok_or_else(|| FunctionError::Malformed {
            detail: "a sampled function must be a stream".to_owned(),
        })?;
        let data =
            document
                .decoded_stream_data(stream)
                .ok_or_else(|| FunctionError::Malformed {
                    detail: "sample data did not decode".to_owned(),
                })?;

        let size: Vec<usize> = numbers(document, dict, "Size", MAX_VALUES)
            .ok_or_else(|| FunctionError::Malformed {
                detail: "no /Size".to_owned(),
            })?
            .iter()
            .map(|value| {
                // A grid size is a count; anything fractional or negative is malformed
                // and is caught by the zero check below.
                if value.is_finite() && *value >= 0.0 && *value < 1e9 {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "guarded finite, non-negative and below a billion"
                    )]
                    {
                        *value as usize
                    }
                } else {
                    0
                }
            })
            .collect();
        if size.len() != domain.len() || size.contains(&0) {
            return Err(FunctionError::Malformed {
                detail: "/Size does not match /Domain".to_owned(),
            });
        }

        let bits = document
            .get_key(dict, "BitsPerSample")
            .as_integer()
            .unwrap_or(8);
        if !matches!(bits, 1 | 2 | 4 | 8 | 12 | 16 | 24 | 32) {
            return Err(FunctionError::Malformed {
                detail: format!("/BitsPerSample is {bits}"),
            });
        }
        let bits = u32::try_from(bits).unwrap_or(8);

        let outputs = range.len();
        let total = size
            .iter()
            .try_fold(outputs, |acc, n| acc.checked_mul(*n))
            .ok_or_else(|| FunctionError::Malformed {
                detail: "/Size overflows".to_owned(),
            })?;
        if total > MAX_SAMPLES {
            return Err(FunctionError::Malformed {
                detail: format!("{total} samples exceeds the limit"),
            });
        }

        // `/Decode` maps raw sample values onto output values; its default is `/Range`.
        let decode = pairs(document, dict, "Decode", MAX_VALUES).unwrap_or_else(|| range.to_vec());
        let max = if bits >= 32 {
            f64::from(u32::MAX)
        } else {
            f64::from((1u32 << bits).saturating_sub(1))
        };

        let mut samples = Vec::with_capacity(total);
        let mut reader = BitReader::new(&data);
        for index in 0..total {
            let raw = reader.read(bits).unwrap_or(0);
            let component = index.checked_rem(outputs).unwrap_or(0);
            let (low, high) = decode.get(component).copied().unwrap_or((0.0, 1.0));
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a sample is at most 32 bits, well inside f32's exact range after \
                          the division below"
            )]
            let normalised = (f64::from(raw) / max) as f32;
            samples.push(low + normalised * (high - low));
        }

        let encode = pairs(document, dict, "Encode", MAX_VALUES).unwrap_or_else(|| {
            size.iter()
                .map(|n| {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "grid sizes are bounded by MAX_SAMPLES"
                    )]
                    let last = n.saturating_sub(1) as f32;
                    (0.0, last)
                })
                .collect()
        });

        Ok(Sampled {
            size,
            outputs,
            samples,
            encode,
        })
    }

    fn parse_postscript(document: &Document, object: &Object) -> Result<Kind, FunctionError> {
        let stream = object.as_stream().ok_or_else(|| FunctionError::Malformed {
            detail: "a PostScript function must be a stream".to_owned(),
        })?;
        let data =
            document
                .decoded_stream_data(stream)
                .ok_or_else(|| FunctionError::Malformed {
                    detail: "program did not decode".to_owned(),
                })?;
        let program = compile_postscript(&data)?;
        Ok(Kind::PostScript(program.into()))
    }
}

impl Sampled {
    /// Evaluates by multilinear interpolation between the surrounding grid samples.
    fn eval(&self, inputs: &[f32]) -> Vec<f32> {
        // The grid position each input falls at, split into the sample below it and the
        // fraction beyond that sample.
        let mut base = Vec::with_capacity(self.size.len());
        let mut fraction = Vec::with_capacity(self.size.len());
        for (index, count) in self.size.iter().enumerate() {
            let x = inputs.get(index).copied().unwrap_or(0.0);
            let (low, high) = self.encode.get(index).copied().unwrap_or((0.0, 1.0));
            #[expect(
                clippy::cast_precision_loss,
                reason = "grid sizes are bounded by MAX_SAMPLES"
            )]
            let last = count.saturating_sub(1) as f32;
            let position = clamp(interpolate(x, 0.0, 1.0, low, high), 0.0, last);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "position is clamped to 0..=last, so the floor is a valid index"
            )]
            let floor = position.floor() as usize;
            base.push(floor.min(count.saturating_sub(1)));
            fraction.push(position - position.floor());
        }

        // Interpolate over the 2^dimensions corner samples. Bounded because a function
        // with many inputs would already have been refused by the sample limit.
        let corners = 1usize.checked_shl(u32::try_from(self.size.len()).unwrap_or(0));
        let Some(corners) = corners else {
            return vec![0.0; self.outputs];
        };

        let mut result = vec![0.0f32; self.outputs];
        for corner in 0..corners {
            let mut weight = 1.0f32;
            let mut offset = 0usize;
            let mut stride = 1usize;
            for (dimension, count) in self.size.iter().enumerate() {
                let up = corner
                    .checked_shr(u32::try_from(dimension).unwrap_or(0))
                    .unwrap_or(0)
                    & 1;
                let f = fraction.get(dimension).copied().unwrap_or(0.0);
                weight *= if up == 1 { f } else { 1.0 - f };
                let index = base
                    .get(dimension)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(up)
                    .min(count.saturating_sub(1));
                offset = offset.saturating_add(index.saturating_mul(stride));
                stride = stride.saturating_mul(*count);
            }
            if weight == 0.0 {
                continue;
            }
            for (component, value) in result.iter_mut().enumerate() {
                let at = offset
                    .saturating_mul(self.outputs)
                    .saturating_add(component);
                *value += weight * self.samples.get(at).copied().unwrap_or(0.0);
            }
        }
        result
    }
}

/// Reads big-endian bit fields of arbitrary width from a byte slice.
pub(crate) struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, bit: 0 }
    }

    /// How many bits have been consumed.
    pub(crate) fn position(&self) -> usize {
        self.bit
    }

    /// Reads the next `bits` bits, or `None` past the end of the data.
    pub(crate) fn read(&mut self, bits: u32) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..bits {
            let byte = self.data.get(self.bit.checked_div(8)?)?;
            let shift = 7u32.checked_sub(u32::try_from(self.bit.checked_rem(8)?).ok()?)?;
            let one = (byte >> shift) & 1;
            value = value.checked_mul(2)?.checked_add(u32::from(one))?;
            self.bit = self.bit.checked_add(1)?;
        }
        Some(value)
    }
}

/// Reads an array of at most `most` numbers, or `None` where it holds more.
///
/// **The bound is the caller's**, because the arrays here are counted in two different
/// units: `/Domain`, `/Range`, `/C0`, `/C1` and `/Size` scale with a function's
/// dimensionality, which [`MAX_VALUES`] bounds, while §7.10.4's `/Bounds` and `/Encode`
/// scale with its k, which [`MAX_FUNCTIONS`] does. One constant serving both is how four
/// shadings in a real document came to be refused, so the unit is named at every call site.
fn numbers(document: &Document, dict: &Dictionary, key: &str, most: usize) -> Option<Vec<f32>> {
    let array = document.get_key(dict, key);
    let items = array.as_array()?;
    if items.len() > most {
        return None;
    }
    Some(
        items
            .iter()
            .filter_map(|item| document.resolve(item).as_number().map(narrow))
            .collect(),
    )
}

/// Reads an array of at most `most` consecutive low/high pairs.
fn pairs(
    document: &Document,
    dict: &Dictionary,
    key: &str,
    most: usize,
) -> Option<Vec<(f32, f32)>> {
    let values = numbers(document, dict, key, most.saturating_mul(2))?;
    if values.is_empty() || values.len() % 2 != 0 {
        return None;
    }
    Some(
        values
            .chunks_exact(2)
            .filter_map(|pair| Some((*pair.first()?, *pair.get(1)?)))
            .collect(),
    )
}

fn clamp(value: f32, low: f32, high: f32) -> f32 {
    // A `NaN` input must not propagate into geometry; the domain's lower bound is as
    // good an answer as any and is finite.
    if value.is_nan() {
        return low;
    }
    if low <= high {
        value.clamp(low, high)
    } else {
        value.clamp(high, low)
    }
}

/// Maps `value` from one interval onto another.
fn interpolate(value: f32, from_low: f32, from_high: f32, to_low: f32, to_high: f32) -> f32 {
    let span = from_high - from_low;
    if span.abs() < f32::EPSILON {
        return to_low;
    }
    to_low + (value - from_low) * (to_high - to_low) / span
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a function bound outside f32's range is not a bound"
    )]
    {
        value as f32
    }
}

/// One step of a compiled type 4 function.
///
/// The source is a PostScript expression, but only a fixed operator set is permitted and
/// there are no loops, so it compiles to a flat instruction list with explicit jumps.
/// That removes the interpreter's need for a call stack and makes non-termination
/// impossible by construction.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Pushes a literal.
    Push(f32),
    /// Applies an operator.
    Operator(Operator),
    /// Pops a boolean; when false, jumps to the given instruction.
    JumpUnless(usize),
    /// Jumps unconditionally.
    Jump(usize),
}

/// The operators a type 4 function may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[expect(
    missing_docs,
    reason = "each variant is the PostScript operator it is named for"
)]
pub enum Operator {
    Abs,
    Add,
    Atan,
    Ceiling,
    Cos,
    Cvi,
    Cvr,
    Div,
    Exp,
    Floor,
    Idiv,
    Ln,
    Log,
    Mod,
    Mul,
    Neg,
    Round,
    Sin,
    Sqrt,
    Sub,
    Truncate,
    And,
    Bitshift,
    Eq,
    False,
    Ge,
    Gt,
    Le,
    Lt,
    Ne,
    Not,
    Or,
    True,
    Xor,
    Copy,
    Dup,
    Exch,
    Index,
    Pop,
    Roll,
}

/// Compiles a type 4 program into a flat instruction list.
fn compile_postscript(source: &[u8]) -> Result<Vec<Instruction>, FunctionError> {
    /// Bounds the program, so a hostile stream cannot compile to an enormous list.
    const MAX_INSTRUCTIONS: usize = 1 << 16;

    let text = String::from_utf8_lossy(source);
    let spaced = text
        .replace('{', " { ")
        .replace('}', " } ")
        .replace('%', " % ");
    let mut tokens = spaced.split_whitespace().peekable();

    // The whole program is wrapped in one pair of braces, which is consumed here so the
    // body compiles as a plain sequence.
    if tokens.peek() == Some(&"{") {
        tokens.next();
    }

    let mut out = Vec::new();
    compile_block(&mut tokens, &mut out, 0)?;
    if out.len() > MAX_INSTRUCTIONS {
        return Err(FunctionError::Malformed {
            detail: format!("{} instructions exceeds the limit", out.len()),
        });
    }
    Ok(out)
}

/// Compiles until the closing brace of the current block.
fn compile_block(
    tokens: &mut std::iter::Peekable<std::str::SplitWhitespace<'_>>,
    out: &mut Vec<Instruction>,
    depth: usize,
) -> Result<(), FunctionError> {
    /// Bounds `{}` nesting, since each level recurses.
    const MAX_DEPTH: usize = 32;

    if depth > MAX_DEPTH {
        return Err(FunctionError::Malformed {
            detail: "procedures nested too deeply".to_owned(),
        });
    }

    while let Some(token) = tokens.next() {
        match token {
            "}" => return Ok(()),
            "%" => {
                // A comment runs to end of line, which whitespace splitting has already
                // destroyed; skipping one token is the closest safe approximation.
                tokens.next();
            }
            "{" => {
                // A procedure, which must be followed by another procedure and `ifelse`,
                // or by `if`. Both are compiled to jumps around the bodies.
                let jump_unless = out.len();
                out.push(Instruction::JumpUnless(0));
                compile_block(tokens, out, depth.saturating_add(1))?;

                match tokens.next() {
                    Some("if") => {
                        let target = out.len();
                        set_jump(out, jump_unless, target)?;
                    }
                    Some("{") => {
                        let jump_over = out.len();
                        out.push(Instruction::Jump(0));
                        let else_start = out.len();
                        set_jump(out, jump_unless, else_start)?;
                        compile_block(tokens, out, depth.saturating_add(1))?;
                        match tokens.next() {
                            Some("ifelse") => {
                                let target = out.len();
                                set_jump(out, jump_over, target)?;
                            }
                            other => {
                                return Err(FunctionError::Malformed {
                                    detail: format!("expected ifelse, found {other:?}"),
                                });
                            }
                        }
                    }
                    other => {
                        return Err(FunctionError::Malformed {
                            detail: format!("expected if or a second procedure, found {other:?}"),
                        });
                    }
                }
            }
            _ => out.push(compile_token(token)?),
        }
    }
    Ok(())
}

fn set_jump(out: &mut [Instruction], at: usize, target: usize) -> Result<(), FunctionError> {
    match out.get_mut(at) {
        Some(Instruction::JumpUnless(slot) | Instruction::Jump(slot)) => {
            *slot = target;
            Ok(())
        }
        _ => Err(FunctionError::Malformed {
            detail: "jump target lost".to_owned(),
        }),
    }
}

fn compile_token(token: &str) -> Result<Instruction, FunctionError> {
    if let Ok(value) = token.parse::<f32>() {
        return Ok(Instruction::Push(value));
    }
    let operator = match token {
        "abs" => Operator::Abs,
        "add" => Operator::Add,
        "atan" => Operator::Atan,
        "ceiling" => Operator::Ceiling,
        "cos" => Operator::Cos,
        "cvi" => Operator::Cvi,
        "cvr" => Operator::Cvr,
        "div" => Operator::Div,
        "exp" => Operator::Exp,
        "floor" => Operator::Floor,
        "idiv" => Operator::Idiv,
        "ln" => Operator::Ln,
        "log" => Operator::Log,
        "mod" => Operator::Mod,
        "mul" => Operator::Mul,
        "neg" => Operator::Neg,
        "round" => Operator::Round,
        "sin" => Operator::Sin,
        "sqrt" => Operator::Sqrt,
        "sub" => Operator::Sub,
        "truncate" => Operator::Truncate,
        "and" => Operator::And,
        "bitshift" => Operator::Bitshift,
        "eq" => Operator::Eq,
        "false" => Operator::False,
        "ge" => Operator::Ge,
        "gt" => Operator::Gt,
        "le" => Operator::Le,
        "lt" => Operator::Lt,
        "ne" => Operator::Ne,
        "not" => Operator::Not,
        "or" => Operator::Or,
        "true" => Operator::True,
        "xor" => Operator::Xor,
        "copy" => Operator::Copy,
        "dup" => Operator::Dup,
        "exch" => Operator::Exch,
        "index" => Operator::Index,
        "pop" => Operator::Pop,
        "roll" => Operator::Roll,
        other => {
            return Err(FunctionError::Malformed {
                detail: format!("unknown operator {other}"),
            });
        }
    };
    Ok(Instruction::Operator(operator))
}

/// Runs a compiled type 4 program.
///
/// The stack is bounded and the instruction list has no backward jumps, so this always
/// terminates. A program that underflows the stack yields whatever it managed to compute
/// rather than failing: a shading with one bad colour is better than no shading.
fn evaluate_postscript(program: &[Instruction], inputs: &[f32]) -> Vec<f32> {
    /// Bounds the operand stack against a program that only pushes.
    const MAX_STACK: usize = 1000;

    let mut stack: Vec<f32> = inputs.to_vec();
    let mut at = 0usize;
    let mut steps = 0usize;

    while let Some(instruction) = program.get(at) {
        steps = steps.saturating_add(1);
        // Forward-only jumps make this unreachable, and it costs one comparison to be
        // certain rather than to argue about it.
        if steps > program.len().saturating_mul(2).saturating_add(16) {
            break;
        }
        at = at.saturating_add(1);

        match instruction {
            Instruction::Push(value) => {
                if stack.len() < MAX_STACK {
                    stack.push(*value);
                }
            }
            Instruction::Jump(target) => at = *target,
            Instruction::JumpUnless(target) => {
                if stack.pop().unwrap_or(0.0) == 0.0 {
                    at = *target;
                }
            }
            Instruction::Operator(operator) => apply_operator(*operator, &mut stack),
        }
    }
    stack
}

/// Applies one operator to the stack.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per PostScript operator reads better as a single table"
)]
fn apply_operator(operator: Operator, stack: &mut Vec<f32>) {
    /// Pops one value, or zero when the stack has underflowed.
    fn pop(stack: &mut Vec<f32>) -> f32 {
        stack.pop().unwrap_or(0.0)
    }
    /// True is 1.0 and false is 0.0, as the specification's booleans are numbers here.
    fn boolean(value: bool) -> f32 {
        if value { 1.0 } else { 0.0 }
    }

    match operator {
        Operator::Abs => {
            let a = pop(stack);
            stack.push(a.abs());
        }
        Operator::Add => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(a + b);
        }
        Operator::Atan => {
            let (den, num) = (pop(stack), pop(stack));
            // PostScript's atan returns degrees in 0..360.
            let mut degrees = num.atan2(den).to_degrees();
            if degrees < 0.0 {
                degrees += 360.0;
            }
            stack.push(degrees);
        }
        Operator::Ceiling => {
            let a = pop(stack);
            stack.push(a.ceil());
        }
        Operator::Cos => {
            let a = pop(stack);
            stack.push(a.to_radians().cos());
        }
        Operator::Cvi | Operator::Truncate => {
            let a = pop(stack);
            stack.push(a.trunc());
        }
        Operator::Cvr => {}
        Operator::Div => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(if b == 0.0 { 0.0 } else { a / b });
        }
        Operator::Exp => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(a.powf(b));
        }
        Operator::Floor => {
            let a = pop(stack);
            stack.push(a.floor());
        }
        Operator::Idiv => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(if b.trunc() == 0.0 {
                0.0
            } else {
                (a.trunc() / b.trunc()).trunc()
            });
        }
        Operator::Ln => {
            let a = pop(stack);
            stack.push(if a > 0.0 { a.ln() } else { 0.0 });
        }
        Operator::Log => {
            let a = pop(stack);
            stack.push(if a > 0.0 { a.log10() } else { 0.0 });
        }
        Operator::Mod => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(if b.trunc() == 0.0 {
                0.0
            } else {
                a.trunc() % b.trunc()
            });
        }
        Operator::Mul => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(a * b);
        }
        Operator::Neg => {
            let a = pop(stack);
            stack.push(-a);
        }
        Operator::Round => {
            let a = pop(stack);
            stack.push(a.round());
        }
        Operator::Sin => {
            let a = pop(stack);
            stack.push(a.to_radians().sin());
        }
        Operator::Sqrt => {
            let a = pop(stack);
            stack.push(if a > 0.0 { a.sqrt() } else { 0.0 });
        }
        Operator::Sub => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(a - b);
        }
        Operator::And => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(bits(a, b, |x, y| x & y));
        }
        Operator::Or => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(bits(a, b, |x, y| x | y));
        }
        Operator::Xor => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(bits(a, b, |x, y| x ^ y));
        }
        Operator::Bitshift => {
            let (shift, value) = (pop(stack), pop(stack));
            let value = to_integer(value);
            let shift = to_integer(shift);
            let result = if shift >= 0 {
                value
                    .checked_shl(u32::try_from(shift).unwrap_or(u32::MAX))
                    .unwrap_or(0)
            } else {
                value
                    .checked_neg()
                    .and_then(|_| shift.checked_neg())
                    .and_then(|amount| u32::try_from(amount).ok())
                    .and_then(|amount| value.checked_shr(amount))
                    .unwrap_or(0)
            };
            #[expect(
                clippy::cast_precision_loss,
                reason = "a shifted integer beyond f32's exact range is already meaningless"
            )]
            stack.push(result as f32);
        }
        Operator::Not => {
            let a = pop(stack);
            // `not` is logical on a boolean and bitwise on an integer; both agree that
            // zero becomes one.
            stack.push(if a == 0.0 { 1.0 } else { 0.0 });
        }
        Operator::Eq => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean((a - b).abs() < f32::EPSILON));
        }
        Operator::Ne => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean((a - b).abs() >= f32::EPSILON));
        }
        Operator::Ge => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean(a >= b));
        }
        Operator::Gt => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean(a > b));
        }
        Operator::Le => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean(a <= b));
        }
        Operator::Lt => {
            let (b, a) = (pop(stack), pop(stack));
            stack.push(boolean(a < b));
        }
        Operator::True => stack.push(1.0),
        Operator::False => stack.push(0.0),
        Operator::Pop => {
            stack.pop();
        }
        Operator::Dup => {
            if let Some(top) = stack.last().copied()
                && stack.len() < MAX_STACK
            {
                stack.push(top);
            }
        }
        Operator::Exch => {
            let len = stack.len();
            if len >= 2 {
                stack.swap(len.saturating_sub(1), len.saturating_sub(2));
            }
        }
        Operator::Copy => {
            let count = pop(stack).trunc();
            if count <= 0.0 {
                return;
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "guarded positive and bounded by the stack limit below"
            )]
            let count = (count as usize).min(MAX_STACK);
            let start = stack.len().saturating_sub(count);
            if stack.len().saturating_add(count) > MAX_STACK {
                return;
            }
            let slice: Vec<f32> = stack.get(start..).map(<[f32]>::to_vec).unwrap_or_default();
            stack.extend_from_slice(&slice);
        }
        Operator::Index => {
            let n = pop(stack).trunc();
            if n < 0.0 {
                stack.push(0.0);
                return;
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "guarded non-negative just above"
            )]
            let n = n as usize;
            let value = stack
                .len()
                .checked_sub(n.saturating_add(1))
                .and_then(|at| stack.get(at).copied())
                .unwrap_or(0.0);
            stack.push(value);
        }
        Operator::Roll => {
            let shift = pop(stack).trunc();
            let count = pop(stack).trunc();
            if count <= 0.0 {
                return;
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "guarded positive just above"
            )]
            let count = (count as usize).min(stack.len());
            let start = stack.len().saturating_sub(count);
            let Some(window) = stack.get_mut(start..) else {
                return;
            };
            if count == 0 {
                return;
            }
            let shift = to_integer(shift);
            let count_i = i64::try_from(count).unwrap_or(1);
            let amount = shift.rem_euclid(count_i);
            let amount = usize::try_from(amount).unwrap_or(0);
            window.rotate_right(amount);
        }
    }
}

/// Bounds the operand stack against a program that only pushes.
const MAX_STACK: usize = 1000;

/// Narrows a value to the integer PostScript's integer operators are defined on.
///
/// Saturating rather than wrapping: a value outside the range is already meaningless as a
/// colour, and wrapping would turn it into a plausible-looking wrong one.
fn to_integer(value: f32) -> i64 {
    if value.is_nan() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an `as` cast from f32 to i64 saturates at the bounds, which is intended"
    )]
    {
        value.trunc() as i64
    }
}

/// Applies a bitwise operator to two values treated as integers.
fn bits(a: f32, b: f32, op: impl Fn(i64, i64) -> i64) -> f32 {
    let (x, y) = (to_integer(a), to_integer(b));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a result beyond f32's exact range is already meaningless as a colour"
    )]
    {
        op(x, y) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Function, compile_postscript, evaluate_postscript};

    /// Compiles and runs a type 4 program, which needs no document to exist.
    fn calculator(source: &str, inputs: &[f32]) -> Vec<f32> {
        let program = compile_postscript(source.as_bytes()).expect("compiles");
        evaluate_postscript(&program, inputs)
    }

    #[test]
    fn arithmetic_follows_postscript_stack_order() {
        // The operand order matters and is easy to reverse: `a b sub` is `a - b`.
        assert_eq!(calculator("{ 7 3 sub }", &[]), vec![4.0]);
        assert_eq!(calculator("{ 8 2 div }", &[]), vec![4.0]);
        assert_eq!(calculator("{ 2 5 exp }", &[]), vec![32.0]);
        assert_eq!(calculator("{ 7 2 idiv }", &[]), vec![3.0]);
        assert_eq!(calculator("{ 7 2 mod }", &[]), vec![1.0]);
        assert_eq!(calculator("{ -3 abs }", &[]), vec![3.0]);
    }

    /// Division by zero must yield a number rather than an infinity.
    ///
    /// A `NaN` or infinity here does not stay here: it becomes a colour component, then a
    /// coordinate, and geometry built from it is unpredictable rather than merely wrong.
    #[test]
    fn division_by_zero_yields_a_finite_value() {
        assert_eq!(calculator("{ 1 0 div }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 1 0 idiv }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 1 0 mod }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 0 ln }", &[]), vec![0.0]);
        assert_eq!(calculator("{ -1 sqrt }", &[]), vec![0.0]);
        for value in calculator("{ 0 log }", &[]) {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn conditionals_take_the_branch_the_test_selects() {
        assert_eq!(calculator("{ 1 { 10 } { 20 } ifelse }", &[]), vec![10.0]);
        assert_eq!(calculator("{ 0 { 10 } { 20 } ifelse }", &[]), vec![20.0]);
        // A bare `if` leaves the stack untouched when the test is false.
        assert_eq!(calculator("{ 5 0 { 99 } if }", &[]), vec![5.0]);
        assert_eq!(calculator("{ 5 1 { 99 } if }", &[]), vec![5.0, 99.0]);
        // Nested, because the jump targets are computed and easy to get wrong.
        assert_eq!(
            calculator("{ 1 { 0 { 1 } { 2 } ifelse } { 3 } ifelse }", &[]),
            vec![2.0]
        );
    }

    #[test]
    fn stack_operators_move_what_they_say() {
        assert_eq!(calculator("{ 1 2 exch }", &[]), vec![2.0, 1.0]);
        assert_eq!(calculator("{ 1 2 dup }", &[]), vec![1.0, 2.0, 2.0]);
        assert_eq!(calculator("{ 1 2 3 pop }", &[]), vec![1.0, 2.0]);
        // `index` counts from the top, with 0 being the top itself.
        assert_eq!(
            calculator("{ 7 8 9 2 index }", &[]),
            vec![7.0, 8.0, 9.0, 7.0]
        );
        assert_eq!(calculator("{ 1 2 2 copy }", &[]), vec![1.0, 2.0, 1.0, 2.0]);
        // `roll` rotates the top n elements upward by j.
        assert_eq!(calculator("{ 1 2 3 3 1 roll }", &[]), vec![3.0, 1.0, 2.0]);
        assert_eq!(calculator("{ 1 2 3 3 -1 roll }", &[]), vec![2.0, 3.0, 1.0]);
    }

    /// A program that underflows must not panic, and must still answer.
    #[test]
    fn an_underflowing_program_yields_a_value_rather_than_panicking() {
        assert_eq!(calculator("{ add }", &[]), vec![0.0]);
        assert_eq!(calculator("{ pop pop pop }", &[]), Vec::<f32>::new());
        assert_eq!(calculator("{ 5 index }", &[]), vec![0.0]);
    }

    /// A program that only pushes must not exhaust memory.
    #[test]
    fn an_unbounded_push_is_bounded() {
        let source = format!("{{ {} }}", "1 dup ".repeat(2000));
        let values = calculator(&source, &[]);
        assert!(
            values.len() <= super::MAX_STACK,
            "stack grew to {}",
            values.len()
        );
    }

    #[test]
    fn a_program_with_an_unknown_operator_is_refused() {
        assert!(compile_postscript(b"{ 1 frobnicate }").is_err());
        assert!(
            compile_postscript(b"{ 1 { 2 } }").is_err(),
            "a procedure needs if or ifelse"
        );
    }

    /// The real shape of a `Separation` tint transform, which is the commonest type 4 use.
    #[test]
    fn a_tint_transform_shaped_program_evaluates() {
        // One input, four CMYK outputs: tint 0 is white, tint 1 is full cyan.
        let out = calculator(
            "{ dup 0 mul exch dup 0 mul exch dup 0 mul exch 1 mul }",
            &[0.5],
        );
        assert_eq!(out, vec![0.0, 0.0, 0.0, 0.5]);
    }

    /// Building a function needs a document, so this exercises the smallest real one.
    #[test]
    fn an_exponential_function_interpolates_between_its_endpoints() {
        let source = b"%PDF-1.7\n1 0 obj\n<< /FunctionType 2 /Domain [0 1] \
                       /C0 [0 0 0] /C1 [1 0.5 0] /N 1 >>\nendobj\n\
                       trailer\n<< /Root 1 0 R >>\n";
        let document = pdf_syntax::Document::open(source.to_vec()).expect("opens");
        let object = document.get(pdf_syntax::ObjectId {
            number: 1,
            generation: 0,
        });
        let function = Function::parse(&document, &object).expect("parses");

        assert_eq!(function.eval(&[0.0]), vec![0.0, 0.0, 0.0]);
        assert_eq!(function.eval(&[1.0]), vec![1.0, 0.5, 0.0]);
        assert_eq!(function.eval(&[0.5]), vec![0.5, 0.25, 0.0]);
        // Inputs outside the domain are clipped, not extrapolated.
        assert_eq!(function.eval(&[-5.0]), vec![0.0, 0.0, 0.0]);
        assert_eq!(function.eval(&[5.0]), vec![1.0, 0.5, 0.0]);
        // A function with no `/Bounds` declares no discontinuity.
        assert!(function.breakpoints().is_empty());
    }

    /// Table 39's eight `/BitsPerSample` widths, each read back as the value it encodes.
    ///
    /// §7.10.2, Table 39:
    ///
    /// > Valid values shall be 1 , 2 , 4 , 8 , 12 , 16 , 24 , and 32 .
    ///
    /// **Eight**, and the ledger's row said five for several sessions — which is the shape a row
    /// naming a whole test *file* as its evidence lets through, and why this test exists. Each
    /// width is checked at both ends of a two-sample table, because the sample-to-output map is
    /// `Interpolate(sample, 0, 2^BitsPerSample − 1, …)` and a width read wrongly moves the
    /// divisor rather than the samples: the endpoints are exactly where that shows.
    ///
    /// A width the clause does not list is refused rather than rounded to the nearest one.
    #[test]
    fn every_width_table_39_lists_is_read_and_no_other() {
        /// Two samples, first all zero bits and second all one bits, at `bits` wide.
        fn two_samples(bits: u32) -> String {
            // `0` then `2^bits − 1`, packed big-endian and byte-aligned at the end: for every
            // width the clause lists, two samples occupy a whole number of bytes.
            let total = (bits as usize).saturating_mul(2).div_ceil(8);
            let mut bytes = vec![0u8; total];
            // The second sample's bits are the trailing `bits` of the buffer.
            for index in 0..bits as usize {
                let at = (bits as usize).saturating_add(index);
                if let Some(byte) = bytes.get_mut(at / 8) {
                    *byte |= 0x80 >> (at % 8);
                }
            }
            bytes.iter().fold(String::new(), |mut out, byte| {
                use std::fmt::Write as _;
                let _ = write!(out, "{byte:02X}");
                out
            })
        }

        for bits in [1u32, 2, 4, 8, 12, 16, 24, 32] {
            let hex = two_samples(bits);
            let source = format!(
                "%PDF-1.7\n1 0 obj\n<< /FunctionType 0 /Domain [0 1] /Range [0 1] /Size [2] \
                 /BitsPerSample {bits} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\n\
                 endstream\nendobj\ntrailer\n<< /Root 1 0 R >>\n",
                hex.len().saturating_add(1)
            );
            let document = pdf_syntax::Document::open(source.into_bytes()).expect("opens");
            let object = document.get(pdf_syntax::ObjectId {
                number: 1,
                generation: 0,
            });
            let function = Function::parse(&document, &object)
                .unwrap_or_else(|error| panic!("/BitsPerSample {bits}: {error}"));
            assert_eq!(
                function.eval(&[0.0]),
                vec![0.0],
                "{bits} bits at the low end"
            );
            assert_eq!(
                function.eval(&[1.0]),
                vec![1.0],
                "{bits} bits at the high end"
            );
        }

        // Not a value the clause lists, so not a function.
        let source = "%PDF-1.7\n1 0 obj\n<< /FunctionType 0 /Domain [0 1] /Range [0 1] \
                      /Size [2] /BitsPerSample 5 /Length 1 >>\nstream\n\x00\nendstream\nendobj\n\
                      trailer\n<< /Root 1 0 R >>\n";
        let document = pdf_syntax::Document::open(source.as_bytes().to_vec()).expect("opens");
        let object = document.get(pdf_syntax::ObjectId {
            number: 1,
            generation: 0,
        });
        assert!(
            Function::parse(&document, &object).is_err(),
            "5 is not a width"
        );
    }

    /// A nested stitching function's bounds come back in the *outer* domain.
    ///
    /// This is `issue10572.pdf`'s shape reduced to two bands: an outer type 3 over `[-2, 2]`
    /// with one bound at 0, whose two sub-functions are each a type 3 over `[0, 1]` with two
    /// equal bounds at 0.5 — the way a producer writes a step. The steps are therefore at
    /// -1 and +1 in the outer domain, and the bound between them at 0, and getting there
    /// needs `/Encode` undone twice.
    #[test]
    fn a_nested_stitching_functions_steps_map_to_the_outer_domain() {
        let source = b"%PDF-1.7\n            1 0 obj\n<< /FunctionType 3 /Domain [-2 2] /Bounds [0] /Encode [0 1 0 1]             /Functions [2 0 R 2 0 R] >>\nendobj\n            2 0 obj\n<< /FunctionType 3 /Domain [0 1] /Bounds [0.5 0.5] /Encode [0 1 0 1 0 1]             /Functions [3 0 R 3 0 R 3 0 R] >>\nendobj\n            3 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>\nendobj\n            trailer\n<< /Root 1 0 R >>\n";
        let document = pdf_syntax::Document::open(source.to_vec()).expect("opens");
        let object = document.get(pdf_syntax::ObjectId {
            number: 1,
            generation: 0,
        });
        let function = Function::parse(&document, &object).expect("parses");

        let breaks = function.breakpoints();
        assert_eq!(breaks.len(), 3, "{breaks:?}");
        for (found, expected) in breaks.iter().zip([-1.0, 0.0, 1.0]) {
            assert!(
                (found - expected).abs() < 1e-5,
                "{breaks:?} should be [-1, 0, 1]"
            );
        }
    }
}
