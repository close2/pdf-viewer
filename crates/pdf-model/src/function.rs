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

use pdf_render::{ProgramOperator, ProgramStep};
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
        let mut outputs = Vec::new();
        let mut stack = Vec::new();
        self.eval_into(inputs, &mut outputs, &mut stack);
        outputs
    }

    /// [`Self::eval`] writing into buffers the caller owns, so that a grid of evaluations
    /// allocates once rather than once per cell.
    ///
    /// The answer is the same in every bit — this is the body [`Self::eval`] now calls — and
    /// what changes is where the memory comes from. §8.7.4.5.2's function of two variables is
    /// read at one cell per device pixel (ADR 0339), so a full-page type 1 shading asks for
    /// this a million times over, and an evaluation that allocated its own operand stack and
    /// its own result turned arithmetic into a million heap round trips. ADR 0364 has the
    /// measurement on the owner's `pi.pdf`.
    ///
    /// `stack` is a type 4 program's operand stack and is nothing to a function of any other
    /// type. It is the caller's for the same reason `outputs` is: it holds [`Value`]s rather
    /// than the `f32`s a caller wants back, so it can no longer *be* the output buffer, and a
    /// stack allocated here would be ADR 0364's million allocations again under another name.
    pub(crate) fn eval_into(&self, inputs: &[f32], outputs: &mut Vec<f32>, stack: &mut Vec<Value>) {
        // `pairs` refuses a `/Domain` longer than [`MAX_VALUES`], so the clipped inputs fit
        // in an array on the stack. The recursion below is bounded by [`MAX_STITCH_DEPTH`],
        // which puts a ceiling of eight of these on the stack at once.
        let mut clipped = [0.0f32; MAX_VALUES];
        let count = self.domain.len().min(MAX_VALUES);
        for (index, (low, high)) in self.domain.iter().enumerate().take(MAX_VALUES) {
            let value = inputs.get(index).copied().unwrap_or(*low);
            if let Some(slot) = clipped.get_mut(index) {
                *slot = clamp(value, *low, *high);
            }
        }
        let clipped = clipped.get(..count).unwrap_or_default();

        outputs.clear();
        match &self.kind {
            Kind::Sampled(sampled) => sampled.eval_into(clipped, outputs),
            Kind::Exponential { c0, c1, exponent } => {
                let x = clipped.first().copied().unwrap_or(0.0);
                let factor = if (*exponent - 1.0).abs() < f32::EPSILON {
                    x
                } else {
                    x.powf(*exponent)
                };
                outputs.extend(
                    c0.iter()
                        .zip(c1.iter())
                        .map(|(start, end)| start + factor * (end - start)),
                );
            }
            Kind::Stitching {
                functions,
                bounds,
                encode,
            } => self.eval_stitching_into(functions, bounds, encode, clipped, outputs, stack),
            // §7.10.5.3: "The input variables shall constitute the initial operand stack; the
            // items remaining on the operand stack after execution of the function shall be the
            // output variables." The inputs arrive as real numbers — they are a `/Domain`'s
            // clipped coordinates — and what is left is read back as numbers, which the same
            // clause requires: "It shall be an error … for any of them to be objects other than
            // numbers." A boolean left there is that error, and the subset has no way to raise
            // one, so it is read as the 1 or 0 it stands for rather than dropped: dropping it
            // would change how many outputs the function has, which is the *other* half of the
            // same sentence.
            // The two `extend`s are the cheapest form of this measured: a `reserve` and a `push`
            // loop, which is what suggests itself, costs 10 million instructions more on
            // `function_based_shading.pdf` because each of these iterators is `TrustedLen` and
            // reserves once already (ADR 0371).
            Kind::PostScript(program) => {
                stack.clear();
                stack.extend(clipped.iter().map(|input| Value::Real(*input)));
                evaluate_postscript(program, stack);
                outputs.extend(stack.iter().map(|value| value.number()));
            }
        }

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

    /// This function as the flat program a device evaluates, when it is one a device may
    /// stand in for over `region`.
    ///
    /// **The whole point of the conditions is that both readings answer the same question.**
    /// A device handed this list evaluates it at a point and clips the result to the `Range`
    /// the caller states beside it; [`Self::eval_into`] does two more things first, and each
    /// is a condition here rather than something the device is trusted to reproduce:
    ///
    /// - **§7.10.1 clips the inputs to the `/Domain`** — "[i]nput values passed to the
    ///   function shall be clipped to the domain" — and a device evaluating a §8.7.4.5.2 type
    ///   1 shading is handed the *shading's* domain rectangle, which §8.7.4.5.2 makes a
    ///   *region* rather than a clamp. Where this function's own `/Domain` contains that
    ///   rectangle the clip is the identity over every point the device will ask about, and
    ///   the two agree; where it does not, one path would fold a strip of the rectangle onto
    ///   its edge and the other would not, so there is no program to hand over.
    /// - **`/Range` is required for a type 4 function** (§7.10.5.3) and this returns nothing
    ///   without one, because the caller has to state it and a device clipping to a range
    ///   nobody declared would be a bound this project invented.
    ///
    /// `region` is `[x_min, x_max, y_min, y_max]`, Table 78's own order.
    #[must_use]
    pub fn device_program(&self, region: [f32; 4]) -> Option<Arc<[ProgramStep]>> {
        let Kind::PostScript(program) = &self.kind else {
            return None;
        };
        self.range.as_ref()?;
        // Two inputs, both clipped, and the clip has to be a no-op over the whole rectangle.
        let [(x_low, x_high), (y_low, y_high)] =
            <[(f32, f32); 2]>::try_from(self.domain.as_slice()).ok()?;
        let [left, right, bottom, top] = region;
        if x_low > left.min(right)
            || x_high < left.max(right)
            || y_low > bottom.min(top)
            || y_high < bottom.max(top)
        {
            return None;
        }
        program
            .iter()
            .map(device_step)
            .collect::<Option<Vec<_>>>()
            .map(Arc::from)
    }

    /// §7.10.1's `/Range`, as `[min, max]` per output component — `None` where the function
    /// declares none, which every type but 0 and 4 is allowed to do.
    #[must_use]
    pub fn range_bounds(&self) -> Option<&[(f32, f32)]> {
        self.range.as_deref()
    }

    /// Selects the sub-function covering an input and re-maps the input onto its domain.
    ///
    /// The sub-function writes into the same buffer, so a chain of stitched functions costs
    /// one allocation for the whole chain rather than one per link.
    fn eval_stitching_into(
        &self,
        functions: &[Function],
        bounds: &[f32],
        encode: &[(f32, f32)],
        clipped: &[f32],
        outputs: &mut Vec<f32>,
        stack: &mut Vec<Value>,
    ) {
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

        // No sub-function at that index leaves the buffer as the caller cleared it, which is
        // the empty answer the allocating form returned.
        if let Some(function) = functions.get(index) {
            function.eval_into(&[mapped], outputs, stack);
        }
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

        holds_the_sample_array(&data, total, bits)?;

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

/// Whether a Type 0 function's stream is long enough for the array it describes.
///
/// ISO 32000-2 §7.10.2:
///
/// > The stream data shall be long enough to contain the entire sample array, as indicated by
/// > Size , Range , and BitsPerSample ; see 7.3.8.2, "Stream extent".
///
/// Without it the bit reader answers 0 for every sample the stream does not hold — a value nobody
/// wrote, mapped through `/Decode` and interpolated into the samples beside it, so a tint
/// transform or a shading is evaluated over a function the file never carried. That is the
/// substitutive half of trap 5's test: a short sample table does not draw part of a gradient, it
/// draws a different one. So the function is refused and its caller reports it, where §7.8.2's
/// content stream keeps its prefix (ADR 0343) because a prefix of *that* is a shorter sequence of
/// the same kind.
///
/// `total` is Size × Range's pairs, which is the sample count the clause names.
///
/// # Errors
///
/// [`FunctionError::Malformed`], naming both numbers.
fn holds_the_sample_array(data: &[u8], total: usize, bits: u32) -> Result<(), FunctionError> {
    let need = total
        .checked_mul(bits as usize)
        .map(|width| width.div_ceil(8))
        .ok_or_else(|| FunctionError::Malformed {
            detail: "the sample array overflows".to_owned(),
        })?;
    if data.len() < need {
        return Err(FunctionError::Malformed {
            detail: format!(
                "the sample array needs {need} bytes and the stream holds {}",
                data.len()
            ),
        });
    }
    Ok(())
}

impl Sampled {
    /// Evaluates by multilinear interpolation between the surrounding grid samples.
    ///
    /// Writes into the caller's buffer for [`Function::eval_into`]'s reason: this is called
    /// once per cell of a device-resolution grid, and the three vectors it used to allocate
    /// were three allocations per cell.
    fn eval_into(&self, inputs: &[f32], out: &mut Vec<f32>) {
        // The grid position each input falls at, split into the sample below it and the
        // fraction beyond that sample. `numbers` refuses a `/Size` longer than
        // [`MAX_VALUES`], so both fit in arrays on the stack.
        let mut base = [0usize; MAX_VALUES];
        let mut fraction = [0.0f32; MAX_VALUES];
        for (index, count) in self.size.iter().enumerate().take(MAX_VALUES) {
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
            if let Some(slot) = base.get_mut(index) {
                *slot = floor.min(count.saturating_sub(1));
            }
            if let Some(slot) = fraction.get_mut(index) {
                *slot = position - position.floor();
            }
        }

        out.clear();
        out.resize(self.outputs, 0.0f32);

        // Interpolate over the 2^dimensions corner samples. Bounded because a function
        // with many inputs would already have been refused by the sample limit.
        let corners = 1usize.checked_shl(u32::try_from(self.size.len()).unwrap_or(0));
        let Some(corners) = corners else {
            return;
        };

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
            for (component, value) in out.iter_mut().enumerate() {
                let at = offset
                    .saturating_mul(self.outputs)
                    .saturating_add(component);
                *value += weight * self.samples.get(at).copied().unwrap_or(0.0);
            }
        }
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

/// One value on a type 4 function's operand stack.
///
/// ISO 32000-2 §7.10.5.1 states the whole of what a type 4 program computes with, and it is
/// three types rather than one:
///
/// > This subset is comprised of the following PostScript language features: … Expressions
/// > involving only integers, real numbers, and boolean values
///
/// **This stack held `f32` until the five-hundred-and-thirty-sixth session**, and Annex B's own
/// operand columns are what that costs. §B.3 types `eq` and `ne` as `any 1 any 2 … bool` — every
/// object, so a boolean *is* an operand there and the operator has to decide equality across the
/// types — while it types `gt`, `ge`, `lt` and `le` as `num 1 num 2`, `and`, `or`, `xor` and
/// `not` as `bool | int`, and `bitshift` as `int 1 shift`. A stack of numbers cannot tell those
/// apart: with a boolean stored as `1.0`, `true 1 eq` answered *true*, which is a colour decided
/// by a type confusion. The quorra team found that in their own device-side evaluator by running
/// this tree's corpus against it and reported that ours had the same shape
/// (`doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 5); ADR 0371 is this side's.
///
/// # Where a type is not what the operator asks for
///
/// §7.10.5.1's subset admits no value meaning *error*, so an operand of a type Annex B's line
/// does not admit cannot be refused the way PostScript refuses it. **The policy, stated once and
/// applied everywhere:** such an operand is *converted* by the reading that loses least — a
/// boolean is the 1 or 0 it stands for where a number is wanted ([`Value::number`]), a number is
/// false exactly when it is zero where a boolean is wanted ([`Value::truth`]), and a real is
/// truncated where an integer is wanted ([`Value::integer`]). The one operator this does not
/// touch is `eq`/`ne`, because there is nothing to convert: `any 1 any 2` admits both types
/// already, and the operator's own answer across them is that they are never equal.
///
/// One of those three directions is not a choice at all, and it is worth separating out: §7.3.3
/// states that
///
/// > Wherever a real number is expected, an integer may be used instead.
///
/// so an integer under `sin` or `div` is an ordinary operand rather than a tolerance. The same
/// clause's next sentence is why the other two directions are error cases and not readings — "A
/// real number shall not be present when an integer is expected" — and a file that does it
/// anyway is a file this viewer still has to draw.
///
/// The alternative — answering the zero of the operator's result type, so that `true 0 gt` were
/// `false` rather than `1 > 0` — was considered and declined for two reasons. It puts a second
/// rule beside the one `div` by zero already follows, and it replaces an answer that is a
/// function of the operands with a constant, which is the ground ADR 0369 gave for `bitshift`'s
/// width and `round`'s tie.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// An integer, which is what `idiv`, `mod`, `bitshift` and the bitwise operators are on.
    ///
    /// Held as `i32`, which is a width ISO 32000-2 does not state and this project therefore
    /// had to choose. ADR 0369 chose the answers that do *not* depend on one — `bitshift` fills
    /// from the sign rather than from a register's left edge — and those are unchanged at any
    /// width. What the choice decides is where an integer stops being one: a sum past 2³¹ becomes
    /// a real here rather than wrapping, which is the same direction the `f32` stack always went
    /// and is nearer to Annex C's "Integer values (such as object numbers) can often be expressed
    /// within 32 bits" than a 64-bit register would be. It also keeps a [`Value`] in eight bytes,
    /// which ADR 0371 measured: the operand stack is copied once per device pixel of a shading.
    Integer(i32),
    /// A real number, which every input is and most arithmetic produces.
    Real(f32),
    /// A boolean, which only `true`, `false` and the relational and boolean operators produce.
    Boolean(bool),
}

impl Value {
    /// The number this value denotes, for an operator Annex B types `num`.
    ///
    /// This is also how a value left on the stack is read back as an output. §7.10.5.3 requires
    /// the outputs to be numbers — "It shall be an error … for any of them to be objects other
    /// than numbers" — and the subset cannot raise that error, so a boolean output is the 1 or 0
    /// it stands for rather than a dropped operand.
    ///
    /// The real arm is written first here and in every match below it, because a type 4
    /// function's inputs are reals and most of what a program computes from them stays one: the
    /// census beside this crate found 3 328 of 7 360 programs writing no integer literal at all.
    /// ADR 0371 measured what the order is worth.
    fn number(self) -> f32 {
        match self {
            Self::Real(value) => value,
            #[expect(
                clippy::cast_precision_loss,
                reason = "an integer beyond f32's exact range is already meaningless as a colour"
            )]
            Self::Integer(value) => value as f32,
            Self::Boolean(value) => {
                if value {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// The integer this value denotes, for an operator Annex B types `int`.
    fn integer(self) -> i32 {
        match self {
            Self::Real(value) => to_integer(value),
            Self::Integer(value) => value,
            Self::Boolean(value) => i32::from(value),
        }
    }

    /// The integer this value *is*, or `None` for a real.
    ///
    /// The discriminator for `add`, `sub` and `mul`, whose §B.2 line — `num 1 num 2 … sum` —
    /// says nothing about the result's type, and for `and`, `or` and `xor`, whose §B.3 line
    /// makes the result's type the operands'. A boolean answers here because the policy above
    /// converts it to an integer rather than to a real: 1 and 0 are exact either way, and an
    /// integer is the type that keeps a later `not` meaning what §B.3's second column says.
    fn as_integer(self) -> Option<i32> {
        match self {
            Self::Real(_) => None,
            Self::Integer(value) => Some(value),
            Self::Boolean(value) => Some(i32::from(value)),
        }
    }

    /// The truth this value denotes, for `if` and `ifelse`, which §B.4 types `bool`.
    fn truth(self) -> bool {
        match self {
            Self::Boolean(value) => value,
            Self::Real(value) => value != 0.0,
            Self::Integer(value) => value != 0,
        }
    }

    /// Whether two values are the operands `eq` compares equal.
    ///
    /// §B.3 types `eq` as `any 1 any 2 … bool`, so it is defined *across* the subset's three
    /// types rather than on numbers alone, and a boolean and a number are two different objects
    /// however they are stored. Two numbers compare by value whatever their own types are, which
    /// is why the comparison is made in `f64`: an integer and a real are equal when they stand
    /// for the same number, and narrowing the integer first would make two distinct large
    /// integers equal.
    #[expect(
        clippy::float_cmp,
        reason = "the operator being implemented is exact equality; ADR 0369 priced the margin \
                  clippy suggests here and removed it"
    )]
    fn equals(self, other: Self) -> bool {
        match (self, other) {
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Boolean(_), _) | (_, Self::Boolean(_)) => false,
            (a, b) => a.as_f64() == b.as_f64(),
        }
    }

    /// The value as an `f64`, which holds every `i32` and every `f32` exactly — so two numbers
    /// of different types are compared without either being rounded first.
    fn as_f64(self) -> f64 {
        match self {
            Self::Real(value) => f64::from(value),
            Self::Integer(value) => f64::from(value),
            Self::Boolean(value) => f64::from(u8::from(value)),
        }
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
    Push(Value),
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

/// One [`Instruction`] as the [`ProgramStep`] a device is handed, or `None` where it cannot be
/// expressed as one.
///
/// The two forms are one-to-one but for three things, and each is written out rather than
/// assumed:
///
/// - **A literal's type moves from the value into the instruction.** [`Value`] carries it here
///   because the evaluator's Annex B coercions need it at run time; a device's code generator
///   needs it before it writes a line, because §7.10.5.1's `not`, `and`, `or` and `xor` are
///   different operations on a boolean and on an integer (ADR 0371).
/// - **`true` and `false` are operators here and literals there.** Table 42 lists them as
///   operators because PostScript source has no other way to write a boolean down; a compiled
///   list does, so they lower to [`ProgramStep::PushBool`] and the device's vocabulary is one
///   variant shorter for it.
/// - **A jump target narrows to a `u32`.** The compiler's bound is [`MAX_INSTRUCTIONS`], well
///   inside it, so the conversion cannot fail on anything this crate produced — and it is
///   written fallibly anyway rather than asserted in a comment, because the day it can fail is
///   the day a bound moved and nobody looked here.
///
/// **The match has no wildcard arm**, which is the point of writing it out: a Table 42 operator
/// added to [`Operator`] stops this compiling instead of silently reaching a device as nothing.
fn device_step(instruction: &Instruction) -> Option<ProgramStep> {
    let operator = |operator| Some(ProgramStep::Operator(operator));
    match instruction {
        Instruction::Push(Value::Integer(value)) => Some(ProgramStep::PushInt(*value)),
        Instruction::Push(Value::Real(value)) => Some(ProgramStep::PushReal(*value)),
        Instruction::Push(Value::Boolean(value)) => Some(ProgramStep::PushBool(*value)),
        Instruction::JumpUnless(target) => u32::try_from(*target)
            .ok()
            .map(|target| ProgramStep::JumpUnless { target }),
        Instruction::Jump(target) => u32::try_from(*target)
            .ok()
            .map(|target| ProgramStep::Jump { target }),
        Instruction::Operator(Operator::True) => Some(ProgramStep::PushBool(true)),
        Instruction::Operator(Operator::False) => Some(ProgramStep::PushBool(false)),
        Instruction::Operator(Operator::Abs) => operator(ProgramOperator::Abs),
        Instruction::Operator(Operator::Add) => operator(ProgramOperator::Add),
        Instruction::Operator(Operator::Atan) => operator(ProgramOperator::Atan),
        Instruction::Operator(Operator::Ceiling) => operator(ProgramOperator::Ceiling),
        Instruction::Operator(Operator::Cos) => operator(ProgramOperator::Cos),
        Instruction::Operator(Operator::Cvi) => operator(ProgramOperator::Cvi),
        Instruction::Operator(Operator::Cvr) => operator(ProgramOperator::Cvr),
        Instruction::Operator(Operator::Div) => operator(ProgramOperator::Div),
        Instruction::Operator(Operator::Exp) => operator(ProgramOperator::Exp),
        Instruction::Operator(Operator::Floor) => operator(ProgramOperator::Floor),
        Instruction::Operator(Operator::Idiv) => operator(ProgramOperator::Idiv),
        Instruction::Operator(Operator::Ln) => operator(ProgramOperator::Ln),
        Instruction::Operator(Operator::Log) => operator(ProgramOperator::Log),
        Instruction::Operator(Operator::Mod) => operator(ProgramOperator::Mod),
        Instruction::Operator(Operator::Mul) => operator(ProgramOperator::Mul),
        Instruction::Operator(Operator::Neg) => operator(ProgramOperator::Neg),
        Instruction::Operator(Operator::Round) => operator(ProgramOperator::Round),
        Instruction::Operator(Operator::Sin) => operator(ProgramOperator::Sin),
        Instruction::Operator(Operator::Sqrt) => operator(ProgramOperator::Sqrt),
        Instruction::Operator(Operator::Sub) => operator(ProgramOperator::Sub),
        Instruction::Operator(Operator::Truncate) => operator(ProgramOperator::Truncate),
        Instruction::Operator(Operator::And) => operator(ProgramOperator::And),
        Instruction::Operator(Operator::Bitshift) => operator(ProgramOperator::Bitshift),
        Instruction::Operator(Operator::Eq) => operator(ProgramOperator::Eq),
        Instruction::Operator(Operator::Ge) => operator(ProgramOperator::Ge),
        Instruction::Operator(Operator::Gt) => operator(ProgramOperator::Gt),
        Instruction::Operator(Operator::Le) => operator(ProgramOperator::Le),
        Instruction::Operator(Operator::Lt) => operator(ProgramOperator::Lt),
        Instruction::Operator(Operator::Ne) => operator(ProgramOperator::Ne),
        Instruction::Operator(Operator::Not) => operator(ProgramOperator::Not),
        Instruction::Operator(Operator::Or) => operator(ProgramOperator::Or),
        Instruction::Operator(Operator::Xor) => operator(ProgramOperator::Xor),
        Instruction::Operator(Operator::Copy) => operator(ProgramOperator::Copy),
        Instruction::Operator(Operator::Dup) => operator(ProgramOperator::Dup),
        Instruction::Operator(Operator::Exch) => operator(ProgramOperator::Exch),
        Instruction::Operator(Operator::Index) => operator(ProgramOperator::Index),
        Instruction::Operator(Operator::Pop) => operator(ProgramOperator::Pop),
        Instruction::Operator(Operator::Roll) => operator(ProgramOperator::Roll),
    }
}

/// The program with §7.2.4's comments cut out, one line at a time.
///
/// ISO 32000-2 §7.2.4 defines what a comment is, and its extent is the whole point:
///
/// > The comment consists of all characters after the PERCENT SIGN and up to but not including
/// > the end-of-the-line marker.
///
/// > PDF processors shall treat comments as single white-space characters for the purposes of
/// > lexical conversion … That is, a comment separates the token preceding it from the one
/// > following it.
///
/// So each line is cut at its first PERCENT SIGN and a LINE FEED is written in the comment's
/// place: dropping the comment without leaving white space behind would join the token before
/// it to the one after, which is the same defect one step smaller.
///
/// **Cutting at the first PERCENT SIGN is safe here and would not be in PostScript at large**,
/// and §7.10.5.1's own list of what the subset contains is why — it admits comments and denies
/// the one construction that could hide a PERCENT SIGN from this rule:
///
/// > This subset is comprised of the following PostScript language features: … No composite
/// > data structures (such as strings or arrays)
///
/// With no string literals in the language, no PERCENT SIGN in a type 4 program can be
/// anything but the start of a comment. §7.2.3's end-of-line marker is a CARRIAGE RETURN, a
/// LINE FEED or the two together, so the cut ends at either byte.
///
/// This is done *before* the program is split on white space, and that order is the whole fix:
/// splitting first destroys the line boundary the clause defines a comment by, and every
/// approximation available afterwards is wrong. Skipping one token after the PERCENT SIGN —
/// what this code did until the five-hundred-and-twenty-sixth session — refuses a program
/// loudly when the word after the sign is not an operator (`% BBP Math for Pi` compiled `Math`)
/// and, worse, *silently compiles the rest of the comment's words as instructions* when they
/// happen to be numbers or operator names.
fn without_comments(program: &str) -> String {
    let mut out = String::with_capacity(program.len());
    for line in program.split(['\r', '\n']) {
        out.push_str(
            line.find('%')
                .map_or(line, |at| line.get(..at).unwrap_or_default()),
        );
        out.push('\n');
    }
    out
}

/// Compiles a type 4 program into a flat instruction list.
///
/// ISO 32000-2 §7.10.5.2 states the syntax the tokens are read under: the operand syntax
/// "shall follow PDF conventions rather than PostScript language conventions", and
///
/// > The entire code stream defining the function shall be enclosed in braces { }
///
/// with braces also delimiting `if` and `ifelse`'s expressions. Braces are therefore spaced
/// apart from their neighbours before the split, since §7.2.3 makes them delimiters rather
/// than white space — `{2 mul}` is three tokens.
fn compile_postscript(source: &[u8]) -> Result<Vec<Instruction>, FunctionError> {
    /// Bounds the program, so a hostile stream cannot compile to an enormous list.
    const MAX_INSTRUCTIONS: usize = 1 << 16;

    let text = String::from_utf8_lossy(source);
    let code = without_comments(&text);
    let spaced = code.replace('{', " { ").replace('}', " } ");
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

/// Compiles one token: a literal, or one of Table 42's operators.
///
/// **Which literals are integers is a question ISO 32000-2 answers itself**, and it is the one
/// place §7.10.5.2 does not defer: "The operand syntax for Type 4 functions shall follow PDF
/// conventions rather than PostScript language conventions." §7.3.2 and §7.3.3 are those
/// conventions — an integer object is written as digits with an optional sign, and a real
/// carries a PERIOD — so `1` is an integer and `1.0` is a real, and [`Value`] can tell `not`'s
/// two meanings apart on the strength of what the file wrote.
///
/// Two edges are choices rather than readings, and both fall out of trying `i32` first. §7.3.3
/// forbids a *writer* the PostScript exponential form, so `1e-8` is not a PDF number and this
/// reads it as a real rather than refusing the whole function over a lexical detail with one
/// obvious meaning. And an integer too large for an `i32` becomes a real, which is what §7.3.3's
/// own sentence about the limits of the machine leaves a processor to do.
fn compile_token(token: &str) -> Result<Instruction, FunctionError> {
    if let Ok(value) = token.parse::<i32>() {
        return Ok(Instruction::Push(Value::Integer(value)));
    }
    if let Ok(value) = token.parse::<f32>() {
        return Ok(Instruction::Push(Value::Real(value)));
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

/// Runs a compiled type 4 program on a stack the caller supplies, already holding the inputs.
///
/// The stack is bounded and the instruction list has no backward jumps, so this always
/// terminates. A program that underflows the stack yields whatever it managed to compute
/// rather than failing: a shading with one bad colour is better than no shading.
///
/// The caller owns the stack so that a grid of a million cells allocates once rather than once
/// per cell. See [`Function::eval_into`].
fn evaluate_postscript(program: &[Instruction], stack: &mut Vec<Value>) {
    let mut at = 0usize;
    let mut steps = 0usize;
    // Forward-only jumps make the ceiling unreachable, and it costs one comparison to be certain
    // rather than to argue about it. Computed once rather than per instruction: the program does
    // not change length while it runs, and this loop is entered once per device pixel of a
    // shading (ADR 0339), where it was 1.6% of the whole page.
    let ceiling = program.len().saturating_mul(2).saturating_add(16);

    while let Some(instruction) = program.get(at) {
        steps = steps.saturating_add(1);
        if steps > ceiling {
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
            // §B.4 types `if` and `ifelse` as taking a `bool`, and a program that pushes a
            // number instead is the conversion [`Value`] states rather than a refusal: zero is
            // false and everything else is true. An empty stack answers `0` (see `pop`), which
            // is false, so a program that lost its condition takes the branch it would have
            // taken before this stack had types.
            Instruction::JumpUnless(target) => {
                if !stack.pop().unwrap_or(EMPTY_STACK).truth() {
                    at = *target;
                }
            }
            Instruction::Operator(operator) => apply_operator(*operator, stack),
        }
    }
}

/// Applies one operator to the stack.
///
/// # Where these semantics come from, and where the standard stops
///
/// ISO 32000-2 §7.10.5.2 lists Table 42's operators and then hands their meaning to a
/// different document:
///
/// > The PostScript Language Reference, Third Edition shall define the semantics of these
/// > operators and all other syntax rules of the PostScript language. Although the semantics
/// > are those of the corresponding PostScript language operators, a full PostScript language
/// > compatible interpreter is not required.
///
/// **That deferral is normative, and this project does not hold the document deferred to.**
/// `CLAUDE.md` principle 5 forbids quoting a document one does not have as though one did, so
/// nothing below claims PLRM3's words; what it does instead is the rule that file states for a
/// clause defining nothing, applied to a clause that defines something *elsewhere*. Where the
/// deferral is the only answer available, the reading is written down **as a choice**, with
/// what it rests on, rather than presented as derived.
///
/// The standard's own summary is Annex B, which §7.10.5.3 points at:
///
/// > Annex B, "Operators in Type 4 Functions", contains a summary of these operators.
///
/// Annex B is *informative* and gives each operator one line. Where that line settles a
/// question it is quoted under §B.2 or §B.3 beside the arm it settles; where it does not, the
/// arm says so and says what was chosen instead.
///
/// Two arms exist as they do because the tolerant reading was wrong, and both were found by the
/// quorra team reading this file to build a device-side evaluator —
/// `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` section 6, ADR 0369. `round` is [`round_to_greater`].
/// `eq` and `ne` are §B.3's:
///
/// > Test equal
///
/// > Test not equal
///
/// which name a relation and admit no tolerance; the arms below say what the tolerance that
/// stood there actually did.
///
/// # What the audit of the rest of Table 42 turned on
///
/// Three arms are choices rather than readings, because the standard's line does not reach them
/// and the deferral cannot be quoted. Each is stated where it is made: `bitshift`'s width for a
/// right shift of a negative value; `div`, `idiv` and `mod` by zero and `ln`, `log` and `sqrt`
/// outside their domains, where PostScript raises an error the subset has no way to express; and
/// a pop from an empty operand stack, which is [`EMPTY_STACK`].
///
/// # What a typed stack decides, and what it left alone
///
/// The stack holds [`Value`]s rather than `f32`s since the five-hundred-and-thirty-sixth session,
/// which is what makes Annex B's operand and result columns implementable rather than decorative:
/// `not` is one's complement on an integer and negation on a boolean, `and`, `or` and `xor`
/// answer in the type they were given, `cvi` and `cvr` are conversions rather than a truncation
/// and a no-op, and `eq` answers §B.3's `any 1 any 2` *across* the three types instead of
/// comparing a boolean as though it were the number 1. [`Value`] states the whole of what happens
/// where an operand's type is not the one its line asks for, and ADR 0371 is the argument.
///
/// What it left alone is worth as much: `and`, `or` and `xor` still agree with the arithmetic
/// they had, because over `{0, 1}` the bitwise operation is the logical one — the typing changes
/// the *type* of their answer rather than its value — and every arm that Annex B types `num`
/// answers exactly what it answered before.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per PostScript operator reads better as a single table"
)]
fn apply_operator(operator: Operator, stack: &mut Vec<Value>) {
    /// Pops one value, or [`EMPTY_STACK`] when the stack has underflowed.
    fn pop(stack: &mut Vec<Value>) -> Value {
        stack.pop().unwrap_or(EMPTY_STACK)
    }
    /// A one-operand operator, applied where its operand already is.
    ///
    /// A pop and a push move the stack's length twice and write it back twice for an answer that
    /// lands in the slot the operand came out of; this writes it there. It is the same operator —
    /// an underflow still answers from [`EMPTY_STACK`] and still leaves one value behind — and on
    /// the type 4 shadings of `function_based_shading.pdf`, whose nine programs are a handful of
    /// operators each, this and [`binary`] together are what keep a typed stack from costing
    /// instructions against ADR 0364's measurement. ADR 0371 has the table.
    fn unary(stack: &mut Vec<Value>, apply: impl Fn(Value) -> Value) {
        match stack.last_mut() {
            Some(slot) => *slot = apply(*slot),
            None => stack.push(apply(EMPTY_STACK)),
        }
    }
    /// A two-operand operator, applied where its *first* operand already is.
    ///
    /// §B.2's and §B.3's two-operand lines all leave one value behind, so the second operand's
    /// slot is the one that goes; see [`unary`] for why that is worth writing out.
    fn binary(stack: &mut Vec<Value>, apply: impl Fn(Value, Value) -> Value) {
        let b = pop(stack);
        match stack.last_mut() {
            Some(slot) => *slot = apply(*slot, b),
            None => stack.push(apply(EMPTY_STACK, b)),
        }
    }
    /// A two-operand arithmetic operator, in the type §B.2's line leaves to the operands.
    ///
    /// `add`, `sub` and `mul` are typed `num 1 num 2 … sum`, and a sum of what type is not
    /// stated. **The choice is that two integers make an integer and anything else makes a
    /// real**, on the ground that it is the only rule under which `2 3 add not` and `5 not`
    /// answer alike — a type that arithmetic threw away would make `not`, `and`, `or`, `xor`,
    /// `idiv`, `mod` and `bitshift` mean one thing on a literal and another on a value computed
    /// from two of them. An integer result that will not fit becomes a real, which keeps the
    /// magnitude approximately right where wrapping would produce a plausible wrong number.
    fn arithmetic2(
        a: Value,
        b: Value,
        integers: impl Fn(i32, i32) -> Option<i32>,
        reals: impl Fn(f32, f32) -> f32,
    ) -> Value {
        // Matched on the *pair* rather than through two `as_integer`s, because the arms are one
        // test each that way and three of the four are the arithmetic a shading spends its day
        // in: two reals, and a real meeting an integer literal — `x 4 mul` — from either side.
        // Only the last arm, where neither operand is a real, can answer an integer at all.
        // ADR 0371 has what this shape is worth against the straightforward one.
        match (a, b) {
            (Value::Real(x), Value::Real(y)) => Value::Real(reals(x, y)),
            (Value::Real(x), other) => Value::Real(reals(x, other.number())),
            (other, Value::Real(y)) => Value::Real(reals(other.number(), y)),
            (x, y) => match (x.as_integer(), y.as_integer()) {
                (Some(x), Some(y)) => integers(x, y).map_or_else(
                    || Value::Real(reals(a.number(), b.number())),
                    Value::Integer,
                ),
                _ => Value::Real(reals(a.number(), b.number())),
            },
        }
    }
    /// A one-operand arithmetic operator §B.2 types `num 1 … num 2`, which carries its type
    /// through: a value that was an integer is still one after `abs`, `floor` or `round`.
    fn arithmetic1(
        a: Value,
        integers: impl Fn(i32) -> Option<i32>,
        reals: impl Fn(f32) -> f32,
    ) -> Value {
        if let Value::Real(x) = a {
            return Value::Real(reals(x));
        }
        a.as_integer().map_or_else(
            || Value::Real(reals(a.number())),
            |x| integers(x).map_or_else(|| Value::Real(reals(a.number())), Value::Integer),
        )
    }
    /// `and`, `or` and `xor`, whose §B.3 line — `bool 1 | int 1 bool 2 | int 2 … bool 3 | int 3`
    /// — makes the result's type the operands'. Two booleans are the logical operator and
    /// anything else is the bitwise one, which is the least-loss direction of [`Value`]'s
    /// conversion policy: a boolean is exactly 1 or 0 as an integer, where an integer read as a
    /// boolean would lose every bit above the first.
    fn logic(
        a: Value,
        b: Value,
        integers: impl Fn(i32, i32) -> i32,
        booleans: impl Fn(bool, bool) -> bool,
    ) -> Value {
        match (a, b) {
            (Value::Boolean(x), Value::Boolean(y)) => Value::Boolean(booleans(x, y)),
            _ => Value::Integer(integers(a.integer(), b.integer())),
        }
    }

    match operator {
        Operator::Abs => unary(stack, |a| arithmetic1(a, i32::checked_abs, f32::abs)),
        Operator::Add => binary(stack, |a, b| {
            arithmetic2(a, b, i32::checked_add, |x, y| x + y)
        }),
        // §B.2: "Return arc tangent of num / den in degrees" — a *quotient* of two operands
        // rather than one ratio, which is why `atan2` is the right primitive and a `num / den`
        // fed to `atan` would be wrong: the quotient loses the quadrant, so `-1 -1 atan` and
        // `1 1 atan` would answer alike where the circle puts them 180° apart. The degrees run
        // over the whole circle rather than over `atan`'s half of it, so a negative answer is
        // brought up by a turn.
        Operator::Atan => binary(stack, |num, den| {
            let mut degrees = num.number().atan2(den.number()).to_degrees();
            if degrees < 0.0 {
                degrees += 360.0;
            }
            Value::Real(degrees)
        }),
        Operator::Ceiling => unary(stack, |a| arithmetic1(a, Some, f32::ceil)),
        Operator::Cos => unary(stack, |a| Value::Real(a.number().to_radians().cos())),
        // §B.2 gives these two a `num` operand and different result columns — `cvi` answers
        // `int` and `cvr` answers `real` — so they are conversions rather than arithmetic, and
        // an untyped stack could implement neither. `cvi` truncates toward zero, since it is
        // `truncate`'s arithmetic with a type change on top: `-1.5 cvi` is `-1` where `floor`
        // answers `-2`. `cvr` on an integer is the widening §7.3.3 already permits everywhere.
        Operator::Cvi => unary(stack, |a| Value::Integer(a.integer())),
        Operator::Cvr => unary(stack, |a| Value::Real(a.number())),
        // §B.2's `truncate` is typed `num 1 … num 2` rather than `num … int`, so unlike `cvi` it
        // keeps the type it was given and only removes the fraction. **The two were one arm
        // until this stack had types**, which was the honest reading of a stack that could not
        // tell them apart and is not the honest reading of one that can.
        Operator::Truncate => unary(stack, |a| arithmetic1(a, Some, f32::trunc)),
        // A quotient by zero, and below it `idiv`, `mod`, `ln`, `log` and `sqrt` outside their
        // domains: PostScript raises an error, and §7.10.5.1's subset has no way to express one
        // — "Expressions involving only integers, real numbers, and boolean values" is the
        // whole of what a program may leave on the stack. So the choice here is `0`, and the
        // reason it is not an infinity is that an infinity does not stay here: it becomes a
        // colour component, then a coordinate, and geometry built from one is unpredictable
        // rather than merely wrong. `/Range` would clamp it (§7.10.5.3 requires one), but the
        // value passes through the rest of the program first.
        //
        // The result is a real whatever the operands were, and §B.2 is the evidence: the `div`
        // row answers a plain `quotient` where the `idiv` row directly below it answers one
        // "as an integer", which is a distinction the annex would not draw if `div` could
        // answer an integer too.
        Operator::Div => binary(stack, |a, b| {
            let (a, b) = (a.number(), b.number());
            Value::Real(if b == 0.0 { 0.0 } else { a / b })
        }),
        // §B.2: "Raise base to exponent power", the operands in that order, answering a `real`.
        // A negative base with a fractional exponent has no real answer and `powf` says so with
        // a `NaN`, which is the one place in this table a non-number is produced deliberately:
        // unlike an infinity it cannot become a plausible coordinate, and §7.10.5.3 requires a
        // `/Range`, whose clamp maps it to a bound before any caller sees it.
        Operator::Exp => binary(stack, |a, b| Value::Real(a.number().powf(b.number()))),
        Operator::Floor => unary(stack, |a| arithmetic1(a, Some, f32::floor)),
        // §B.2: "Return int 1 divided by int 2 as an integer", and `mod` below it "Return
        // remainder after dividing int 1 by int 2" — both typed on integers in and an integer
        // out. A real operand is what §7.3.3 calls a real number present where an integer is
        // expected, so it is truncated toward zero rather than refused ([`Value`]), which is
        // also what this arm did when the stack had no types: `-7 2 idiv` is `-3` rather than
        // `floor`'s `-4`.
        Operator::Idiv => binary(stack, |a, b| {
            Value::Integer(a.integer().checked_div(b.integer()).unwrap_or(0))
        }),
        // The remainder that goes with `idiv`'s truncated quotient is the one whose sign follows
        // the *dividend*, which is what Rust's `%` computes and what a Euclidean remainder does
        // not: `-7 2 mod` is `-1` here and `1` under `rem_euclid`. The pair has to agree, since
        // `a` is `b (a b idiv) mul (a b mod) add` in either convention only when both come from
        // the same one.
        Operator::Mod => binary(stack, |a, b| {
            Value::Integer(a.integer().checked_rem(b.integer()).unwrap_or(0))
        }),
        Operator::Ln => unary(stack, |a| {
            let a = a.number();
            Value::Real(if a > 0.0 { a.ln() } else { 0.0 })
        }),
        Operator::Log => unary(stack, |a| {
            let a = a.number();
            Value::Real(if a > 0.0 { a.log10() } else { 0.0 })
        }),
        Operator::Mul => binary(stack, |a, b| {
            arithmetic2(a, b, i32::checked_mul, |x, y| x * y)
        }),
        Operator::Neg => unary(stack, |a| arithmetic1(a, i32::checked_neg, |x| -x)),
        Operator::Round => unary(stack, |a| arithmetic1(a, Some, round_to_greater)),
        Operator::Sin => unary(stack, |a| Value::Real(a.number().to_radians().sin())),
        Operator::Sqrt => unary(stack, |a| {
            let a = a.number();
            Value::Real(if a > 0.0 { a.sqrt() } else { 0.0 })
        }),
        Operator::Sub => binary(stack, |a, b| {
            arithmetic2(a, b, i32::checked_sub, |x, y| x - y)
        }),
        // §B.3 types `and`, `or` and `xor` as taking `bool | int` and returning `bool | int` —
        // "Perform logical | bitwise and" — so each is two operators sharing a name, and
        // `logic` above is which one runs. **Their arithmetic did not change and could not**:
        // a boolean here used to be `1.0` or `0.0`, and over the set {0, 1} the bitwise
        // operation *is* the logical one. What changed is the type of the answer, which the
        // `not` below it can tell apart.
        Operator::And => binary(stack, |a, b| logic(a, b, |x, y| x & y, |x, y| x && y)),
        Operator::Or => binary(stack, |a, b| logic(a, b, |x, y| x | y, |x, y| x || y)),
        Operator::Xor => binary(stack, |a, b| logic(a, b, |x, y| x ^ y, |x, y| x != y)),
        // §B.3: "Perform bitwise shift of int 1 (positive is left)", which fixes the direction
        // and nothing else. A *right* shift of a negative value is where implementations part,
        // and it parts on a number the standard never states: the width of the integer. A shift
        // that fills from the left with zeros answers `2147483646` for `-4 -1 bitshift` at 32
        // bits and `9223372036854775806` at 64, so choosing that convention means choosing a
        // width, and ISO 32000-2 states one nowhere — Annex C's "Integer values (such as object
        // numbers) can often be expressed within 32 bits" is informative and is about object
        // numbers. §7.10.5.2 defers the rest to a document this project does not hold.
        //
        // **So the choice is the sign-preserving shift**, which is the only one of the two that
        // is a function of the *value* rather than of a width nobody stated: `-4 -1 bitshift` is
        // `-2` under it whatever the register is. It is a choice and not a reading, and the
        // instrument that says what it costs is `examples/type4_operator_census`, which counts
        // the programs in the corpora that reach `bitshift` at all — none of 7 360, at the
        // five-hundred-and-thirty-sixth session's run.
        //
        // A shift wider than the register is where that principle used to leak, and the last
        // line is what closes it: shifting right by more bits than an integer has leaves the
        // sign repeated, so it is `-1` for a negative value and `0` for a non-negative one
        // rather than `0` for both. Answering `0` for a negative value would have made the
        // answer depend on the width after all — `-8 -40 bitshift` was `-1` while the integer
        // was 64 bits wide and `0` when it became 32 — which is exactly what this arm's choice
        // says it will not do (ADR 0371).
        Operator::Bitshift => binary(stack, |value, shift| {
            let (value, shift) = (value.integer(), shift.integer());
            Value::Integer(if shift >= 0 {
                value
                    .checked_shl(u32::try_from(shift).unwrap_or(u32::MAX))
                    .unwrap_or(0)
            } else {
                shift
                    .checked_neg()
                    .and_then(|amount| u32::try_from(amount).ok())
                    .and_then(|amount| value.checked_shr(amount))
                    .unwrap_or(if value < 0 { -1 } else { 0 })
            })
        }),
        // §B.3: "Perform logical | bitwise not" — two operators wearing one name, like `and`
        // above, except that here the two *disagree*. Logical `not` of true is false; the one's
        // complement of the integer `1` is `-2`, and of `63` is `-64`. Which is meant depends on
        // the operand's type, and until the five-hundred-and-thirty-sixth session a compiled
        // literal had none, so this evaluator could implement only the logical one and `63 not`
        // answered `0`. It answers `-64` now, off the type §7.3.2 and §7.3.3 give the literal
        // `63` in the file itself.
        //
        // A real reaching here is the error §7.3.3 names — a real number present where an
        // integer is expected — and [`Value`]'s policy truncates it rather than refusing, so
        // `1.5 not` is `-2`.
        Operator::Not => unary(stack, |a| match a {
            Value::Boolean(value) => Value::Boolean(!value),
            other => Value::Integer(!other.integer()),
        }),
        // §B.3 gives `eq` one line — "Test equal" — and no tolerance, and types it `any 1 any 2`
        // rather than `num 1 num 2`. Both halves of that were wrong here in turn. The tolerance
        // was `f32::EPSILON` until the five-hundred-and-thirty-fourth session, which is not a
        // conservative reading of the deferral but a different operator: `f32::EPSILON` is the
        // gap between 1.0 and its successor, so near zero it made millions of distinct values
        // equal — every value under 1.2e-7 equalled every other, and equalled zero — while at
        // any magnitude above about 8.4 million it is smaller than one unit in the last place
        // and the comparison was exact anyway. It was loosest exactly where a type 4 program
        // tests a boundary.
        //
        // **And `any 1 any 2` is the half this round takes.** An operator defined on every
        // object has to answer across the types rather than through them, so a boolean is never
        // equal to a number — `true 1 eq` is false — and two numbers are equal when they stand
        // for the same number whether they were written `1` or `1.0`. [`Value::equals`] is the
        // whole of it. The quorra team found this in their own device-side evaluator by running
        // this tree's corpus against it and reported that ours had the same shape
        // (`doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 5).
        //
        // A `NaN` operand makes `eq` false and `ne` true, which is IEEE 754's answer rather
        // than a chosen one — and unreachable through `Function::eval`, whose §7.10.5.3
        // `/Range` clamp maps a `NaN` output to a bound before any caller sees it.
        Operator::Eq => binary(stack, |a, b| Value::Boolean(a.equals(b))),
        Operator::Ne => binary(stack, |a, b| Value::Boolean(!a.equals(b))),
        // §B.3 types these four `num 1 num 2 … bool`, so unlike `eq` they do not admit a
        // boolean at all and PostScript refuses one. The subset cannot express that refusal,
        // and [`Value`]'s policy therefore converts: `true 0 gt` compares 1 with 0 and is true.
        // That is a choice, it is the one quorra left with this tree
        // (`doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 3), and `doc/QUORRA_FEEDBACK.md`
        // section 26 is the answer sent back with its ground.
        Operator::Ge => binary(stack, |a, b| Value::Boolean(a.as_f64() >= b.as_f64())),
        Operator::Gt => binary(stack, |a, b| Value::Boolean(a.as_f64() > b.as_f64())),
        Operator::Le => binary(stack, |a, b| Value::Boolean(a.as_f64() <= b.as_f64())),
        Operator::Lt => binary(stack, |a, b| Value::Boolean(a.as_f64() < b.as_f64())),
        Operator::True => stack.push(Value::Boolean(true)),
        Operator::False => stack.push(Value::Boolean(false)),
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
        // §B.5's stack operators are typed `any` throughout, so each moves a [`Value`] without
        // reading it; only the count they take is a number, and it is an `int`.
        Operator::Copy => {
            let count = pop(stack).integer();
            if count <= 0 {
                return;
            }
            let count = usize::try_from(count).unwrap_or(MAX_STACK).min(MAX_STACK);
            let start = stack.len().saturating_sub(count);
            if stack.len().saturating_add(count) > MAX_STACK {
                return;
            }
            // `extend_from_within` copies the window in place, where a `to_vec` used to
            // stand: `copy` is the one operator that allocated, and the owner's `pi.pdf`
            // reaches it once per cell. The range cannot be out of bounds, because `start`
            // came from a `saturating_sub` on this same length.
            stack.extend_from_within(start..);
        }
        Operator::Index => {
            let n = pop(stack).integer();
            if n < 0 {
                stack.push(EMPTY_STACK);
                return;
            }
            let n = usize::try_from(n).unwrap_or(usize::MAX);
            let value = stack
                .len()
                .checked_sub(n.saturating_add(1))
                .and_then(|at| stack.get(at).copied())
                .unwrap_or(EMPTY_STACK);
            stack.push(value);
        }
        // §B.5: "Roll n elements up j times", where *up* is toward the top of the stack — which
        // is a rotation to the right of a window whose last element is the top. A negative `j`
        // rolls the other way and is not a different operation: `rem_euclid` turns it into the
        // rotation by `n - |j|` that §B.5's own `mod` in the result column describes.
        Operator::Roll => {
            let shift = pop(stack).integer();
            let count = pop(stack).integer();
            if count <= 0 {
                return;
            }
            let count = usize::try_from(count)
                .unwrap_or(usize::MAX)
                .min(stack.len());
            let start = stack.len().saturating_sub(count);
            let Some(window) = stack.get_mut(start..) else {
                return;
            };
            if count == 0 {
                return;
            }
            let count_i = i32::try_from(count).unwrap_or(1);
            let amount = shift.rem_euclid(count_i);
            let amount = usize::try_from(amount).unwrap_or(0);
            window.rotate_right(amount);
        }
    }
}

/// What a pop from an empty operand stack answers, and why it is an integer.
///
/// §7.10.5.3 makes a program that underflows malformed twice over — the inputs "shall constitute
/// the initial operand stack", and it is "an error for the number of remaining operands to differ
/// from the number of output variables specified by Range" — but §7.10.5.1's subset holds only
/// integers, reals and booleans, so there is no value that means *underflow*. Refusing the
/// program instead would refuse a document that draws: `doc/corpora-own/pi_seven_segment.pdf` is
/// hand-written, reads an empty stack in three places, and renders.
///
/// **So it is a value, and this round decided which one.** `unwrap_or(0.0)` answered a number
/// with no type, which the seven operators that can tell an integer from a boolean would each
/// have read differently the moment the stack gained types. The integer is chosen over the real
/// because §7.3.3 makes an integer usable "[w]herever a real number is expected" while the
/// reverse is an error, so it is the one of the two that is an operand everywhere; and over the
/// boolean because a `false` here would silently satisfy `if` and `not` — the two operators that
/// decide what the *rest* of the program does — where an integer only feeds the arithmetic.
///
/// The quorra team's device-side evaluator chose integer `0` too and raises a report at upload
/// (`doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 3). This side does not report, and the reason is
/// not that it would be unwelcome: they can count the underflows statically because they refuse a
/// `copy`, `index` or `roll` whose count is not a constant, and this evaluator admits those, so
/// the depth is not a static quantity here. A report per evaluation would be one per device pixel
/// of a shading (ADR 0339).
const EMPTY_STACK: Value = Value::Integer(0);

/// Bounds the operand stack against a program that only pushes.
const MAX_STACK: usize = 1000;

/// `round`'s nearest integer, with a value halfway between two of them taken to the greater.
///
/// ISO 32000-2 §B.2 (informative) states the operator and, in stating it, states the whole of
/// the ambiguity:
///
/// > Round num 1 to nearest integer
///
/// A value exactly halfway between two integers is nearest to both, and neither §B.2 nor
/// §7.10.5.2 chooses between them; §7.10.5.2 defers that to a document this project does not
/// hold (see [`apply_operator`]). **So the tie is a documented choice, and the choice is the
/// greater of the two**: `-1.5` rounds to `-1`, `2.5` to `3`. Two things recommend it over the
/// alternatives rather than one. It is PostScript's own convention, which is what the deferral
/// points at even when it cannot be quoted. And it is the only one of the three candidates that
/// is a *function of the value* rather than of how the value is written down — half away from
/// zero puts a discontinuity at the origin, where `-1.5` and `1.5` round in opposite directions
/// by different rules, and a type 4 program's inputs cross zero as a matter of course
/// (§7.10.5.3's own example has a `/Domain` of `[-1.0 1.0 -1.0 1.0]`).
///
/// **This was `f32::round` until the five-hundred-and-thirty-fourth session**, which rounds half
/// away from zero, so every negative tie went the wrong way — `-6.5` to `-7` where the greater is
/// `-6`. The quorra team found it reading this file to build a device-side evaluator and reported
/// it in `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` section 6; ADR 0369 is this side's. Their
/// observation that WGSL's `round` is half to *even* belongs beside this one, because it agrees
/// with the greater at `-6.5` and disagrees at `2.5`: a generated shader is not this function.
///
/// Written as a tie test against the floor rather than as `(value + 0.5).floor()`, which is the
/// idiom that suggests itself and is wrong: adding a half to a value whose exponent is large
/// enough rounds *before* the floor sees it, so an exactly representable integer comes back as
/// its successor.
fn round_to_greater(value: f32) -> f32 {
    let below = value.floor();
    // Exact for every finite value: where the difference could round, `below` equals `value`
    // and the difference is zero. A non-finite value falls through to `f32::round`, which
    // returns it unchanged.
    #[expect(
        clippy::float_cmp,
        reason = "a tie is an exact half and nothing else; the margin clippy suggests here is \
                  the defect this function was written to remove from `eq`"
    )]
    if value - below == 0.5 {
        below + 1.0
    } else {
        value.round()
    }
}

/// Narrows a value to the integer PostScript's integer operators are defined on.
///
/// Saturating rather than wrapping: a value outside the range is already meaningless as a
/// colour, and wrapping would turn it into a plausible-looking wrong one.
fn to_integer(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an `as` cast from f32 to i32 saturates at the bounds, which is intended"
    )]
    {
        value.trunc() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::{Function, Value, compile_postscript, evaluate_postscript};

    /// Compiles and runs a type 4 program, which needs no document to exist, and reads what it
    /// left behind the way §7.10.5.3 does — as numbers.
    fn calculator(source: &str, inputs: &[f32]) -> Vec<f32> {
        typed(source, inputs)
            .iter()
            .map(|value| value.number())
            .collect()
    }

    /// [`calculator`] keeping the types, for the operators whose answer is one.
    fn typed(source: &str, inputs: &[f32]) -> Vec<Value> {
        let program = compile_postscript(source.as_bytes()).expect("compiles");
        // §7.10.5.3: the inputs are the initial operand stack, and they are real numbers.
        let mut stack: Vec<Value> = inputs.iter().copied().map(Value::Real).collect();
        evaluate_postscript(&program, &mut stack);
        stack
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
        // §B.5 takes j modulo n, so a roll further than the window is the same roll.
        assert_eq!(calculator("{ 1 2 3 3 4 roll }", &[]), vec![3.0, 1.0, 2.0]);
        assert_eq!(calculator("{ 1 2 3 3 0 roll }", &[]), vec![1.0, 2.0, 3.0]);
        // `copy` of nothing is nothing, which is the boundary the count is guarded at.
        assert_eq!(calculator("{ 1 2 0 copy }", &[]), vec![1.0, 2.0]);
    }

    /// §B.2's four ways of removing a fraction, side by side, on a negative tie.
    ///
    /// The four disagree only there, which is why one of them was wrong for the tree's whole
    /// life and no test saw it. `round`'s tie goes to the greater of the two integers
    /// ([`super::round_to_greater`]); `floor` goes down, `ceiling` up, and `truncate` and `cvi`
    /// toward zero.
    #[test]
    fn the_four_operators_that_drop_a_fraction_disagree_only_on_a_tie() {
        assert_eq!(calculator("{ -1.5 floor }", &[]), vec![-2.0]);
        assert_eq!(calculator("{ -1.5 ceiling }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ -1.5 truncate }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ -1.5 cvi }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ -1.5 round }", &[]), vec![-1.0]);
        // Away from a tie all five agree with arithmetic and with each other where they can.
        assert_eq!(calculator("{ -1.4 round }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ -1.6 round }", &[]), vec![-2.0]);
    }

    /// `round`'s tie goes to the greater integer, in both half-planes and at zero.
    ///
    /// Until the five-hundred-and-thirty-fourth session this was `f32::round`, which is half
    /// away from zero, so every value in the first row answered one lower. ADR 0369, and
    /// `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` section 6, which found it.
    #[test]
    fn round_takes_a_tie_to_the_greater_integer() {
        assert_eq!(calculator("{ -6.5 round }", &[]), vec![-6.0]);
        assert_eq!(calculator("{ -1.5 round }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ -0.5 round }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 0.5 round }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 1.5 round }", &[]), vec![2.0]);
        // Half to even — which is what a WGSL `round` does, and therefore what a device-side
        // evaluator of the same program would answer — agrees at -6.5 and parts here.
        assert_eq!(calculator("{ 2.5 round }", &[]), vec![3.0]);
        // A value large enough that no half exists between two of its neighbours comes back
        // unchanged. `(value + 0.5).floor()` answers 8388610 for the second of these.
        assert_eq!(calculator("{ 8388608 round }", &[]), vec![8_388_608.0]);
        assert_eq!(calculator("{ 8388609 round }", &[]), vec![8_388_609.0]);
    }

    /// §B.3's `eq` is a relation, not a proximity: it holds for equal values and nothing else.
    ///
    /// The `f32::EPSILON` tolerance that stood here until the five-hundred-and-thirty-fourth
    /// session is what the first two of these measure — it made every value under 1.2e-7 equal
    /// to zero and to every other, which is where a type 4 program tests a boundary.
    #[test]
    fn equality_is_exact_rather_than_approximate() {
        assert_eq!(calculator("{ 0 1e-8 eq }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 0 1e-8 ne }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 0.1 0.1 eq }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 1 1.0 eq }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 1 2 eq }", &[]), vec![0.0]);
        // Each is the other's complement for every pair, which the tolerant pair also was and
        // which is worth pinning: two thresholds are two chances to disagree.
        for operands in ["0 1e-8", "3 3", "-1 1", "1e-30 -1e-30", "1e20 1e20"] {
            let equal = calculator(&format!("{{ {operands} eq }}"), &[]);
            let not_equal = calculator(&format!("{{ {operands} ne }}"), &[]);
            assert_eq!(
                equal.first().copied().map(|value| 1.0 - value),
                not_equal.first().copied(),
                "eq and ne disagree on {operands}"
            );
        }
    }

    /// §B.2's `atan` answers over the whole circle, and takes its quadrant from two operands.
    ///
    /// A `num den atan` implemented as the arc tangent of one quotient loses the quadrant, so
    /// the second and fourth of these would answer 45 and 225 the wrong way round.
    #[test]
    fn atan_answers_in_degrees_over_the_whole_circle() {
        assert_eq!(calculator("{ 0 1 atan }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 1 0 atan }", &[]), vec![90.0]);
        assert_eq!(calculator("{ 0 -1 atan }", &[]), vec![180.0]);
        assert_eq!(calculator("{ -1 0 atan }", &[]), vec![270.0]);
        assert_eq!(calculator("{ 1 1 atan }", &[]), vec![45.0]);
        assert_eq!(calculator("{ -1 -1 atan }", &[]), vec![225.0]);
        // Nothing leaves the circle, over a sweep of both operands including the origin, which
        // has no arc tangent and must still answer a number.
        for num in -8_i32..=8 {
            for den in -8_i32..=8 {
                let out = calculator(&format!("{{ {num} {den} atan }}"), &[]);
                let angle = out.first().copied().expect("one value");
                assert!(
                    (0.0..=360.0).contains(&angle),
                    "{num} {den} atan left the circle at {angle}"
                );
            }
        }
    }

    /// §B.2's `idiv` and `mod` are the truncating pair, so a remainder follows its dividend.
    #[test]
    fn integer_division_truncates_toward_zero_and_its_remainder_follows_the_dividend() {
        assert_eq!(calculator("{ -7 2 idiv }", &[]), vec![-3.0]);
        assert_eq!(calculator("{ 7 -2 idiv }", &[]), vec![-3.0]);
        assert_eq!(calculator("{ -7 2 mod }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ 7 -2 mod }", &[]), vec![1.0]);
        // The pair has to reconstruct the dividend, which is what makes it a pair.
        assert_eq!(
            calculator("{ -7 2 idiv 2 mul -7 2 mod add }", &[]),
            vec![-7.0]
        );
    }

    /// §B.3's `and`, `or` and `xor` agree in *arithmetic* whichever operator was meant.
    ///
    /// The clause types them `bool | int` in and `bool | int` out, so each is two operators
    /// sharing a name — and over `{0, 1}` the bitwise operation computes what the logical one
    /// computes, which is why this arm was right before the stack had types and is still right
    /// now. What the types changed is which *type* comes back, and
    /// [`the_bitwise_and_logical_operators_differ_in_the_type_of_their_answer`] is where that
    /// shows.
    #[test]
    fn the_boolean_and_bitwise_operators_are_the_same_operator_on_zero_and_one() {
        assert_eq!(calculator("{ true true and }", &[]), vec![1.0]);
        assert_eq!(calculator("{ true false and }", &[]), vec![0.0]);
        assert_eq!(calculator("{ true false or }", &[]), vec![1.0]);
        assert_eq!(calculator("{ false false or }", &[]), vec![0.0]);
        assert_eq!(calculator("{ true true xor }", &[]), vec![0.0]);
        assert_eq!(calculator("{ true false xor }", &[]), vec![1.0]);
        // The same three arms on integers, where the answer is the bit pattern's.
        assert_eq!(calculator("{ 6 3 and }", &[]), vec![2.0]);
        assert_eq!(calculator("{ 6 3 or }", &[]), vec![7.0]);
        assert_eq!(calculator("{ 6 3 xor }", &[]), vec![5.0]);
    }

    /// `not` is whichever of §B.3's two operators the operand's type selects.
    ///
    /// The clause makes `not` two operators sharing a name — `bool 1 | int 1 … bool 2 | int 2` —
    /// and unlike `and`, `or` and `xor` the two disagree on `{0, 1}`: the one's complement of the
    /// integer `1` is `-2` and of `63` is `-64`, where the logical operator answers false and
    /// false. **Until the five-hundred-and-thirty-sixth session this evaluator could implement
    /// only one of them**, because `Instruction::Push` carried an `f32` and a literal `63` and a
    /// `true` were the same value by the time it ran; the test that stood here pinned `63 not`
    /// at `0` and said so. It is `-64` now, and nothing had to be inferred to get there: §7.3.2
    /// and §7.3.3 give the token `63` its type in the file itself.
    ///
    /// A real reaching `not` is §7.3.3's error — a real where an integer is expected — and
    /// [`Value`]'s policy truncates rather than refusing, which is the last line.
    #[test]
    fn not_is_the_operator_its_operands_type_selects() {
        assert_eq!(typed("{ true not }", &[]), vec![Value::Boolean(false)]);
        assert_eq!(typed("{ false not }", &[]), vec![Value::Boolean(true)]);
        assert_eq!(typed("{ 63 not }", &[]), vec![Value::Integer(-64)]);
        assert_eq!(typed("{ 1 not }", &[]), vec![Value::Integer(-2)]);
        assert_eq!(typed("{ 0 not }", &[]), vec![Value::Integer(-1)]);
        assert_eq!(typed("{ 1.5 not }", &[]), vec![Value::Integer(-2)]);
    }

    /// §B.3 types `eq` and `ne` `any 1 any 2`, so a boolean is an operand and is never a number.
    ///
    /// **This is the defect the typed stack was built for**, and the first line is the whole of
    /// it: with a boolean stored as `1.0`, `true 1 eq` answered *true*. The quorra team found it
    /// in their own device-side evaluator by running this tree's corpus against it and reported
    /// that ours had the same shape (`doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 5).
    ///
    /// The other half of `any 1 any 2` is that two *numbers* are equal when they stand for the
    /// same number, whichever type each was written in — §7.3.3 says an integer may be used
    /// wherever a real is expected, so `1` and `1.0` are one number written two ways.
    #[test]
    fn a_boolean_is_never_equal_to_a_number_and_a_number_never_to_a_boolean() {
        assert_eq!(calculator("{ true 1 eq }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 1 true eq }", &[]), vec![0.0]);
        assert_eq!(calculator("{ true 1 ne }", &[]), vec![1.0]);
        assert_eq!(calculator("{ false 0 eq }", &[]), vec![0.0]);
        assert_eq!(calculator("{ false 0 ne }", &[]), vec![1.0]);
        // Two booleans compare as booleans, which is the operator's other half.
        assert_eq!(calculator("{ true true eq }", &[]), vec![1.0]);
        assert_eq!(calculator("{ true false eq }", &[]), vec![0.0]);
        // And two numbers compare by value across their own two types.
        assert_eq!(calculator("{ 1 1.0 eq }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 0 0.0 eq }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 2 1.0 eq }", &[]), vec![0.0]);
    }

    /// The bitwise and logical operators answer in the type of what they were given.
    ///
    /// §B.3's result column is `bool 3 | int 3` and the operands decide which. The arithmetic is
    /// the same either way over `{0, 1}` — that is the test above — so the difference is only
    /// visible through an operator that reads the type, and `not` is the one that does.
    #[test]
    fn the_bitwise_and_logical_operators_differ_in_the_type_of_their_answer() {
        assert_eq!(typed("{ true true and }", &[]), vec![Value::Boolean(true)]);
        assert_eq!(typed("{ 1 1 and }", &[]), vec![Value::Integer(1)]);
        // Which is what the next operator sees: false against the one's complement of 1.
        assert_eq!(calculator("{ true true and not }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 1 1 and not }", &[]), vec![-2.0]);
        // A boolean meeting an integer is the error §7.3.3 names, and [`Value`]'s policy reads
        // the boolean as the 1 or 0 it stands for rather than the integer as a truth value.
        assert_eq!(typed("{ true 6 and }", &[]), vec![Value::Integer(0)]);
        assert_eq!(typed("{ true 6 or }", &[]), vec![Value::Integer(7)]);
    }

    /// Arithmetic carries its operands' type, and above 2²⁴ that is the difference between an
    /// exact answer and a rounded one.
    ///
    /// §B.2 types `add` as `num 1 num 2 … sum` and does not say what a sum is. Two integers make
    /// an integer here, which is what keeps `not` and the bitwise operators meaning one thing on
    /// a literal and the same thing on a value computed from two — and, as a second consequence
    /// nobody chose it for, it is exact where `f32` is not: `16777216` is the last integer with
    /// no `f32` neighbour below it, so the second line is the arithmetic this evaluator did
    /// before it had types.
    #[test]
    fn integer_arithmetic_stays_integer_and_is_exact_where_a_float_is_not() {
        assert_eq!(typed("{ 2 3 add }", &[]), vec![Value::Integer(5)]);
        assert_eq!(typed("{ 2 3.0 add }", &[]), vec![Value::Real(5.0)]);
        assert_eq!(
            calculator("{ 16777216 1 add 1 add }", &[]),
            vec![16_777_218.0]
        );
        assert_eq!(
            calculator("{ 16777216.0 1 add 1 add }", &[]),
            vec![16_777_216.0]
        );
        // §B.2 gives `div` a plain quotient where `idiv` directly below it answers one "as an
        // integer", so a quotient is a real even when it divides exactly.
        assert_eq!(typed("{ 6 3 div }", &[]), vec![Value::Real(2.0)]);
        assert_eq!(typed("{ 6 3 idiv }", &[]), vec![Value::Integer(2)]);
        // `truncate` keeps its operand's type (`num 1 … num 2`) where `cvi` converts
        // (`num … int`), which is the pair Annex B distinguishes and an untyped stack could not.
        assert_eq!(typed("{ 1.5 truncate }", &[]), vec![Value::Real(1.0)]);
        assert_eq!(typed("{ 1.5 cvi }", &[]), vec![Value::Integer(1)]);
        assert_eq!(typed("{ 1 cvr }", &[]), vec![Value::Real(1.0)]);
    }

    /// The four ordering operators take a boolean as the 1 or 0 it stands for, deliberately.
    ///
    /// §B.3 types them `num 1 num 2`, so unlike `eq` they do not admit a boolean at all and
    /// PostScript refuses one. The subset has no value meaning *error*, so [`Value`]'s policy
    /// converts instead of refusing — and this test is the contract answer quorra asked for in
    /// `doc/QUORRA_FUNCTION_PAINT_BUILT.md` section 3, pinned so that the two evaluators cannot
    /// drift apart on it. `doc/QUORRA_FEEDBACK.md` section 26 carries the ground.
    #[test]
    fn an_ordering_operator_reads_a_boolean_as_a_number_rather_than_refusing_it() {
        assert_eq!(calculator("{ true 0 gt }", &[]), vec![1.0]);
        assert_eq!(calculator("{ false 1 lt }", &[]), vec![1.0]);
        assert_eq!(calculator("{ true true ge }", &[]), vec![1.0]);
        assert_eq!(calculator("{ false true le }", &[]), vec![1.0]);
        // The conforming operands are untouched by any of it.
        assert_eq!(calculator("{ 2 1 gt }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 1 2 gt }", &[]), vec![0.0]);
    }

    /// A pop from an empty operand stack is the integer zero, and `not` is what says so.
    ///
    /// [`EMPTY_STACK`] has the argument. The observable half is that it is not a *boolean*:
    /// a `false` there would answer `true` under `not` and would silently satisfy `if`.
    #[test]
    fn an_empty_operand_stack_reads_as_the_integer_zero() {
        assert_eq!(typed("{ not }", &[]), vec![Value::Integer(-1)]);
        assert_eq!(typed("{ add }", &[]), vec![Value::Integer(0)]);
        assert_eq!(typed("{ 5 index }", &[]), vec![Value::Integer(0)]);
        // Zero is false, so a program that lost its condition takes the else branch.
        assert_eq!(calculator("{ { 10 } { 20 } ifelse }", &[]), vec![20.0]);
    }

    /// §B.4's condition is a `bool`, and a number in its place is read rather than refused.
    ///
    /// The conforming form is the first two lines. The rest is [`Value`]'s conversion policy in
    /// the one direction §7.3.3 does not sanction, kept because a malformed condition that skips
    /// a branch is a page with something missing from it, where the numeric reading is the one
    /// this evaluator has always taken and the one a producer meant.
    #[test]
    fn a_conditions_number_is_true_exactly_when_it_is_not_zero() {
        assert_eq!(calculator("{ true { 10 } { 20 } ifelse }", &[]), vec![10.0]);
        assert_eq!(
            calculator("{ false { 10 } { 20 } ifelse }", &[]),
            vec![20.0]
        );
        assert_eq!(calculator("{ 1 { 10 } { 20 } ifelse }", &[]), vec![10.0]);
        assert_eq!(calculator("{ 0 { 10 } { 20 } ifelse }", &[]), vec![20.0]);
        assert_eq!(calculator("{ -0.5 { 10 } { 20 } ifelse }", &[]), vec![10.0]);
    }

    /// The integer operators truncate a real operand rather than refusing it.
    ///
    /// §B.2 types `idiv` and `mod` on integers and §B.3 types `bitshift`'s first operand as one,
    /// so a real is §7.3.3's "real number … present when an integer is expected". This is what
    /// the arms did when nothing could tell a real from an integer, and it stays the answer.
    #[test]
    fn an_integer_operator_truncates_a_real_operand() {
        assert_eq!(typed("{ 7.5 2 idiv }", &[]), vec![Value::Integer(3)]);
        assert_eq!(typed("{ 7.5 2 mod }", &[]), vec![Value::Integer(1)]);
        assert_eq!(typed("{ 1.5 3 bitshift }", &[]), vec![Value::Integer(8)]);
        assert_eq!(typed("{ 6.9 3.9 and }", &[]), vec![Value::Integer(2)]);
    }

    /// §B.3's `bitshift`, and the one place its answer is this project's choice rather than
    /// the standard's.
    ///
    /// The direction is stated — positive is left — and the width of the integer is not, which
    /// only shows on a right shift of a negative value. The sign-preserving shift is chosen
    /// because it is the answer that does not depend on a width nobody stated; the last two
    /// lines are what that choice looks like.
    #[test]
    fn a_bit_shift_moves_left_for_a_positive_count_and_keeps_its_sign_going_right() {
        assert_eq!(calculator("{ 1 3 bitshift }", &[]), vec![8.0]);
        assert_eq!(calculator("{ 8 -3 bitshift }", &[]), vec![1.0]);
        assert_eq!(calculator("{ 5 0 bitshift }", &[]), vec![5.0]);
        assert_eq!(calculator("{ -8 1 bitshift }", &[]), vec![-16.0]);
        assert_eq!(calculator("{ -8 -1 bitshift }", &[]), vec![-4.0]);
        assert_eq!(calculator("{ -1 -1 bitshift }", &[]), vec![-1.0]);
        // Wider than the register the value is held in, which is where the sign-preserving
        // reading has to keep answering or it was a reading about the register after all: a
        // negative value shifted right past its own width is the sign repeated, and a
        // non-negative one is zero.
        assert_eq!(calculator("{ -8 -40 bitshift }", &[]), vec![-1.0]);
        assert_eq!(calculator("{ 8 -40 bitshift }", &[]), vec![0.0]);
        assert_eq!(calculator("{ 8 40 bitshift }", &[]), vec![0.0]);
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

    /// §7.10.5.1 admits comments, and §7.2.4 ends one at the end of its line.
    ///
    /// The words inside a comment are not tokens of the program, whatever they spell. Until
    /// the five-hundred-and-twenty-sixth session the compiler split the whole stream on white
    /// space first and then skipped a single token after each PERCENT SIGN, which made the
    /// first of these refuse the program and the second and third compile a program the file
    /// does not contain.
    #[test]
    fn a_comment_runs_to_the_end_of_its_line() {
        // The project owner's own file, whose first line is exactly this shape: the word
        // after the sign is not an operator, so the old rule reached `Math` and refused.
        assert_eq!(
            calculator(
                "{\n% BBP Math for Pi (leaves 3.141 on stack)\n7 3 sub\n}",
                &[]
            ),
            vec![4.0]
        );
        // The silent half, and the reason this is a defect rather than an inconvenience:
        // every word after the first was compiled, so the comment added two instructions.
        assert_eq!(
            calculator("{\n% add 100 mul here\n7 3 sub\n}", &[]),
            vec![4.0]
        );
        // A comment need not start a line, and the token before it must survive.
        assert_eq!(calculator("{ 7 3 sub % subtract\n}", &[]), vec![4.0]);
        // A CARRIAGE RETURN ends a line as much as a LINE FEED does (§7.2.3), and this is the
        // shape of the one commented type 4 program in 67 461 documents: a `Separation` tint
        // transform naming its CMYK components, with CARRIAGE RETURN endings and not one LINE
        // FEED anywhere (`SafeDocs` cc-main-2021-31 5097152.pdf object 19, ADR 0361). A cut
        // that looked only for LINE FEED would read the whole program as one comment.
        assert_eq!(
            calculator("{\r0 %c\r0 %m\r0 %y\r3 index %k\r5 -1 roll pop\r}", &[0.25]),
            vec![0.0, 0.0, 0.0, 0.25]
        );
        // A comment separates the tokens either side of it rather than joining them: with
        // the comment removed and nothing left in its place, this would read `4dup`.
        assert_eq!(calculator("{ 4% squared\ndup mul }", &[]), vec![16.0]);
        // A brace inside a comment is text, not structure.
        assert_eq!(
            calculator("{ 1 % { two } if\n7 3 sub }", &[]),
            vec![1.0, 4.0]
        );
    }

    /// The half of the same defect that reported nothing, which is the expensive half.
    ///
    /// Every comment here is one whose words are *all* valid tokens, which is what a comment
    /// looks like when it quotes the arithmetic below it. Skipping one token after the PERCENT
    /// SIGN then compiled the rest into the program: no error, no report, and a function that
    /// computes something the file does not contain — in a shading or a tint transform, a
    /// plausible picture in the wrong colours.
    #[test]
    fn a_comments_words_are_not_instructions() {
        // Old rule: `mul` compiled, so the result was 4 × nothing = 0.
        assert_eq!(calculator("{ 7 3 sub % 100 mul\n}", &[]), vec![4.0]);
        // Old rule: `1 add` compiled, so the result was one too many.
        assert_eq!(calculator("{ 7 3 sub % add 1 add\n}", &[]), vec![4.0]);
        // Old rule: `2 mul` compiled ahead of the line it describes.
        assert_eq!(calculator("{ % dup 2 mul\n7 3 sub }", &[]), vec![4.0]);
        // A brace inside such a comment took the block structure with it.
        assert_eq!(calculator("{ 1 % { 2 } if\n7 3 sub }", &[]), vec![1.0, 4.0]);
    }

    /// A comment cannot change what a program computes, whatever it says.
    ///
    /// The unit-level half of the fixture pair in `test-scenes`: two programs written out
    /// separately, differing only in comments, compiling to the same instructions.
    #[test]
    fn commenting_a_program_does_not_change_it() {
        let commented = "{\n\
             % dup mul squares the input\n\
             dup mul\n\
             % 1 add\n\
             1 add\n\
             }";
        let plain = "{\n\
             dup mul\n\
             1 add\n\
             }";
        assert_eq!(
            compile_postscript(commented.as_bytes()).expect("compiles"),
            compile_postscript(plain.as_bytes()).expect("compiles")
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

    /// The one operator that can produce a non-number, and the clause that stops it escaping.
    ///
    /// §B.2's `exp` raises a base to a power, and a negative base with a fractional exponent has
    /// no real answer. §7.10.5.3 makes `/Range` required for a type 4 function — "The Domain and
    /// Range entries shall both be required" — and the range clamp is where a `NaN` stops, so no
    /// caller of [`Function::eval`] can be handed one. Checked through a real function rather
    /// than through the calculator, because the calculator is the half of the path without the
    /// clause in it.
    #[test]
    fn a_program_that_computes_no_real_number_still_answers_within_its_range() {
        let program = "{ -8 0.5 exp }";
        let source = format!(
            "%PDF-1.7\n1 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length {} >>\n\
             stream\n{program}\nendstream\nendobj\ntrailer\n<< /Root 1 0 R >>\n",
            program.len().saturating_add(1)
        );
        let document = pdf_syntax::Document::open(source.into_bytes()).expect("opens");
        let object = document.get(pdf_syntax::ObjectId {
            number: 1,
            generation: 0,
        });
        let function = Function::parse(&document, &object).expect("parses");

        let out = function.eval(&[0.5]);
        assert_eq!(out.len(), 1);
        for value in out {
            assert!(
                value.is_finite(),
                "a non-number reached a caller as {value}"
            );
            assert!((0.0..=1.0).contains(&value));
        }
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
